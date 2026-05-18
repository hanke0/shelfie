import { useState } from "react";
import { Link } from "react-router-dom";
import { useQueryClient } from "@tanstack/react-query";
import {
  useListKoreaderLinks,
  useListKoreaderProgress,
  useSetKoreaderLink,
  useDeleteKoreaderLink,
} from "@/api/generated/koreader/koreader";
import { useListLibraries } from "@/api/generated/libraries/libraries";
import { useSearch } from "@/api/generated/search/search";
import { useLibrary } from "@/context/LibraryContext";
import styles from "./AdminPage.module.css";
import tableStyles from "./AdminKoreaderPage.module.css";

export function AdminKoreaderPage() {
  const qc = useQueryClient();
  const { libraryId: contextLibraryId, setLibraryId } = useLibrary();
  const { data: libraries } = useListLibraries();
  const [libraryFilter, setLibraryFilter] = useState(contextLibraryId ?? "");

  const libParam = libraryFilter ? { library_id: libraryFilter } : undefined;
  const { data: progressRows } = useListKoreaderProgress(libParam ?? {}, {
    query: { enabled: true },
  });
  const { data: links } = useListKoreaderLinks(libParam ?? {}, {
    query: { enabled: true },
  });

  const setLink = useSetKoreaderLink();
  const deleteLink = useDeleteKoreaderLink();

  const [documentId, setDocumentId] = useState("");
  const [bookSearch, setBookSearch] = useState("");
  const [selectedBookId, setSelectedBookId] = useState("");
  const [message, setMessage] = useState<string | null>(null);

  const { data: searchResults } = useSearch(
    { q: bookSearch, library_id: libraryFilter || undefined, limit: 20 },
    { query: { enabled: bookSearch.trim().length >= 1 } },
  );

  const handleSetLink = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!documentId.trim() || !selectedBookId) return;
    await setLink.mutateAsync({
      data: { document: documentId.trim(), book_id: selectedBookId },
    });
    setDocumentId("");
    setSelectedBookId("");
    setBookSearch("");
    setMessage("已保存匹配关系");
    qc.invalidateQueries({ queryKey: ["/koreader/links"] });
    qc.invalidateQueries({ queryKey: ["/koreader/progress"] });
  };

  const handleDeleteLink = async (doc: string) => {
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
        <h2>筛选图书馆</h2>
        <select
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
        </select>
      </div>

      <form className={styles.card} onSubmit={(e) => void handleSetLink(e)}>
        <h2>手动匹配 document → 图书</h2>
        <label>
          KOReader document（32 位 MD5）
          <input
            value={documentId}
            onChange={(e) => setDocumentId(e.target.value)}
            placeholder="41cce710f34e5ec21315e19c99821415"
            required
          />
        </label>
        <label>
          搜索图书
          <input
            value={bookSearch}
            onChange={(e) => {
              setBookSearch(e.target.value);
              setSelectedBookId("");
            }}
            placeholder="书名或作者"
          />
        </label>
        {searchResults && searchResults.length > 0 && (
          <ul className={styles.list}>
            {searchResults.map((b) => (
              <li key={b.id}>
                <button
                  type="button"
                  className={selectedBookId === b.id ? styles.active : ""}
                  onClick={() => setSelectedBookId(b.id)}
                >
                  {b.title} — {b.author || "未知作者"}
                </button>
              </li>
            ))}
          </ul>
        )}
        {selectedBookId && (
          <p className={styles.muted}>已选图书 ID: {selectedBookId}</p>
        )}
        <button type="submit" className="btn" disabled={!selectedBookId}>
          保存匹配
        </button>
        {message && <p>{message}</p>}
      </form>

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
    </div>
  );
}
