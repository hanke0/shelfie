import { Link, Navigate } from "react-router-dom";
import { useState } from "react";
import { useListUsers, useListUserMemberships } from "@/api/generated/users/users";
import { CreateUserModal } from "@/components/CreateUserModal";
import { UserAccountModal } from "@/components/UserAccountModal";
import { LibraryMemberPermissionsEditor } from "@/components/LibraryMemberPermissionsEditor";
import { getUser } from "@/lib/auth";
import styles from "./AdminPage.module.css";

export function AdminUsersPage() {
  const user = getUser();
  const isSystemAdmin = user?.role === "system_admin";
  const { data: users } = useListUsers({ query: { enabled: isSystemAdmin } });

  const [selectedUserId, setSelectedUserId] = useState<string | null>(null);
  const [createOpen, setCreateOpen] = useState(false);
  const [accountOpen, setAccountOpen] = useState(false);

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

  return (
    <div className={`app-shell ${styles.page}`}>
      <Link to="/" className={styles.back}>
        ← 返回首页
      </Link>
      <h1>管理用户</h1>
      <p className={styles.muted}>
        创建用户、管理账号，以及配置各图书馆内的成员角色与权限。
      </p>

      <div className={styles.card}>
        <div className={styles.cardHeader}>
          <h2>用户列表</h2>
          <button type="button" className="btn" onClick={() => setCreateOpen(true)}>
            创建用户
          </button>
        </div>
        <p className={styles.muted}>点击用户名查看详情与权限。</p>
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
          <div className={styles.cardHeader}>
            <h2>{selectedUser.username}</h2>
            <button type="button" className="btn btn-ghost" onClick={() => setAccountOpen(true)}>
              账号管理
            </button>
          </div>
          <p className={styles.muted}>
            {selectedUser.role === "system_admin" ? "系统管理员" : "普通用户"}
          </p>

          <h3 className={styles.subheading}>图书馆权限</h3>
          {membershipsLoading && <p className={styles.muted}>加载中…</p>}
          {membershipsError && (
            <p className={styles.error}>
              无法加载：{" "}
              {membershipsErrorDetail instanceof Error
                ? membershipsErrorDetail.message
                : "请求失败"}{" "}
              <button
                type="button"
                className="btn btn-ghost"
                onClick={() => void refetchMemberships()}
              >
                重试
              </button>
            </p>
          )}
          {!membershipsLoading && !membershipsError && selectedUser.role === "system_admin" && (
            <p className={styles.muted}>
              系统管理员可访问所有图书馆。
              {memberships && memberships.length > 0 ? " 下方为额外的馆内成员记录。" : ""}
            </p>
          )}
          {!membershipsLoading &&
            !membershipsError &&
            memberships &&
            memberships.length > 0 && (
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
                      username={selectedUser.username}
                      libraryName={m.library_name}
                      member={{
                        role: m.role,
                        can_view: m.can_view,
                        can_edit: m.can_edit,
                        can_delete: m.can_delete,
                      }}
                      canEditPermissions={selectedUser.role !== "system_admin"}
                      canRemoveMember={false}
                    />
                  </li>
                ))}
              </ul>
            )}
          {!membershipsLoading &&
            !membershipsError &&
            selectedUser.role !== "system_admin" &&
            (!memberships || memberships.length === 0) && (
              <p className={styles.muted}>尚未加入任何图书馆。可在「图书馆管理」中添加成员。</p>
            )}
        </div>
      )}

      <CreateUserModal open={createOpen} onClose={() => setCreateOpen(false)} />
      {selectedUser && (
        <UserAccountModal
          open={accountOpen}
          onClose={() => setAccountOpen(false)}
          targetUserId={selectedUser.id}
          targetUsername={selectedUser.username}
          onDeleted={() => setSelectedUserId(null)}
        />
      )}
    </div>
  );
}
