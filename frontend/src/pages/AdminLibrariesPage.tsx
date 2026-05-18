import { useMemo, useState } from "react";
import { Link } from "react-router-dom";
import { useQueryClient } from "@tanstack/react-query";
import {
  useDeleteLibrary,
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
  const qc = useQueryClient();
  const { libraryId: contextLibraryId, setLibraryId, library } = useLibrary();
  const { data: libraries } = useListLibraries();
  const refreshLibrary = useRefreshLibrary();
  const deleteLibrary = useDeleteLibrary();

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
  const [deleteError, setDeleteError] = useState<string | null>(null);

  const selectedLibrary = libraries?.find((lib) => lib.id === selectedLib);

  const handleDeleteLibrary = async () => {
    if (!selectedLib || !selectedLibrary) return;
    const ok = window.confirm(
      `确定删除图书馆「${selectedLibrary.name}」？\n仅当该馆目录下没有任何文件时才能删除（数据库记录与成员将一并清除）。`,
    );
    if (!ok) return;
    setDeleteError(null);
    try {
      await deleteLibrary.mutateAsync({ id: selectedLib });
      await qc.invalidateQueries({ queryKey: ["/libraries"] });
      if (contextLibraryId === selectedLib) {
        setLibraryId(null);
      }
      setSelectedLib("");
    } catch (err) {
      setDeleteError(err instanceof Error ? err.message : "删除失败");
    }
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
        {isSystemAdmin && selectedLib && selectedLibrary && (
          <div className={styles.dangerZone}>
            <p className={styles.muted}>
              删除前请确认磁盘目录为空（无图书、封面、metadata 等任何文件）。可先移走或删除文件后再操作。
            </p>
            {deleteError && <p className={styles.deleteError}>{deleteError}</p>}
            <button
              type="button"
              className={`btn ${styles.deleteBtn}`}
              disabled={deleteLibrary.isPending}
              onClick={() => void handleDeleteLibrary()}
            >
              {deleteLibrary.isPending ? "删除中…" : "删除此图书馆"}
            </button>
          </div>
        )}
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
