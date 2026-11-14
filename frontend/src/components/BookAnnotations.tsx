import type { BookAnnotationsResponse } from "@/api/generated/models";
import styles from "./BookAnnotations.module.css";

const KO_COLORS: Record<string, string> = {
  yellow: "#f2cf57",
  red: "#e8776f",
  orange: "#f0a35e",
  green: "#8cc084",
  olive: "#a9ad5c",
  cyan: "#6cc6d4",
  blue: "#7aa5e8",
  purple: "#b48ad6",
  gray: "#b5b0a8",
};

export function annotationColor(color?: string | null): string {
  if (color?.startsWith("#")) return color;
  return KO_COLORS[color?.toLowerCase() ?? ""] ?? KO_COLORS.yellow;
}

function location(chapter?: string | null, pageno?: number | null): string {
  return [chapter, pageno != null ? `第 ${pageno} 页` : null].filter(Boolean).join(" · ");
}

export function BookAnnotations({ data }: { data?: BookAnnotationsResponse }) {
  const annotations = data?.annotations ?? [];
  const bookmarks = data?.bookmarks ?? [];

  return (
    <>
      <p className={styles.sectionLabel}>标注与笔记（{annotations.length}）</p>
      {annotations.length === 0 ? (
        <p className={styles.empty}>暂无标注。在 Readest 等 BookOrbit 客户端中划线或写笔记后会同步到这里。</p>
      ) : (
        <ul className={styles.list}>
          {annotations.map((a) => (
            <li key={a.id} className={styles.item}>
              <blockquote
                className={styles.quote}
                data-drawer={a.drawer}
                style={{ borderLeftColor: annotationColor(a.color) }}
              >
                {a.text || "（无原文）"}
              </blockquote>
              {a.note ? <p className={styles.note}>{a.note}</p> : null}
              <p className={styles.meta}>{location(a.chapter, a.pageno) || "—"}</p>
            </li>
          ))}
        </ul>
      )}

      <p className={styles.sectionLabel}>书签（{bookmarks.length}）</p>
      {bookmarks.length === 0 ? (
        <p className={styles.empty}>暂无书签。</p>
      ) : (
        <ul className={styles.list}>
          {bookmarks.map((b) => (
            <li key={b.id} className={styles.bookmark}>
              <span className={styles.bookmarkTitle}>{b.title}</span>
              <span className={styles.meta}>{location(b.chapter, b.pageno)}</span>
            </li>
          ))}
        </ul>
      )}
    </>
  );
}
