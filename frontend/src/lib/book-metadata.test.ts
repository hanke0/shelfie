import { describe, expect, it } from "vitest";
import type { BookDetail, BookMetadata } from "@/api/generated/models";
import { prepareMetadataForSave } from "./book-metadata";

const book = { category: "小说" } as BookDetail;

describe("prepareMetadataForSave", () => {
  it("trims string fields and drops reading_progress", () => {
    const form = {
      title: "  三体 ",
      author: " 刘慈欣",
      isbn: " 978-7 ",
      reading_progress: { percent: 50 },
    } as unknown as BookMetadata;

    const result = prepareMetadataForSave(form, book);

    expect(result.title).toBe("三体");
    expect(result.author).toBe("刘慈欣");
    expect(result.isbn).toBe("978-7");
    expect(result.translator).toBe("");
    expect(result.reading_progress).toBeUndefined();
  });

  it("falls back to the book category when blank", () => {
    expect(prepareMetadataForSave({ category: "  " } as BookMetadata, book).category).toBe("小说");
    expect(prepareMetadataForSave({} as BookMetadata, book).category).toBe("小说");
    expect(prepareMetadataForSave({ category: " 科幻 " } as BookMetadata, book).category).toBe(
      "科幻",
    );
  });

  it("keeps rating only within 1..5", () => {
    expect(prepareMetadataForSave({ rating: 3 } as BookMetadata, book).rating).toBe(3);
    expect(prepareMetadataForSave({ rating: 0 } as BookMetadata, book).rating).toBeUndefined();
    expect(prepareMetadataForSave({ rating: 6 } as BookMetadata, book).rating).toBeUndefined();
  });

  it("drops non-finite page_count", () => {
    expect(prepareMetadataForSave({ page_count: 120 } as BookMetadata, book).page_count).toBe(120);
    expect(
      prepareMetadataForSave({ page_count: Number.NaN } as BookMetadata, book).page_count,
    ).toBeUndefined();
  });
});
