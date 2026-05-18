import { useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { useAddMember } from "@/api/generated/libraries/libraries";
import { useListUsers } from "@/api/generated/users/users";
import { Modal } from "@/components/ui/Modal";
import { Select } from "@/components/ui/Select";
import formStyles from "@/components/ui/Form.module.css";

interface AddMemberModalProps {
  open: boolean;
  onClose: () => void;
  libraryId: string;
  isSystemAdmin: boolean;
}

export function AddMemberModal({
  open,
  onClose,
  libraryId,
  isSystemAdmin,
}: AddMemberModalProps) {
  const qc = useQueryClient();
  const addMember = useAddMember();
  const { data: users } = useListUsers({ query: { enabled: open && isSystemAdmin } });
  const [memberUserId, setMemberUserId] = useState("");
  const [memberUsername, setMemberUsername] = useState("");
  const [error, setError] = useState<string | null>(null);

  const reset = () => {
    setMemberUserId("");
    setMemberUsername("");
    setError(null);
  };

  const handleClose = () => {
    reset();
    onClose();
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!libraryId) return;
    setError(null);
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
    if (isSystemAdmin && !memberUserId) {
      setError("请选择用户");
      return;
    }
    if (!isSystemAdmin && !memberUsername.trim()) {
      setError("请输入用户名");
      return;
    }
    try {
      await addMember.mutateAsync({ id: libraryId, data });
      await qc.invalidateQueries({ queryKey: [`/libraries/${libraryId}/members`] });
      handleClose();
    } catch (err) {
      setError(err instanceof Error ? err.message : "添加失败");
    }
  };

  return (
    <Modal
      open={open}
      onClose={handleClose}
      title="添加成员"
      footer={
        <>
          <button type="button" className="btn btn-ghost" onClick={handleClose}>
            取消
          </button>
          <button
            type="submit"
            form="add-member-form"
            className="btn"
            disabled={addMember.isPending}
          >
            {addMember.isPending ? "添加中…" : "添加"}
          </button>
        </>
      }
    >
      <form id="add-member-form" className={formStyles.form} onSubmit={(e) => void handleSubmit(e)}>
        {error && <p className={formStyles.error}>{error}</p>}
        {isSystemAdmin && users ? (
          <label className={formStyles.field}>
            用户
            <Select
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
            </Select>
          </label>
        ) : (
          <label className={formStyles.field}>
            用户名
            <input
              type="text"
              value={memberUsername}
              onChange={(e) => setMemberUsername(e.target.value)}
              placeholder="Shelfie 登录用户名"
              required
            />
          </label>
        )}
        <p className={formStyles.hint}>新成员默认为「成员」角色，细粒度权限由系统管理员配置。</p>
      </form>
    </Modal>
  );
}
