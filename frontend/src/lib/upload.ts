import type { BookDetail, BookMetadata } from "@/api/generated/models";
import { getToken } from "./auth";

const API_BASE = import.meta.env.VITE_API_BASE ?? "/api/v1";
const MAX_UPLOAD_BYTES = 512 * 1024 * 1024;

async function parseUploadError(res: Response): Promise<string> {
  const text = await res.text();
  if (!text) {
    if (res.status === 413) {
      return "上传文件过大，请确保单个请求不超过 512MB";
    }
    return res.statusText || `上传失败 (${res.status})`;
  }
  try {
    const err = JSON.parse(text) as { message?: string; code?: string };
    return err.message ?? text;
  } catch {
    return text;
  }
}

function assertFileSize(file: File, label: string) {
  if (file.size > MAX_UPLOAD_BYTES) {
    throw new Error(`${label}超过 512MB 限制`);
  }
}

export async function uploadBookMultipart(
  libraryId: string,
  category: string,
  file: File,
  metadata?: Partial<BookMetadata>,
  cover?: File | null,
): Promise<BookDetail> {
  assertFileSize(file, "图书文件");
  if (cover) {
    assertFileSize(cover, "封面");
  }

  const form = new FormData();
  form.append("library_id", libraryId);
  form.append("category", category);
  form.append("file", file, file.name || "book.pdf");
  if (cover) {
    form.append("cover", cover, cover.name || "cover.jpg");
  }
  if (metadata) {
    form.append("metadata", JSON.stringify(metadata));
  }

  const token = getToken();
  const res = await fetch(`${API_BASE}/books`, {
    method: "POST",
    headers: token ? { Authorization: `Bearer ${token}` } : {},
    body: form,
  });

  if (!res.ok) {
    throw new Error(await parseUploadError(res));
  }

  return res.json();
}

export async function updateCoverMultipart(bookId: string, cover: File): Promise<BookDetail> {
  assertFileSize(cover, "封面");

  const form = new FormData();
  form.append("cover", cover, cover.name || "cover.jpg");

  const token = getToken();
  const res = await fetch(`${API_BASE}/books/${bookId}/cover`, {
    method: "PUT",
    headers: token ? { Authorization: `Bearer ${token}` } : {},
    body: form,
  });

  if (!res.ok) {
    throw new Error(await parseUploadError(res));
  }

  return res.json();
}
