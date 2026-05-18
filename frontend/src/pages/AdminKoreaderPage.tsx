import { useState } from "react";
import { Link } from "react-router-dom";
import { useQueryClient } from "@tanstack/react-query";
import {
  useListKoreaderLinks,
  useListKoreaderProgress,
  useDeleteKoreaderLink,
} from "@/api/generated/koreader/koreader";
import { useListLibraries } from "@/api/generated/libraries/libraries";
import { KoreaderLinkModal } from "@/components/KoreaderLinkModal";
import { Select } from "@/components/ui/Select";
import { useLibrary } from "@/context/LibraryContext";
import { useConfirmTwice } from "@/context/ConfirmContext";
import styles from "./AdminPage.module.css";
import tableStyles from "./AdminKoreaderPage.module.css";

export function AdminKoreaderPage() {
  const qc = useQueryClient();
  const { libraryId: contextLibraryId, setLibraryId } = useLibrary();
  const { data: libraries } = useListLibraries();
  const [libraryFilter, setLibraryFilter] = useState(contextLibraryId ?? "");
  const [linkModalOpen, setLinkModalOpen] = useState(false);

  const libParam = libraryFilter ? { library_id: libraryFilter } : undefined;
  const { data: progressRows } = useListKoreaderProgress(libParam ?? {}, {
    query: { enabled: true },
  });
  const { data: links } = useListKoreaderLinks(libParam ?? {}, {
    query: { enabled: true },
  });

  const deleteLink = useDeleteKoreaderLink();
  const confirmTwice = useConfirmTwice();

  const handleDeleteLink = async (doc: string) => {
    if (
      !(await confirmTwice(
        `确定删除 KOReader 匹配（document: ${doc}）？`,
        "再次确认：删除后需重新匹配图书，确定继续吗？",
      ))
    ) {
      return;
    }
    await deleteLink.mutateAsync({ document: doc });
    qc.invalidateQueries({ queryKey: ["/koreader/links"] });
    qc.invalidateQueries({ queryKey: ["/koreader/progress"] });
  };

  const pct = (v: number) => `${(v * 100).toFixed(1)}%`;

  return (
    <div className={`app-shell ${styles.page}`}>
      <Link to="/" className={styles.back}>
        ← 返回首页
      </Link>
      <h1>KOReader 同步</h1>
      <p className={styles.muted}>
        设备端将同步服务器设为 <code>http://你的域名/api/v1</code>，用户名/密码与 Shelfie
        相同（KOReader 会对密码做 MD5）。首次使用前请在网页登录一次以启用同步密钥。
      </p>

      <div className={styles.card}>
        <div className={styles.cardHeader}>
          <h2>筛选与匹配</h2>
          <button type="button" className="btn" onClick={() => setLinkModalOpen(true)}>
            新建匹配
          </button>
        </div>
        <label className={`${styles.filterRow} ${styles.fieldInline}`}>
          <span className={styles.muted}>图书馆</span>
          <Select
            value={libraryFilter}
            onChange={(e) => {
              setLibraryFilter(e.target.value);
              if (e.target.value) setLibraryId(e.target.value);
            }}
          >
            <option value="">全部</option>
            {libraries?.map((lib) => (
              <option key={lib.id} value={lib.id}>
                {lib.name}
              </option>
            ))}
          </Select>
        </label>
      </div>

      <div className={styles.card}>
        <h2>已匹配 ({links?.length ?? 0})</h2>
        <table className={tableStyles.table}>
          <thead>
            <tr>
              <th>document</th>
              <th>图书</th>
              <th>来源</th>
              <th></th>
            </tr>
          </thead>
          <tbody>
            {links?.map((l) => (
              <tr key={l.document}>
                <td>
                  <code>{l.document}</code>
                </td>
                <td>
                  <Link to={`/books/${l.book_id}`}>
                    {l.book_title} — {l.book_author}
                  </Link>
                </td>
                <td>{l.link_source === "md5" ? "MD5 自动" : "手动"}</td>
                <td>
                  <button
                    type="button"
                    className="btn btn-ghost"
                    onClick={() => void handleDeleteLink(l.document)}
                  >
                    删除
                  </button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      <div className={styles.card}>
        <h2>同步进度记录</h2>
        <table className={tableStyles.table}>
          <thead>
            <tr>
              <th>用户</th>
              <th>document</th>
              <th>进度</th>
              <th>关联图书</th>
            </tr>
          </thead>
          <tbody>
            {progressRows?.map((r) => (
              <tr key={`${r.user_id}-${r.document}`}>
                <td>{r.username}</td>
                <td>
                  <code>{r.document}</code>
                </td>
                <td>
                  {pct(r.percentage)} — {r.progress}
                  {r.device ? ` (${r.device})` : ""}
                </td>
                <td>
                  {r.book_id && r.book_title ? (
                    <Link to={`/books/${r.book_id}`}>{r.book_title}</Link>
                  ) : (
                    <span className={styles.muted}>未关联</span>
                  )}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      <KoreaderLinkModal
        open={linkModalOpen}
        onClose={() => setLinkModalOpen(false)}
        libraryFilter={libraryFilter}
      />
    </div>
  );
}
