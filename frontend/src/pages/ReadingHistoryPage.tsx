import { useMemo, useState } from "react";
import { Link } from "react-router-dom";
import { useInfiniteQuery } from "@tanstack/react-query";
import {
  getListMyReadingHistoryQueryKey,
  listMyReadingHistory,
} from "@/api/generated/users/users";
import { useListLibraries } from "@/api/generated/libraries/libraries";
import type { ReadingHistoryEntry } from "@/api/generated/models";
import { BackLink } from "@/components/BackLink";
import { Dropdown } from "@/components/ui/Dropdown";
import { TitleCoverImage } from "@/components/TitleCoverImage";
import { useLibrary } from "@/context/LibraryContext";
import { useBookCover } from "@/hooks/useBookCover";
import adminStyles from "./AdminPage.module.css";
import styles from "./ReadingHistoryPage.module.css";

const PAGE_SIZE = 50;

function sourceLabel(source: string) {
  if (source === "koreader") return "KOReader";
  if (source === "web") return "网页";
  return source;
}

function formatTime(iso: string) {
  try {
    return new Date(iso).toLocaleString("zh-CN", {
      year: "numeric",
      month: "2-digit",
      day: "2-digit",
      hour: "2-digit",
      minute: "2-digit",
    });
  } catch {
    return iso;
  }
}

function HistoryCover({ bookId, title, author }: { bookId: string; title: string; author: string }) {
  const cover = useBookCover(bookId, { thumbnail: true });

  return (
    <div className={styles.cover}>
      {cover.src ? (
        <img src={cover.src} alt={title} />
      ) : cover.missing ? (
        <TitleCoverImage title={title} author={author} className={styles.generatedCover} />
      ) : null}
    </div>
  );
}

function HistoryRow({ entry }: { entry: ReadingHistoryEntry }) {
  const pct = entry.percent ?? 0;
  const page =
    entry.current_page != null && entry.current_page > 0
      ? ` · 第 ${entry.current_page} 页`
      : "";

  return (
    <Link to={`/books/${entry.book_id}`} className={styles.row}>
      <HistoryCover bookId={entry.book_id} title={entry.book_title} author={entry.book_author} />
      <div className={styles.body}>
        <h3 className={styles.title}>{entry.book_title || "未命名"}</h3>
        <p className={styles.author}>{entry.book_author || "未知作者"}</p>
        <p className={styles.meta}>
          <span className={styles.pct}>{pct > 0 ? `${pct.toFixed(1)}%` : "—"}</span>
          {page}
          <span className={styles.dot}>·</span>
          <span className={styles.source}>{sourceLabel(entry.source)}</span>
          <span className={styles.dot}>·</span>
          <span className={styles.time}>{formatTime(entry.recorded_at)}</span>
        </p>
        {entry.library_name && (
          <p className={styles.library}>{entry.library_name}</p>
        )}
      </div>
    </Link>
  );
}

export function ReadingHistoryPage() {
  const { libraryId: contextLibraryId, setLibraryId } = useLibrary();
  const { data: libraries } = useListLibraries();
  const [libraryFilter, setLibraryFilter] = useState(contextLibraryId ?? "");

  const params = { library_id: libraryFilter || undefined, limit: PAGE_SIZE };
  const { data, isLoading, isError, hasNextPage, fetchNextPage, isFetchingNextPage } =
    useInfiniteQuery({
      queryKey: [...getListMyReadingHistoryQueryKey(params), "infinite"],
      queryFn: ({ pageParam, signal }) =>
        listMyReadingHistory({ ...params, offset: pageParam }, undefined, signal),
      initialPageParam: 0,
      getNextPageParam: (lastPage, allPages) =>
        lastPage.length === PAGE_SIZE ? allPages.length * PAGE_SIZE : undefined,
    });

  // 分页期间若有新记录写入，offset 会错位产生重复项
  const rows = useMemo(
    () => [...new Map((data?.pages.flat() ?? []).map((r) => [r.id, r])).values()],
    [data],
  );

  const libraryOptions = [
    { value: "", label: "全部图书馆" },
    ...(libraries ?? []).map((l) => ({ value: l.id, label: l.name })),
  ];

  return (
    <div className="app-shell">
      <main className={styles.main}>
        <BackLink />
        <h1 className={styles.heading}>阅读记录</h1>

        <label className={styles.filter}>
          <span>图书馆</span>
          <Dropdown
            value={libraryFilter}
            onChange={(id) => {
              setLibraryFilter(id);
              if (id) setLibraryId(id);
            }}
            options={libraryOptions}
          />
        </label>

        {isLoading && <p className={adminStyles.muted}>加载中…</p>}
        {isError && (
          <p className={adminStyles.muted} role="alert">
            加载失败，请稍后重试
          </p>
        )}

        {!isLoading && !isError && rows.length === 0 && (
          <p className={adminStyles.muted}>暂无阅读记录</p>
        )}

        <ul className={styles.list}>
          {rows.map((entry) => (
            <li key={entry.id}>
              <HistoryRow entry={entry} />
            </li>
          ))}
        </ul>

        {hasNextPage && rows.length > 0 && (
          <button
            type="button"
            className="btn btn-ghost"
            disabled={isFetchingNextPage}
            onClick={() => void fetchNextPage()}
          >
            {isFetchingNextPage ? "加载中…" : "加载更多"}
          </button>
        )}
      </main>
    </div>
  );
}
