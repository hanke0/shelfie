import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { BookReadingSummary, formatDuration } from "./BookReadingSummary";

describe("BookReadingSummary", () => {
  it("shows status and reading stats", () => {
    render(
      <BookReadingSummary
        summary={{
          status: "complete",
          status_modified: "2026-01-03",
          total_seconds: 5400,
          pages_read: 42,
          total_pages: 100,
          reading_days: 3,
          first_read_at: 1_700_000_000,
          last_read_at: 1_700_000_040,
        }}
      />,
    );
    expect(screen.getByText("已读完")).toBeInTheDocument();
    expect(screen.getByText("（2026-01-03）")).toBeInTheDocument();
    expect(screen.getByText("1 小时 30 分钟")).toBeInTheDocument();
    expect(screen.getByText("42 / 100")).toBeInTheDocument();
    expect(screen.getByText("3 天")).toBeInTheDocument();
  });

  it("handles a book without sync data", () => {
    render(
      <BookReadingSummary summary={{ total_seconds: 0, pages_read: 0, reading_days: 0 }} />,
    );
    expect(screen.getByText("—")).toBeInTheDocument();
    expect(screen.getByText("暂无阅读统计")).toBeInTheDocument();
  });

  it("formats durations", () => {
    expect(formatDuration(59)).toBe("1 分钟");
    expect(formatDuration(7200)).toBe("2 小时");
  });
});
