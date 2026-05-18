import { useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { useGetHome, getGetHomeQueryKey } from "@/api/generated/home/home";
import { useLibrary } from "@/context/LibraryContext";
import { Header } from "@/components/Header";
import { BookSection } from "@/components/BookSection";
import { UploadModal } from "@/components/UploadModal";

export function HomePage() {
  const [search, setSearch] = useState("");
  const [uploadOpen, setUploadOpen] = useState(false);
  const [ahaSeed, setAhaSeed] = useState<number | undefined>();
  const qc = useQueryClient();

  const { libraryId } = useLibrary();
  const { data, isLoading, error } = useGetHome(
    { library_id: libraryId ?? undefined, limit: 12, seed: ahaSeed },
    { query: { enabled: !!libraryId } },
  );

  const refreshAha = () => {
    setAhaSeed(Date.now());
    qc.invalidateQueries({
      queryKey: getGetHomeQueryKey({ library_id: libraryId ?? undefined, limit: 12, seed: ahaSeed }),
    });
  };

  return (
    <div className="app-shell">
      <Header
        search={search}
        onSearchChange={setSearch}
        onUploadClick={() => setUploadOpen(true)}
      />

      {!libraryId && <p>请先在顶栏选择图书馆</p>}
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
              <button type="button" className="btn btn-ghost" onClick={refreshAha}>
                换一批
              </button>
            }
          />
        </>
      )}

      <UploadModal open={uploadOpen} onClose={() => setUploadOpen(false)} />
    </div>
  );
}
