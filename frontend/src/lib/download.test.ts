import { beforeEach, describe, expect, it, vi } from "vitest";
import { downloadBookFile, parseContentDispositionFilename } from "./download";

describe("parseContentDispositionFilename", () => {
  it.each([
    [null, null],
    ["attachment", null],
    ["attachment; filename=\"a b.pdf\"", "a b.pdf"],
    ["attachment; filename=plain.epub", "plain.epub"],
    [
      "attachment; filename=\"fallback.pdf\"; filename*=UTF-8''%E4%B8%89%E4%BD%93.pdf",
      "三体.pdf",
    ],
    ["attachment; filename*=UTF-8''%E0%A4%A; filename=\"safe.pdf\"", "safe.pdf"],
  ])("%s -> %s", (header, expected) => {
    expect(parseContentDispositionFilename(header)).toBe(expected);
  });
});

describe("downloadBookFile", () => {
  let fetchMock: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    fetchMock = vi.fn();
    vi.stubGlobal("fetch", fetchMock);
  });

  it("triggers an anchor download with header filename", async () => {
    fetchMock.mockResolvedValue(
      new Response("data", { headers: { "Content-Disposition": 'attachment; filename="x.pdf"' } }),
    );
    let downloaded: string | undefined;
    vi.spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(function (
      this: HTMLAnchorElement,
    ) {
      downloaded = this.download;
    });

    await downloadBookFile("b1", "suggested.pdf");

    expect(fetchMock.mock.calls[0][0]).toBe("/api/v1/books/b1/download");
    expect(downloaded).toBe("x.pdf");
    expect(document.querySelector("a")).toBeNull();
  });

  it("falls back to suggested name", async () => {
    fetchMock.mockResolvedValue(new Response("data"));
    let downloaded: string | undefined;
    vi.spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(function (
      this: HTMLAnchorElement,
    ) {
      downloaded = this.download;
    });

    await downloadBookFile("b1", "suggested.pdf");
    expect(downloaded).toBe("suggested.pdf");
  });

  it("throws server message on failure", async () => {
    fetchMock.mockResolvedValue(Response.json({ message: "不存在" }, { status: 404 }));
    await expect(downloadBookFile("b1")).rejects.toThrow("不存在");
  });

  it("throws default message when body is not JSON", async () => {
    fetchMock.mockResolvedValue(new Response("x", { status: 500 }));
    await expect(downloadBookFile("b1")).rejects.toThrow("下载失败");
  });
});
