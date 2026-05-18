import { useEffect, useState } from "react";
import { Link, useNavigate, useParams } from "react-router-dom";
import { useQueryClient } from "@tanstack/react-query";
import {
  useGetBook,
  useUpdateBook,
  useUpdateProgress,
  useDeleteBook,
  getGetBookQueryKey,
} from "@/api/generated/books/books";
import { getGetHomeQueryKey } from "@/api/generated/home/home";
import { fetchCoverBlob } from "@/lib/custom-fetch";
import { updateCoverMultipart } from "@/lib/upload";
import { downloadBookFile } from "@/lib/download";
import styles from "./BookDetailPage.module.css";

export function BookDetailPage() {
  const { id = "" } = useParams();
  const navigate = useNavigate();
  const qc = useQueryClient();
  const { data: book, isLoading } = useGetBook(id);
  const updateBook = useUpdateBook();
  const updateProgress = useUpdateProgress();
  const deleteBook = useDeleteBook();

  const [coverSrc, setCoverSrc] = useState<string | null>(null);
  const [metaForm, setMetaForm] = useState(book?.metadata);
  const [currentPage, setCurrentPage] = useState("");
  const [percent, setPercent] = useState("");

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

  if (isLoading || !book || !metaForm) {
    return (
      <div className="app-shell">
        <p>加载中…</p>
      </div>
    );
  }

  const canEdit = book.permissions?.can_edit;
  const canDelete = book.permissions?.can_delete;

  const saveMetadata = async () => {
    await updateBook.mutateAsync({ id, data: { metadata: metaForm } });
    qc.invalidateQueries({ queryKey: getGetBookQueryKey(id) });
    qc.invalidateQueries({ queryKey: getGetHomeQueryKey() });
  };

  const saveProgress = async () => {
    await updateProgress.mutateAsync({
      id,
      data: {
        reading_progress: {
          current_page: currentPage ? Number(currentPage) : undefined,
          percent: percent ? Number(percent) : undefined,
        },
      },
    });
    qc.invalidateQueries({ queryKey: getGetBookQueryKey(id) });
    qc.invalidateQueries({ queryKey: getGetHomeQueryKey() });
  };

  const handleDelete = async () => {
    if (!confirm("确定删除这本图书？")) return;
    await deleteBook.mutateAsync({ id });
    navigate("/");
  };

  const handleCoverChange = async (file: File) => {
    await updateCoverMultipart(id, file);
    qc.invalidateQueries({ queryKey: getGetBookQueryKey(id) });
    const u = await fetchCoverBlob(id);
    setCoverSrc((prev) => {
      if (prev) URL.revokeObjectURL(prev);
      return u;
    });
  };

  const fields: { key: keyof typeof metaForm; label: string }[] = [
    { key: "title", label: "书名" },
    { key: "author", label: "作者" },
    { key: "language", label: "语言" },
    { key: "translator", label: "译者" },
    { key: "publisher", label: "出版社" },
    { key: "isbn", label: "ISBN" },
    { key: "original_title", label: "原作名" },
    { key: "series", label: "丛书" },
    { key: "category", label: "分类" },
    { key: "notes", label: "备注" },
  ];

  return (
    <div className={`app-shell ${styles.page}`}>
      <Link to="/" className={styles.back}>
        ← 返回首页
      </Link>

      <div className={styles.layout}>
        <div className={styles.coverCol}>
          {coverSrc ? <img src={coverSrc} alt={metaForm.title} /> : <div className={styles.coverPh} />}
          {canEdit && (
            <label className={styles.coverUpload}>
              更换封面
              <input
                type="file"
                accept=".jpg,.jpeg,.png"
                hidden
                onChange={(e) => {
                  const f = e.target.files?.[0];
                  if (f) void handleCoverChange(f);
                }}
              />
            </label>
          )}
        </div>

        <div className={styles.infoCol}>
          <h1>{metaForm.title}</h1>
          <p className={styles.sync}>同步状态：{book.sync_status}</p>

          <table className={styles.table}>
            <tbody>
              {fields.map(({ key, label }) => (
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
            </tbody>
          </table>

          <div className={styles.actionRow}>
            <button
              type="button"
              className="btn btn-ghost"
              onClick={() => void downloadBookFile(id)}
            >
              下载图书
            </button>
            {canEdit && (
              <button type="button" className="btn" onClick={() => void saveMetadata()}>
                保存元数据
              </button>
            )}
          </div>

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
              <button type="button" className="btn" onClick={() => void saveProgress()}>
                更新进度
              </button>
            )}
          </section>

          {canDelete && (
            <button type="button" className={`btn ${styles.deleteBtn}`} onClick={() => void handleDelete()}>
              删除图书
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
