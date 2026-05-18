import { useEffect, useRef, useState } from "react";
import { Link, useNavigate } from "react-router-dom";
import { clearAuth, getUser } from "@/lib/auth";
import styles from "./UserMenu.module.css";

export function UserMenu() {
  const user = getUser();
  const navigate = useNavigate();
  const [open, setOpen] = useState(false);
  const rootRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    const onDocClick = (e: MouseEvent) => {
      if (rootRef.current && !rootRef.current.contains(e.target as Node)) {
        setOpen(false);
      }
    };
    document.addEventListener("mousedown", onDocClick);
    return () => document.removeEventListener("mousedown", onDocClick);
  }, [open]);

  const logout = () => {
    clearAuth();
    setOpen(false);
    navigate("/login");
  };

  if (!user) {
    return (
      <Link to="/login" className="btn btn-ghost">
        登录
      </Link>
    );
  }

  const isSystemAdmin = user.role === "system_admin";

  return (
    <div className={styles.wrap} ref={rootRef}>
      <button
        type="button"
        className={styles.trigger}
        onClick={() => setOpen((v) => !v)}
        aria-expanded={open}
        aria-haspopup="menu"
      >
        <span className={styles.avatar}>{user.username.slice(0, 1).toUpperCase()}</span>
        <span className={styles.name}>{user.username}</span>
        <span className={styles.chevron} aria-hidden>
          ▾
        </span>
      </button>

      {open && (
        <div className={styles.menu} role="menu">
          <Link
            to="/admin/libraries"
            className={styles.item}
            role="menuitem"
            onClick={() => setOpen(false)}
          >
            图书馆管理
          </Link>
          <Link
            to="/admin/koreader"
            className={styles.item}
            role="menuitem"
            onClick={() => setOpen(false)}
          >
            KOReader 同步
          </Link>
          {isSystemAdmin && (
            <Link
              to="/admin/users"
              className={styles.item}
              role="menuitem"
              onClick={() => setOpen(false)}
            >
              管理用户
            </Link>
          )}
          <Link
            to="/account/password"
            className={styles.item}
            role="menuitem"
            onClick={() => setOpen(false)}
          >
            修改密码
          </Link>
          <button type="button" className={styles.item} role="menuitem" onClick={logout}>
            退出登录
          </button>
        </div>
      )}
    </div>
  );
}
