import { useLibrary } from "@/context/LibraryContext";
import { Dropdown } from "@/components/ui/Dropdown";
import styles from "./LibrarySelector.module.css";

interface LibrarySelectorProps {
  className?: string;
}

export function LibrarySelector({ className }: LibrarySelectorProps) {
  const { libraries, libraryId, setLibraryId, isLoading } = useLibrary();

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

  return (
    <Dropdown
      className={rootClass}
      variant="header"
      icon="◈"
      aria-label="选择图书馆"
      value={libraryId ?? libraries[0]?.id ?? ""}
      onChange={setLibraryId}
      placeholder="选择图书馆"
      options={libraries.map((lib) => ({ value: lib.id, label: lib.name }))}
    />
  );
}
