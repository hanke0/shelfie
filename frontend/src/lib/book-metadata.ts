import type { BookDetail, BookMetadata } from "@/api/generated/models";

/** 提交 PATCH 前整理 metadata，避免无效字段导致后端 400 */
export function prepareMetadataForSave(
  form: BookMetadata,
  book: BookDetail,
): BookMetadata {
  const category = (form.category ?? book.category ?? "").trim() || book.category;
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
