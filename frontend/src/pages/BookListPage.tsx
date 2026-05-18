import { useSearchParams, Link } from "react-router-dom";
import { useListBooks } from "@/api/generated/books/books";
import { useLibrary } from "@/context/LibraryContext";
import { BookCard } from "@/components/BookCard";

export function BookListPage() {
  const [params] = useSearchParams();
  const sort = params.get("sort") ?? undefined;
  const { libraryId } = useLibrary();
  const libFromUrl = params.get("library_id");
  const activeLibraryId = libFromUrl ?? libraryId ?? undefined;

  const { data, isLoading } = useListBooks(
    { sort: sort ?? undefined, library_id: activeLibraryId, limit: 50 },
    { query: { enabled: !!activeLibraryId } },
  );

  const title = sort === "recent" ? "最近阅读" : sort === "new" ? "新书速递" : "全部图书";

  return (
    <div className="app-shell">
      <Link to="/" style={{ display: "inline-block", margin: "1rem 0", color: "var(--accent)" }}>
        ← 返回首页
      </Link>
      <h1 style={{ fontFamily: "var(--font-display)" }}>{title}</h1>
      {!activeLibraryId && <p>请先在顶栏选择图书馆</p>}
      {isLoading && activeLibraryId && <p>加载中…</p>}
      <div className="book-scroll" style={{ gridTemplateRows: "auto" }}>
        {data?.map((book) => (
          <BookCard key={book.id} book={book} />
        ))}
      </div>
    </div>
  );
}
