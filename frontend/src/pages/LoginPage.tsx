import { useState } from "react";
import { useNavigate } from "react-router-dom";
import { useLogin } from "@/api/generated/auth/auth";
import { setAuth } from "@/lib/auth";
import { FieldLabel } from "@/components/ui/FieldLabel";
import styles from "./LoginPage.module.css";

export function LoginPage() {
  const navigate = useNavigate();
  const [username, setUsername] = useState("admin");
  const [password, setPassword] = useState("admin123");
  const login = useLogin();

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    try {
      const res = await login.mutateAsync({ data: { username, password } });
      setAuth(res.token, res.user);
      navigate("/");
    } catch {
      /* error shown below */
    }
  };

  return (
    <div className={styles.page}>
      <form className={styles.card} onSubmit={handleSubmit}>
        <h1>
          <span className={styles.mark}>◈</span> Shelfie
        </h1>
        <p className={styles.sub}>登录你的书架</p>

        <label>
          <FieldLabel required>用户名</FieldLabel>
          <input value={username} onChange={(e) => setUsername(e.target.value)} autoComplete="username" required />
        </label>
        <label>
          <FieldLabel required>密码</FieldLabel>
          <input
            type="password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            autoComplete="current-password"
          />
        </label>

        {login.isError && <p className={styles.error}>登录失败，请检查凭据</p>}

        <button type="submit" className="btn" disabled={login.isPending}>
          {login.isPending ? "登录中…" : "登录"}
        </button>

        <p className={styles.hint}>默认管理员：admin / admin123</p>
      </form>
    </div>
  );
}
