import type JSZip from "jszip";
import type { BookMetadata } from "@/api/generated/models";
import { extractFrontMatter, fillMissingMetadata, htmlToPlainText } from "@/lib/front-matter";
import { parseMobi } from "@/lib/mobi-metadata";
import {
  coverMimeFromPath,
  joinOpfPath,
  normalizeIsbn,
  normalizePublishDate,
  opfDirname,
  opfMetaToPartial,
  parseContainerOpfPath,
  parseOpf,
} from "@/lib/opf-metadata";

export type LocalExtractedBook = {
  metadata: Partial<BookMetadata>;
  cover?: File;
};

const FRONT_PAGE_LIMIT = 5;

export async function extractBookMetadataLocal(file: File): Promise<LocalExtractedBook> {
  const ext = file.name.split(".").pop()?.toLowerCase() ?? "";
  switch (ext) {
    case "epub":
      return extractEpub(file);
    case "pdf":
      return extractPdf(file);
    case "mobi":
      return extractMobi(file);
    default:
      return { metadata: { title: fallbackTitle(file) } };
  }
}

function fallbackTitle(file: File): string {
  return file.name.replace(/\.[^.]+$/i, "");
}

async function extractEpub(file: File): Promise<LocalExtractedBook> {
  const { default: JSZipLib } = await import("jszip");
  const zip = await JSZipLib.loadAsync(await file.arrayBuffer());
  const container = await readZipText(zip, "META-INF/container.xml");
  const opfPath = parseContainerOpfPath(container);
  const opfDir = opfDirname(opfPath);
  const opf = await readZipText(zip, opfPath);
  const parsed = parseOpf(opf);

  let cover: File | undefined;
  if (parsed.coverHref) {
    const href = joinOpfPath(opfDir, parsed.coverHref);
    const entry = zip.file(href);
    if (entry) {
      const bytes = await entry.async("arraybuffer");
      const mime = coverMimeFromPath(href);
      const ext = mime.includes("png") ? "png" : "jpg";
      cover = new File([bytes], `${fallbackTitle(file)}-cover.${ext}`, { type: mime });
    }
  }

  let metadata = opfMetaToPartial(parsed.meta, fallbackTitle(file));
  if (needsFrontMatter(metadata)) {
    const frontText = await readEpubFrontText(zip, opfDir, parsed.spineHrefs);
    metadata = fillMissingMetadata(metadata, extractFrontMatter(frontText));
  }

  return { metadata, cover };
}

async function extractPdf(file: File): Promise<LocalExtractedBook> {
  const { getDocument, GlobalWorkerOptions } = await import("pdfjs-dist");
  GlobalWorkerOptions.workerSrc = (
    await import("pdfjs-dist/build/pdf.worker.min.mjs?url")
  ).default;

  const data = new Uint8Array(await file.arrayBuffer());
  const doc = await getDocument({ data }).promise;
  const meta = await doc.getMetadata();
  const info = meta.info as Record<string, string | undefined>;
  const pageCount = doc.numPages;

  let metadata: Partial<BookMetadata> = {
    title: info.Title?.trim() || fallbackTitle(file),
    author: info.Author?.trim() ?? "",
    publisher: info.Producer?.trim() || info.Creator?.trim() || "",
    language: info.Language?.trim() ?? "",
    isbn: normalizeIsbn(info.ISBN ?? ""),
    publish_date: normalizePublishDate(info.CreationDate ?? info.ModDate ?? ""),
    page_count: Number.isFinite(pageCount) ? pageCount : undefined,
    notes: info.Subject?.trim() ?? "",
  };

  if (needsFrontMatter(metadata)) {
    const frontText = await readPdfFrontText(doc, FRONT_PAGE_LIMIT);
    metadata = fillMissingMetadata(metadata, extractFrontMatter(frontText));
  }

  return { metadata };
}

async function extractMobi(file: File): Promise<LocalExtractedBook> {
  const buffer = await file.arrayBuffer();
  const parsed = parseMobi(buffer, fallbackTitle(file));
  let metadata = parsed.metadata;

  if (needsFrontMatter(metadata) && parsed.frontText) {
    metadata = fillMissingMetadata(
      metadata,
      extractFrontMatter(htmlToPlainText(parsed.frontText)),
    );
  }

  let cover: File | undefined;
  if (parsed.coverBytes && parsed.coverMime) {
    const ext = parsed.coverMime.includes("png") ? "png" : "jpg";
    const bytes = new Uint8Array(parsed.coverBytes);
    cover = new File([bytes], `${fallbackTitle(file)}-cover.${ext}`, {
      type: parsed.coverMime,
    });
  }

  return { metadata, cover };
}

function needsFrontMatter(meta: Partial<BookMetadata>): boolean {
  return !(meta.translator?.trim()) || !(meta.author?.trim()) || !(meta.publisher?.trim()) || !(meta.isbn?.trim());
}

async function readEpubFrontText(
  zip: JSZip,
  opfDir: string,
  spineHrefs: string[],
): Promise<string> {
  const parts: string[] = [];
  let total = 0;
  for (const href of spineHrefs.slice(0, FRONT_PAGE_LIMIT)) {
    const path = joinOpfPath(opfDir, href);
    const entry = zip.file(path);
    if (!entry) continue;
    try {
      const html = await entry.async("text");
      const text = htmlToPlainText(html);
      parts.push(text);
      total += text.length;
      if (total >= 8_000) break;
    } catch {
      // skip unreadable spine item
    }
  }
  return parts.join("\n").slice(0, 8_000);
}

async function readPdfFrontText(
  doc: { numPages: number; getPage: (n: number) => Promise<{ getTextContent: () => Promise<{ items: unknown[] }> }> },
  pageLimit: number,
): Promise<string> {
  const parts: string[] = [];
  const limit = Math.min(doc.numPages, pageLimit);
  for (let i = 1; i <= limit; i++) {
    const page = await doc.getPage(i);
    const content = await page.getTextContent();
    const text = content.items
      .map((item) => ("str" in (item as object) ? String((item as { str?: string }).str ?? "") : ""))
      .join(" ");
    parts.push(text);
  }
  return parts.join("\n");
}

async function readZipText(zip: JSZip, path: string): Promise<string> {
  const entry = zip.file(path);
  if (!entry) throw new Error(`Missing ${path}`);
  return entry.async("text");
}
