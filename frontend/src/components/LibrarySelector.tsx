import { useLibrary } from "@/context/LibraryContext";
import { Select } from "@/components/ui/Select";
import styles from "./LibrarySelector.module.css";

export function LibrarySelector() {
  const { libraries, libraryId, setLibraryId, isLoading } = useLibrary();

  if (isLoading) {
    return <span className={styles.wrap}>加载图书馆…</span>;
  }

  if (libraries.length === 0) {
    return (
      <span className={styles.wrap}>
        <a href="/admin/libraries" className={styles.link}>
          请先创建图书馆
        </a>
      </span>
    );
  }

  return (
    <label className={styles.wrap}>
      <span className={styles.label}>当前图书馆</span>
      <Select
        value={libraryId ?? ""}
        onChange={(e) => setLibraryId(e.target.value || null)}
        aria-label="选择图书馆"
        className={styles.select}
      >
        {libraries.map((lib) => (
          <option key={lib.id} value={lib.id}>
            {lib.name}
          </option>
        ))}
      </Select>
    </label>
  );
}
