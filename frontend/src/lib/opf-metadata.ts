import type { BookMetadata } from "@/api/generated/models";

export type OpfMeta = Record<string, string[]>;

export type ParsedOpf = {
  meta: OpfMeta;
  coverHref?: string;
};

export function parseOpf(opf: string): ParsedOpf {
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

export function opfMetaToPartial(meta: OpfMeta, fallbackTitle = ""): Partial<BookMetadata> {
  return {
    title: firstMeta(meta, "title") || fallbackTitle,
    author: joinMeta(meta, "creator"),
    translator: joinMeta(meta, "contributor"),
    publisher: firstMeta(meta, "publisher"),
    language: firstMeta(meta, "language"),
    isbn: isbnFromMeta(meta),
    publish_date: normalizePublishDate(firstMeta(meta, "date")),
    original_title: firstMeta(meta, "alternative"),
    series: firstMeta(meta, "calibre:series") || firstMeta(meta, "series"),
    notes: firstMeta(meta, "description"),
  };
}

function pushMeta(meta: OpfMeta, key: string, value: string) {
  meta[key] ??= [];
  meta[key].push(value);
}

export function firstMeta(meta: OpfMeta, key: string): string {
  return meta[key]?.[0]?.trim() ?? "";
}

export function joinMeta(meta: OpfMeta, key: string): string {
  return meta[key]?.join(", ").trim() ?? "";
}

export function isbnFromMeta(meta: OpfMeta): string {
  for (const value of meta.identifier ?? []) {
    const normalized = normalizeIsbn(value);
    if (normalized) return normalized;
  }
  return "";
}

export function normalizeIsbn(raw: string): string {
  const cleaned = raw.replace(/[^\dXx-]/g, "");
  if (cleaned.replace(/-/g, "").length >= 10) return cleaned;
  if (raw.toLowerCase().includes("isbn")) return raw.trim();
  return "";
}

export function normalizePublishDate(raw: string): string {
  const value = raw.trim();
  if (!value) return "";
  if (/^\d{4}-\d{2}/.test(value)) return value.slice(0, 7);
  if (/^\d{4}$/.test(value)) return `${value}-01`;
  const pdfMatch = value.match(/^D:(\d{4})(\d{2})?/);
  if (pdfMatch) {
    const month = pdfMatch[2] ?? "01";
    return `${pdfMatch[1]}-${month}`;
  }
  const unix = Number(value);
  if (Number.isFinite(unix) && unix > 1_000_000_000) {
    const d = new Date(unix * 1000);
    const y = d.getUTCFullYear();
    const m = String(d.getUTCMonth() + 1).padStart(2, "0");
    return `${y}-${m}`;
  }
  return "";
}

function localName(el: Element): string {
  return el.localName || el.tagName.split(":").pop() || "";
}

export function coverMimeFromPath(path: string): string {
  const ext = path.split(".").pop()?.toLowerCase();
  if (ext === "png") return "image/png";
  if (ext === "gif") return "image/gif";
  if (ext === "webp") return "image/webp";
  return "image/jpeg";
}

export function parseContainerOpfPath(container: string): string {
  const doc = new DOMParser().parseFromString(container, "application/xml");
  const rootfile = doc.querySelector("rootfile[full-path]");
  const path = rootfile?.getAttribute("full-path");
  if (!path) throw new Error("Invalid EPUB container");
  return path;
}

export function opfDirname(path: string): string {
  const idx = path.lastIndexOf("/");
  return idx >= 0 ? path.slice(0, idx) : "";
}

export function joinOpfPath(dir: string, href: string): string {
  return dir ? `${dir}/${href}` : href;
}
