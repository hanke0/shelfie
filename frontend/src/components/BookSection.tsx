import {
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type CSSProperties,
  type ReactNode,
} from "react";
import { Link } from "react-router-dom";
import type { BookCard as BookCardType } from "@/api/generated/models";
import { BookCard } from "./BookCard";
import styles from "./BookSection.module.css";

/*
 * Constraint brief (Refactoring UI Workflow B/C):
 * Ranking — primary: book covers/titles; secondary: section title;
 *   tertiary: 查看更多 / 换一换 + bottom pager (link-tier / chrome, §2.8.a).
 * Layout — 1-row grid, no free scroll; click pager with slide (§3.3, §3.6).
 * Pager — round prev/next (SVG, optically centered), center dots (§2.1, §5.7).
 * Tokens — --text-sm/xl, --ink* / --accent*, --radius-pill, --shadow-sm, --border.
 */

interface BookSectionProps {
  title: string;
  books: BookCardType[];
  moreLink?: string;
  action?: ReactNode;
}

const MIN_CARD_WIDTH = 132;
const GRID_GAP_PX = 14; // ~0.85rem at 16px root
const ROWS = 1;

function columnsForWidth(width: number): number {
  if (width <= 0) return 1;
  return Math.max(1, Math.floor((width + GRID_GAP_PX) / (MIN_CARD_WIDTH + GRID_GAP_PX)));
}

function Chevron({ dir }: { dir: "left" | "right" }) {
  return (
    <svg
      className={[styles.chevron, dir === "left" ? styles.chevronLeft : styles.chevronRight].join(
        " ",
      )}
      viewBox="0 0 16 16"
      width="8"
      height="8"
      aria-hidden="true"
      focusable="false"
    >
      <path
        d={dir === "left" ? "M10 3.5 5.5 8 10 12.5" : "M6 3.5 10.5 8 6 12.5"}
        fill="none"
        stroke="currentColor"
        strokeWidth="1.75"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

export function BookSection({ title, books, moreLink, action }: BookSectionProps) {
  const viewportRef = useRef<HTMLDivElement>(null);
  const [cols, setCols] = useState(1);
  const [page, setPage] = useState(0);

  const booksKey = useMemo(() => books.map((b) => b.id).join(","), [books]);

  useLayoutEffect(() => {
    const el = viewportRef.current;
    if (!el) return;

    const measure = () => setCols(columnsForWidth(el.clientWidth));
    measure();

    const ro = new ResizeObserver(measure);
    ro.observe(el);
    return () => ro.disconnect();
  }, [booksKey]);

  const pageSize = Math.max(1, cols * ROWS);
  const pages = useMemo(() => {
    const chunks: BookCardType[][] = [];
    for (let i = 0; i < books.length; i += pageSize) {
      chunks.push(books.slice(i, i + pageSize));
    }
    return chunks.length ? chunks : [[]];
  }, [books, pageSize]);

  const pageCount = pages.length;

  useEffect(() => {
    setPage((p) => Math.min(p, pageCount - 1));
  }, [pageCount]);

  useEffect(() => {
    setPage(0);
  }, [booksKey]);

  if (!books.length) return null;

  const showPager = pageCount > 1;

  return (
    <section className={styles.section}>
      <div className={styles.sectionTitle}>
        <h2>{title}</h2>
        <div className={styles.sectionActions}>
          {action}
          {moreLink && (
            <Link to={moreLink} className={styles.action}>
              查看更多 →
            </Link>
          )}
        </div>
      </div>

      <div ref={viewportRef} className={styles.viewport}>
        <div
          className={styles.track}
          style={
            {
              "--cols": cols,
              transform: `translate3d(-${page * 100}%, 0, 0)`,
            } as CSSProperties
          }
        >
          {pages.map((pageBooks, i) => (
            <div
              key={i}
              className={styles.page}
              aria-hidden={i !== page}
              inert={i !== page ? true : undefined}
            >
              {pageBooks.map((book) => (
                <BookCard key={book.id} book={book} variant="shelf" />
              ))}
            </div>
          ))}
        </div>
      </div>

      {showPager && (
        <div className={styles.pager} role="navigation" aria-label={`${title}分页`}>
          <button
            type="button"
            className={styles.navBtn}
            aria-label="上一页"
            disabled={page === 0}
            onClick={() => setPage((p) => Math.max(0, p - 1))}
          >
            <Chevron dir="left" />
          </button>

          <div className={styles.dots} role="tablist" aria-label="页码">
            {Array.from({ length: pageCount }, (_, i) => (
              <button
                key={i}
                type="button"
                role="tab"
                aria-label={`第 ${i + 1} 页`}
                aria-selected={i === page}
                className={[styles.dot, i === page ? styles.dotActive : ""].filter(Boolean).join(" ")}
                onClick={() => setPage(i)}
              />
            ))}
          </div>

          <button
            type="button"
            className={styles.navBtn}
            aria-label="下一页"
            disabled={page >= pageCount - 1}
            onClick={() => setPage((p) => Math.min(pageCount - 1, p + 1))}
          >
            <Chevron dir="right" />
          </button>
        </div>
      )}
    </section>
  );
}
