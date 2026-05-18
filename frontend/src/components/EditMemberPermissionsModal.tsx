import { useEffect, useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { useUpdatePermissions } from "@/api/generated/libraries/libraries";
import { Modal } from "@/components/ui/Modal";
import { Select } from "@/components/ui/Select";
import { FieldLabel } from "@/components/ui/FieldLabel";
import formStyles from "@/components/ui/Form.module.css";
import { useApiAction } from "@/hooks/useApiAction";
import type { MemberPermState } from "@/components/LibraryMemberPermissionsEditor";

interface EditMemberPermissionsModalProps {
  open: boolean;
  onClose: () => void;
  libraryId: string;
  userId: string;
  username: string;
  libraryName?: string;
  member: MemberPermState;
}

export function EditMemberPermissionsModal({
  open,
  onClose,
  libraryId,
  userId,
  username,
  libraryName,
  member,
}: EditMemberPermissionsModalProps) {
  const qc = useQueryClient();
  const updatePermissions = useUpdatePermissions();
  const run = useApiAction();
  const [role, setRole] = useState(member.role);
  const [canView, setCanView] = useState(member.can_view ?? false);
  const [canEdit, setCanEdit] = useState(member.can_edit ?? false);
  const [canDelete, setCanDelete] = useState(member.can_delete ?? false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!open) return;
    setRole(member.role);
    setCanView(member.can_view ?? false);
    setCanEdit(member.can_edit ?? false);
    setCanDelete(member.can_delete ?? false);
    setError(null);
  }, [open, member]);

  const isLibraryAdmin = role === "admin";

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    const ok = await run(
      async () => {
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
      },
      { successMessage: "权限已保存", errorMessage: "保存失败" },
    );
    if (ok) onClose();
  };

  const title = libraryName
    ? `编辑权限 · ${username} @ ${libraryName}`
    : `编辑权限 · ${username}`;

  return (
    <Modal
      open={open}
      onClose={onClose}
      title={title}
      footer={
        <>
          <button type="button" className="btn btn-ghost" onClick={onClose}>
            取消
          </button>
          <button
            type="submit"
            form="edit-member-perm-form"
            className="btn"
            disabled={updatePermissions.isPending}
          >
            {updatePermissions.isPending ? "保存中…" : "保存"}
          </button>
        </>
      }
    >
      <form
        id="edit-member-perm-form"
        className={formStyles.form}
        onSubmit={(e) => void handleSubmit(e)}
      >
        {error && <p className={formStyles.error}>{error}</p>}
        <label className={formStyles.field}>
          <FieldLabel>馆内角色</FieldLabel>
          <Select value={role} onChange={(e) => setRole(e.target.value)}>
            <option value="member">成员</option>
            <option value="admin">馆管理员</option>
          </Select>
        </label>
        {isLibraryAdmin ? (
          <p className={formStyles.hint}>馆管理员在本馆拥有全部图书权限，并可拉人/踢人。</p>
        ) : (
          <fieldset className={formStyles.permFieldset}>
            <legend>馆内权限</legend>
            <label className={formStyles.checkLabel}>
              <input
                type="checkbox"
                checked={canView}
                onChange={(e) => setCanView(e.target.checked)}
              />
              查看
            </label>
            <label className={formStyles.checkLabel}>
              <input
                type="checkbox"
                checked={canEdit}
                onChange={(e) => setCanEdit(e.target.checked)}
              />
              编辑
            </label>
            <label className={formStyles.checkLabel}>
              <input
                type="checkbox"
                checked={canDelete}
                onChange={(e) => setCanDelete(e.target.checked)}
              />
              删除
            </label>
          </fieldset>
        )}
      </form>
    </Modal>
  );
}
