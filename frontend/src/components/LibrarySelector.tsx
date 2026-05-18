import { useEffect, useId, useRef, useState } from "react";
import { useLibrary } from "@/context/LibraryContext";
import styles from "./LibrarySelector.module.css";

interface LibrarySelectorProps {
  className?: string;
}

export function LibrarySelector({ className }: LibrarySelectorProps) {
  const { libraries, libraryId, setLibraryId, isLoading } = useLibrary();
  const listId = useId();
  const rootRef = useRef<HTMLDivElement>(null);
  const [open, setOpen] = useState(false);
  const [highlight, setHighlight] = useState(0);

  const selected = libraries.find((lib) => lib.id === libraryId) ?? libraries[0];
  const selectedIndex = Math.max(
    0,
    libraries.findIndex((lib) => lib.id === (libraryId ?? selected?.id)),
  );

  useEffect(() => {
    if (!open) return;
    setHighlight(selectedIndex);
  }, [open, selectedIndex]);

  useEffect(() => {
    if (!open) return;
    const onDoc = (e: MouseEvent) => {
      if (!rootRef.current?.contains(e.target as Node)) {
        setOpen(false);
      }
    };
    document.addEventListener("mousedown", onDoc);
    return () => document.removeEventListener("mousedown", onDoc);
  }, [open]);

  const pick = (id: string) => {
    setLibraryId(id);
    setOpen(false);
  };

  const rootClass = [styles.root, className].filter(Boolean).join(" ");

  if (isLoading) {
    return (
      <div className={rootClass} aria-busy="true" aria-label="加载图书馆">
        <div className={styles.shell}>
          <span className={styles.icon} aria-hidden>
            ◈
          </span>
          <span className={styles.skeleton} />
        </div>
      </div>
    );
  }

  if (libraries.length === 0) {
    return (
      <div className={rootClass}>
        <div className={styles.shell}>
          <span className={styles.icon} aria-hidden>
            ◈
          </span>
          <a href="/admin/libraries" className={styles.emptyLink}>
            创建图书馆
          </a>
        </div>
      </div>
    );
  }

  const activeId = libraryId ?? selected?.id ?? "";

  return (
    <div ref={rootRef} className={rootClass}>
      <div className={[styles.shell, open ? styles.shellOpen : ""].filter(Boolean).join(" ")}>
        <button
          type="button"
          className={styles.trigger}
          aria-haspopup="listbox"
          aria-expanded={open}
          aria-controls={listId}
          aria-label="选择图书馆"
          onClick={() => setOpen((v) => !v)}
          onKeyDown={(e) => {
            if (e.key === "Escape") {
              setOpen(false);
              return;
            }
            if (!open) {
              if (e.key === "ArrowDown" || e.key === "Enter" || e.key === " ") {
                e.preventDefault();
                setOpen(true);
              }
              return;
            }
            if (e.key === "ArrowDown") {
              e.preventDefault();
              setHighlight((i) => Math.min(i + 1, libraries.length - 1));
            } else if (e.key === "ArrowUp") {
              e.preventDefault();
              setHighlight((i) => Math.max(i - 1, 0));
            } else if (e.key === "Enter" && libraries[highlight]) {
              e.preventDefault();
              pick(libraries[highlight].id);
            }
          }}
        >
          <span className={styles.icon} aria-hidden>
            ◈
          </span>
          <span className={styles.label}>{selected?.name ?? "选择图书馆"}</span>
          <span
            className={[styles.chevron, open ? styles.chevronOpen : ""].filter(Boolean).join(" ")}
            aria-hidden
          >
            ▾
          </span>
        </button>
      </div>

      {open && (
        <ul id={listId} role="listbox" className={styles.menu} aria-label="图书馆列表">
          {libraries.map((lib, index) => {
            const isSelected = lib.id === activeId;
            return (
              <li
                key={lib.id}
                role="option"
                aria-selected={isSelected}
                className={[
                  styles.option,
                  index === highlight ? styles.optionHighlighted : "",
                  isSelected ? styles.optionSelected : "",
                ]
                  .filter(Boolean)
                  .join(" ")}
                onMouseDown={(e) => {
                  e.preventDefault();
                  pick(lib.id);
                }}
                onMouseEnter={() => setHighlight(index)}
              >
                <span className={styles.optionName}>{lib.name}</span>
                {isSelected && (
                  <span className={styles.check} aria-hidden>
                    ✓
                  </span>
                )}
              </li>
            );
          })}
        </ul>
      )}
    </div>
  );
}
