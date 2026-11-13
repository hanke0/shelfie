import { describe, expect, it } from "vitest";
import type { BookMetadata } from "@/api/generated/models";
import {
  fallbackBookTitle,
  formatFileSize,
  isAcceptedBookFile,
  metadataFormFromExtract,
  prepareMetadataForUpload,
} from "./book-upload-state";

const file = (name: string) => new File(["x"], name);

describe("isAcceptedBookFile", () => {
  it("accepts pdf/epub/mobi case-insensitively", () => {
    expect(isAcceptedBookFile(file("a.pdf"))).toBe(true);
    expect(isAcceptedBookFile(file("a.EPUB"))).toBe(true);
    expect(isAcceptedBookFile(file("a.b.mobi"))).toBe(true);
  });

  it("rejects other extensions", () => {
    expect(isAcceptedBookFile(file("a.txt"))).toBe(false);
    expect(isAcceptedBookFile(file("a.pdf.zip"))).toBe(false);
  });
});

describe("fallbackBookTitle", () => {
  it("strips only the last extension", () => {
    expect(fallbackBookTitle(file("三体.v2.epub"))).toBe("三体.v2");
  });
});

describe("metadataFormFromExtract", () => {
  it("uses file name as title and 未分类 as category when missing", () => {
    const result = metadataFormFromExtract({ title: "  ", author: " 作者 " }, file("书名.pdf"));
    expect(result.title).toBe("书名");
    expect(result.author).toBe("作者");
    expect(result.category).toBe("未分类");
    expect(result.page_count).toBeUndefined();
  });

  it("keeps extracted values", () => {
    const result = metadataFormFromExtract(
      { title: "T", category: "C", page_count: 10 },
      file("x.pdf"),
    );
    expect(result).toMatchObject({ title: "T", category: "C", page_count: 10 });
  });
});

describe("prepareMetadataForUpload", () => {
  it("defaults blank category to 未分类", () => {
    expect(prepareMetadataForUpload({ category: " " } as BookMetadata).category).toBe("未分类");
    expect(prepareMetadataForUpload({} as BookMetadata).category).toBe("未分类");
  });

  it("trims fields and validates rating", () => {
    const result = prepareMetadataForUpload({
      title: " T ",
      rating: 9,
      page_count: Number.POSITIVE_INFINITY,
    } as BookMetadata);
    expect(result.title).toBe("T");
    expect(result.rating).toBeUndefined();
    expect(result.page_count).toBeUndefined();
  });
});

describe("formatFileSize", () => {
  it("formats B / KB / MB", () => {
    expect(formatFileSize(512)).toBe("512 B");
    expect(formatFileSize(1536)).toBe("1.5 KB");
    expect(formatFileSize(5 * 1024 * 1024)).toBe("5.0 MB");
  });
});
