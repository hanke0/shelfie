import { Link } from "react-router-dom";
import { useFindDuplicateBooks } from "@/api/generated/libraries/libraries";
import { DuplicateMatchKind } from "@/api/generated/models";
import { BookCard } from "@/components/BookCard";
import { useLibrary } from "@/context/LibraryContext";
import styles from "./DuplicatesPage.module.css";

function kindLabel(kind: string) {
  if (kind === DuplicateMatchKind.isbn) return "ISBN";
  return "书名";
}

export function DuplicatesPage() {
  const { libraryId, library } = useLibrary();

  const { data, isLoading, isError } = useFindDuplicateBooks(libraryId ?? "", {
    query: { enabled: !!libraryId },
  });

  const groups = data?.groups ?? [];

  return (
    <div className="app-shell">
      <Link to="/books" className={styles.back}>
        ← 返回图书列表
      </Link>
      <h1 className={styles.heading}>重复图书</h1>
      <p className={styles.hint}>
        在当前图书馆中，按相同 ISBN 或相同书名（忽略大小写与多余空格）分组展示可能重复的图书。
        {library ? ` 图书馆：${library.name}` : ""}
      </p>

      {!libraryId && <p className={styles.empty}>请先在顶栏选择图书馆</p>}
      {isLoading && libraryId && <p>扫描中…</p>}
      {isError && (
        <p className={styles.empty} role="alert">
          加载失败，请稍后重试
        </p>
      )}
      {!isLoading && !isError && libraryId && groups.length === 0 && (
        <p className={styles.empty}>未发现重复图书</p>
      )}

      {groups.map((group) => (
        <section key={`${group.kind}-${group.key}`} className={styles.group}>
          <div className={styles.groupHeader}>
            <span
              className={`${styles.badge} ${
                group.kind === DuplicateMatchKind.title ? styles.badgeTitle : ""
              }`}
            >
              {kindLabel(group.kind)}
            </span>
            <span className={styles.groupKey}>{group.key}</span>
            <span className={styles.groupCount}>{group.books.length} 本</span>
          </div>
          <div className="book-scroll">
            {group.books.map((book) => (
              <BookCard key={book.id} book={book} />
            ))}
          </div>
        </section>
      ))}
    </div>
  );
}
