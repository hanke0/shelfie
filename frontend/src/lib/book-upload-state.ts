import type { BookMetadata } from "@/api/generated/models";

const ACCEPTED_EXTENSIONS = new Set(["pdf", "epub", "mobi"]);

export type UploadDraft =
  | { phase: "extracting"; file: File }
  | {
      phase: "form";
      file: File;
      metadata: BookMetadata;
      cover?: File;
      extractError?: string;
    };

export function isAcceptedBookFile(file: File): boolean {
  const ext = file.name.split(".").pop()?.toLowerCase() ?? "";
  return ACCEPTED_EXTENSIONS.has(ext);
}

export function fallbackBookTitle(file: File): string {
  return file.name.replace(/\.[^.]+$/i, "");
}

export function metadataFormFromExtract(
  extracted: Partial<BookMetadata>,
  file: File,
): BookMetadata {
  const fallbackTitle = fallbackBookTitle(file);
  return {
    title: extracted.title?.trim() || fallbackTitle,
    author: extracted.author?.trim() ?? "",
    language: extracted.language?.trim() ?? "",
    translator: extracted.translator?.trim() ?? "",
    publisher: extracted.publisher?.trim() ?? "",
    isbn: extracted.isbn?.trim() ?? "",
    original_title: extracted.original_title?.trim() ?? "",
    series: extracted.series?.trim() ?? "",
    notes: extracted.notes?.trim() ?? "",
    publish_date: extracted.publish_date?.trim() ?? "",
    category: extracted.category?.trim() || "未分类",
    page_count:
      extracted.page_count != null && Number.isFinite(extracted.page_count)
        ? extracted.page_count
        : undefined,
  };
}

export function prepareMetadataForUpload(form: BookMetadata): BookMetadata {
  const category = (form.category ?? "未分类").trim() || "未分类";
  const rating =
    form.rating != null && form.rating >= 1 && form.rating <= 5 ? form.rating : undefined;

  return {
    ...form,
    title: form.title?.trim() ?? "",
    author: form.author?.trim() ?? "",
    language: form.language?.trim() ?? "",
    translator: form.translator?.trim() ?? "",
    publisher: form.publisher?.trim() ?? "",
    isbn: form.isbn?.trim() ?? "",
    original_title: form.original_title?.trim() ?? "",
    series: form.series?.trim() ?? "",
    notes: form.notes?.trim() ?? "",
    publish_date: form.publish_date?.trim() ?? "",
    category,
    rating,
    page_count:
      form.page_count != null && Number.isFinite(form.page_count) ? form.page_count : undefined,
  };
}

export function formatFileSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}
