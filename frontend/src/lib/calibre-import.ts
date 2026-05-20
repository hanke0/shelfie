import initSqlJs, { type Database } from "sql.js";
import type { BookMetadata } from "@/api/generated/models";
import {
  coverMimeFromPath,
  firstMeta,
  normalizePublishDate,
  opfMetaToPartial,
  parseOpf,
} from "@/lib/opf-metadata";

const BOOK_EXTENSIONS = ["epub", "pdf", "mobi"] as const;
const EXT_PRIORITY: Record<string, number> = { epub: 0, pdf: 1, mobi: 2 };

export type CategoryMappingMode = "fixed" | "first_tag";

export type CalibreBookRow = {
  id: number;
  title: string;
  path: string;
  pubdate: string | null;
  authors: string | null;
  firstTag: string | null;
};

export type PreparedCalibreBook = {
  row: CalibreBookRow;
  bookFile: File;
  cover?: File;
  metadata: Partial<BookMetadata>;
  firstTag: string | null;
};

let sqlJsInit: ReturnType<typeof initSqlJs> | null = null;

async function getSqlJs() {
  if (!sqlJsInit) {
    sqlJsInit = initSqlJs({ locateFile: () => "/sql-wasm.wasm" });
  }
  return sqlJsInit;
}

export function isCalibreImportSupported(): boolean {
  return typeof window !== "undefined" && "showDirectoryPicker" in window;
}

export function resolveCategory(
  mode: CategoryMappingMode,
  fixed: string,
  fallback: string,
  firstTag: string | null,
  existingCategories: Set<string>,
): { category: string; usedFallback: boolean } {
  if (mode === "fixed") {
    return { category: fixed, usedFallback: false };
  }
  const tag = firstTag?.trim();
  if (tag && existingCategories.has(tag)) {
    return { category: tag, usedFallback: false };
  }
  return { category: fallback, usedFallback: true };
}

export async function loadCalibreBooks(
  root: FileSystemDirectoryHandle,
): Promise<CalibreBookRow[]> {
  const dbFile = await root.getFileHandle("metadata.db");
  const file = await dbFile.getFile();
  const buffer = await file.arrayBuffer();
  const SQL = await getSqlJs();
  const db = new SQL.Database(new Uint8Array(buffer));
  try {
    return queryCalibreBooks(db);
  } finally {
    db.close();
  }
}

function queryCalibreBooks(db: Database): CalibreBookRow[] {
  const stmt = db.prepare(`
    SELECT b.id, b.title, b.path, b.pubdate,
      (SELECT group_concat(a.name, ' & ')
       FROM books_authors_link bal
       JOIN authors a ON a.id = bal.author
       WHERE bal.book = b.id) AS authors,
      (SELECT t.name
       FROM books_tags_link btl
       JOIN tags t ON t.id = btl.tag
       WHERE btl.book = b.id
       ORDER BY btl.id
       LIMIT 1) AS first_tag
    FROM books b
    ORDER BY b.title
  `);

  const rows: CalibreBookRow[] = [];
  while (stmt.step()) {
    const r = stmt.getAsObject() as Record<string, unknown>;
    rows.push({
      id: Number(r.id),
      title: String(r.title ?? ""),
      path: String(r.path ?? ""),
      pubdate: r.pubdate != null ? String(r.pubdate) : null,
      authors: r.authors != null ? String(r.authors) : null,
      firstTag: r.first_tag != null ? String(r.first_tag) : null,
    });
  }
  stmt.free();
  return rows;
}

export async function prepareCalibreBook(
  root: FileSystemDirectoryHandle,
  row: CalibreBookRow,
): Promise<PreparedCalibreBook | null> {
  const bookDir = await resolveBookDir(root, row.path);
  const bookFile = await pickBookFile(bookDir);
  if (!bookFile) return null;

  const opfMeta = await readMetadataOpf(bookDir);
  const metadata = mergeCalibreMetadata(row, opfMeta, bookFile.name);
  const cover = (await readCoverFromDir(bookDir)) ?? (await readCoverFromOpf(bookDir, opfMeta));

  return {
    row,
    bookFile,
    cover,
    metadata,
    firstTag: row.firstTag,
  };
}

async function resolveBookDir(
  root: FileSystemDirectoryHandle,
  path: string,
): Promise<FileSystemDirectoryHandle> {
  const parts = path.split(/[/\\]/).filter(Boolean);
  let current = root;
  for (const part of parts) {
    current = await current.getDirectoryHandle(part);
  }
  return current;
}

async function pickBookFile(dir: FileSystemDirectoryHandle): Promise<File | null> {
  const candidates: { ext: string; file: File }[] = [];

  for await (const [name, handle] of dir.entries()) {
    if (handle.kind !== "file") continue;
    const fileHandle = handle as FileSystemFileHandle;
    const ext = name.split(".").pop()?.toLowerCase() ?? "";
    if (!BOOK_EXTENSIONS.includes(ext as (typeof BOOK_EXTENSIONS)[number])) continue;
    const file = await fileHandle.getFile();
    candidates.push({ ext, file });
  }

  if (candidates.length === 0) return null;

  candidates.sort((a, b) => (EXT_PRIORITY[a.ext] ?? 9) - (EXT_PRIORITY[b.ext] ?? 9));
  return candidates[0].file;
}

async function readMetadataOpf(dir: FileSystemDirectoryHandle) {
  try {
    const handle = await dir.getFileHandle("metadata.opf");
    const text = await (await handle.getFile()).text();
    return parseOpf(text);
  } catch {
    return null;
  }
}

async function readCoverFromDir(dir: FileSystemDirectoryHandle): Promise<File | undefined> {
  for (const name of ["cover.jpg", "cover.jpeg", "cover.png"]) {
    try {
      const handle = await dir.getFileHandle(name);
      return handle.getFile();
    } catch {
      continue;
    }
  }
  return undefined;
}

async function readCoverFromOpf(
  dir: FileSystemDirectoryHandle,
  parsed: ReturnType<typeof parseOpf> | null,
): Promise<File | undefined> {
  if (!parsed?.coverHref) return undefined;
  const href = parsed.coverHref;
  try {
    const handle = await dir.getFileHandle(href.split("/").pop() ?? href);
    const file = await handle.getFile();
    const mime = coverMimeFromPath(href);
    const ext = mime.includes("png") ? "png" : "jpg";
    return new File([file], `cover.${ext}`, { type: mime });
  } catch {
    const parts = href.split("/");
    if (parts.length > 1) {
      try {
        let sub = dir;
        for (let i = 0; i < parts.length - 1; i++) {
          sub = await sub.getDirectoryHandle(parts[i]);
        }
        const handle = await sub.getFileHandle(parts[parts.length - 1]);
        const file = await handle.getFile();
        const mime = coverMimeFromPath(href);
        const ext = mime.includes("png") ? "png" : "jpg";
        return new File([file], `cover.${ext}`, { type: mime });
      } catch {
        return undefined;
      }
    }
    return undefined;
  }
}

function mergeCalibreMetadata(
  row: CalibreBookRow,
  opf: ReturnType<typeof parseOpf> | null,
  bookFileName: string,
): Partial<BookMetadata> {
  const fallbackTitle = bookFileName.replace(/\.[^.]+$/i, "");
  const fromOpf = opf ? opfMetaToPartial(opf.meta, fallbackTitle) : {};

  const title = fromOpf.title?.trim() || row.title.trim() || fallbackTitle;
  const author = fromOpf.author?.trim() || row.authors?.trim() || "";
  const publishDate =
    fromOpf.publish_date?.trim() ||
    normalizePublishDate(row.pubdate ?? "") ||
    normalizeCalibrePubdate(row.pubdate ?? "");

  return {
    ...fromOpf,
    title,
    author,
    publish_date: publishDate,
    series: fromOpf.series || firstMeta(opf?.meta ?? {}, "calibre:series"),
  };
}

function normalizeCalibrePubdate(raw: string): string {
  const value = raw.trim();
  if (!value) return "";
  const dotted = value.match(/^(\d{4})\.(\d{2})\.(\d{2})/);
  if (dotted) return `${dotted[1]}-${dotted[2]}`;
  const dashed = value.match(/^(\d{4})-(\d{2})/);
  if (dashed) return `${dashed[1]}-${dashed[2]}`;
  return normalizePublishDate(value);
}
