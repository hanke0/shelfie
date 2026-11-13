import { describe, expect, it } from "vitest";
import { formatApiError } from "./api-error";

describe("formatApiError", () => {
  it("uses Error.message", () => {
    expect(formatApiError(new Error("boom"))).toBe("boom");
  });

  it("uses non-empty strings", () => {
    expect(formatApiError("bad request")).toBe("bad request");
  });

  it("uses message field of plain objects", () => {
    expect(formatApiError({ message: "denied" })).toBe("denied");
  });

  it("falls back for empty or unknown values", () => {
    expect(formatApiError(new Error("  "))).toBe("操作失败");
    expect(formatApiError("")).toBe("操作失败");
    expect(formatApiError({ message: 42 })).toBe("操作失败");
    expect(formatApiError(null, "自定义")).toBe("自定义");
  });
});
