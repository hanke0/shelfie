import { useState } from "react";
import { Link, useNavigate } from "react-router-dom";
import { useChangePassword } from "@/api/generated/users/users";
import { getUser } from "@/lib/auth";
import styles from "./AdminPage.module.css";

export function ChangePasswordPage() {
  const user = getUser();
  const navigate = useNavigate();
  const changePassword = useChangePassword();

  const [currentPassword, setCurrentPassword] = useState("");
  const [newPassword, setNewPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");
  const [error, setError] = useState<string | null>(null);

  if (!user) {
    return null;
  }

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    if (newPassword.length < 6) {
      setError("新密码至少 6 位");
      return;
    }
    if (newPassword !== confirmPassword) {
      setError("两次输入的新密码不一致");
      return;
    }
    try {
      await changePassword.mutateAsync({
        userId: user.id,
        data: {
          current_password: currentPassword,
          new_password: newPassword,
        },
      });
      navigate("/");
    } catch (err) {
      setError(err instanceof Error ? err.message : "修改失败");
    }
  };

  return (
    <div className={`app-shell ${styles.page}`}>
      <Link to="/" className={styles.back}>
        ← 返回首页
      </Link>
      <h1>修改密码</h1>
      <form className={styles.card} onSubmit={(e) => void handleSubmit(e)}>
        {error && <p className={styles.error}>{error}</p>}
        <label>
          当前密码
          <input
            type="password"
            value={currentPassword}
            onChange={(e) => setCurrentPassword(e.target.value)}
            required
            autoComplete="current-password"
          />
        </label>
        <label>
          新密码
          <input
            type="password"
            value={newPassword}
            onChange={(e) => setNewPassword(e.target.value)}
            required
            minLength={6}
            autoComplete="new-password"
          />
        </label>
        <label>
          确认新密码
          <input
            type="password"
            value={confirmPassword}
            onChange={(e) => setConfirmPassword(e.target.value)}
            required
            minLength={6}
            autoComplete="new-password"
          />
        </label>
        <button type="submit" className="btn" disabled={changePassword.isPending}>
          {changePassword.isPending ? "保存中…" : "保存"}
        </button>
      </form>
    </div>
  );
}
