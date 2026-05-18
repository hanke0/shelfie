import { useMemo, useState } from "react";
import { Link } from "react-router-dom";
import { useQueryClient } from "@tanstack/react-query";
import {
  useListLibraries,
  useCreateLibrary,
  useListMembers,
  useAddMember,
} from "@/api/generated/libraries/libraries";
import { useRefreshLibrary } from "@/api/generated/sync/sync";
import { useListUsers } from "@/api/generated/users/users";
import { LibraryMemberPermissionsEditor } from "@/components/LibraryMemberPermissionsEditor";
import { getUser } from "@/lib/auth";
import { useLibrary } from "@/context/LibraryContext";
import styles from "./AdminPage.module.css";

export function AdminLibrariesPage() {
  const user = getUser();
  const isSystemAdmin = user?.role === "system_admin";
  const { libraryId: contextLibraryId, setLibraryId, library } = useLibrary();
  const qc = useQueryClient();
  const { data: libraries } = useListLibraries();
  const { data: users } = useListUsers({
    query: { enabled: isSystemAdmin },
  });
  const createLibrary = useCreateLibrary();
  const addMember = useAddMember();
  const refreshLibrary = useRefreshLibrary();

  const [selectedLib, setSelectedLib] = useState<string>(contextLibraryId ?? "");
  const { data: members } = useListMembers(selectedLib, {
    query: { enabled: !!selectedLib },
  });

  const canManageMembers = useMemo(() => {
    if (isSystemAdmin) return true;
    if (!user || !members) return false;
    return members.some((m) => m.user_id === user.id && m.role === "admin");
  }, [isSystemAdmin, user, members]);

  const [name, setName] = useState("");
  const [slug, setSlug] = useState("");
  const [memberUserId, setMemberUserId] = useState("");
  const [memberUsername, setMemberUsername] = useState("");
  const [refreshResult, setRefreshResult] = useState<string | null>(null);

  const handleCreate = async (e: React.FormEvent) => {
    e.preventDefault();
    await createLibrary.mutateAsync({ data: { name, slug } });
    setName("");
    setSlug("");
    qc.invalidateQueries({ queryKey: ["/libraries"] });
  };

  const handleAddMember = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!selectedLib) return;
    const data = isSystemAdmin
      ? {
          user_id: memberUserId,
          role: "member" as const,
          can_view: true,
          can_edit: false,
          can_delete: false,
        }
      : {
          username: memberUsername.trim(),
          role: "member" as const,
          can_view: true,
          can_edit: false,
          can_delete: false,
        };
    if (isSystemAdmin && !memberUserId) return;
    if (!isSystemAdmin && !memberUsername.trim()) return;

    await addMember.mutateAsync({ id: selectedLib, data });
    setMemberUserId("");
    setMemberUsername("");
    qc.invalidateQueries({ queryKey: [`/libraries/${selectedLib}/members`] });
  };

  const handleRefresh = async () => {
    if (!selectedLib) return;
    const res = await refreshLibrary.mutateAsync({ id: selectedLib });
    setRefreshResult(
      res.result
        ? `新增 ${res.result.added.length}，更新 ${res.result.updated.length}，孤儿 ${res.result.removed.length}`
        : "完成",
    );
  };

  return (
    <div className={`app-shell ${styles.page}`}>
      <Link to="/" className={styles.back}>
        ← 返回首页
      </Link>
      <h1>图书馆管理</h1>
      {library && (
        <p className={styles.muted}>
          当前图书馆：<strong>{library.name}</strong>
        </p>
      )}

      {isSystemAdmin && (
        <form className={styles.card} onSubmit={(e) => void handleCreate(e)}>
          <h2>创建图书馆</h2>
          <p className={styles.muted}>仅系统管理员可创建新图书馆。</p>
          <label>
            名称
            <input value={name} onChange={(e) => setName(e.target.value)} required />
          </label>
          <label>
            Slug
            <input value={slug} onChange={(e) => setSlug(e.target.value)} required />
          </label>
          <button type="submit" className="btn">
            创建
          </button>
        </form>
      )}

      <div className={styles.card}>
        <h2>图书馆列表</h2>
        <ul className={styles.list}>
          {libraries?.map((lib) => (
            <li key={lib.id}>
              <button
                type="button"
                className={selectedLib === lib.id ? styles.active : ""}
                onClick={() => {
                  setSelectedLib(lib.id);
                  setLibraryId(lib.id);
                }}
              >
                {lib.name}
              </button>
              <span className={styles.muted}>{lib.root_path}</span>
            </li>
          ))}
        </ul>
      </div>

      {selectedLib && (
        <>
          <div className={styles.card}>
            <h2>{canManageMembers ? "成员管理" : "我的权限"}</h2>
            {canManageMembers && !isSystemAdmin && (
              <p className={styles.muted}>
                馆管理员可添加/移出普通成员；角色与细粒度权限由系统管理员设置。
              </p>
            )}
            {!canManageMembers && (
              <p className={styles.muted}>你只能查看自己在本馆的权限；成员管理请联系馆管理员。</p>
            )}
            <ul className={styles.memberList}>
              {members?.map((m) => (
                <li key={m.user_id} className={styles.memberRow}>
                  <div className={styles.memberHead}>
                    <strong>{m.username}</strong>
                    <span className={styles.muted}>
                      {m.role === "admin" ? "馆管理员" : "成员"}
                    </span>
                  </div>
                  <LibraryMemberPermissionsEditor
                    libraryId={selectedLib}
                    userId={m.user_id}
                    member={{
                      role: m.role,
                      can_view: m.can_view,
                      can_edit: m.can_edit,
                      can_delete: m.can_delete,
                    }}
                    canEditPermissions={isSystemAdmin}
                    canRemoveMember={
                      canManageMembers &&
                      (isSystemAdmin || m.role !== "admin") &&
                      m.user_id !== user?.id
                    }
                  />
                </li>
              ))}
            </ul>

            {canManageMembers && (
              <form className={styles.inlineForm} onSubmit={(e) => void handleAddMember(e)}>
                {isSystemAdmin && users ? (
                  <select
                    value={memberUserId}
                    onChange={(e) => setMemberUserId(e.target.value)}
                    required
                  >
                    <option value="">选择用户…</option>
                    {users.map((u) => (
                      <option key={u.id} value={u.id}>
                        {u.username}
                      </option>
                    ))}
                  </select>
                ) : (
                  <input
                    type="text"
                    placeholder="用户名"
                    value={memberUsername}
                    onChange={(e) => setMemberUsername(e.target.value)}
                    required
                  />
                )}
                <button type="submit" className="btn">
                  添加成员
                </button>
              </form>
            )}
          </div>

          {canManageMembers && (
            <div className={styles.card}>
              <h2>文件夹同步</h2>
              <p className={styles.muted}>
                扫描磁盘目录并与数据库 reconcile（默认以 FS metadata 为准）
              </p>
              <button type="button" className="btn" onClick={() => void handleRefresh()}>
                立即刷新
              </button>
              {refreshResult && <p>{refreshResult}</p>}
            </div>
          )}
        </>
      )}
    </div>
  );
}
