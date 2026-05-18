import { getToken } from "./auth";

const API_BASE = import.meta.env.VITE_API_BASE ?? "/api/v1";

/** Parse filename from Content-Disposition (RFC 5987 filename* preferred). */
export function parseContentDispositionFilename(disposition: string | null): string | null {
  if (!disposition) return null;

  const utf8Match = /filename\*=UTF-8''([^;\n]+)/i.exec(disposition);
  if (utf8Match?.[1]) {
    try {
      return decodeURIComponent(utf8Match[1].trim());
    } catch {
      /* fall through */
    }
  }

  const quoted = /filename="([^"]+)"/i.exec(disposition);
  if (quoted?.[1]) return quoted[1];

  const plain = /filename=([^;\n]+)/i.exec(disposition);
  if (plain?.[1]) return plain[1].trim().replace(/^['"]|['"]$/g, "");

  return null;
}

export async function downloadBookFile(bookId: string, suggestedName?: string) {
  const token = getToken();
  const res = await fetch(`${API_BASE}/books/${bookId}/download`, {
    headers: token ? { Authorization: `Bearer ${token}` } : {},
  });

  if (!res.ok) {
    const err = (await res.json().catch(() => ({}))) as { message?: string };
    throw new Error(err.message ?? "下载失败");
  }

  const blob = await res.blob();
  const fromHeader = parseContentDispositionFilename(res.headers.get("Content-Disposition"));
  const filename = fromHeader ?? suggestedName ?? "book";

  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  a.style.display = "none";
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  window.setTimeout(() => URL.revokeObjectURL(url), 60_000);
}
