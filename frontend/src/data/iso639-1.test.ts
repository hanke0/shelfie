import { describe, expect, it } from "vitest";
import {
  displayLanguageValue,
  filterLanguageOptions,
  formatLanguageLabel,
  getLanguageOption,
  ISO639_1_LANGUAGES,
  resolveLanguageCode,
} from "./iso639-1";

describe("iso639-1", () => {
  it("pins zh and en first", () => {
    expect(ISO639_1_LANGUAGES.slice(0, 2).map((l) => l.code)).toEqual(["zh", "en"]);
  });

  it("looks up codes case-insensitively", () => {
    expect(getLanguageOption("EN")?.code).toBe("en");
    expect(getLanguageOption("xx")).toBeUndefined();
  });

  it("formats label with native name when different", () => {
    expect(formatLanguageLabel({ code: "fr", name: "French", nativeName: "français" })).toBe(
      "français (fr)",
    );
    expect(formatLanguageLabel({ code: "aa", name: "Afar" })).toBe("Afar (aa)");
  });

  it("displays stored values", () => {
    expect(displayLanguageValue(undefined)).toBe("—");
    expect(displayLanguageValue("  ")).toBe("—");
    expect(displayLanguageValue("en")).toBe(formatLanguageLabel(getLanguageOption("en")!));
    expect(displayLanguageValue("古汉语")).toBe("古汉语");
  });

  it("filters by code, name and native name", () => {
    expect(filterLanguageOptions("")).toBe(ISO639_1_LANGUAGES);
    expect(filterLanguageOptions("english").map((l) => l.code)).toContain("en");
    expect(filterLanguageOptions("zzzz")).toEqual([]);
  });

  it("resolves input to a code", () => {
    expect(resolveLanguageCode("")).toBe("");
    expect(resolveLanguageCode(" EN ")).toBe("en");
    expect(resolveLanguageCode("english")).toBe("en");
    expect(resolveLanguageCode(formatLanguageLabel(getLanguageOption("zh")!))).toBe("zh");
    expect(resolveLanguageCode("klingon")).toBeNull();
  });
});
