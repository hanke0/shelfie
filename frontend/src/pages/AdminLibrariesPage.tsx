import { useState } from "react";
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
import { getUser } from "@/lib/auth";
import { useLibrary } from "@/context/LibraryContext";
import styles from "./AdminPage.module.css";

export function AdminLibrariesPage() {
  const user = getUser();
  const { libraryId: contextLibraryId, setLibraryId } = useLibrary();
  const qc = useQueryClient();
  const { data: libraries } = useListLibraries();
  const { data: users } = useListUsers({
    query: { enabled: user?.role === "system_admin" },
  });
  const createLibrary = useCreateLibrary();
  const addMember = useAddMember();
  const refreshLibrary = useRefreshLibrary();

  const [selectedLib, setSelectedLib] = useState<string>(contextLibraryId ?? "");
  const { data: members } = useListMembers(selectedLib, {
    query: { enabled: !!selectedLib },
  });

  const [name, setName] = useState("");
  const [slug, setSlug] = useState("");
  const [memberUserId, setMemberUserId] = useState("");
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
    if (!selectedLib || !memberUserId) return;
    await addMember.mutateAsync({
      id: selectedLib,
      data: {
        user_id: memberUserId,
        role: "member",
        can_view: true,
        can_edit: false,
        can_delete: false,
      },
    });
    setMemberUserId("");
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

      {user?.role === "system_admin" && (
        <form className={styles.card} onSubmit={(e) => void handleCreate(e)}>
          <h2>创建图书馆</h2>
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
            <h2>成员</h2>
            <ul className={styles.list}>
              {members?.map((m) => (
                <li key={m.user_id}>
                  {m.username} — {m.role}
                  {m.role === "member" && (
                    <span className={styles.muted}>
                      {" "}
                      (看{m.can_view ? "✓" : "✗"} 编{m.can_edit ? "✓" : "✗"} 删
                      {m.can_delete ? "✓" : "✗"})
                    </span>
                  )}
                </li>
              ))}
            </ul>

            {users && (
              <form className={styles.inlineForm} onSubmit={(e) => void handleAddMember(e)}>
                <select value={memberUserId} onChange={(e) => setMemberUserId(e.target.value)} required>
                  <option value="">添加用户…</option>
                  {users.map((u) => (
                    <option key={u.id} value={u.id}>
                      {u.username}
                    </option>
                  ))}
                </select>
                <button type="submit" className="btn">
                  添加成员
                </button>
              </form>
            )}
          </div>

          <div className={styles.card}>
            <h2>文件夹同步</h2>
            <p className={styles.muted}>扫描磁盘目录并与数据库 reconcile（默认以 FS metadata 为准）</p>
            <button type="button" className="btn" onClick={() => void handleRefresh()}>
              立即刷新
            </button>
            {refreshResult && <p>{refreshResult}</p>}
          </div>
        </>
      )}
    </div>
  );
}
