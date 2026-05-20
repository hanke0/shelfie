import { Link, useNavigate } from "react-router-dom";
import { LibrarySelector } from "@/components/LibrarySelector";
import { UserMenu } from "@/components/UserMenu";
import { useLibrary } from "@/context/LibraryContext";
import styles from "./Header.module.css";

interface HeaderProps {
  search: string;
  onSearchChange: (v: string) => void;
  onUploadClick: () => void;
  onUploadDrop?: (file: File) => void;
}

export function Header({ search, onSearchChange, onUploadClick, onUploadDrop }: HeaderProps) {
  const navigate = useNavigate();
  const { libraryId } = useLibrary();

  return (
    <header className={styles.header}>
      <div className={styles.start}>
        <Link to="/" className={styles.brand}>
          <span className={styles.brandMark}>◈</span>
          <span className={styles.brandText}>
            Shelfie<span className={styles.dot}>·</span>书架
          </span>
        </Link>
        <LibrarySelector className={styles.headerControl} />
      </div>

      <div className={styles.searchWrap}>
        <input
          type="text"
          role="searchbox"
          enterKeyHint="search"
          className={styles.searchInput}
          placeholder="搜索书名、作者、ISBN…"
          value={search}
          onChange={(e) => onSearchChange(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter" && search.trim()) {
              const lib = libraryId ? `&library_id=${libraryId}` : "";
              navigate(`/search?q=${encodeURIComponent(search.trim())}${lib}`);
            }
          }}
          aria-label="全局搜索"
        />
      </div>

      <div className={styles.actions}>
        <button
          type="button"
          className={`btn ${styles.headerBtn} ${styles.uploadBtn}`}
          onClick={onUploadClick}
          onDragOver={(e) => {
            if (!onUploadDrop) return;
            e.preventDefault();
            e.stopPropagation();
          }}
          onDrop={(e) => {
            if (!onUploadDrop) return;
            e.preventDefault();
            e.stopPropagation();
            const file = e.dataTransfer.files[0];
            if (file) onUploadDrop(file);
          }}
        >
          上传
        </button>
        <UserMenu className={styles.headerControl} />
      </div>
    </header>
  );
}
