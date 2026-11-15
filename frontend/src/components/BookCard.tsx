import { Link } from "react-router-dom";
import type { MouseEvent } from "react";
import type { BookCard as BookCardType } from "@/api/generated/models";
import { TitleCoverImage } from "@/components/TitleCoverImage";
import { useBookCover } from "@/hooks/useBookCover";
import styles from "./BookCard.module.css";

interface BookCardProps {
  book: BookCardType;
  /** 首页书架格：封面圆角阴影，下方轻量标题 */
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
  const cover = useBookCover(book.id, { thumbnail: true });

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
        {cover.src ? (
          <img src={cover.src} alt={book.title} />
        ) : cover.missing ? (
          <TitleCoverImage
            title={book.title}
            author={book.author}
            className={styles.generatedCover}
          />
        ) : null}
      </div>
      <div className={styles.meta}>
        <h3 title={book.title}>{book.title}</h3>
        <p title={book.author || undefined}>{book.author || "未知作者"}</p>
        {percent > 0 && (
          <div className={styles.progressRow}>
            <div className={styles.progress}>
              <div style={{ width: `${Math.min(percent, 100)}%` }} />
            </div>
            <span className={styles.percent}>{Math.round(Math.min(percent, 100))}%</span>
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
