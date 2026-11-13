import { describe, expect, it } from "vitest";
import {
  coverMimeFromPath,
  joinOpfPath,
  normalizeIsbn,
  normalizePublishDate,
  opfDirname,
  opfMetaToPartial,
  parseContainerOpfPath,
  parseOpf,
} from "./opf-metadata";

const OPF = `<?xml version="1.0" encoding="UTF-8"?>
<package xmlns="http://www.idpf.org/2007/opf" version="2.0">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:opf="http://www.idpf.org/2007/opf">
    <dc:title>三体</dc:title>
    <dc:creator>刘慈欣</dc:creator>
    <dc:creator>某合著者</dc:creator>
    <dc:contributor>译者甲</dc:contributor>
    <dc:publisher>重庆出版社</dc:publisher>
    <dc:language>zh</dc:language>
    <dc:identifier opf:scheme="uuid">urn:uuid:1234</dc:identifier>
    <dc:identifier opf:scheme="ISBN">978-7-5366-9293-0</dc:identifier>
    <dc:date>2008-01-01T00:00:00+00:00</dc:date>
    <dc:description>  简介  </dc:description>
    <meta name="calibre:series" content="地球往事"/>
    <meta name="cover" content="cover-img"/>
  </metadata>
  <manifest>
    <item id="cover-img" href="images/cover.jpg" media-type="image/jpeg"/>
  </manifest>
</package>`;

describe("parseOpf / opfMetaToPartial", () => {
  it("extracts metadata and cover href", () => {
    const { meta, coverHref } = parseOpf(OPF);
    expect(coverHref).toBe("images/cover.jpg");
    expect(opfMetaToPartial(meta)).toEqual({
      title: "三体",
      author: "刘慈欣, 某合著者",
      translator: "译者甲",
      publisher: "重庆出版社",
      language: "zh",
      isbn: "978-7-5366-9293-0",
      publish_date: "2008-01",
      original_title: "",
      series: "地球往事",
      notes: "简介",
    });
  });

  it("uses fallback title when missing", () => {
    const { meta } = parseOpf(
      `<package xmlns="http://www.idpf.org/2007/opf"><metadata></metadata></package>`,
    );
    expect(opfMetaToPartial(meta, "file-name").title).toBe("file-name");
  });
});

describe("normalizeIsbn", () => {
  it("keeps values with >= 10 digits", () => {
    expect(normalizeIsbn("urn:isbn:9787536692930")).toBe("9787536692930");
    expect(normalizeIsbn("0-306-40615-2")).toBe("0-306-40615-2");
  });

  it("rejects short non-isbn identifiers", () => {
    expect(normalizeIsbn("urn:uuid:1234")).toBe("");
  });
});

describe("normalizePublishDate", () => {
  it.each([
    ["2008-05-01", "2008-05"],
    ["2008", "2008-01"],
    ["D:20190312120000", "2019-03"],
    ["D:2019", "2019-01"],
    ["1577836800", "2020-01"],
    ["", ""],
    ["unknown", ""],
  ])("%s -> %s", (input, expected) => {
    expect(normalizePublishDate(input)).toBe(expected);
  });
});

describe("container & path helpers", () => {
  it("parses container rootfile path", () => {
    const xml = `<container><rootfiles><rootfile full-path="OEBPS/content.opf"/></rootfiles></container>`;
    expect(parseContainerOpfPath(xml)).toBe("OEBPS/content.opf");
  });

  it("throws on invalid container", () => {
    expect(() => parseContainerOpfPath("<container/>")).toThrow("Invalid EPUB container");
  });

  it("handles dirname and join", () => {
    expect(opfDirname("OEBPS/content.opf")).toBe("OEBPS");
    expect(opfDirname("content.opf")).toBe("");
    expect(joinOpfPath("OEBPS", "a.jpg")).toBe("OEBPS/a.jpg");
    expect(joinOpfPath("", "a.jpg")).toBe("a.jpg");
  });

  it("maps cover mime by extension", () => {
    expect(coverMimeFromPath("a.PNG")).toBe("image/png");
    expect(coverMimeFromPath("a.webp")).toBe("image/webp");
    expect(coverMimeFromPath("a.gif")).toBe("image/gif");
    expect(coverMimeFromPath("a.jpeg")).toBe("image/jpeg");
  });
});
