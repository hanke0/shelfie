import type { BookMetadata } from "@/api/generated/models";
import { normalizePublishDate } from "@/lib/opf-metadata";

/** Fill only empty fields on `base` from `extra`. */
export function fillMissingMetadata(
  base: Partial<BookMetadata>,
  extra: Partial<BookMetadata>,
): Partial<BookMetadata> {
  const out: Partial<BookMetadata> = { ...base };
  for (const [key, value] of Object.entries(extra) as [keyof BookMetadata, BookMetadata[keyof BookMetadata]][]) {
    if (value == null || value === "") continue;
    const current = out[key];
    if (current == null || current === "") {
      (out as Record<string, unknown>)[key] = value;
    }
  }
  return out;
}

/**
 * Infer common bibliographic fields from front-matter / copyright-page text
 * (typically the first pages of a book).
 */
export function extractFrontMatter(text: string): Partial<BookMetadata> {
  const normalized = text
    .replace(/\u00a0/g, " ")
    .replace(/\u3000/g, " ")
    .replace(/[–—－]/g, "-")
    .replace(/[ \t]+/g, " ")
    .replace(/\r\n?/g, "\n");

  const out: Partial<BookMetadata> = {};
  const take = (key: keyof BookMetadata, value: string | undefined) => {
    if (!value?.trim()) return;
    if (out[key] == null || out[key] === "") {
      (out as Record<string, unknown>)[key] = value.trim();
    }
  };

  // 1) Explicit labeled fields (书名：/著者：/出版时间：…)
  take("title", matchLabeled(normalized, /书名|书\s*名/));
  take("author", matchLabeled(normalized, /著者|作者/));
  take("translator", findTranslator(normalized));
  take(
    "publisher",
    cleanPublisher(
      matchLabeled(normalized, /出版发行|出版社|出版者|出版公司|出版集团|出版/) ??
        firstLineMatch(normalized, /^(?!出版)(.{2,40}出版社)\s*$/m) ??
        "",
    ),
  );
  take(
    "publish_date",
    normalizePublishDate(
      matchLabeled(normalized, /出版(?:时间|日期|年月)|出版年|版\s*次/) ?? "",
    ),
  );
  take(
    "original_title",
    matchLabeled(
      normalized,
      /书名原文|原(?:作|书)?名|原题|英文(?:书)?名|Original\s*Title/i,
    ) ?? firstLineMatch(normalized, /^原(?:作|书)?名[：:]\s*(.+)$/m),
  );

  // 2) 版权页：「甲 著 乙 译」/ 出版发行 / 版次（完整署名优先于 CIP 简写）
  const credit = parseAuthorTranslatorLine(normalized);
  take("author", credit.author);
  take("translator", credit.translator);
  take("author", matchSuffixRole(normalized, "著"));
  take("publisher", cleanPublisher(matchSpacedLabel(normalized, /出版\s*发行|发\s*行/) ?? ""));
  take("publish_date", normalizePublishDate(matchSpacedLabel(normalized, /版\s*次/) ?? ""));

  // 3) CIP 数据行：题名／责任者．—出版地：出版者，出版年
  const cip = parseCipLine(normalized);
  take("title", cip.title);
  take("author", cip.author);
  take("translator", cip.translator);
  take("publisher", cip.publisher);
  take("publish_date", cip.publish_date);

  const isbn = normalized.match(/ISBN(?:-1[03])?[：:\s]*([\dXx-]{10,22})/i)?.[1];
  if (isbn) take("isbn", isbn.replace(/\s/g, ""));

  return out;
}

/** 图书在版编目（CIP）责任说明行. */
function parseCipLine(text: string): Partial<BookMetadata> {
  // 信号与噪声 /（美）西尔弗著；胡晓姣，张新，朱辰辰译 .—北京：中信出版社，2013.8
  const m = text.match(
    /(?:^|\n)\s*([^\n／/]{1,80}?)\s*[／/]\s*([^\n]{2,160}?)\s*[．.]\s*[—–\-－]+\s*([^\n]{3,160})/,
  );
  if (!m) return {};

  const title = cleanPerson(m[1] ?? "");
  const responsibility = (m[2] ?? "").trim();
  const publication = (m[3] ?? "").trim();

  const out: Partial<BookMetadata> = {};
  if (title && !/^(图书在版编目|CIP|Ⅰ|Ⅱ|Ⅲ|Ⅳ|I\.|II\.|III\.|IV\.)/i.test(title)) {
    out.title = title;
  }

  const authors: string[] = [];
  const translators: string[] = [];
  for (const part of responsibility.split(/[；;]/)) {
    const chunk = part.trim();
    if (!chunk) continue;
    if (/译\s*$/.test(chunk)) {
      pushPeople(translators, chunk.replace(/译\s*$/, ""));
    } else if (/[著编撰]\s*$/.test(chunk)) {
      pushPeople(authors, chunk.replace(/[著编撰]\s*$/, ""));
    }
  }
  if (authors.length) out.author = uniqueJoin(authors);
  if (translators.length) out.translator = uniqueJoin(translators);

  // 北京：中信出版社，2013.8
  const pub = publication.match(
    /^(?:[^：:]{1,20}[：:])?\s*([^，,]{2,80}?)\s*[，,]\s*(\d{4}(?:\s*[.\-／/年]\s*\d{1,2}(?:\s*月\s*\d{1,2}\s*日?|[.\-／/]\d{1,2})?)?)/,
  );
  if (pub) {
    out.publisher = cleanPublisher(pub[1] ?? "");
    out.publish_date = normalizePublishDate(pub[2] ?? "");
  }

  return out;
}

/** 「［奥］斯蒂芬·茨威格 著 韩耀成 沈锡良 译」 */
function parseAuthorTranslatorLine(text: string): Partial<BookMetadata> {
  const m = text.match(
    /(?:^|\n)\s*([^\n]{2,60}?)\s+著\s+([^\n]{2,60}?)\s+译\s*(?=\n|$)/,
  );
  if (!m) return {};
  const author = cleanPerson(m[1] ?? "");
  const translators: string[] = [];
  pushPeople(translators, m[2] ?? "");
  return {
    author: author || undefined,
    translator: uniqueJoin(translators) || undefined,
  };
}

const NEXT_FIELD =
  /策划(?:推广)?|出版发行|出版社|出版者|著者|作者|译者|责任编辑|特约编辑|助理编辑|封面设计|版\s*次|印\s*次|开\s*本|印\s*张|字\s*数|书\s*号|定\s*价|ISBN|承\s*印/;

function findTranslator(text: string): string {
  const found: string[] = [];

  for (const m of text.matchAll(/(?:译者|翻\s*译)\s*[：:]\s*([^\n。]{1,120})/g)) {
    pushPeople(found, cutAtNextField(m[1] ?? ""));
  }

  for (const m of text.matchAll(
    /(?:^|[\n\r]|[（(【\[［])\s*([^\n（）()【】\[\]［］：:著]{1,40}?)\s+译\s*(?:$|[\n\r]|[）)】\]］])/gm,
  )) {
    const name = m[1]?.trim() ?? "";
    if (!name || /^(编译|译注|译者|翻译|不可|详见)/.test(name)) continue;
    if (/[著编撰选校注评]$/.test(name)) continue;
    pushPeople(found, name);
  }

  return uniqueJoin(found);
}

function cutAtNextField(raw: string): string {
  const parts = raw.split(new RegExp(`\\s+(?=${NEXT_FIELD.source})`));
  return parts[0]?.trim() ?? raw.trim();
}

function matchLabeled(text: string, label: RegExp): string | undefined {
  const re = new RegExp(
    `(?:${label.source})\\s*[：:]\\s*([^\\n]{1,100})`,
    label.flags.includes("i") ? "gi" : "g",
  );
  for (const m of text.matchAll(re)) {
    const value = cleanPerson(cutAtNextField(m[1] ?? ""));
    if (value) return value;
  }
  return undefined;
}

/** 版权页常见「标签 + 可选冒号 + 值」. */
function matchSpacedLabel(text: string, label: RegExp): string | undefined {
  const re = new RegExp(`(?:${label.source})\\s*[：:]?\\s+([^\\n]{1,80})`, "g");
  for (const m of text.matchAll(re)) {
    const value = cleanPerson(cutAtNextField(m[1] ?? ""));
    if (!value) continue;
    if (NEXT_FIELD.test(value) && !/^\d{4}/.test(value)) continue;
    return value;
  }
  return undefined;
}

function matchSuffixRole(text: string, role: string): string | undefined {
  const re = new RegExp(
    `(?:^|[\\n\\r]|[（(【\\[［])\\s*([^\\n（）()【】\\[\\]［］：:]{1,40}?)(?:\\s+|◎)${role}\\s*(?:$|[\\n\\r]|[）)】\\]］])`,
    "gm",
  );
  for (const m of text.matchAll(re)) {
    const value = cleanPerson(m[1] ?? "");
    if (value && !/^(作者|编者|编)$/.test(value)) return value;
  }
  return undefined;
}

function firstLineMatch(text: string, re: RegExp): string | undefined {
  const m = text.match(re);
  return m?.[1]?.trim() || undefined;
}

function pushPeople(into: string[], raw: string) {
  let parts = raw.split(/[/／、,，;；|｜]+/).map((p) => p.trim()).filter(Boolean);
  if (parts.length === 1 && /\s/.test(parts[0]!)) {
    const tokens = parts[0]!.split(/\s+/).filter(Boolean);
    if (tokens.length >= 2 && tokens.every((t) => /^[\u4e00-\u9fff·•]{2,4}$/.test(t))) {
      parts = tokens;
    }
  }
  for (const part of parts) {
    const name = cleanPerson(part);
    if (name) into.push(name);
  }
}

function cleanPerson(raw: string): string {
  return raw
    .replace(/^[\s·•.．、，,；;：:]+/, "")
    .replace(/[\s·•.．、，,；;：:]+$/, "")
    .replace(/\s+/g, " ")
    .trim();
}

function cleanPublisher(raw: string): string {
  return cleanPerson(raw)
    .replace(/^(出版发行|出版社|出版者|出版公司|出版|发行)[：:\s]*/, "")
    .replace(/\s*[（(].*$/, "")
    .trim();
}

function uniqueJoin(values: string[]): string {
  const seen = new Set<string>();
  const out: string[] = [];
  for (const v of values) {
    if (seen.has(v)) continue;
    seen.add(v);
    out.push(v);
  }
  return out.join(", ");
}

/** Strip tags / scripts from HTML-ish ebook markup for front-matter scanning. */
export function htmlToPlainText(html: string): string {
  return html
    .replace(/<script[\s\S]*?<\/script>/gi, " ")
    .replace(/<style[\s\S]*?<\/style>/gi, " ")
    .replace(/<br\s*\/?>/gi, "\n")
    .replace(/<\/p>/gi, "\n")
    .replace(/<\/div>/gi, "\n")
    .replace(/<\/h[1-6]>/gi, "\n")
    .replace(/<[^>]+>/g, " ")
    .replace(/&nbsp;/gi, " ")
    .replace(/&amp;/gi, "&")
    .replace(/&lt;/gi, "<")
    .replace(/&gt;/gi, ">")
    .replace(/&#(\d+);/g, (_, n) => String.fromCharCode(Number(n)))
    .replace(/&quot;/gi, '"');
}
