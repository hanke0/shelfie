import { describe, expect, it } from "vitest";
import { parseMobi } from "./mobi-metadata";

/** Minimal uncompressed MOBI-like PDB with EXTH metadata. */
function buildTinyMobi(opts: {
  title: string;
  author: string;
  publisher?: string;
  text: string;
}): ArrayBuffer {
  const textBytes = new TextEncoder().encode(opts.text);
  const fullName = new TextEncoder().encode(opts.title);

  // Build EXTH
  const exthRecords: Uint8Array[] = [];
  const pushExth = (type: number, data: Uint8Array) => {
    const rec = new Uint8Array(8 + data.length);
    const v = new DataView(rec.buffer);
    v.setUint32(0, type);
    v.setUint32(4, rec.length);
    rec.set(data, 8);
    exthRecords.push(rec);
  };
  pushExth(100, new TextEncoder().encode(opts.author));
  if (opts.publisher) pushExth(101, new TextEncoder().encode(opts.publisher));
  pushExth(503, new TextEncoder().encode(opts.title));

  const exthBody = concat(exthRecords);
  const exth = new Uint8Array(12 + exthBody.length);
  const exthView = new DataView(exth.buffer);
  exth.set(new TextEncoder().encode("EXTH"), 0);
  exthView.setUint32(4, exth.length);
  exthView.setUint32(8, exthRecords.length);
  exth.set(exthBody, 12);

  // MOBI header length: enough to cover through EXTH flags (0x80) and pad to place full name after header+EXTH
  const mobiHeaderLength = 0xe8; // common size; fields up through 0x80+
  const fullNameOffset = 16 + mobiHeaderLength + exth.length;
  // pad EXTH to 4-byte boundary already ok

  const record0 = new Uint8Array(fullNameOffset + fullName.length + 2);
  const r0 = new DataView(record0.buffer);

  // PalmDOC
  r0.setUint16(0, 1); // no compression
  r0.setUint16(8, 1); // 1 text record
  r0.setUint16(10, 4096);
  r0.setUint16(12, 0); // no encryption

  // MOBI
  record0.set(new TextEncoder().encode("MOBI"), 16);
  r0.setUint32(20, mobiHeaderLength);
  r0.setUint32(0x1c, 65001); // utf-8
  r0.setUint32(0x54, fullNameOffset);
  r0.setUint32(0x58, fullName.length);
  r0.setUint32(0x5c, 0x0804); // zh-CN-ish
  r0.setUint32(0x6c, 2); // first image index (none used)
  r0.setUint32(0x80, 0x40); // EXTH present
  record0.set(exth, 16 + mobiHeaderLength);
  record0.set(fullName, fullNameOffset);

  // PDB header + 2 records (record0 + text)
  const record0Start = 78 + 2 * 8;
  const textStart = record0Start + record0.length;
  const file = new Uint8Array(textStart + textBytes.length);
  const fv = new DataView(file.buffer);
  const name = new TextEncoder().encode(opts.title.slice(0, 31));
  file.set(name, 0);
  fv.setUint16(76, 2);
  fv.setUint32(78, record0Start);
  fv.setUint32(86, textStart);
  file.set(record0, record0Start);
  file.set(textBytes, textStart);
  return file.buffer;
}

function concat(parts: Uint8Array[]): Uint8Array {
  const total = parts.reduce((n, p) => n + p.length, 0);
  const out = new Uint8Array(total);
  let o = 0;
  for (const p of parts) {
    out.set(p, o);
    o += p.length;
  }
  return out;
}

describe("parseMobi", () => {
  it("reads EXTH title/author/publisher and front text", () => {
    const buf = buildTinyMobi({
      title: "三体",
      author: "刘慈欣",
      publisher: "重庆出版社",
      text: "<html><body>周泽栋 译</body></html>",
    });
    const parsed = parseMobi(buf, "fallback");
    expect(parsed.metadata.title).toBe("三体");
    expect(parsed.metadata.author).toBe("刘慈欣");
    expect(parsed.metadata.publisher).toBe("重庆出版社");
    expect(parsed.metadata.language).toBe("zh");
    expect(parsed.frontText).toContain("周泽栋 译");
  });
});
