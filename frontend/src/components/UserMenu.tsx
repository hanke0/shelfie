import { useEffect, useRef, useState } from "react";
import { Link, useNavigate } from "react-router-dom";
import { clearAuth, useStoredUser } from "@/lib/auth";
import { AccountSettingsModal } from "@/components/AccountSettingsModal";
import { CalibreImportModal } from "@/components/CalibreImportModal";
import styles from "./UserMenu.module.css";

interface UserMenuProps {
  className?: string;
}

export function UserMenu({ className }: UserMenuProps) {
  const user = useStoredUser();
  const navigate = useNavigate();
  const [open, setOpen] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [calibreOpen, setCalibreOpen] = useState(false);
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
      <Link
        to="/login"
        className={["btn", "btn-ghost", className].filter(Boolean).join(" ")}
      >
        登录
      </Link>
    );
  }

  const isSystemAdmin = user.role === "system_admin";

  return (
    <>
      <div className={[styles.wrap, className].filter(Boolean).join(" ")} ref={rootRef}>
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
            <button
              type="button"
              className={styles.item}
              role="menuitem"
              onClick={() => {
                setOpen(false);
                setCalibreOpen(true);
              }}
            >
              从 Calibre 导入
            </button>
            <Link
              to="/reading-history"
              className={styles.item}
              role="menuitem"
              onClick={() => setOpen(false)}
            >
              阅读记录
            </Link>
            <button
              type="button"
              className={styles.item}
              role="menuitem"
              onClick={() => {
                setOpen(false);
                setSettingsOpen(true);
              }}
            >
              账号设置
            </button>
            <button type="button" className={styles.item} role="menuitem" onClick={logout}>
              退出登录
            </button>
          </div>
        )}
      </div>

      <AccountSettingsModal open={settingsOpen} onClose={() => setSettingsOpen(false)} />
      <CalibreImportModal open={calibreOpen} onClose={() => setCalibreOpen(false)} />
    </>
  );
}
