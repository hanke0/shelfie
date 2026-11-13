import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { setAuth, getToken } from "./auth";
import { customFetch, fetchCoverBlob } from "./custom-fetch";

function jsonResponse(body: unknown, init?: ResponseInit) {
  return new Response(JSON.stringify(body), {
    headers: { "Content-Type": "application/json" },
    ...init,
  });
}

let fetchMock: ReturnType<typeof vi.fn>;

beforeEach(() => {
  fetchMock = vi.fn();
  vi.stubGlobal("fetch", fetchMock);
});

afterEach(() => {
  window.history.replaceState(null, "", "/");
});

function lastCall() {
  const [url, init] = fetchMock.mock.calls.at(-1)!;
  return { url: url as string, init: init as RequestInit, headers: new Headers(init.headers) };
}

describe("customFetch", () => {
  it("builds URL with params, skipping null/undefined", async () => {
    fetchMock.mockResolvedValue(jsonResponse({ ok: 1 }));
    const data = await customFetch<{ ok: number }>({
      url: "/books",
      method: "GET",
      params: { q: "三体", page: 2, empty: undefined, none: null },
    });
    expect(data).toEqual({ ok: 1 });
    expect(lastCall().url).toBe("/api/v1/books?q=%E4%B8%89%E4%BD%93&page=2");
  });

  it("attaches bearer token and serializes JSON body", async () => {
    setAuth("tok", { id: "1", username: "a", role: "user" });
    fetchMock.mockResolvedValue(jsonResponse({}));
    await customFetch({ url: "/x", method: "POST", data: { a: 1 } });

    const { init, headers } = lastCall();
    expect(headers.get("Authorization")).toBe("Bearer tok");
    expect(headers.get("Content-Type")).toBe("application/json");
    expect(init.body).toBe('{"a":1}');
  });

  it("sends FormData without forcing Content-Type", async () => {
    fetchMock.mockResolvedValue(jsonResponse({}));
    const form = new FormData();
    await customFetch({
      url: "/x",
      method: "POST",
      data: form,
      headers: { "Content-Type": "multipart/form-data" },
    });
    const { init, headers } = lastCall();
    expect(init.body).toBe(form);
    expect(headers.has("Content-Type")).toBe(false);
  });

  it("throws server error message", async () => {
    fetchMock.mockResolvedValue(jsonResponse({ message: "无权限" }, { status: 403 }));
    await expect(customFetch({ url: "/x", method: "GET" })).rejects.toThrow("无权限");
  });

  it("falls back to statusText when error body is not JSON", async () => {
    fetchMock.mockResolvedValue(new Response("oops", { status: 500, statusText: "Server Error" }));
    await expect(customFetch({ url: "/x", method: "GET" })).rejects.toThrow("Server Error");
  });

  it("clears auth on 401", async () => {
    window.history.replaceState(null, "", "/login");
    setAuth("tok", { id: "1", username: "a", role: "user" });
    fetchMock.mockResolvedValue(jsonResponse({ message: "expired" }, { status: 401 }));
    await expect(customFetch({ url: "/x", method: "GET" })).rejects.toThrow("expired");
    expect(getToken()).toBeNull();
  });

  it("returns undefined for 204 and blob for responseType blob", async () => {
    fetchMock.mockResolvedValueOnce(new Response(null, { status: 204 }));
    expect(await customFetch({ url: "/x", method: "DELETE" })).toBeUndefined();

    fetchMock.mockResolvedValueOnce(new Response("bin"));
    const blob = await customFetch<Blob>({ url: "/x", method: "GET", responseType: "blob" });
    expect(await blob.text()).toBe("bin");
  });
});

describe("fetchCoverBlob", () => {
  it("requests thumbnail and returns object URL", async () => {
    vi.spyOn(URL, "createObjectURL").mockReturnValue("blob:cover");
    fetchMock.mockResolvedValue(new Response("img"));

    expect(await fetchCoverBlob("b1", { thumbnail: true })).toBe("blob:cover");
    expect(lastCall().url).toBe("/api/v1/assets/covers/b1?size=thumb");
  });

  it("throws on failure", async () => {
    fetchMock.mockResolvedValue(new Response(null, { status: 404 }));
    await expect(fetchCoverBlob("b1")).rejects.toThrow("Failed to load cover");
  });
});
