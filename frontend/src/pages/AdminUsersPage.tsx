import { Link, Navigate } from "react-router-dom";
import { useState } from "react";
import {
  useListUsers,
  useListUserMemberships,
  useChangePassword,
  useDeleteUser,
} from "@/api/generated/users/users";
import { useListLibraries } from "@/api/generated/libraries/libraries";
import { useRegister } from "@/api/generated/auth/auth";
import { useQueryClient } from "@tanstack/react-query";
import { LibraryMemberPermissionsEditor } from "@/components/LibraryMemberPermissionsEditor";
import { getUser } from "@/lib/auth";
import styles from "./AdminPage.module.css";

export function AdminUsersPage() {
  const user = getUser();
  const isSystemAdmin = user?.role === "system_admin";
  const { data: users } = useListUsers({ query: { enabled: isSystemAdmin } });
  const { data: libraries } = useListLibraries({ query: { enabled: isSystemAdmin } });
  const register = useRegister();
  const changePassword = useChangePassword();
  const deleteUser = useDeleteUser();
  const qc = useQueryClient();

  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [role, setRole] = useState("user");
  const [libraryId, setLibraryId] = useState("");
  const [libraryRole, setLibraryRole] = useState("member");
  const [canView, setCanView] = useState(true);
  const [canEdit, setCanEdit] = useState(false);
  const [canDelete, setCanDelete] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [selectedUserId, setSelectedUserId] = useState<string | null>(null);
  const [newPassword, setNewPassword] = useState("");
  const [passwordMsg, setPasswordMsg] = useState<string | null>(null);

  const selectedUser = users?.find((u) => u.id === selectedUserId);
  const {
    data: memberships,
    isLoading: membershipsLoading,
    isError: membershipsError,
    error: membershipsErrorDetail,
    refetch: refetchMemberships,
  } = useListUserMemberships(selectedUserId ?? "", {
    query: { enabled: isSystemAdmin && !!selectedUserId },
  });

  if (!isSystemAdmin) {
    return <Navigate to="/admin/libraries" replace />;
  }

  const isRegularUser = role === "user";
  const isLibraryAdmin = libraryRole === "admin";

  const handleCreate = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    if (isRegularUser && !libraryId) {
      setError("请为普通用户选择所属图书馆");
      return;
    }
    try {
      await register.mutateAsync({
        data: {
          username,
          password,
          role,
          library_id: isRegularUser ? libraryId : undefined,
          library_role: isRegularUser ? libraryRole : undefined,
          can_view: isRegularUser && !isLibraryAdmin ? canView : true,
          can_edit: isRegularUser && !isLibraryAdmin ? canEdit : false,
          can_delete: isRegularUser && !isLibraryAdmin ? canDelete : false,
        },
      });
      setUsername("");
      setPassword("");
      setLibraryId("");
      setLibraryRole("member");
      setCanView(true);
      setCanEdit(false);
      setCanDelete(false);
      qc.invalidateQueries({ queryKey: ["/users"] });
    } catch (err) {
      setError(err instanceof Error ? err.message : "创建失败");
    }
  };

  return (
    <div className={`app-shell ${styles.page}`}>
      <Link to="/" className={styles.back}>
        ← 返回首页
      </Link>
      <h1>管理用户</h1>
      <p className={styles.muted}>
        仅系统管理员可创建用户与图书馆。新建普通用户时必须指定所属图书馆及权限；选中用户后可编辑其在各馆的成员角色与权限。
      </p>

      <form className={styles.card} onSubmit={(e) => void handleCreate(e)}>
        <h2>创建用户</h2>
        {error && <p className={styles.error}>{error}</p>}

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
          系统角色
          <select value={role} onChange={(e) => setRole(e.target.value)}>
            <option value="user">普通用户</option>
            <option value="system_admin">系统管理员</option>
          </select>
        </label>

        {isRegularUser ? (
          <>
            <label>
              所属图书馆 <span className={styles.req}>*</span>
              <select
                value={libraryId}
                onChange={(e) => setLibraryId(e.target.value)}
                required
              >
                <option value="">选择图书馆…</option>
                {libraries?.map((lib) => (
                  <option key={lib.id} value={lib.id}>
                    {lib.name}
                  </option>
                ))}
              </select>
            </label>
            <label>
              馆内角色
              <select
                value={libraryRole}
                onChange={(e) => setLibraryRole(e.target.value)}
              >
                <option value="member">成员</option>
                <option value="admin">馆管理员</option>
              </select>
            </label>
            {isLibraryAdmin ? (
              <p className={styles.muted}>馆管理员拥有该馆全部权限（查看、编辑、删除、成员管理）。</p>
            ) : (
              <fieldset className={styles.permFieldset}>
                <legend>馆内权限</legend>
                <label>
                  <input
                    type="checkbox"
                    checked={canView}
                    onChange={(e) => setCanView(e.target.checked)}
                  />
                  查看
                </label>
                <label>
                  <input
                    type="checkbox"
                    checked={canEdit}
                    onChange={(e) => setCanEdit(e.target.checked)}
                  />
                  编辑
                </label>
                <label>
                  <input
                    type="checkbox"
                    checked={canDelete}
                    onChange={(e) => setCanDelete(e.target.checked)}
                  />
                  删除
                </label>
              </fieldset>
            )}
          </>
        ) : (
          <p className={styles.muted}>系统管理员可访问所有图书馆，无需单独分配馆内权限。</p>
        )}

        <button type="submit" className="btn">
          创建
        </button>
      </form>

      <div className={styles.card}>
        <h2>用户列表</h2>
        <p className={styles.muted}>点击用户名展开并编辑其图书馆权限。</p>
        <ul className={styles.list}>
          {users?.map((u) => (
            <li key={u.id}>
              <button
                type="button"
                className={selectedUserId === u.id ? styles.active : ""}
                onClick={() =>
                  setSelectedUserId((prev) => (prev === u.id ? null : u.id))
                }
              >
                {u.username}
              </button>
              <span className={styles.muted}>
                {u.role === "system_admin" ? "系统管理员" : "普通用户"}
              </span>
            </li>
          ))}
        </ul>
      </div>

      {selectedUser && (
        <div className={styles.card}>
          <h2>账号操作</h2>
          <form
            className={styles.inlineForm}
            onSubmit={(e) => {
              e.preventDefault();
              void (async () => {
                setPasswordMsg(null);
                if (newPassword.length < 6) {
                  setPasswordMsg("密码至少 6 位");
                  return;
                }
                try {
                  await changePassword.mutateAsync({
                    userId: selectedUser.id,
                    data: { new_password: newPassword },
                  });
                  setNewPassword("");
                  setPasswordMsg("密码已更新");
                } catch (err) {
                  setPasswordMsg(err instanceof Error ? err.message : "修改失败");
                }
              })();
            }}
          >
            <input
              type="password"
              placeholder="新密码（至少 6 位）"
              value={newPassword}
              onChange={(e) => setNewPassword(e.target.value)}
              minLength={6}
              required
            />
            <button type="submit" className="btn">
              重置密码
            </button>
          </form>
          {passwordMsg && <p className={styles.muted}>{passwordMsg}</p>}
          {selectedUser.id !== user?.id && (
            <button
              type="button"
              className="btn btn-ghost"
              style={{ marginTop: "0.75rem" }}
              onClick={() => {
                if (!window.confirm(`确定删除用户「${selectedUser.username}」？此操作不可恢复。`)) {
                  return;
                }
                void (async () => {
                  try {
                    await deleteUser.mutateAsync({ userId: selectedUser.id });
                    setSelectedUserId(null);
                    qc.invalidateQueries({ queryKey: ["/users"] });
                  } catch (err) {
                    setPasswordMsg(err instanceof Error ? err.message : "删除失败");
                  }
                })();
              }}
            >
              删除用户
            </button>
          )}
        </div>
      )}

      {selectedUser && (
        <div className={styles.card}>
          <h2>{selectedUser.username} 的图书馆权限</h2>
          {membershipsLoading && <p className={styles.muted}>加载中…</p>}
          {membershipsError && (
            <p className={styles.error}>
              无法加载图书馆权限：{" "}
              {membershipsErrorDetail instanceof Error
                ? membershipsErrorDetail.message
                : "请求失败"}
              。若刚更新过后端，请重启后端服务后{" "}
              <button
                type="button"
                className="btn btn-ghost"
                onClick={() => void refetchMemberships()}
              >
                重试
              </button>
            </p>
          )}
          {!membershipsLoading && !membershipsError && selectedUser.role === "system_admin" ? (
            <p className={styles.muted}>
              系统管理员可访问所有图书馆。
              {memberships && memberships.length > 0
                ? " 下方为额外的馆内成员记录（通常不必配置）。"
                : " 无需单独分配馆内权限。"}
            </p>
          ) : null}
          {!membershipsLoading &&
          !membershipsError &&
          selectedUser.role !== "system_admin" &&
          memberships &&
          memberships.length > 0 ? (
            <ul className={styles.memberList}>
              {memberships.map((m) => (
                <li key={m.library_id} className={styles.memberRow}>
                  <div className={styles.memberHead}>
                    <strong>{m.library_name}</strong>
                    <span className={styles.muted}>
                      {m.role === "admin" ? "馆管理员" : "成员"}
                    </span>
                  </div>
                  <LibraryMemberPermissionsEditor
                    libraryId={m.library_id}
                    userId={selectedUser.id}
                    member={{
                      role: m.role,
                      can_view: m.can_view,
                      can_edit: m.can_edit,
                      can_delete: m.can_delete,
                    }}
                    canEditPermissions={true}
                    canRemoveMember={false}
                  />
                </li>
              ))}
            </ul>
          ) : null}
          {!membershipsLoading &&
          !membershipsError &&
          selectedUser.role !== "system_admin" &&
          (!memberships || memberships.length === 0) ? (
            <p className={styles.muted}>
              该用户尚未加入任何图书馆。可在「图书馆管理」中添加成员，或创建用户时指定所属馆。
            </p>
          ) : null}
          {!membershipsLoading &&
          !membershipsError &&
          selectedUser.role === "system_admin" &&
          memberships &&
          memberships.length > 0 ? (
            <ul className={styles.memberList}>
              {memberships.map((m) => (
                <li key={m.library_id} className={styles.memberRow}>
                  <div className={styles.memberHead}>
                    <strong>{m.library_name}</strong>
                    <span className={styles.muted}>
                      {m.role === "admin" ? "馆管理员" : "成员"} · 看{" "}
                      {m.can_view ? "✓" : "✗"} · 编 {m.can_edit ? "✓" : "✗"} · 删{" "}
                      {m.can_delete ? "✓" : "✗"}
                    </span>
                  </div>
                </li>
              ))}
            </ul>
          ) : null}
        </div>
      )}
    </div>
  );
}
