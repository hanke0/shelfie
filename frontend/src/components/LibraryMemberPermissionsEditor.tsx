import { useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { useRemoveMember } from "@/api/generated/libraries/libraries";
import { EditMemberPermissionsModal } from "@/components/EditMemberPermissionsModal";
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
  username,
  libraryName,
  member,
  canEditPermissions,
  canRemoveMember,
  onSaved,
}: {
  libraryId: string;
  userId: string;
  username?: string;
  libraryName?: string;
  member: MemberPermState;
  canEditPermissions: boolean;
  canRemoveMember: boolean;
  onSaved?: () => void;
}) {
  const qc = useQueryClient();
  const removeMember = useRemoveMember();
  const [editOpen, setEditOpen] = useState(false);
  const [removing, setRemoving] = useState(false);
  const [error, setError] = useState<string | null>(null);

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
    <>
      <div className={styles.memberActions}>
        <span className={styles.muted}>{permLabel(member)}</span>
        <div className={styles.rowActions}>
          {canEditPermissions && (
            <button type="button" className="btn btn-ghost" onClick={() => setEditOpen(true)}>
              编辑权限
            </button>
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
        </div>
        {error && <span className={styles.error}>{error}</span>}
      </div>
      {canEditPermissions && username && (
        <EditMemberPermissionsModal
          open={editOpen}
          onClose={() => setEditOpen(false)}
          libraryId={libraryId}
          userId={userId}
          username={username}
          libraryName={libraryName}
          member={member}
        />
      )}
    </>
  );
}
