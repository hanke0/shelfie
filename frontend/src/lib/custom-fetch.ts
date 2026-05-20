import { getToken, clearAuth } from "./auth";

const API_BASE = import.meta.env.VITE_API_BASE ?? "/api/v1";

export type BodyType<BodyData> = BodyData;

export interface FetchConfig {
  url: string;
  method: string;
  headers?: Record<string, string>;
  data?: unknown;
  params?: Record<string, unknown>;
  signal?: AbortSignal;
}

function buildUrl(path: string, params?: Record<string, unknown>): string {
  const base = `${API_BASE}${path}`;
  if (!params) return base;
  const search = new URLSearchParams();
  for (const [key, value] of Object.entries(params)) {
    if (value !== undefined && value !== null) {
      search.set(key, String(value));
    }
  }
  const qs = search.toString();
  return qs ? `${base}?${qs}` : base;
}

export const customFetch = async <T>(
  config: FetchConfig,
  _options?: RequestInit,
): Promise<T> => {
  const token = getToken();
  const headers = new Headers(config.headers);

  if (token) {
    headers.set("Authorization", `Bearer ${token}`);
  }

  let body: BodyInit | undefined;
  if (config.data instanceof FormData) {
    body = config.data;
    headers.delete("Content-Type");
  } else if (config.data !== undefined) {
    if (!headers.has("Content-Type")) {
      headers.set("Content-Type", "application/json");
    }
    body = JSON.stringify(config.data);
  }

  const response = await fetch(buildUrl(config.url, config.params), {
    method: config.method,
    headers,
    body,
    signal: config.signal,
  });

  if (response.status === 401) {
    clearAuth();
    if (!window.location.pathname.startsWith("/login")) {
      window.location.href = "/login";
    }
  }

  if (!response.ok) {
    let message = response.statusText;
    try {
      const err = await response.json();
      message = err.message ?? message;
    } catch {
      /* ignore */
    }
    throw new Error(message);
  }

  if (response.status === 204) {
    return undefined as T;
  }

  const contentType = response.headers.get("content-type");
  if (contentType?.includes("application/json")) {
    return response.json() as Promise<T>;
  }

  return response as unknown as T;
};

export async function fetchCoverBlob(
  bookId: string,
  options?: { thumbnail?: boolean },
): Promise<string> {
  const token = getToken();
  const qs = options?.thumbnail ? "?size=thumb" : "";
  const res = await fetch(`${API_BASE}/assets/covers/${bookId}${qs}`, {
    headers: token ? { Authorization: `Bearer ${token}` } : {},
    cache: "default",
  });
  if (!res.ok) throw new Error("Failed to load cover");
  const blob = await res.blob();
  return URL.createObjectURL(blob);
}
