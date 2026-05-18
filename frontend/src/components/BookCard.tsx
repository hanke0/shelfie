import { Link } from "react-router-dom";
import { useEffect, useState } from "react";
import type { BookCard as BookCardType } from "@/api/generated/models";
import { fetchCoverBlob } from "@/lib/custom-fetch";
import styles from "./BookCard.module.css";

export function BookCard({ book }: { book: BookCardType }) {
  const [coverSrc, setCoverSrc] = useState<string | null>(null);

  useEffect(() => {
    let url: string | null = null;
    let cancelled = false;
    fetchCoverBlob(book.id)
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
  }, [book.id]);

  const percent = book.reading_percent ?? 0;

  return (
    <Link to={`/books/${book.id}`} className={styles.card}>
      <div className={styles.cover}>
        {coverSrc ? (
          <img src={coverSrc} alt={book.title} />
        ) : (
          <div className={styles.placeholder}>{book.title.slice(0, 1)}</div>
        )}
      </div>
      <div className={styles.meta}>
        <h3>{book.title}</h3>
        <p>{book.author || "未知作者"}</p>
        {percent > 0 && (
          <div className={styles.progress}>
            <div style={{ width: `${Math.min(percent, 100)}%` }} />
          </div>
        )}
      </div>
    </Link>
  );
}
