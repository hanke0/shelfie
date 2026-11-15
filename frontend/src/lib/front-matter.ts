import type { BookMetadata } from "@/api/generated/models";

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
    .replace(/[ \t]+/g, " ")
    .replace(/\r\n?/g, "\n");

  const out: Partial<BookMetadata> = {};

  const translator = findTranslator(normalized);
  if (translator) out.translator = translator;

  const author = matchLabeled(normalized, /作者/) ?? matchSuffixRole(normalized, "著");
  if (author) out.author = author;

  const publisher =
    matchLabeled(normalized, /出版(?:社|者|公司)?/) ??
    firstLineMatch(normalized, /^(.{2,40}出版社)\s*$/m);
  if (publisher) out.publisher = cleanPublisher(publisher);

  const isbn = normalized.match(/ISBN(?:-1[03])?[：:\s]*([\dXx-]{10,17})/i)?.[1];
  if (isbn) out.isbn = isbn.replace(/\s/g, "");

  const original =
    matchLabeled(normalized, /原(?:作|书)?名|原题|英文(?:书)?名|Original\s*Title/i) ??
    firstLineMatch(normalized, /^原(?:作|书)?名[：:]\s*(.+)$/m);
  if (original) out.original_title = cleanPerson(original);

  return out;
}

function findTranslator(text: string): string {
  const found: string[] = [];

  for (const m of text.matchAll(/(?:译者|翻\s*译)\s*[：:]\s*([^\n，,；;。/｜|]{1,40})/g)) {
    pushPeople(found, m[1]);
  }

  for (const m of text.matchAll(
    /(?:^|[\n\r]|[（(【\[])\s*([^\n（）()【】\[\]：:]{1,30}?)\s+译\s*(?:$|[\n\r]|[）)】\]])/gm,
  )) {
    const name = m[1]?.trim() ?? "";
    if (!name || /^(编译|译注|译者|翻译|不可|详见)/.test(name)) continue;
    if (/[著编撰选校注评]$/.test(name)) continue;
    pushPeople(found, name);
  }

  return uniqueJoin(found);
}

function matchLabeled(text: string, label: RegExp): string | undefined {
  const re = new RegExp(`(?:${label.source})\\s*[：:]\\s*([^\\n，,；;。]{1,60})`, label.flags.includes("i") ? "gi" : "g");
  for (const m of text.matchAll(re)) {
    const value = cleanPerson(m[1] ?? "");
    if (value) return value;
  }
  return undefined;
}

function matchSuffixRole(text: string, role: string): string | undefined {
  const re = new RegExp(
    `(?:^|[\\n\\r]|[（(【\\[])\\s*([^\\n（）()【】\\[\\]：:]{1,30}?)\\s+${role}\\s*(?:$|[\\n\\r]|[）)】\\]])`,
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
  for (const part of raw.split(/[/／、,，;；|｜]/)) {
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
  return cleanPerson(raw).replace(/^(出版社|出版者|出版公司)[：:\s]*/, "");
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
