import { useCallback, useRef, useState } from "react";
import { Link } from "react-router-dom";
import { useQueryClient } from "@tanstack/react-query";
import { useGetHome, getGetHomeQueryKey } from "@/api/generated/home/home";
import { useLibrary } from "@/context/LibraryContext";
import { Header } from "@/components/Header";
import { BookSection } from "@/components/BookSection";
import { UploadModal } from "@/components/UploadModal";
import { extractBookMetadataLocal } from "@/lib/extract-book-metadata";
import {
  isAcceptedBookFile,
  metadataFormFromExtract,
  type UploadDraft,
} from "@/lib/book-upload-state";
import { useApiAction } from "@/hooks/useApiAction";

const BOOK_ACCEPT = ".pdf,.epub,.mobi";

export function HomePage() {
  const [search, setSearch] = useState("");
  const [uploadDraft, setUploadDraft] = useState<UploadDraft | null>(null);
  const [ahaSeed, setAhaSeed] = useState<number | undefined>();
  const fileInputRef = useRef<HTMLInputElement>(null);
  const qc = useQueryClient();
  const run = useApiAction();

  const { libraryId, libraries } = useLibrary();
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

  const beginUpload = useCallback(
    async (file: File) => {
      if (!libraryId) {
        await run(async () => {
          throw new Error("请先在顶栏选择图书馆");
        });
        return;
      }
      if (!isAcceptedBookFile(file)) {
        await run(async () => {
          throw new Error("仅支持 PDF、EPUB、MOBI 格式");
        });
        return;
      }

      setUploadDraft({ phase: "extracting", file });

      try {
        const extracted = await extractBookMetadataLocal(file);
        setUploadDraft({
          phase: "form",
          file,
          metadata: metadataFormFromExtract(extracted.metadata, file),
          cover: extracted.cover,
        });
      } catch {
        setUploadDraft({
          phase: "form",
          file,
          metadata: metadataFormFromExtract({}, file),
          extractError: "元数据识别失败，请手动填写",
        });
      }
    },
    [libraryId, run],
  );

  const handleUploadClick = () => {
    fileInputRef.current?.click();
  };

  const handleFileInputChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    e.target.value = "";
    if (file) void beginUpload(file);
  };

  return (
    <div className="app-shell">
      <input
        ref={fileInputRef}
        type="file"
        accept={BOOK_ACCEPT}
        hidden
        onChange={handleFileInputChange}
      />

      <Header
        search={search}
        onSearchChange={setSearch}
        onUploadClick={handleUploadClick}
        onUploadDrop={(file) => void beginUpload(file)}
      />

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
              <button type="button" className="btn btn-ghost" onClick={refreshAha}>
                换一批
              </button>
            }
          />
        </>
      )}

      <UploadModal draft={uploadDraft} onClose={() => setUploadDraft(null)} />
    </div>
  );
}
