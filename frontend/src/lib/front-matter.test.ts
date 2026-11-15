import { describe, expect, it } from "vitest";
import { extractFrontMatter, fillMissingMetadata, htmlToPlainText } from "./front-matter";

describe("extractFrontMatter", () => {
  it("parses labeled translator", () => {
    expect(extractFrontMatter("书名\n译者：沈锡良\n出版社：译林出版社").translator).toBe("沈锡良");
  });

  it("parses name + 译 suffix", () => {
    expect(extractFrontMatter("周泽栋 译\n作者：某某").translator).toBe("周泽栋");
  });

  it("parses author / publisher / isbn / original title", () => {
    const text = `
      原作名：The Three-Body Problem
      作者：刘慈欣
      上海译文出版社
      ISBN 978-7-5327-6888-8
    `;
    expect(extractFrontMatter(text)).toMatchObject({
      original_title: "The Three-Body Problem",
      author: "刘慈欣",
      publisher: "上海译文出版社",
      isbn: "978-7-5327-6888-8",
    });
  });

  it("ignores 编译-like false positives", () => {
    expect(extractFrontMatter("本书由某某编译完成").translator).toBeFalsy();
  });
});

describe("fillMissingMetadata", () => {
  it("only fills empty fields", () => {
    expect(
      fillMissingMetadata(
        { title: "A", author: "已知作者", translator: "" },
        { author: "文本作者", translator: "周泽栋", publisher: "译林" },
      ),
    ).toEqual({
      title: "A",
      author: "已知作者",
      translator: "周泽栋",
      publisher: "译林",
    });
  });
});

describe("htmlToPlainText", () => {
  it("strips tags and keeps line breaks", () => {
    expect(htmlToPlainText("<p>周泽栋 译</p><div>译者：沈锡良</div>")).toContain("周泽栋 译");
  });
});
