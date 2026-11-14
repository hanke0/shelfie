import { useState } from "react";
import { Link } from "react-router-dom";
import { useQueryClient } from "@tanstack/react-query";
import {
  getListBookorbitDevicesQueryKey,
  useListBookorbitBooks,
  useListBookorbitDevices,
  useResetBookorbitDevice,
} from "@/api/generated/book-orbit/book-orbit";
import { useListLibraries } from "@/api/generated/libraries/libraries";
import { Dropdown } from "@/components/ui/Dropdown";
import { formatDuration, READING_STATUS_LABELS } from "@/components/BookReadingSummary";
import { useConfirmTwice } from "@/context/ConfirmContext";
import { useToast } from "@/context/ToastContext";
import { useApiAction } from "@/hooks/useApiAction";
import { useStoredUser } from "@/lib/auth";
import styles from "./AdminPage.module.css";
import tableStyles from "./AdminKoreaderPage.module.css";

function formatTimestamp(value?: string | null): string {
  return value ? new Date(value).toLocaleString("zh-CN") : "—";
}

export function AdminBookOrbitPage() {
  const qc = useQueryClient();
  const user = useStoredUser();
  const toast = useToast();
  const confirmTwice = useConfirmTwice();
  const run = useApiAction();
  const { data: libraries } = useListLibraries();
  const [libraryFilter, setLibraryFilter] = useState("");
  const { data: devices } = useListBookorbitDevices();
  const { data: books } = useListBookorbitBooks(
    libraryFilter ? { library_id: libraryFilter } : undefined,
  );
  const resetDevice = useResetBookorbitDevice();

  const serverUrl = window.location.origin;

  const copyServerUrl = async () => {
    try {
      await navigator.clipboard.writeText(serverUrl);
      toast.success("已复制服务器地址");
    } catch {
      toast.error("复制失败");
    }
  };

  const handleReset = async (deviceId: string) => {
    if (
      !(await confirmTwice(
        `确定重置设备「${deviceId}」的同步状态？`,
        "再次确认：重置后下次同步时服务端会把全部标注和书签重新下发给该设备（阅读统计保留），确定继续吗？",
      ))
    ) {
      return;
    }
    await run(
      async () => {
        await resetDevice.mutateAsync({ data: { device_id: deviceId } });
        await qc.invalidateQueries({ queryKey: getListBookorbitDevicesQueryKey() });
      },
      { successMessage: "已重置设备同步状态", errorMessage: "重置失败" },
    );
  };

  return (
    <div className={`app-shell ${styles.page}`}>
      <Link to="/" className={styles.back}>
        ← 返回首页
      </Link>
      <h1>BookOrbit 同步</h1>
      <p className={styles.muted}>
        Readest 等 BookOrbit 客户端可同步阅读进度、标注/笔记、书签、阅读状态与阅读统计。
        以下只显示你自己的同步数据。
      </p>

      <div className={styles.card}>
        <h2>连接设置</h2>
        <p className={styles.muted}>在 Readest 的 BookOrbit 同步设置中，服务器地址填写：</p>
        <div className={styles.opdsRow}>
          <code className={styles.opdsUrl}>{serverUrl}</code>
          <button type="button" className="btn btn-ghost" onClick={() => void copyServerUrl()}>
            复制
          </button>
        </div>
        <p className={styles.muted}>
          用户名：<code>{user?.username}</code>，密码与 Shelfie 登录密码相同。
          图书按文件 MD5 / Partial MD5 自动匹配，匹配不上时可在「
          <Link to="/admin/koreader">KOReader 同步</Link>」页手动匹配。
        </p>
      </div>

      <div className={styles.card}>
        <h2>同步设备 ({devices?.length ?? 0})</h2>
        {devices?.length ? (
          <table className={tableStyles.table}>
            <thead>
              <tr>
                <th>设备 ID</th>
                <th>图书</th>
                <th>标注</th>
                <th>书签</th>
                <th>统计条数</th>
                <th>最近同步变更</th>
                <th></th>
              </tr>
            </thead>
            <tbody>
              {devices.map((d) => (
                <tr key={d.device_id}>
                  <td>
                    <code>{d.device_id}</code>
                  </td>
                  <td>{d.books}</td>
                  <td>{d.annotations}</td>
                  <td>{d.bookmarks}</td>
                  <td>{d.page_stats}</td>
                  <td>{formatTimestamp(d.last_change_at)}</td>
                  <td>
                    <button
                      type="button"
                      className="btn btn-ghost"
                      disabled={resetDevice.isPending}
                      onClick={() => void handleReset(d.device_id)}
                    >
                      重置同步状态
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        ) : (
          <p className={styles.muted}>还没有设备同步过标注、书签或阅读统计。</p>
        )}
      </div>

      <div className={styles.card}>
        <div className={styles.cardHeader}>
          <h2>已同步图书 ({books?.length ?? 0})</h2>
        </div>
        <label className={`${styles.filterRow} ${styles.fieldInline}`}>
          <span className={styles.muted}>图书馆</span>
          <Dropdown
            value={libraryFilter}
            onChange={setLibraryFilter}
            placeholder="全部"
            options={[
              { value: "", label: "全部" },
              ...(libraries?.map((lib) => ({ value: lib.id, label: lib.name })) ?? []),
            ]}
            aria-label="图书馆"
          />
        </label>
        {books?.length ? (
          <table className={tableStyles.table}>
            <thead>
              <tr>
                <th>图书</th>
                <th>状态</th>
                <th>标注</th>
                <th>书签</th>
                <th>阅读时长</th>
                <th>最近阅读</th>
              </tr>
            </thead>
            <tbody>
              {books.map((b) => (
                <tr key={b.book_id}>
                  <td>
                    <Link to={`/books/${b.book_id}`}>
                      {b.title}
                      {b.author ? ` — ${b.author}` : ""}
                    </Link>
                  </td>
                  <td>{b.status ? (READING_STATUS_LABELS[b.status] ?? b.status) : "—"}</td>
                  <td>{b.annotations}</td>
                  <td>{b.bookmarks}</td>
                  <td>{b.total_seconds ? formatDuration(b.total_seconds) : "—"}</td>
                  <td>
                    {b.last_read_at
                      ? new Date(b.last_read_at * 1000).toLocaleDateString("zh-CN")
                      : "—"}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        ) : (
          <p className={styles.muted}>暂无同步数据。</p>
        )}
      </div>
    </div>
  );
}
