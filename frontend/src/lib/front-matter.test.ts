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

  it("parses copyright-page style Chinese front matter", () => {
    const text = `
书名：一如既往：不变的人性与致富心态
作者：【美】摩根·豪泽尔
译者：宋红波；符李桃
出版社：中信出版集团
出版时间：2024年4月
ISBN：9787521764222
字数：91千字
    `;
    expect(extractFrontMatter(text)).toMatchObject({
      title: "一如既往：不变的人性与致富心态",
      author: "【美】摩根·豪泽尔",
      translator: "宋红波, 符李桃",
      publisher: "中信出版集团",
      publish_date: "2024-04",
      isbn: "9787521764222",
    });
  });

  it("parses CIP and colophon copyright page", () => {
    const text = `
图书代号SK12N0115

图书在版编目（CIP）数据

恐惧／（奥）茨威格（Zweig，S.）著；韩耀成，沈锡良译．—西安：陕西师范大学出版总社有限公司，2013.3
ISBN 978-7-5613-6018-7

Ⅰ．①恐…　Ⅱ．①茨…　②韩…　③沈…　Ⅲ．①中篇小说—小说集—奥地利—现代　Ⅳ．①I521.45

中国版本图书馆CIP数据核字（2012）第047332号

恐　惧

［奥］斯蒂芬·茨威格　著　韩耀成　沈锡良　译

出版发行　陕西师范大学出版总社有限公司
　　　　　　　　　（西安市长安南路199号　邮编　710062）

版　　次　2013年3月第1版
印　　次　2013年3月第1次印刷
    `;
    expect(extractFrontMatter(text)).toMatchObject({
      title: "恐惧",
      author: "［奥］斯蒂芬·茨威格",
      translator: "韩耀成, 沈锡良",
      publisher: "陕西师范大学出版总社有限公司",
      publish_date: "2013-03",
      isbn: "978-7-5613-6018-7",
    });
  });

  it("parses CITIC-style CIP with 书名原文 / 著者 / en-dash ISBN", () => {
    const text = `
图书在版编目（CIP）数据

信号与噪声 /（美）西尔弗著；胡晓姣，张新，朱辰辰译 .—北京：中信出版社，2013.8

书名原文：The Signal and the Noise

ISBN 978–7–5086–4114–0

信号与噪声

著者：[ 美 ] 纳特 · 西尔弗

译者：胡晓姣; 张新; 朱辰辰 策划推广：中信出版社（China CITIC Press） 出版发行：中信出版集团股份有限公司

（北京市朝阳区惠新东街甲 4 号富盛大厦 2 座; 邮编 ;100029）

版 次：2013 年 8 月第 1 版;;

印 次：2013 年 8 月第 1 次印刷

书 号：ISBN 978–7–5086–4114–0/F·2968
    `;
    expect(extractFrontMatter(text)).toMatchObject({
      title: "信号与噪声",
      original_title: "The Signal and the Noise",
      author: "[ 美 ] 纳特 · 西尔弗",
      translator: "胡晓姣, 张新, 朱辰辰",
      publisher: "中信出版集团股份有限公司",
      publish_date: "2013-08",
      isbn: "978-7-5086-4114-0",
    });
  });

  it("parses 出版：label without keeping the 出版 prefix", () => {
    const text = `
版权信息
了不起的盖茨比
菲茨杰拉德◎著
出版：上海文艺出版社
开本：720×1280／768×1024
    `;
    expect(extractFrontMatter(text)).toMatchObject({
      author: "菲茨杰拉德",
      publisher: "上海文艺出版社",
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
