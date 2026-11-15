import { useState } from "react";
import { Link } from "react-router-dom";
import { useQueryClient } from "@tanstack/react-query";
import {
  useListKoreaderLinks,
  useListKoreaderProgress,
  useDeleteKoreaderLink,
} from "@/api/generated/koreader/koreader";
import { useListLibraries } from "@/api/generated/libraries/libraries";
import { BackLink } from "@/components/BackLink";
import { KoreaderLinkModal } from "@/components/KoreaderLinkModal";
import { Dropdown } from "@/components/ui/Dropdown";
import { useLibrary } from "@/context/LibraryContext";
import { useConfirmTwice } from "@/context/ConfirmContext";
import { useToast } from "@/context/ToastContext";
import { useApiAction } from "@/hooks/useApiAction";
import { useStoredUser } from "@/lib/auth";
import styles from "./AdminPage.module.css";
import tableStyles from "./AdminKoreaderPage.module.css";

export function AdminKoreaderPage() {
  const qc = useQueryClient();
  const user = useStoredUser();
  const toast = useToast();
  const { libraryId: contextLibraryId, setLibraryId } = useLibrary();
  const { data: libraries } = useListLibraries();
  const [libraryFilter, setLibraryFilter] = useState(contextLibraryId ?? "");
  const [linkModalOpen, setLinkModalOpen] = useState(false);

  const serverUrl = `${window.location.origin}/api/v1`;

  const copyServerUrl = async () => {
    try {
      await navigator.clipboard.writeText(serverUrl);
      toast.success("已复制服务器地址");
    } catch {
      toast.error("复制失败");
    }
  };

  const libParam = libraryFilter ? { library_id: libraryFilter } : undefined;
  const { data: progressRows } = useListKoreaderProgress(libParam ?? {}, {
    query: { enabled: true },
  });
  const { data: links } = useListKoreaderLinks(libParam ?? {}, {
    query: { enabled: true },
  });

  const deleteLink = useDeleteKoreaderLink();
  const confirmTwice = useConfirmTwice();
  const run = useApiAction();

  const handleDeleteLink = async (doc: string) => {
    if (
      !(await confirmTwice(
        `确定删除 KOReader 匹配（document: ${doc}）？`,
        "再次确认：删除后需重新匹配图书，确定继续吗？",
      ))
    ) {
      return;
    }
    await run(
      async () => {
        await deleteLink.mutateAsync({ document: doc });
        await qc.invalidateQueries({ queryKey: ["/koreader/links"] });
        await qc.invalidateQueries({ queryKey: ["/koreader/progress"] });
      },
      { successMessage: "已删除匹配", errorMessage: "删除失败" },
    );
  };

  const pct = (v: number) => `${(v * 100).toFixed(1)}%`;

  return (
    <div className={`app-shell ${styles.page}`}>
      <BackLink />
      <h1>KOReader 同步</h1>
      <p className={styles.muted}>管理 KOReader 进度同步与 document ↔ 图书匹配。</p>

      <div className={styles.card}>
        <h2>连接设置</h2>
        <p className={styles.muted}>在 KOReader 的同步设置中，服务器地址填写：</p>
        <div className={styles.opdsRow}>
          <code className={styles.opdsUrl}>{serverUrl}</code>
          <button type="button" className="btn btn-ghost" onClick={() => void copyServerUrl()}>
            复制
          </button>
        </div>
        <p className={styles.muted}>
          用户名：<code>{user?.username}</code>，密码与 Shelfie 登录密码相同（KOReader
          会对密码做 MD5）。首次使用前请在网页登录一次以启用同步密钥。
        </p>
      </div>

      <div className={styles.card}>
        <div className={styles.cardHeader}>
          <h2>筛选与匹配</h2>
          <button type="button" className="btn" onClick={() => setLinkModalOpen(true)}>
            新建匹配
          </button>
        </div>
        <label className={`${styles.filterRow} ${styles.fieldInline}`}>
          <span className={styles.muted}>图书馆</span>
          <Dropdown
            value={libraryFilter}
            onChange={(id) => {
              setLibraryFilter(id);
              if (id) setLibraryId(id);
            }}
            placeholder="全部"
            options={[
              { value: "", label: "全部" },
              ...(libraries?.map((lib) => ({ value: lib.id, label: lib.name })) ?? []),
            ]}
            aria-label="图书馆"
          />
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
                    className="btn btn-danger"
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
