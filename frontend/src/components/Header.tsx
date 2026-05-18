import { Link, useNavigate } from "react-router-dom";
import { getUser, clearAuth } from "@/lib/auth";
import { LibrarySelector } from "@/components/LibrarySelector";
import { useLibrary } from "@/context/LibraryContext";
import styles from "./Header.module.css";

interface HeaderProps {
  search: string;
  onSearchChange: (v: string) => void;
  onUploadClick: () => void;
}

export function Header({ search, onSearchChange, onUploadClick }: HeaderProps) {
  const user = getUser();
  const navigate = useNavigate();
  const { libraryId } = useLibrary();

  return (
    <header className={styles.header}>
      <Link to="/" className={styles.brand}>
        <span className={styles.brandMark}>◈</span>
        <span className={styles.brandText}>
          Shelfie<span className={styles.dot}>·</span>书架
        </span>
      </Link>

      <LibrarySelector />

      <div className={styles.searchWrap}>
        <input
          type="search"
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
        <button type="button" className="btn" onClick={onUploadClick}>
          上传
        </button>
        <Link to="/admin/libraries" className="btn btn-ghost">
          图书馆
        </Link>
        <Link to="/admin/koreader" className="btn btn-ghost">
          KOReader
        </Link>
        {user?.role === "system_admin" && (
          <Link to="/admin/users" className="btn btn-ghost">
            用户
          </Link>
        )}
        <button
          type="button"
          className="btn btn-ghost"
          onClick={() => {
            clearAuth();
            navigate("/login");
          }}
        >
          {user?.username ?? "登录"}
        </button>
      </div>
    </header>
  );
}
