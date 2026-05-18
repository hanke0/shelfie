import { useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { getGetHomeQueryKey } from "@/api/generated/home/home";
import { useLibrary } from "@/context/LibraryContext";
import { uploadBookMultipart } from "@/lib/upload";
import {
  emptyMetadata,
  metadataForUpload,
  MetadataFormFields,
} from "@/components/MetadataFormFields";
import { FieldLabel } from "@/components/ui/FieldLabel";
import styles from "./UploadModal.module.css";

interface UploadModalProps {
  open: boolean;
  onClose: () => void;
}

export function UploadModal({ open, onClose }: UploadModalProps) {
  const qc = useQueryClient();
  const { libraryId, library } = useLibrary();
  const [category, setCategory] = useState("未分类");
  const [metadata, setMetadata] = useState(emptyMetadata);
  const [file, setFile] = useState<File | null>(null);
  const [cover, setCover] = useState<File | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const resetForm = () => {
    setCategory("未分类");
    setMetadata(emptyMetadata());
    setFile(null);
    setCover(null);
    setError(null);
  };

  if (!open) return null;

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!libraryId || !file) {
      setError("请选择图书馆和图书文件");
      return;
    }
    const payload = metadataForUpload(metadata, category, file);
    if (!payload.title) {
      setError("请填写书名");
      return;
    }
    setLoading(true);
    setError(null);
    try {
      await uploadBookMultipart(libraryId, category, file, payload, cover);
      await qc.invalidateQueries({
        queryKey: getGetHomeQueryKey({ library_id: libraryId, limit: 12 }),
      });
      resetForm();
      onClose();
    } catch (err) {
      setError(err instanceof Error ? err.message : "上传失败");
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className={styles.overlay} onClick={onClose}>
      <form
        className={styles.modal}
        onClick={(e) => e.stopPropagation()}
        onSubmit={handleSubmit}
      >
        <h2>上传图书</h2>
        <p className={styles.libraryHint}>
          当前图书馆：<strong>{library?.name ?? "未选择"}</strong>
        </p>
        {error && <p className={styles.error}>{error}</p>}

        <div className={styles.body}>
          <MetadataFormFields
            metadata={metadata}
            onChange={setMetadata}
            category={category}
            onCategoryChange={setCategory}
          />

          <div className={styles.fileSection}>
            <label>
              <FieldLabel required>图书文件 (PDF / EPUB / MOBI)</FieldLabel>
              <input
                type="file"
                accept=".pdf,.epub,.mobi"
                onChange={(e) => setFile(e.target.files?.[0] ?? null)}
                required
              />
            </label>

            <label>
              <FieldLabel>封面 (JPG / PNG，可选)</FieldLabel>
              <input
                type="file"
                accept=".jpg,.jpeg,.png"
                onChange={(e) => setCover(e.target.files?.[0] ?? null)}
              />
              <span className={styles.hint}>
                未上传时由前端根据书名{metadata.author?.trim() ? "与作者" : ""}生成展示封面（不保存到服务器）
              </span>
            </label>
          </div>
        </div>

        <div className={styles.actions}>
          <button type="button" className="btn btn-ghost" onClick={onClose}>
            取消
          </button>
          <button type="submit" className="btn" disabled={loading || !libraryId}>
            {loading ? "上传中…" : "上传"}
          </button>
        </div>
      </form>
    </div>
  );
}
