import type { BookMetadata } from "@/api/generated/models";
import { FieldLabel } from "@/components/ui/FieldLabel";
import styles from "./MetadataFormFields.module.css";

export const emptyMetadata = (): BookMetadata => ({
  title: "",
  author: "",
  language: "",
  translator: "",
  publisher: "",
  isbn: "",
  original_title: "",
  series: "",
  notes: "",
});

export function metadataForUpload(
  metadata: BookMetadata,
  category: string,
  file: File,
): BookMetadata {
  const title = metadata.title?.trim() || file.name.replace(/\.[^.]+$/i, "");
  return {
    ...metadata,
    title,
    author: metadata.author?.trim() ?? "",
    language: metadata.language?.trim() ?? "",
    translator: metadata.translator?.trim() ?? "",
    publisher: metadata.publisher?.trim() ?? "",
    isbn: metadata.isbn?.trim() ?? "",
    original_title: metadata.original_title?.trim() ?? "",
    series: metadata.series?.trim() ?? "",
    notes: metadata.notes?.trim() ?? "",
    category: category.trim(),
    publish_date: metadata.publish_date?.trim() ?? "",
    page_count: metadata.page_count,
  };
}

interface MetadataFormFieldsProps {
  metadata: BookMetadata;
  onChange: (metadata: BookMetadata) => void;
  category: string;
  onCategoryChange: (category: string) => void;
}

export function MetadataFormFields({
  metadata,
  onChange,
  category,
  onCategoryChange,
}: MetadataFormFieldsProps) {
  const set =
    (key: keyof BookMetadata) =>
    (e: React.ChangeEvent<HTMLInputElement | HTMLTextAreaElement>) => {
      onChange({ ...metadata, [key]: e.target.value });
    };

  const setPageCount = (e: React.ChangeEvent<HTMLInputElement>) => {
    const v = e.target.value;
    onChange({
      ...metadata,
      page_count: v === "" ? undefined : Number(v),
    });
  };

  return (
    <div className={styles.grid}>
      <label>
        <FieldLabel required>分类（文件夹名）</FieldLabel>
        <input value={category} onChange={(e) => onCategoryChange(e.target.value)} required />
      </label>

      <label>
        <FieldLabel required>书名</FieldLabel>
        <input
          value={metadata.title ?? ""}
          onChange={set("title")}
          placeholder="用于文件名：书名_作者"
        />
      </label>

      <label>
        <FieldLabel>作者</FieldLabel>
        <input value={metadata.author ?? ""} onChange={set("author")} />
      </label>

      <label>
        <FieldLabel>语言</FieldLabel>
        <input
          value={metadata.language ?? ""}
          onChange={set("language")}
          placeholder="如：中文、English、日本語"
        />
      </label>

      <label>
        <FieldLabel>译者</FieldLabel>
        <input value={metadata.translator ?? ""} onChange={set("translator")} />
      </label>

      <label>
        <FieldLabel>出版社</FieldLabel>
        <input value={metadata.publisher ?? ""} onChange={set("publisher")} />
      </label>

      <label>
        <FieldLabel>出版日期</FieldLabel>
        <input
          type="month"
          value={metadata.publish_date ?? ""}
          onChange={set("publish_date")}
          placeholder="YYYY-MM"
        />
      </label>

      <label>
        <FieldLabel>ISBN</FieldLabel>
        <input value={metadata.isbn ?? ""} onChange={set("isbn")} />
      </label>

      <label>
        <FieldLabel>页数</FieldLabel>
        <input
          type="number"
          value={metadata.page_count ?? ""}
          onChange={setPageCount}
        />
      </label>

      <label>
        <FieldLabel>原作名</FieldLabel>
        <input value={metadata.original_title ?? ""} onChange={set("original_title")} />
      </label>

      <label>
        <FieldLabel>丛书</FieldLabel>
        <input value={metadata.series ?? ""} onChange={set("series")} />
      </label>

      <label className={styles.fullWidth}>
        <FieldLabel>备注</FieldLabel>
        <textarea value={metadata.notes ?? ""} onChange={set("notes")} rows={2} />
      </label>
    </div>
  );
}
