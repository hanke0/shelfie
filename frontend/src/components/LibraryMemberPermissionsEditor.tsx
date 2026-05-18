import { useEffect, useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import {
  useRemoveMember,
  useUpdatePermissions,
} from "@/api/generated/libraries/libraries";
import styles from "@/pages/AdminPage.module.css";

export type MemberPermState = {
  role: string;
  can_view?: boolean;
  can_edit?: boolean;
  can_delete?: boolean;
};

function permLabel(m: MemberPermState) {
  if (m.role === "admin") return "馆管理员（本馆全部权限，可拉人/踢人）";
  return `看 ${m.can_view ? "✓" : "✗"} · 编 ${m.can_edit ? "✓" : "✗"} · 删 ${m.can_delete ? "✓" : "✗"}`;
}

export function LibraryMemberPermissionsEditor({
  libraryId,
  userId,
  member,
  canEditPermissions,
  canRemoveMember,
  onSaved,
}: {
  libraryId: string;
  userId: string;
  member: MemberPermState;
  /** 系统管理员：可改角色与细粒度权限 */
  canEditPermissions: boolean;
  /** 馆管理员可踢普通成员；系统管理员可踢任何人 */
  canRemoveMember: boolean;
  onSaved?: () => void;
}) {
  const qc = useQueryClient();
  const updatePermissions = useUpdatePermissions();
  const removeMember = useRemoveMember();
  const [role, setRole] = useState(member.role);
  const [canView, setCanView] = useState(member.can_view ?? false);
  const [canEdit, setCanEdit] = useState(member.can_edit ?? false);
  const [canDelete, setCanDelete] = useState(member.can_delete ?? false);
  const [saving, setSaving] = useState(false);
  const [removing, setRemoving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    setRole(member.role);
    setCanView(member.can_view ?? false);
    setCanEdit(member.can_edit ?? false);
    setCanDelete(member.can_delete ?? false);
    setError(null);
  }, [libraryId, userId, member.role, member.can_view, member.can_edit, member.can_delete]);

  const isLibraryAdmin = role === "admin";

  const handleSave = async (e: React.FormEvent) => {
    e.preventDefault();
    setSaving(true);
    setError(null);
    try {
      await updatePermissions.mutateAsync({
        id: libraryId,
        userId,
        data: {
          role,
          can_view: isLibraryAdmin ? true : canView,
          can_edit: isLibraryAdmin ? true : canEdit,
          can_delete: isLibraryAdmin ? true : canDelete,
        },
      });
      await qc.invalidateQueries({ queryKey: [`/libraries/${libraryId}/members`] });
      await qc.invalidateQueries({ queryKey: [`/users/${userId}/memberships`] });
      onSaved?.();
    } catch (err) {
      setError(err instanceof Error ? err.message : "保存失败");
    } finally {
      setSaving(false);
    }
  };

  const handleRemove = async () => {
    if (!window.confirm("确定将该用户移出本图书馆？")) return;
    setRemoving(true);
    setError(null);
    try {
      await removeMember.mutateAsync({ id: libraryId, userId });
      await qc.invalidateQueries({ queryKey: [`/libraries/${libraryId}/members`] });
      await qc.invalidateQueries({ queryKey: [`/users/${userId}/memberships`] });
      onSaved?.();
    } catch (err) {
      setError(err instanceof Error ? err.message : "移除失败");
    } finally {
      setRemoving(false);
    }
  };

  return (
    <div className={styles.memberActions}>
      {canEditPermissions ? (
        <form className={styles.permForm} onSubmit={(e) => void handleSave(e)}>
          <label>
            馆内角色
            <select value={role} onChange={(e) => setRole(e.target.value)}>
              <option value="member">成员</option>
              <option value="admin">馆管理员</option>
            </select>
          </label>
          {!isLibraryAdmin && (
            <>
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
            </>
          )}
          {isLibraryAdmin && (
            <span className={styles.muted}>馆管理员在本馆拥有全部图书权限，并可拉人/踢人。</span>
          )}
          <button type="submit" className="btn btn-ghost" disabled={saving}>
            {saving ? "保存中…" : "保存权限"}
          </button>
        </form>
      ) : (
        <span className={styles.muted}>{permLabel(member)}</span>
      )}
      {canRemoveMember && (
        <button
          type="button"
          className="btn btn-ghost"
          disabled={removing}
          onClick={() => void handleRemove()}
        >
          {removing ? "移除中…" : "移出图书馆"}
        </button>
      )}
      {error && <span className={styles.error}>{error}</span>}
    </div>
  );
}
