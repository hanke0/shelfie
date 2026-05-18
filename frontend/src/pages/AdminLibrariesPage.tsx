import { useMemo, useState } from "react";
import { Link } from "react-router-dom";
import {
  useListLibraries,
  useListMembers,
} from "@/api/generated/libraries/libraries";
import { useRefreshLibrary } from "@/api/generated/sync/sync";
import { CreateLibraryModal } from "@/components/CreateLibraryModal";
import { AddMemberModal } from "@/components/AddMemberModal";
import { LibraryMemberPermissionsEditor } from "@/components/LibraryMemberPermissionsEditor";
import { getUser } from "@/lib/auth";
import { useLibrary } from "@/context/LibraryContext";
import styles from "./AdminPage.module.css";

export function AdminLibrariesPage() {
  const user = getUser();
  const isSystemAdmin = user?.role === "system_admin";
  const { libraryId: contextLibraryId, setLibraryId, library } = useLibrary();
  const { data: libraries } = useListLibraries();
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

  const [createLibOpen, setCreateLibOpen] = useState(false);
  const [addMemberOpen, setAddMemberOpen] = useState(false);
  const [refreshResult, setRefreshResult] = useState<string | null>(null);

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

      <div className={styles.card}>
        <div className={styles.cardHeader}>
          <h2>图书馆列表</h2>
          {isSystemAdmin && (
            <button type="button" className="btn" onClick={() => setCreateLibOpen(true)}>
              创建图书馆
            </button>
          )}
        </div>
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
            <div className={styles.cardHeader}>
              <h2>{canManageMembers ? "成员管理" : "我的权限"}</h2>
              {canManageMembers && (
                <button type="button" className="btn" onClick={() => setAddMemberOpen(true)}>
                  添加成员
                </button>
              )}
            </div>
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
                    username={m.username}
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

      {isSystemAdmin && (
        <CreateLibraryModal open={createLibOpen} onClose={() => setCreateLibOpen(false)} />
      )}
      {selectedLib && (
        <AddMemberModal
          open={addMemberOpen}
          onClose={() => setAddMemberOpen(false)}
          libraryId={selectedLib}
          isSystemAdmin={isSystemAdmin}
        />
      )}
    </div>
  );
}
