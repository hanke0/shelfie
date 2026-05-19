import { useMemo } from "react";
import { useSearchParams, Link } from "react-router-dom";
import { useListBooks } from "@/api/generated/books/books";
import { useLibrary } from "@/context/LibraryContext";
import { BookCard } from "@/components/BookCard";
import { CategorySelect } from "@/components/CategorySelect";
import styles from "./BookListPage.module.css";

export function BookListPage() {
  const [params, setParams] = useSearchParams();
  const sort = params.get("sort") ?? undefined;
  const category = params.get("category") ?? "";
  const { libraryId } = useLibrary();
  const libFromUrl = params.get("library_id");
  const activeLibraryId = libFromUrl ?? libraryId ?? undefined;

  const { data, isLoading } = useListBooks(
    {
      sort: sort ?? undefined,
      library_id: activeLibraryId,
      category: category || undefined,
      limit: 50,
    },
    { query: { enabled: !!activeLibraryId } },
  );

  const baseTitle =
    sort === "recent" ? "最近阅读" : sort === "new" ? "新书速递" : "全部图书";

  const title = useMemo(() => {
    if (!category) return baseTitle;
    return `${baseTitle} · ${category}`;
  }, [baseTitle, category]);

  const setCategory = (name: string) => {
    const next = new URLSearchParams(params);
    if (name) {
      next.set("category", name);
    } else {
      next.delete("category");
    }
    setParams(next, { replace: true });
  };

  return (
    <div className="app-shell">
      <Link to="/" className={styles.back}>
        ← 返回首页
      </Link>
      <h1 className={styles.title}>{title}</h1>

      {!activeLibraryId && <p>请先在顶栏选择图书馆</p>}

      {activeLibraryId && (
        <div className={styles.toolbar}>
          <label className={styles.filter}>
            <span className={styles.filterLabel}>分类</span>
            <CategorySelect
              libraryId={activeLibraryId}
              value={category}
              onChange={setCategory}
              includeAllOption
            />
          </label>
        </div>
      )}

      {isLoading && activeLibraryId && <p>加载中…</p>}
      {!isLoading && activeLibraryId && data?.length === 0 && (
        <p className={styles.empty}>该分类下暂无图书</p>
      )}
      <div className="book-scroll">
        {data?.map((book) => (
          <BookCard key={book.id} book={book} />
        ))}
      </div>
    </div>
  );
}
