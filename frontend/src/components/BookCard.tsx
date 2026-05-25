import { Link } from "react-router-dom";
import { useEffect, useState, type MouseEvent } from "react";
import type { BookCard as BookCardType } from "@/api/generated/models";
import { fetchCoverBlob } from "@/lib/custom-fetch";
import { TitleCoverImage } from "@/components/TitleCoverImage";
import styles from "./BookCard.module.css";

interface BookCardProps {
  book: BookCardType;
  /** 首页横滑：封面圆角阴影，下方轻量标题 */
  variant?: "default" | "shelf";
  selectable?: boolean;
  selected?: boolean;
  onSelectToggle?: () => void;
}

export function BookCard({
  book,
  variant = "default",
  selectable,
  selected,
  onSelectToggle,
}: BookCardProps) {
  const [coverSrc, setCoverSrc] = useState<string | null>(null);

  useEffect(() => {
    let url: string | null = null;
    let cancelled = false;
    fetchCoverBlob(book.id, { thumbnail: true })
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

  const handleClick = (e: MouseEvent) => {
    if (!selectable || !onSelectToggle) return;
    e.preventDefault();
    onSelectToggle();
  };

  const body = (
    <>
      {selectable && (
        <span
          className={[styles.check, selected ? styles.checkOn : ""].filter(Boolean).join(" ")}
          aria-checked={selected}
          role="checkbox"
        />
      )}
      <div className={styles.cover}>
        {coverSrc ? (
          <img src={coverSrc} alt={book.title} />
        ) : (
          <TitleCoverImage
            title={book.title}
            author={book.author}
            className={styles.generatedCover}
          />
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
    </>
  );

  const cardClass = [
    styles.card,
    variant === "shelf" ? styles.cardShelf : "",
    selectable ? styles.cardSelectable : "",
    selected ? styles.cardSelected : "",
  ]
    .filter(Boolean)
    .join(" ");

  if (selectable) {
    return (
      <button
        type="button"
        className={cardClass}
        onClick={handleClick}
        aria-pressed={selected}
      >
        {body}
      </button>
    );
  }

  return (
    <Link to={`/books/${book.id}`} className={cardClass}>
      {body}
    </Link>
  );
}
