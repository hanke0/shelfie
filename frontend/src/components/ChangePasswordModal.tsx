import { useState } from "react";
import { useChangePassword } from "@/api/generated/users/users";
import { getUser } from "@/lib/auth";
import { Modal } from "@/components/ui/Modal";
import { FieldLabel } from "@/components/ui/FieldLabel";
import formStyles from "@/components/ui/Form.module.css";

interface ChangePasswordModalProps {
  open: boolean;
  onClose: () => void;
}

export function ChangePasswordModal({ open, onClose }: ChangePasswordModalProps) {
  const user = getUser();
  const changePassword = useChangePassword();
  const [currentPassword, setCurrentPassword] = useState("");
  const [newPassword, setNewPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");
  const [error, setError] = useState<string | null>(null);

  const reset = () => {
    setCurrentPassword("");
    setNewPassword("");
    setConfirmPassword("");
    setError(null);
  };

  const handleClose = () => {
    reset();
    onClose();
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!user) return;
    setError(null);
    if (newPassword.length < 6) {
      setError("新密码至少 6 位");
      return;
    }
    if (newPassword !== confirmPassword) {
      setError("两次输入的新密码不一致");
      return;
    }
    try {
      await changePassword.mutateAsync({
        userId: user.id,
        data: { current_password: currentPassword, new_password: newPassword },
      });
      handleClose();
    } catch (err) {
      setError(err instanceof Error ? err.message : "修改失败");
    }
  };

  return (
    <Modal
      open={open}
      onClose={handleClose}
      title="修改密码"
      footer={
        <>
          <button type="button" className="btn btn-ghost" onClick={handleClose}>
            取消
          </button>
          <button
            type="submit"
            form="change-password-form"
            className="btn"
            disabled={changePassword.isPending}
          >
            {changePassword.isPending ? "保存中…" : "保存"}
          </button>
        </>
      }
    >
      <form id="change-password-form" className={formStyles.form} onSubmit={(e) => void handleSubmit(e)}>
        {error && <p className={formStyles.error}>{error}</p>}
        <label className={formStyles.field}>
          <FieldLabel required>当前密码</FieldLabel>
          <input
            type="password"
            value={currentPassword}
            onChange={(e) => setCurrentPassword(e.target.value)}
            required
            autoComplete="current-password"
          />
        </label>
        <label className={formStyles.field}>
          <FieldLabel required>新密码</FieldLabel>
          <input
            type="password"
            value={newPassword}
            onChange={(e) => setNewPassword(e.target.value)}
            required
            minLength={6}
            autoComplete="new-password"
          />
        </label>
        <label className={formStyles.field}>
          <FieldLabel required>确认新密码</FieldLabel>
          <input
            type="password"
            value={confirmPassword}
            onChange={(e) => setConfirmPassword(e.target.value)}
            required
            minLength={6}
            autoComplete="new-password"
          />
        </label>
      </form>
    </Modal>
  );
}
