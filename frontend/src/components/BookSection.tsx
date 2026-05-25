import { useEffect, useRef } from "react";
import { Link } from "react-router-dom";
import type { BookCard as BookCardType } from "@/api/generated/models";
import { BookCard } from "./BookCard";
import styles from "./BookSection.module.css";

interface BookSectionProps {
  title: string;
  books: BookCardType[];
  moreLink?: string;
  action?: React.ReactNode;
}

export function BookSection({ title, books, moreLink, action }: BookSectionProps) {
  const rowRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const el = rowRef.current;
    if (!el) return;

    const onWheel = (e: WheelEvent) => {
      if (el.scrollWidth <= el.clientWidth) return;
      if (Math.abs(e.deltaY) <= Math.abs(e.deltaX)) return;
      e.preventDefault();
      el.scrollBy({ left: e.deltaY, behavior: "smooth" });
    };

    el.addEventListener("wheel", onWheel, { passive: false });
    return () => el.removeEventListener("wheel", onWheel);
  }, [books.length]);

  if (!books.length) return null;

  return (
    <section className={styles.section}>
      <div className={styles.sectionTitle}>
        <h2>{title}</h2>
        <div className={styles.sectionActions}>
          {action}
          {moreLink && <Link to={moreLink}>查看更多 →</Link>}
        </div>
      </div>
      <div ref={rowRef} className={styles.rowScroll}>
        {books.map((book) => (
          <BookCard key={book.id} book={book} variant="shelf" />
        ))}
      </div>
    </section>
  );
}
