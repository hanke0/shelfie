import { useState } from "react";
import { Link } from "react-router-dom";
import { useGetHome } from "@/api/generated/home/home";
import { useLibrary } from "@/context/LibraryContext";
import { BookSection } from "@/components/BookSection";

export function HomePage() {
  const [ahaSeed, setAhaSeed] = useState<number | undefined>();
  const { libraryId, libraries } = useLibrary();
  const { data, isLoading, error } = useGetHome(
    { library_id: libraryId ?? undefined, limit: 24, seed: ahaSeed },
    { query: { enabled: !!libraryId } },
  );

  const refreshAha = () => {
    setAhaSeed(Date.now());
  };

  return (
    <div className="app-shell">
      {libraries.length === 0 && (
        <p>
          暂无图书馆，请前往{" "}
          <Link to="/admin/libraries">图书馆管理</Link> 创建。
        </p>
      )}
      {isLoading && libraryId && <p>加载中…</p>}
      {error != null && <p>加载失败：{String(error)}</p>}

      {data && (
        <>
          <BookSection
            title="最近阅读"
            books={data.recent}
            moreLink={`/books?sort=recent&library_id=${libraryId}`}
          />
          <BookSection
            title="新书速递"
            books={data.new_arrivals}
            moreLink={`/books?sort=new&library_id=${libraryId}`}
          />
          <BookSection
            title="啊哈时刻"
            books={data.aha_moment}
            action={
              <button type="button" onClick={refreshAha}>
                换一换
              </button>
            }
          />
        </>
      )}
    </div>
  );
}
