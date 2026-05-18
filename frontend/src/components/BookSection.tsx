import { Link } from "react-router-dom";
import type { BookCard as BookCardType } from "@/api/generated/models";
import { BookCard } from "./BookCard";

interface BookSectionProps {
  title: string;
  books: BookCardType[];
  moreLink?: string;
  action?: React.ReactNode;
}

export function BookSection({ title, books, moreLink, action }: BookSectionProps) {
  if (!books.length) return null;

  return (
    <section>
      <div className="section-title">
        <h2>{title}</h2>
        <div style={{ display: "flex", gap: "1rem", alignItems: "center" }}>
          {action}
          {moreLink && <Link to={moreLink}>查看更多 →</Link>}
        </div>
      </div>
      <div className="book-scroll">
        {books.map((book) => (
          <BookCard key={book.id} book={book} />
        ))}
      </div>
    </section>
  );
}
