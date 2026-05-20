import { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import {
  listMyReadingHistory,
  useListMyReadingHistory,
} from "@/api/generated/users/users";
import { useListLibraries } from "@/api/generated/libraries/libraries";
import type { ReadingHistoryEntry } from "@/api/generated/models";
import { Header } from "@/components/Header";
import { Dropdown } from "@/components/ui/Dropdown";
import { TitleCoverImage } from "@/components/TitleCoverImage";
import { useLibrary } from "@/context/LibraryContext";
import { fetchCoverBlob } from "@/lib/custom-fetch";
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
  const [coverSrc, setCoverSrc] = useState<string | null>(null);

  useEffect(() => {
    let url: string | null = null;
    let cancelled = false;
    fetchCoverBlob(bookId, { thumbnail: true })
      .then((blobUrl) => {
        if (!cancelled) {
          url = blobUrl;
          setCoverSrc(blobUrl);
        }
      })
      .catch(() => setCoverSrc(null));
    return () => {
      cancelled = true;
      if (url) URL.revokeObjectURL(url);
    };
  }, [bookId]);

  return (
    <div className={styles.cover}>
      {coverSrc ? (
        <img src={coverSrc} alt={title} />
      ) : (
        <TitleCoverImage title={title} author={author} className={styles.generatedCover} />
      )}
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
  const [search, setSearch] = useState("");
  const { libraryId: contextLibraryId, setLibraryId } = useLibrary();
  const { data: libraries } = useListLibraries();
  const [libraryFilter, setLibraryFilter] = useState(contextLibraryId ?? "");
  const [extraRows, setExtraRows] = useState<ReadingHistoryEntry[]>([]);
  const [hasMore, setHasMore] = useState(false);
  const [loadingMore, setLoadingMore] = useState(false);

  const libParam = libraryFilter || undefined;

  const { data, isLoading, isError } = useListMyReadingHistory(
    { library_id: libParam, limit: PAGE_SIZE, offset: 0 },
    { query: { enabled: true } },
  );

  useEffect(() => {
    setExtraRows([]);
  }, [libParam]);

  useEffect(() => {
    setHasMore((data?.length ?? 0) === PAGE_SIZE);
  }, [data]);

  const rows = [...(data ?? []), ...extraRows];

  const loadMore = async () => {
    setLoadingMore(true);
    try {
      const more = await listMyReadingHistory({
        library_id: libParam,
        limit: PAGE_SIZE,
        offset: rows.length,
      });
      setExtraRows((prev) => {
        const seen = new Set([...(data ?? []), ...prev].map((r) => r.id));
        return [...prev, ...more.filter((r) => !seen.has(r.id))];
      });
      setHasMore(more.length === PAGE_SIZE);
    } finally {
      setLoadingMore(false);
    }
  };

  const libraryOptions = [
    { value: "", label: "全部图书馆" },
    ...(libraries ?? []).map((l) => ({ value: l.id, label: l.name })),
  ];

  return (
    <div className="app-shell">
      <Header
        search={search}
        onSearchChange={setSearch}
        onUploadClick={() => {}}
      />
      <main className={styles.main}>
        <Link to="/" className={adminStyles.back}>
          ← 返回首页
        </Link>
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

        {hasMore && rows.length > 0 && (
          <button
            type="button"
            className="btn btn-ghost"
            disabled={loadingMore}
            onClick={() => void loadMore()}
          >
            {loadingMore ? "加载中…" : "加载更多"}
          </button>
        )}
      </main>
    </div>
  );
}
