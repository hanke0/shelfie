const API_BASE = (import.meta.env.VITE_API_BASE ?? "/api/v1").replace(/\/$/, "");

/** 对外可访问的 API 源（开发时走 Vite 代理则为 5173，生产可设 VITE_API_BASE 为完整 URL） */
export function apiPublicOrigin(): string {
  if (API_BASE.startsWith("http://") || API_BASE.startsWith("https://")) {
    return new URL(API_BASE).origin;
  }
  return window.location.origin;
}

export function opdsLibraryCatalogPath(libraryId: string): string {
  return `${API_BASE}/opds/libraries/${libraryId}`;
}

export function opdsRootCatalogPath(): string {
  return `${API_BASE}/opds`;
}

export function toAbsoluteApiUrl(path: string): string {
  if (path.startsWith("http://") || path.startsWith("https://")) {
    return path;
  }
  const normalized = path.startsWith("/") ? path : `/${path}`;
  return `${apiPublicOrigin()}${normalized}`;
}

export function opdsLibraryCatalogUrl(
  libraryId: string,
  opdsUrlFromApi?: string | null,
): string {
  const path =
    opdsUrlFromApi?.trim() || `/api/v1/opds/libraries/${libraryId}`;
  return toAbsoluteApiUrl(path);
}

export function opdsRootCatalogUrl(): string {
  return toAbsoluteApiUrl(opdsRootCatalogPath());
}
