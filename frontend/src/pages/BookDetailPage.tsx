import { useCallback, useEffect, useState } from "react";
import { Link, useNavigate, useParams } from "react-router-dom";
import { useQueryClient } from "@tanstack/react-query";
import {
  useGetBook,
  useUpdateBook,
  useUpdateProgress,
  useDeleteBook,
  getGetBookQueryKey,
} from "@/api/generated/books/books";
import { fetchCoverBlob } from "@/lib/custom-fetch";
import { TitleCoverImage } from "@/components/TitleCoverImage";
import { FileInput } from "@/components/ui/FileInput";
import { updateCoverMultipart } from "@/lib/upload";
import { useConfirmTwice } from "@/context/ConfirmContext";
import { useApiAction } from "@/hooks/useApiAction";
import { displayLanguageValue } from "@/data/iso639-1";
import { CategorySelect } from "@/components/CategorySelect";
import { LanguageCombobox } from "@/components/ui/LanguageCombobox";
import { RatingPicker } from "@/components/ui/RatingPicker";
import { prepareMetadataForSave } from "@/lib/book-metadata";
import { downloadBookFile } from "@/lib/download";
import styles from "./BookDetailPage.module.css";

type ActionKind = "save" | "download" | "progress" | "cover" | "delete" | null;

export function BookDetailPage() {
  const { id = "" } = useParams();
  const navigate = useNavigate();
  const qc = useQueryClient();
  const { data: book, isLoading } = useGetBook(id);
  const updateBook = useUpdateBook();
  const updateProgress = useUpdateProgress();
  const deleteBook = useDeleteBook();
  const confirmTwice = useConfirmTwice();
  const run = useApiAction();

  const [coverSrc, setCoverSrc] = useState<string | null>(null);
  const [metaForm, setMetaForm] = useState(book?.metadata);
  const [currentPage, setCurrentPage] = useState("");
  const [percent, setPercent] = useState("");
  const [actionBusy, setActionBusy] = useState<ActionKind>(null);

  useEffect(() => {
    if (!book) return;
    setMetaForm(book.metadata);
    setCurrentPage(String(book.metadata.reading_progress?.current_page ?? ""));
    setPercent(String(book.metadata.reading_progress?.percent ?? ""));
  }, [book]);

  useEffect(() => {
    if (!id) return;
    let url: string | null = null;
    fetchCoverBlob(id)
      .then((u) => {
        url = u;
        setCoverSrc(u);
      })
      .catch(() => setCoverSrc(null));
    return () => {
      if (url) URL.revokeObjectURL(url);
    };
  }, [id]);

  const saveMetadata = useCallback(async () => {
    if (!book || !metaForm) return;
    setActionBusy("save");
    await run(
      async () => {
        const metadata = prepareMetadataForSave(metaForm, book);
        await updateBook.mutateAsync({ id, data: { metadata } });
        await qc.invalidateQueries({ queryKey: getGetBookQueryKey(id) });
        await qc.invalidateQueries({ queryKey: ["/home"] });
        setMetaForm(metadata);
      },
      { successMessage: "元数据已保存", errorMessage: "保存失败" },
    );
    setActionBusy(null);
  }, [book, metaForm, id, run, updateBook, qc]);

  const saveProgress = useCallback(async () => {
    if (!book) return;
    setActionBusy("progress");
    await run(
      async () => {
        await updateProgress.mutateAsync({
          id,
          data: {
            reading_progress: {
              current_page: currentPage ? Number(currentPage) : undefined,
              percent: percent ? Number(percent) : undefined,
            },
          },
        });
        await qc.invalidateQueries({ queryKey: getGetBookQueryKey(id) });
        await qc.invalidateQueries({ queryKey: ["/home"] });
      },
      { successMessage: "阅读进度已更新", errorMessage: "更新失败" },
    );
    setActionBusy(null);
  }, [book, id, run, updateProgress, qc, currentPage, percent]);

  const handleDelete = useCallback(async () => {
    if (!metaForm) return;
    if (
      !(await confirmTwice(
        `确定删除《${metaForm.title}》？`,
        "再次确认：删除后图书文件与记录将无法恢复，确定继续吗？",
      ))
    ) {
      return;
    }
    setActionBusy("delete");
    const ok = await run(
      async () => {
        await deleteBook.mutateAsync({ id });
        qc.removeQueries({ queryKey: getGetBookQueryKey(id) });
        await qc.invalidateQueries({ queryKey: ["/home"] });
        await qc.invalidateQueries({ queryKey: ["/books"] });
      },
      { errorMessage: "删除失败" },
    );
    setActionBusy(null);
    if (ok) navigate("/");
  }, [metaForm, confirmTwice, run, deleteBook, id, qc, navigate]);

  const handleCoverChange = useCallback(
    async (file: File) => {
      setActionBusy("cover");
      await run(
        async () => {
          await updateCoverMultipart(id, file);
          await qc.invalidateQueries({ queryKey: getGetBookQueryKey(id) });
          const u = await fetchCoverBlob(id);
          setCoverSrc((prev) => {
            if (prev) URL.revokeObjectURL(prev);
            return u;
          });
        },
        { successMessage: "封面已更新", errorMessage: "更换封面失败" },
      );
      setActionBusy(null);
    },
    [id, run, qc],
  );

  const handleDownload = useCallback(async () => {
    if (!metaForm) return;
    setActionBusy("download");
    await run(
      () => downloadBookFile(id, metaForm.title),
      { successMessage: "下载已开始", errorMessage: "下载失败" },
    );
    setActionBusy(null);
  }, [id, metaForm, run]);

  if (isLoading || !book || !metaForm) {
    return (
      <div className="app-shell">
        <p>加载中…</p>
      </div>
    );
  }

  const canEdit = book.permissions?.can_edit;
  const canDelete = book.permissions?.can_delete;
  const isBusy = actionBusy !== null;

  const fields: { key: keyof typeof metaForm; label: string }[] = [
    { key: "title", label: "书名" },
    { key: "author", label: "作者" },
    { key: "translator", label: "译者" },
    { key: "publisher", label: "出版社" },
    { key: "isbn", label: "ISBN" },
    { key: "original_title", label: "原作名" },
    { key: "series", label: "丛书" },
    { key: "notes", label: "备注" },
  ];

  return (
    <div className={`app-shell ${styles.page}`}>
      <Link to="/" className={styles.back}>
        ← 返回首页
      </Link>

      <div className={styles.layout}>
        <div className={styles.coverCol}>
          {coverSrc ? (
            <img src={coverSrc} alt={metaForm.title} />
          ) : (
            <TitleCoverImage
              title={metaForm.title}
              author={metaForm.author}
              className={styles.coverPh}
            />
          )}
          {canEdit && (
            <FileInput
              variant="link"
              accept=".jpg,.jpeg,.png"
              onChange={(f) => {
                if (f) void handleCoverChange(f);
              }}
              placeholder="更换封面（点击或拖拽图片）"
              className={styles.coverUpload}
            />
          )}

          <section className={styles.progressSection}>
            <h2>阅读进度</h2>
            <div className={styles.progressFields}>
              <label>
                当前页
                <input
                  type="number"
                  value={currentPage}
                  onChange={(e) => setCurrentPage(e.target.value)}
                  disabled={!canEdit}
                />
              </label>
              <label>
                百分比
                <input
                  type="number"
                  step="0.1"
                  value={percent}
                  onChange={(e) => setPercent(e.target.value)}
                  disabled={!canEdit}
                />
              </label>
            </div>
            {canEdit && (
              <div className={styles.progressActions}>
                <button
                  type="button"
                  className="btn"
                  disabled={isBusy}
                  onClick={() => void saveProgress()}
                >
                  {actionBusy === "progress" ? "更新中…" : "更新进度"}
                </button>
              </div>
            )}
          </section>

          {canDelete && (
            <section className={styles.dangerSection}>
              <button
                type="button"
                className={`btn ${styles.deleteBtn}`}
                disabled={isBusy}
                onClick={() => void handleDelete()}
              >
                {actionBusy === "delete" ? "删除中…" : "删除图书"}
              </button>
            </section>
          )}
        </div>

        <div className={styles.infoCol}>
          <h1>{metaForm.title}</h1>
          <p className={styles.sync}>同步状态：{book.sync_status}</p>

          <table className={styles.table}>
            <tbody>
              {fields.slice(0, 2).map(({ key, label }) => (
                <tr key={key}>
                  <th>{label}</th>
                  <td>
                    {canEdit ? (
                      <input
                        value={String(metaForm[key] ?? "")}
                        onChange={(e) =>
                          setMetaForm({ ...metaForm, [key]: e.target.value })
                        }
                      />
                    ) : (
                      String(metaForm[key] ?? "—")
                    )}
                  </td>
                </tr>
              ))}
              <tr>
                <th>分类</th>
                <td>
                  {canEdit ? (
                    <div className={styles.languageField}>
                      <CategorySelect
                        libraryId={book.library_id}
                        value={metaForm.category ?? book.category}
                        onChange={(name) =>
                          setMetaForm({ ...metaForm, category: name })
                        }
                        required
                      />
                    </div>
                  ) : (
                    metaForm.category || book.category || "—"
                  )}
                </td>
              </tr>
              <tr>
                <th>语言 (ISO 639-1)</th>
                <td>
                  {canEdit ? (
                    <div className={styles.languageField}>
                      <LanguageCombobox
                        value={metaForm.language ?? ""}
                        onChange={(code) =>
                          setMetaForm({ ...metaForm, language: code })
                        }
                      />
                    </div>
                  ) : (
                    displayLanguageValue(metaForm.language)
                  )}
                </td>
              </tr>
              {fields.slice(2).map(({ key, label }) => (
                <tr key={key}>
                  <th>{label}</th>
                  <td>
                    {canEdit ? (
                      <input
                        value={String(metaForm[key] ?? "")}
                        onChange={(e) =>
                          setMetaForm({ ...metaForm, [key]: e.target.value })
                        }
                      />
                    ) : (
                      String(metaForm[key] ?? "—")
                    )}
                  </td>
                </tr>
              ))}
              <tr>
                <th>出版日期</th>
                <td>
                  {canEdit ? (
                    <input
                      type="month"
                      value={metaForm.publish_date ?? ""}
                      onChange={(e) =>
                        setMetaForm({
                          ...metaForm,
                          publish_date: e.target.value,
                        })
                      }
                    />
                  ) : (
                    metaForm.publish_date || "—"
                  )}
                </td>
              </tr>
              <tr>
                <th>页数</th>
                <td>
                  {canEdit ? (
                    <input
                      type="number"
                      value={metaForm.page_count ?? ""}
                      onChange={(e) =>
                        setMetaForm({
                          ...metaForm,
                          page_count: e.target.value ? Number(e.target.value) : undefined,
                        })
                      }
                    />
                  ) : (
                    metaForm.page_count ?? "—"
                  )}
                </td>
              </tr>
              <tr>
                <th>评分</th>
                <td>
                  {canEdit ? (
                    <RatingPicker
                      value={metaForm.rating}
                      onChange={(rating) => setMetaForm({ ...metaForm, rating })}
                    />
                  ) : metaForm.rating ? (
                    `${metaForm.rating} / 5`
                  ) : (
                    "—"
                  )}
                </td>
              </tr>
              <tr>
                <th>MD5</th>
                <td className={styles.hashCell}>
                  {metaForm.file_md5 ?? "—"}
                </td>
              </tr>
              <tr>
                <th>SHA-256</th>
                <td className={styles.hashCell}>
                  {metaForm.file_sha256 ?? "—"}
                </td>
              </tr>
            </tbody>
          </table>

          <div className={styles.actionRow}>
            <button
              type="button"
              className="btn btn-ghost"
              disabled={isBusy}
              onClick={() => void handleDownload()}
            >
              {actionBusy === "download" ? "下载中…" : "下载图书"}
            </button>
            {canEdit && (
              <button
                type="button"
                className="btn"
                disabled={isBusy}
                onClick={() => void saveMetadata()}
              >
                {actionBusy === "save" ? "保存中…" : "保存元数据"}
              </button>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
