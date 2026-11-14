import { describe, expect, it } from "vitest";
import { formatDuration, formatProgress } from "./maintenance";

describe("maintenance helpers", () => {
  it("formats durations", () => {
    expect(formatDuration(0)).toBe("0.0秒");
    expect(formatDuration(3_240)).toBe("3.2秒");
    expect(formatDuration(42_000)).toBe("42秒");
    expect(formatDuration(125_000)).toBe("2分5秒");
    expect(formatDuration(3_723_000)).toBe("1小时2分3秒");
  });

  it("formats progress", () => {
    expect(formatProgress({ current: 3, total: 10, message: "a.pdf" })).toBe("3/10 · a.pdf");
    expect(formatProgress({ current: 0, total: null, message: "扫描中" })).toBe("扫描中");
    expect(formatProgress({ current: 0 })).toBe("");
  });
});
