import type { BookMetadata } from "@/api/generated/models";
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
        分类（文件夹名）
        <input value={category} onChange={(e) => onCategoryChange(e.target.value)} required />
      </label>

      <label>
        书名 <span className={styles.req}>*</span>
        <input
          value={metadata.title ?? ""}
          onChange={set("title")}
          placeholder="用于文件名：书名_作者"
        />
      </label>

      <label>
        作者
        <input value={metadata.author ?? ""} onChange={set("author")} />
      </label>

      <label>
        语言
        <input
          value={metadata.language ?? ""}
          onChange={set("language")}
          placeholder="如：中文、English、日本語"
        />
      </label>

      <label>
        译者
        <input value={metadata.translator ?? ""} onChange={set("translator")} />
      </label>

      <label>
        出版社
        <input value={metadata.publisher ?? ""} onChange={set("publisher")} />
      </label>

      <label>
        出版日期
        <input
          type="month"
          value={metadata.publish_date ?? ""}
          onChange={set("publish_date")}
          placeholder="YYYY-MM"
        />
      </label>

      <label>
        ISBN
        <input value={metadata.isbn ?? ""} onChange={set("isbn")} />
      </label>

      <label>
        页数
        <input
          type="number"
          value={metadata.page_count ?? ""}
          onChange={setPageCount}
        />
      </label>

      <label>
        原作名
        <input value={metadata.original_title ?? ""} onChange={set("original_title")} />
      </label>

      <label>
        丛书
        <input value={metadata.series ?? ""} onChange={set("series")} />
      </label>

      <label className={styles.fullWidth}>
        备注
        <textarea value={metadata.notes ?? ""} onChange={set("notes")} rows={2} />
      </label>
    </div>
  );
}
