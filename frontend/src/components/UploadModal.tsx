import { useEffect, useMemo, useState } from "react";
import { useNavigate } from "react-router-dom";
import { useQueryClient } from "@tanstack/react-query";
import type { BookMetadata } from "@/api/generated/models";
import { getGetHomeQueryKey } from "@/api/generated/home/home";
import { useLibrary } from "@/context/LibraryContext";
import { uploadBookMultipart } from "@/lib/upload";
import {
  formatFileSize,
  prepareMetadataForUpload,
  type UploadDraft,
} from "@/lib/book-upload-state";
import { FieldLabel } from "@/components/ui/FieldLabel";
import { CategorySelect } from "@/components/CategorySelect";
import { LanguageCombobox } from "@/components/ui/LanguageCombobox";
import { RatingPicker } from "@/components/ui/RatingPicker";
import styles from "./UploadModal.module.css";
import { useApiAction } from "@/hooks/useApiAction";

interface UploadModalProps {
  draft: UploadDraft | null;
  onClose: () => void;
}

const META_FIELDS: { key: keyof BookMetadata; label: string }[] = [
  { key: "title", label: "书名" },
  { key: "author", label: "作者" },
  { key: "translator", label: "译者" },
  { key: "publisher", label: "出版社" },
  { key: "isbn", label: "ISBN" },
  { key: "original_title", label: "原作名" },
  { key: "series", label: "丛书" },
  { key: "notes", label: "备注" },
];

export function UploadModal({ draft, onClose }: UploadModalProps) {
  const navigate = useNavigate();
  const qc = useQueryClient();
  const run = useApiAction();
  const { libraryId, library } = useLibrary();
  const [metaForm, setMetaForm] = useState<BookMetadata | null>(null);
  const [cover, setCover] = useState<File | undefined>();
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const coverPreview = useMemo(() => {
    if (!cover) return null;
    return URL.createObjectURL(cover);
  }, [cover]);

  useEffect(() => {
    if (!coverPreview) return;
    return () => URL.revokeObjectURL(coverPreview);
  }, [coverPreview]);

  useEffect(() => {
    if (draft?.phase === "form") {
      setMetaForm(draft.metadata);
      setCover(draft.cover);
      setError(draft.extractError ?? null);
      return;
    }
    setMetaForm(null);
    setCover(undefined);
    setError(null);
  }, [draft]);

  if (!draft) return null;

  const handleClose = () => {
    if (loading) return;
    onClose();
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (draft.phase !== "form" || !libraryId || !metaForm) {
      setError("请填写完整信息");
      return;
    }
    if (!metaForm.title.trim()) {
      setError("书名不能为空");
      return;
    }

    setLoading(true);
    setError(null);

    const metadata = prepareMetadataForUpload(metaForm);
    const category = metadata.category ?? "未分类";
    const ok = await run(
      async () => {
        const detail = await uploadBookMultipart(
          libraryId,
          category,
          draft.file,
          metadata,
          cover,
        );

        await qc.invalidateQueries({
          queryKey: getGetHomeQueryKey({ library_id: libraryId, limit: 12 }),
        });
        await qc.invalidateQueries({ queryKey: ["/libraries", libraryId, "categories"] });

        navigate(`/books/${detail.id}`);
      },
      { errorMessage: "上传失败" },
    );

    setLoading(false);
    if (ok) {
      onClose();
    }
  };

  if (draft.phase === "extracting") {
    return (
      <div className={styles.overlay} onClick={handleClose}>
        <div className={styles.modal} onClick={(e) => e.stopPropagation()}>
          <h2>上传图书</h2>
          <p className={styles.libraryHint}>
            当前图书馆：<strong>{library?.name ?? "未选择"}</strong>
          </p>
          <p className={styles.extracting}>正在从本地文件识别元数据…</p>
          <div className={styles.fileReadonly}>
            <span className={styles.fileName}>{draft.file.name}</span>
            <span className={styles.fileMeta}>{formatFileSize(draft.file.size)}</span>
          </div>
          <div className={styles.actions}>
            <button type="button" className="btn btn-ghost" onClick={handleClose}>
              取消
            </button>
          </div>
        </div>
      </div>
    );
  }

  if (!metaForm) return null;

  return (
    <div className={styles.overlay} onClick={handleClose}>
      <form
        className={styles.modal}
        onClick={(e) => e.stopPropagation()}
        onSubmit={handleSubmit}
      >
        <h2>确认上传</h2>
        <p className={styles.libraryHint}>
          当前图书馆：<strong>{library?.name ?? "未选择"}</strong>
        </p>
        <p className={styles.hint}>元数据已从本地文件识别，请确认或修改后上传。</p>
        {error && <p className={styles.error}>{error}</p>}

        <div className={styles.body}>
          <div className={styles.fileSection}>
            <FieldLabel>图书文件</FieldLabel>
            <div className={styles.fileReadonly}>
              <span className={styles.fileName}>{draft.file.name}</span>
              <span className={styles.fileMeta}>{formatFileSize(draft.file.size)}</span>
            </div>
          </div>

          {coverPreview && (
            <div className={styles.coverPreview}>
              <FieldLabel>识别到的封面</FieldLabel>
              <img src={coverPreview} alt="封面预览" />
            </div>
          )}

          <div className={styles.metadataSection}>
            <div className={styles.formGrid}>
              {META_FIELDS.slice(0, 2).map(({ key, label }) => (
                <label key={key} className={styles.field}>
                  <FieldLabel required={key === "title"}>{label}</FieldLabel>
                  <input
                    value={String(metaForm[key] ?? "")}
                    onChange={(e) => setMetaForm({ ...metaForm, [key]: e.target.value })}
                    required={key === "title"}
                  />
                </label>
              ))}

              <label className={styles.field}>
                <FieldLabel required>分类</FieldLabel>
                {libraryId ? (
                  <CategorySelect
                    libraryId={libraryId}
                    value={metaForm.category ?? "未分类"}
                    onChange={(name) => setMetaForm({ ...metaForm, category: name })}
                    required
                  />
                ) : (
                  <input value={metaForm.category ?? "未分类"} readOnly />
                )}
              </label>

              <label className={styles.field}>
                <FieldLabel>语言 (ISO 639-1)</FieldLabel>
                <LanguageCombobox
                  value={metaForm.language ?? ""}
                  onChange={(code) => setMetaForm({ ...metaForm, language: code })}
                />
              </label>

              {META_FIELDS.slice(2).map(({ key, label }) => (
                <label
                  key={key}
                  className={`${styles.field} ${key === "notes" ? styles.fieldWide : ""}`}
                >
                  <FieldLabel>{label}</FieldLabel>
                  {key === "notes" ? (
                    <textarea
                      rows={3}
                      value={String(metaForm[key] ?? "")}
                      onChange={(e) => setMetaForm({ ...metaForm, [key]: e.target.value })}
                    />
                  ) : (
                    <input
                      value={String(metaForm[key] ?? "")}
                      onChange={(e) => setMetaForm({ ...metaForm, [key]: e.target.value })}
                    />
                  )}
                </label>
              ))}

              <label className={styles.field}>
                <FieldLabel>出版日期</FieldLabel>
                <input
                  type="month"
                  value={metaForm.publish_date ?? ""}
                  onChange={(e) =>
                    setMetaForm({ ...metaForm, publish_date: e.target.value })
                  }
                />
              </label>

              <label className={styles.field}>
                <FieldLabel>页数</FieldLabel>
                <input
                  type="number"
                  min={1}
                  value={metaForm.page_count ?? ""}
                  onChange={(e) =>
                    setMetaForm({
                      ...metaForm,
                      page_count: e.target.value ? Number(e.target.value) : undefined,
                    })
                  }
                />
              </label>

              <label className={styles.field}>
                <FieldLabel>评分</FieldLabel>
                <RatingPicker
                  value={metaForm.rating}
                  onChange={(rating) => setMetaForm({ ...metaForm, rating })}
                />
              </label>
            </div>
          </div>
        </div>

        <div className={styles.actions}>
          <button type="button" className="btn btn-ghost" onClick={handleClose} disabled={loading}>
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
