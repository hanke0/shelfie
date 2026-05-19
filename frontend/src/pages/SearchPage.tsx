import { useSearchParams, Link } from "react-router-dom";
import { useSearch } from "@/api/generated/search/search";
import { useLibrary } from "@/context/LibraryContext";
import { BookCard } from "@/components/BookCard";

export function SearchPage() {
  const [params] = useSearchParams();
  const q = params.get("q") ?? "";
  const { libraryId } = useLibrary();
  const libFromUrl = params.get("library_id");
  const activeLibraryId = libFromUrl ?? libraryId ?? undefined;

  const { data, isLoading } = useSearch(
    { q, library_id: activeLibraryId, limit: 30 },
    { query: { enabled: !!q && !!activeLibraryId } },
  );

  return (
    <div className="app-shell">
      <Link to="/" style={{ display: "inline-block", margin: "1rem 0", color: "var(--accent)" }}>
        ← 返回首页
      </Link>
      <h1 style={{ fontFamily: "var(--font-display)" }}>
        搜索：{q || "（空）"}
      </h1>
      {!activeLibraryId && <p>请先在顶栏选择图书馆</p>}
      {isLoading && <p>搜索中…</p>}
      {!data?.length && !isLoading && activeLibraryId && <p>没有找到相关图书</p>}
      <div className="book-scroll">
        {data?.map((book) => (
          <BookCard key={book.id} book={book} />
        ))}
      </div>
    </div>
  );
}
