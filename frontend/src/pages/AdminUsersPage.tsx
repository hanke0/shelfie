import { Link } from "react-router-dom";
import { useState } from "react";
import { useListUsers } from "@/api/generated/users/users";
import { useRegister } from "@/api/generated/auth/auth";
import { useQueryClient } from "@tanstack/react-query";
import styles from "./AdminPage.module.css";

export function AdminUsersPage() {
  const { data: users } = useListUsers();
  const register = useRegister();
  const qc = useQueryClient();
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [role, setRole] = useState("user");

  const handleCreate = async (e: React.FormEvent) => {
    e.preventDefault();
    await register.mutateAsync({ data: { username, password, role } });
    setUsername("");
    setPassword("");
    qc.invalidateQueries({ queryKey: ["/users"] });
  };

  return (
    <div className={`app-shell ${styles.page}`}>
      <Link to="/" className={styles.back}>
        ← 返回首页
      </Link>
      <h1>用户管理</h1>

      <form className={styles.card} onSubmit={(e) => void handleCreate(e)}>
        <h2>创建用户</h2>
        <label>
          用户名
          <input value={username} onChange={(e) => setUsername(e.target.value)} required />
        </label>
        <label>
          密码
          <input
            type="password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            required
          />
        </label>
        <label>
          角色
          <select value={role} onChange={(e) => setRole(e.target.value)}>
            <option value="user">普通用户</option>
            <option value="system_admin">系统管理员</option>
          </select>
        </label>
        <button type="submit" className="btn">
          创建
        </button>
      </form>

      <div className={styles.card}>
        <h2>用户列表</h2>
        <ul className={styles.list}>
          {users?.map((u) => (
            <li key={u.id}>
              {u.username} <span className={styles.muted}>({u.role})</span>
            </li>
          ))}
        </ul>
      </div>
    </div>
  );
}
