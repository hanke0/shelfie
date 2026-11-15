import type { BookMetadata } from "@/api/generated/models";
import { normalizeIsbn, normalizePublishDate } from "@/lib/opf-metadata";

export type ParsedMobi = {
  metadata: Partial<BookMetadata>;
  coverBytes?: Uint8Array;
  coverMime?: string;
  /** Uncompressed front text (HTML/plain) for front-matter fallback. */
  frontText: string;
};

const EXTH = {
  author: 100,
  publisher: 101,
  description: 103,
  isbn: 104,
  publishingDate: 106,
  coverOffset: 201,
  updatedTitle: 503,
  language: 524,
} as const;

/** Roughly the first ~5 pages of body text. */
const FRONT_TEXT_LIMIT = 8_000;

export function parseMobi(buffer: ArrayBuffer, fallbackTitle: string): ParsedMobi {
  const bytes = new Uint8Array(buffer);
  const view = new DataView(buffer);
  if (bytes.length < 78) {
    return { metadata: { title: fallbackTitle }, frontText: "" };
  }

  const numRecords = view.getUint16(76);
  if (numRecords < 1 || bytes.length < 78 + numRecords * 8) {
    return { metadata: { title: fallbackTitle }, frontText: "" };
  }

  const offsets: number[] = [];
  for (let i = 0; i < numRecords; i++) {
    offsets.push(view.getUint32(78 + i * 8));
  }

  const record0 = sliceRecord(bytes, offsets, 0);
  if (record0.length < 16) {
    return { metadata: { title: fallbackTitle }, frontText: "" };
  }

  const r0 = new DataView(record0.buffer, record0.byteOffset, record0.byteLength);
  const compression = r0.getUint16(0);
  const textRecordCount = r0.getUint16(8);
  const encryption = r0.getUint16(12);

  let title = decodeCString(bytes.subarray(0, 32)) || fallbackTitle;
  let codepage = 1252;
  let firstImageIndex = numRecords;
  let language = "";
  const authors: string[] = [];
  let publisher = "";
  let description = "";
  let isbn = "";
  let publishDate = "";
  let coverOffset: number | undefined;

  if (record0.length >= 20 && asciiAt(record0, 16, 4) === "MOBI") {
    // Field offsets are absolute from the start of record 0 (PalmDOC + MOBI).
    const mobiHeaderLength = r0.getUint32(20);
    const mobiEnd = 16 + mobiHeaderLength;
    if (mobiEnd >= 0x20) codepage = r0.getUint32(0x1c);
    if (mobiEnd >= 0x60) language = localeToLanguage(r0.getUint32(0x5c));
    if (mobiEnd >= 0x70) firstImageIndex = r0.getUint32(0x6c);

    if (mobiEnd >= 0x5c) {
      const fullNameOffset = r0.getUint32(0x54);
      const fullNameLength = r0.getUint32(0x58);
      if (fullNameOffset > 0 && fullNameLength > 0 && fullNameOffset + fullNameLength <= record0.length) {
        const fullName = decodeBytes(
          record0.subarray(fullNameOffset, fullNameOffset + fullNameLength),
          codepage,
        );
        if (fullName) title = fullName;
      }
    }

    const exthFlags = mobiEnd >= 0x84 ? r0.getUint32(0x80) : 0;
    if (exthFlags & 0x40) {
      const exthStart = mobiEnd;
      if (exthStart + 12 <= record0.length && asciiAt(record0, exthStart, 4) === "EXTH") {
        const recordCount = r0.getUint32(exthStart + 8);
        let off = exthStart + 12;
        for (let i = 0; i < recordCount && off + 8 <= record0.length; i++) {
          const type = r0.getUint32(off);
          const length = r0.getUint32(off + 4);
          if (length < 8 || off + length > record0.length) break;
          const data = record0.subarray(off + 8, off + length);
          switch (type) {
            case EXTH.author:
              authors.push(decodeBytes(data, codepage));
              break;
            case EXTH.publisher:
              publisher = decodeBytes(data, codepage);
              break;
            case EXTH.description:
              description = decodeBytes(data, codepage);
              break;
            case EXTH.isbn:
              isbn = normalizeIsbn(decodeBytes(data, codepage));
              break;
            case EXTH.publishingDate:
              publishDate = normalizePublishDate(decodeBytes(data, codepage));
              break;
            case EXTH.updatedTitle: {
              const t = decodeBytes(data, codepage);
              if (t) title = t;
              break;
            }
            case EXTH.language:
              language = decodeBytes(data, codepage) || language;
              break;
            case EXTH.coverOffset:
              if (data.length >= 4) {
                coverOffset = new DataView(data.buffer, data.byteOffset, data.byteLength).getUint32(0);
              }
              break;
            default:
              break;
          }
          off += length;
        }
      }
    }
  }

  let coverBytes: Uint8Array | undefined;
  let coverMime: string | undefined;
  if (coverOffset != null && Number.isFinite(coverOffset)) {
    const coverIndex = firstImageIndex + coverOffset;
    if (coverIndex > 0 && coverIndex < numRecords) {
      const image = sliceRecord(bytes, offsets, coverIndex);
      if (image.length > 0) {
        coverBytes = image;
        coverMime = sniffImageMime(image);
      }
    }
  }

  const frontText =
    encryption === 0
      ? extractMobiFrontText(bytes, offsets, compression, textRecordCount, codepage)
      : "";

  return {
    metadata: {
      title: title.trim() || fallbackTitle,
      author: authors.map((a) => a.trim()).filter(Boolean).join(", "),
      publisher: publisher.trim(),
      language: language.trim(),
      isbn,
      publish_date: publishDate,
      notes: description.trim(),
    },
    coverBytes,
    coverMime,
    frontText,
  };
}

function extractMobiFrontText(
  bytes: Uint8Array,
  offsets: number[],
  compression: number,
  textRecordCount: number,
  codepage: number,
): string {
  if (textRecordCount <= 0) return "";
  const parts: string[] = [];
  let total = 0;
  const limit = Math.min(textRecordCount, offsets.length - 1, 40);

  for (let i = 1; i <= limit && total < FRONT_TEXT_LIMIT; i++) {
    const raw = sliceRecord(bytes, offsets, i);
    let decoded: Uint8Array;
    try {
      if (compression === 1) decoded = raw;
      else if (compression === 2) decoded = palmDocDecompress(raw);
      else return parts.join(""); // HUFF/CDIC etc. unsupported
    } catch {
      break;
    }
    const chunk = decodeBytes(decoded, codepage);
    parts.push(chunk);
    total += chunk.length;
  }

  return parts.join("").slice(0, FRONT_TEXT_LIMIT);
}

function palmDocDecompress(data: Uint8Array): Uint8Array {
  const out: number[] = [];
  let i = 0;
  while (i < data.length) {
    const byte = data[i++]!;
    if (byte === 0x00 || (byte >= 0x09 && byte <= 0x7f)) {
      out.push(byte);
    } else if (byte >= 0x01 && byte <= 0x08) {
      for (let n = 0; n < byte && i < data.length; n++) out.push(data[i++]!);
    } else if (byte >= 0x80 && byte <= 0xbf) {
      if (i >= data.length) break;
      const next = data[i++]!;
      const distance = (((byte << 8) | next) >> 3) & 0x07ff;
      const length = (next & 0x07) + 3;
      if (distance === 0 || distance > out.length) break;
      for (let n = 0; n < length; n++) out.push(out[out.length - distance]!);
    } else {
      out.push(0x20, byte ^ 0x80);
    }
  }
  return Uint8Array.from(out);
}

function sliceRecord(bytes: Uint8Array, offsets: number[], index: number): Uint8Array {
  const start = offsets[index]!;
  const end = index + 1 < offsets.length ? offsets[index + 1]! : bytes.length;
  return bytes.subarray(start, Math.min(end, bytes.length));
}

function asciiAt(bytes: Uint8Array, offset: number, length: number): string {
  return String.fromCharCode(...bytes.subarray(offset, offset + length));
}

function decodeCString(bytes: Uint8Array): string {
  let end = bytes.indexOf(0);
  if (end < 0) end = bytes.length;
  return decodeBytes(bytes.subarray(0, end), 1252).trim();
}

function decodeBytes(bytes: Uint8Array, codepage: number): string {
  const encoding = encodingForCodepage(codepage);
  try {
    return new TextDecoder(encoding).decode(bytes).replace(/\0/g, "").trim();
  } catch {
    try {
      return new TextDecoder("utf-8").decode(bytes).replace(/\0/g, "").trim();
    } catch {
      return Array.from(bytes)
        .map((b) => String.fromCharCode(b))
        .join("")
        .replace(/\0/g, "")
        .trim();
    }
  }
}

function encodingForCodepage(codepage: number): string {
  switch (codepage) {
    case 65001:
      return "utf-8";
    case 936:
      return "gbk";
    case 950:
      return "big5";
    case 932:
      return "shift_jis";
    case 1252:
    default:
      return "windows-1252";
  }
}

function localeToLanguage(locale: number): string {
  // MOBI locale low byte matches Windows LANGID primary language.
  const lang = locale & 0xff;
  const map: Record<number, string> = {
    0x01: "ar",
    0x04: "zh",
    0x05: "cs",
    0x06: "da",
    0x07: "de",
    0x08: "el",
    0x09: "en",
    0x0a: "es",
    0x0b: "fi",
    0x0c: "fr",
    0x0e: "hu",
    0x10: "it",
    0x11: "ja",
    0x12: "ko",
    0x13: "nl",
    0x14: "no",
    0x15: "pl",
    0x16: "pt",
    0x19: "ru",
    0x1d: "sv",
    0x1f: "tr",
  };
  return map[lang] ?? "";
}

function sniffImageMime(bytes: Uint8Array): string {
  if (bytes.length >= 3 && bytes[0] === 0xff && bytes[1] === 0xd8 && bytes[2] === 0xff) {
    return "image/jpeg";
  }
  if (
    bytes.length >= 8 &&
    bytes[0] === 0x89 &&
    bytes[1] === 0x50 &&
    bytes[2] === 0x4e &&
    bytes[3] === 0x47
  ) {
    return "image/png";
  }
  if (bytes.length >= 6 && asciiAt(bytes, 0, 3) === "GIF") return "image/gif";
  return "image/jpeg";
}
