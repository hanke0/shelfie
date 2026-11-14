import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { annotationColor, BookAnnotations } from "./BookAnnotations";

describe("BookAnnotations", () => {
  it("renders highlights with notes and bookmarks", () => {
    render(
      <BookAnnotations
        data={{
          annotations: [
            {
              id: 1,
              text: "划线原文",
              note: "我的笔记",
              chapter: "第三章",
              pageno: 12,
              color: "blue",
              drawer: "underscore",
              created_at: "2026-01-01T00:00:00.000Z",
              updated_at: "2026-01-01T00:00:00.000Z",
            },
          ],
          bookmarks: [
            { id: 2, title: "书签一", pageno: 5, created_at: "2026-01-01T00:00:00.000Z" },
          ],
        }}
      />,
    );

    const quote = screen.getByText("划线原文");
    expect(quote).toHaveAttribute("data-drawer", "underscore");
    expect(quote).toHaveStyle({ borderLeftColor: "#7aa5e8" });
    expect(screen.getByText("我的笔记")).toBeInTheDocument();
    expect(screen.getByText("第三章 · 第 12 页")).toBeInTheDocument();
    expect(screen.getByText("书签一")).toBeInTheDocument();
    expect(screen.getByText("书签（1）")).toBeInTheDocument();
  });

  it("shows empty states", () => {
    render(<BookAnnotations data={{ annotations: [], bookmarks: [] }} />);
    expect(screen.getByText("标注与笔记（0）")).toBeInTheDocument();
    expect(screen.getByText("暂无书签。")).toBeInTheDocument();
  });

  it("maps KOReader color names and falls back to yellow", () => {
    expect(annotationColor("RED")).toBe("#e8776f");
    expect(annotationColor("#123456")).toBe("#123456");
    expect(annotationColor(undefined)).toBe("#f2cf57");
  });
});
