import JSZip from "jszip";
import type { BookMetadata } from "@/api/generated/models";

export type LocalExtractedBook = {
  metadata: Partial<BookMetadata>;
  cover?: File;
};

export async function extractBookMetadataLocal(file: File): Promise<LocalExtractedBook> {
  const ext = file.name.split(".").pop()?.toLowerCase() ?? "";
  switch (ext) {
    case "epub":
      return extractEpub(file);
    case "pdf":
      return extractPdf(file);
    case "mobi":
      return { metadata: { title: fallbackTitle(file) } };
    default:
      return { metadata: { title: fallbackTitle(file) } };
  }
}

function fallbackTitle(file: File): string {
  return file.name.replace(/\.[^.]+$/i, "");
}

async function extractEpub(file: File): Promise<LocalExtractedBook> {
  const zip = await JSZip.loadAsync(await file.arrayBuffer());
  const container = await readZipText(zip, "META-INF/container.xml");
  const opfPath = parseContainerOpfPath(container);
  const opfDir = opfDirname(opfPath);
  const opf = await readZipText(zip, opfPath);
  const parsed = parseOpf(opf);

  let cover: File | undefined;
  if (parsed.coverHref) {
    const href = joinZipPath(opfDir, parsed.coverHref);
    const entry = zip.file(href);
    if (entry) {
      const bytes = await entry.async("uint8array");
      const mime = coverMimeFromPath(href);
      const ext = mime.includes("png") ? "png" : "jpg";
      cover = new File([bytes], `${fallbackTitle(file)}-cover.${ext}`, { type: mime });
    }
  }

  return {
    metadata: {
      title: firstMeta(parsed.meta, "title") || fallbackTitle(file),
      author: joinMeta(parsed.meta, "creator"),
      translator: joinMeta(parsed.meta, "contributor"),
      publisher: firstMeta(parsed.meta, "publisher"),
      language: firstMeta(parsed.meta, "language"),
      isbn: isbnFromMeta(parsed.meta),
      publish_date: normalizePublishDate(firstMeta(parsed.meta, "date")),
      original_title: firstMeta(parsed.meta, "alternative"),
      series:
        firstMeta(parsed.meta, "calibre:series") || firstMeta(parsed.meta, "series"),
      notes: firstMeta(parsed.meta, "description"),
    },
    cover,
  };
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

  return {
    metadata: {
      title: info.Title?.trim() || fallbackTitle(file),
      author: info.Author?.trim() ?? "",
      publisher: info.Producer?.trim() || info.Creator?.trim() || "",
      language: info.Language?.trim() ?? "",
      isbn: normalizeIsbn(info.ISBN ?? ""),
      publish_date: normalizePublishDate(info.CreationDate ?? info.ModDate ?? ""),
      page_count: Number.isFinite(pageCount) ? pageCount : undefined,
      notes: info.Subject?.trim() ?? "",
    },
  };
}

type OpfMeta = Record<string, string[]>;

type ParsedOpf = {
  meta: OpfMeta;
  coverHref?: string;
};

async function readZipText(zip: JSZip, path: string): Promise<string> {
  const entry = zip.file(path);
  if (!entry) throw new Error(`Missing ${path}`);
  return entry.async("text");
}

function parseContainerOpfPath(container: string): string {
  const doc = new DOMParser().parseFromString(container, "application/xml");
  const rootfile = doc.querySelector("rootfile[full-path]");
  const path = rootfile?.getAttribute("full-path");
  if (!path) throw new Error("Invalid EPUB container");
  return path;
}

function parseOpf(opf: string): ParsedOpf {
  const doc = new DOMParser().parseFromString(opf, "application/xml");
  const meta: OpfMeta = {};
  const manifest = new Map<string, string>();
  let coverHref: string | undefined;

  for (const item of doc.querySelectorAll("manifest > item")) {
    const id = item.getAttribute("id");
    const href = item.getAttribute("href");
    if (id && href) manifest.set(id, href);
  }

  for (const el of doc.querySelectorAll("metadata > *")) {
    const local = localName(el);
    if (!local) continue;
    if (local === "meta") {
      const name = el.getAttribute("name") ?? el.getAttribute("property") ?? "";
      const content = el.getAttribute("content") ?? "";
      if ((name === "cover" || name === "cover-image") && content) {
        coverHref = manifest.get(content) ?? coverHref;
      } else if (name && content) {
        pushMeta(meta, name, content);
      }
      continue;
    }
    const text = el.textContent?.trim();
    if (text) pushMeta(meta, local, text);
  }

  return { meta, coverHref };
}

function pushMeta(meta: OpfMeta, key: string, value: string) {
  meta[key] ??= [];
  meta[key].push(value);
}

function firstMeta(meta: OpfMeta, key: string): string {
  return meta[key]?.[0]?.trim() ?? "";
}

function joinMeta(meta: OpfMeta, key: string): string {
  return meta[key]?.join(", ").trim() ?? "";
}

function isbnFromMeta(meta: OpfMeta): string {
  for (const value of meta.identifier ?? []) {
    const normalized = normalizeIsbn(value);
    if (normalized) return normalized;
  }
  return "";
}

function normalizeIsbn(raw: string): string {
  const cleaned = raw.replace(/[^\dXx-]/g, "");
  if (cleaned.replace(/-/g, "").length >= 10) return cleaned;
  if (raw.toLowerCase().includes("isbn")) return raw.trim();
  return "";
}

function normalizePublishDate(raw: string): string {
  const value = raw.trim();
  if (!value) return "";
  if (/^\d{4}-\d{2}/.test(value)) return value.slice(0, 7);
  if (/^\d{4}$/.test(value)) return `${value}-01`;
  const pdfMatch = value.match(/^D:(\d{4})(\d{2})?/);
  if (pdfMatch) {
    const month = pdfMatch[2] ?? "01";
    return `${pdfMatch[1]}-${month}`;
  }
  return "";
}

function localName(el: Element): string {
  return el.localName || el.tagName.split(":").pop() || "";
}

function opfDirname(path: string): string {
  const idx = path.lastIndexOf("/");
  return idx >= 0 ? path.slice(0, idx) : "";
}

function joinZipPath(dir: string, href: string): string {
  return dir ? `${dir}/${href}` : href;
}

function coverMimeFromPath(path: string): string {
  const ext = path.split(".").pop()?.toLowerCase();
  if (ext === "png") return "image/png";
  if (ext === "gif") return "image/gif";
  if (ext === "webp") return "image/webp";
  return "image/jpeg";
}
