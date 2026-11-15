import { Suspense, useCallback, useRef, useState } from "react";
import { Outlet } from "react-router-dom";
import { useLibrary } from "@/context/LibraryContext";
import { Header } from "@/components/Header";
import { UploadModal } from "@/components/UploadModal";
import { extractBookMetadataLocal } from "@/lib/extract-book-metadata";
import {
  isAcceptedBookFile,
  metadataFormFromExtract,
  type UploadDraft,
} from "@/lib/book-upload-state";
import { useApiAction } from "@/hooks/useApiAction";
import styles from "./AppLayout.module.css";

const BOOK_ACCEPT = ".pdf,.epub,.mobi";

export function AppLayout() {
  const [uploadDraft, setUploadDraft] = useState<UploadDraft | null>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);
  const run = useApiAction();
  const { libraryId } = useLibrary();

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

  const handleFileInputChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    e.target.value = "";
    if (file) void beginUpload(file);
  };

  return (
    <>
      <div className={styles.bar}>
        <div className={styles.inner}>
          <Header
            onUploadClick={() => fileInputRef.current?.click()}
            onUploadDrop={(file) => void beginUpload(file)}
          />
        </div>
      </div>
      <input
        ref={fileInputRef}
        type="file"
        accept={BOOK_ACCEPT}
        hidden
        onChange={handleFileInputChange}
      />
      <Suspense
        fallback={
          <div className="app-shell">
            <p>加载中…</p>
          </div>
        }
      >
        <Outlet />
      </Suspense>
      <UploadModal draft={uploadDraft} onClose={() => setUploadDraft(null)} />
    </>
  );
}
