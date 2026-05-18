import { useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { useChangePassword, useDeleteUser } from "@/api/generated/users/users";
import { getUser } from "@/lib/auth";
import { Modal } from "@/components/ui/Modal";
import { FieldLabel } from "@/components/ui/FieldLabel";
import formStyles from "@/components/ui/Form.module.css";

interface UserAccountModalProps {
  open: boolean;
  onClose: () => void;
  targetUserId: string;
  targetUsername: string;
  onDeleted?: () => void;
}

export function UserAccountModal({
  open,
  onClose,
  targetUserId,
  targetUsername,
  onDeleted,
}: UserAccountModalProps) {
  const currentUser = getUser();
  const qc = useQueryClient();
  const changePassword = useChangePassword();
  const deleteUser = useDeleteUser();
  const [newPassword, setNewPassword] = useState("");
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const isSelf = currentUser?.id === targetUserId;

  const reset = () => {
    setNewPassword("");
    setMessage(null);
    setError(null);
  };

  const handleClose = () => {
    reset();
    onClose();
  };

  const handlePassword = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    setMessage(null);
    if (newPassword.length < 6) {
      setError("密码至少 6 位");
      return;
    }
    try {
      await changePassword.mutateAsync({
        userId: targetUserId,
        data: { new_password: newPassword },
      });
      setNewPassword("");
      setMessage("密码已更新");
    } catch (err) {
      setError(err instanceof Error ? err.message : "修改失败");
    }
  };

  const handleDelete = async () => {
    if (!window.confirm(`确定删除用户「${targetUsername}」？此操作不可恢复。`)) return;
    setError(null);
    try {
      await deleteUser.mutateAsync({ userId: targetUserId });
      await qc.invalidateQueries({ queryKey: ["/users"] });
      onDeleted?.();
      handleClose();
    } catch (err) {
      setError(err instanceof Error ? err.message : "删除失败");
    }
  };

  return (
    <Modal open={open} onClose={handleClose} title={`账号 · ${targetUsername}`}>
      {error && <p className={formStyles.error}>{error}</p>}
      {message && <p className={formStyles.hint}>{message}</p>}
      <form className={formStyles.form} onSubmit={(e) => void handlePassword(e)}>
        <label className={formStyles.field}>
          <FieldLabel>新密码</FieldLabel>
          <input
            type="password"
            placeholder="至少 6 位"
            value={newPassword}
            onChange={(e) => setNewPassword(e.target.value)}
            minLength={6}
          />
        </label>
        <button type="submit" className="btn" disabled={changePassword.isPending || !newPassword}>
          {changePassword.isPending ? "保存中…" : "重置密码"}
        </button>
      </form>
      {!isSelf && (
        <button
          type="button"
          className="btn btn-ghost"
          style={{ marginTop: "1rem" }}
          onClick={() => void handleDelete()}
          disabled={deleteUser.isPending}
        >
          {deleteUser.isPending ? "删除中…" : "删除用户"}
        </button>
      )}
    </Modal>
  );
}
