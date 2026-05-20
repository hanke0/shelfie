import JSZip from "jszip";
import type { BookMetadata } from "@/api/generated/models";
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
    const href = joinOpfPath(opfDir, parsed.coverHref);
    const entry = zip.file(href);
    if (entry) {
      const bytes = await entry.async("uint8array");
      const mime = coverMimeFromPath(href);
      const ext = mime.includes("png") ? "png" : "jpg";
      cover = new File([bytes], `${fallbackTitle(file)}-cover.${ext}`, { type: mime });
    }
  }

  return {
    metadata: opfMetaToPartial(parsed.meta, fallbackTitle(file)),
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

async function readZipText(zip: JSZip, path: string): Promise<string> {
  const entry = zip.file(path);
  if (!entry) throw new Error(`Missing ${path}`);
  return entry.async("text");
}
