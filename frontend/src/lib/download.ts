import { getToken } from "./auth";

const API_BASE = import.meta.env.VITE_API_BASE ?? "/api/v1";

export async function downloadBookFile(bookId: string, suggestedName?: string) {
  const token = getToken();
  const res = await fetch(`${API_BASE}/books/${bookId}/download`, {
    headers: token ? { Authorization: `Bearer ${token}` } : {},
  });

  if (!res.ok) {
    const err = await res.json().catch(() => ({}));
    throw new Error(err.message ?? "下载失败");
  }

  const blob = await res.blob();
  const disposition = res.headers.get("Content-Disposition");
  let filename = suggestedName ?? "book";
  if (disposition) {
    const match = /filename="([^"]+)"/.exec(disposition);
    if (match?.[1]) filename = match[1];
  }

  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  a.click();
  URL.revokeObjectURL(url);
}
