import { describe, expect, it } from "vitest";
import {
  apiPublicOrigin,
  opdsLibraryCatalogPath,
  opdsLibraryCatalogUrl,
  opdsRootCatalogUrl,
  toAbsoluteApiUrl,
} from "./opds-url";

const origin = window.location.origin;

describe("opds-url", () => {
  it("uses window origin for relative API base", () => {
    expect(apiPublicOrigin()).toBe(origin);
  });

  it("builds catalog paths and URLs", () => {
    expect(opdsLibraryCatalogPath("lib1")).toBe("/api/v1/opds/libraries/lib1");
    expect(opdsRootCatalogUrl()).toBe(`${origin}/api/v1/opds`);
  });

  it("normalizes relative paths and keeps absolute URLs", () => {
    expect(toAbsoluteApiUrl("api/v1/x")).toBe(`${origin}/api/v1/x`);
    expect(toAbsoluteApiUrl("https://example.com/opds")).toBe("https://example.com/opds");
  });

  it("prefers API-provided OPDS url", () => {
    expect(opdsLibraryCatalogUrl("lib1", " /custom/opds ")).toBe(`${origin}/custom/opds`);
    expect(opdsLibraryCatalogUrl("lib1", "")).toBe(`${origin}/api/v1/opds/libraries/lib1`);
    expect(opdsLibraryCatalogUrl("lib1", null)).toBe(`${origin}/api/v1/opds/libraries/lib1`);
  });
});
