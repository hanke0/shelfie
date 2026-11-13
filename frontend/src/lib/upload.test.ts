import { beforeEach, describe, expect, it, vi } from "vitest";
import { setAuth } from "./auth";
import { updateCoverMultipart, uploadBookMultipart } from "./upload";

let fetchMock: ReturnType<typeof vi.fn>;

beforeEach(() => {
  fetchMock = vi.fn();
  vi.stubGlobal("fetch", fetchMock);
});

const book = new File(["pdf"], "book.pdf");
const cover = new File(["img"], "c.png");

describe("uploadBookMultipart", () => {
  it("posts multipart form with metadata and cover", async () => {
    setAuth("tok", { id: "1", username: "a", role: "user" });
    fetchMock.mockResolvedValue(Response.json({ id: "b1" }));

    const result = await uploadBookMultipart("lib1", "小说", book, { title: "T" }, cover);

    expect(result).toEqual({ id: "b1" });
    const [url, init] = fetchMock.mock.calls[0] as [string, RequestInit];
    expect(url).toBe("/api/v1/books");
    expect(init.method).toBe("POST");
    expect(init.headers).toEqual({ Authorization: "Bearer tok" });
    const form = init.body as FormData;
    expect(form.get("library_id")).toBe("lib1");
    expect(form.get("category")).toBe("小说");
    expect((form.get("file") as File).name).toBe("book.pdf");
    expect((form.get("cover") as File).name).toBe("c.png");
    expect(form.get("metadata")).toBe('{"title":"T"}');
  });

  it("rejects files over 512MB before sending", async () => {
    const huge = new File(["x"], "huge.pdf");
    Object.defineProperty(huge, "size", { value: 512 * 1024 * 1024 + 1 });
    await expect(uploadBookMultipart("lib1", "c", huge)).rejects.toThrow("图书文件超过 512MB 限制");
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it("surfaces JSON error message", async () => {
    fetchMock.mockResolvedValue(Response.json({ message: "重复图书" }, { status: 409 }));
    await expect(uploadBookMultipart("lib1", "c", book)).rejects.toThrow("重复图书");
  });

  it("explains empty 413 responses", async () => {
    fetchMock.mockResolvedValue(new Response(null, { status: 413 }));
    await expect(uploadBookMultipart("lib1", "c", book)).rejects.toThrow("上传文件过大");
  });

  it("uses raw text when error body is not JSON", async () => {
    fetchMock.mockResolvedValue(new Response("bad gateway", { status: 502 }));
    await expect(uploadBookMultipart("lib1", "c", book)).rejects.toThrow("bad gateway");
  });
});

describe("updateCoverMultipart", () => {
  it("PUTs the cover", async () => {
    fetchMock.mockResolvedValue(Response.json({ id: "b1" }));
    await updateCoverMultipart("b1", cover);
    const [url, init] = fetchMock.mock.calls[0] as [string, RequestInit];
    expect(url).toBe("/api/v1/books/b1/cover");
    expect(init.method).toBe("PUT");
    expect(init.headers).toEqual({});
    expect((init.body as FormData).get("cover")).toBeInstanceOf(File);
  });
});
