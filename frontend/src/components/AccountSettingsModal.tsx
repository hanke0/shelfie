import { useEffect, useState } from "react";
import { useChangePassword, useUpdateUsername } from "@/api/generated/users/users";
import { getUser, setAuth, useStoredUser } from "@/lib/auth";
import { Modal } from "@/components/ui/Modal";
import { FieldLabel } from "@/components/ui/FieldLabel";
import formStyles from "@/components/ui/Form.module.css";
import { useApiAction } from "@/hooks/useApiAction";

interface AccountSettingsModalProps {
  open: boolean;
  onClose: () => void;
}

export function AccountSettingsModal({ open, onClose }: AccountSettingsModalProps) {
  const storedUser = useStoredUser();
  const user = storedUser ?? getUser();
  const updateUsername = useUpdateUsername();
  const changePassword = useChangePassword();
  const run = useApiAction();

  const [username, setUsername] = useState("");
  const [usernameError, setUsernameError] = useState<string | null>(null);

  const [currentPassword, setCurrentPassword] = useState("");
  const [newPassword, setNewPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");
  const [passwordError, setPasswordError] = useState<string | null>(null);

  useEffect(() => {
    if (open && user) {
      setUsername(user.username);
    }
  }, [open, user]);

  const reset = () => {
    setUsername(user?.username ?? "");
    setUsernameError(null);
    setCurrentPassword("");
    setNewPassword("");
    setConfirmPassword("");
    setPasswordError(null);
  };

  const handleClose = () => {
    reset();
    onClose();
  };

  const handleUsernameSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!user) return;
    setUsernameError(null);
    const trimmed = username.trim();
    if (!trimmed) {
      setUsernameError("用户名不能为空");
      return;
    }
    if (trimmed === user.username) {
      return;
    }
    const ok = await run(
      async () => {
        const res = await updateUsername.mutateAsync({
          userId: user.id,
          data: { username: trimmed },
        });
        setAuth(res.token, res.user);
      },
      { successMessage: "用户名已更新", errorMessage: "修改用户名失败" },
    );
    if (ok) reset();
  };

  const handlePasswordSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!user) return;
    setPasswordError(null);
    if (newPassword.length < 6) {
      setPasswordError("新密码至少 6 位");
      return;
    }
    if (newPassword !== confirmPassword) {
      setPasswordError("两次输入的新密码不一致");
      return;
    }
    const ok = await run(
      () =>
        changePassword.mutateAsync({
          userId: user.id,
          data: { current_password: currentPassword, new_password: newPassword },
        }),
      { successMessage: "密码已修改", errorMessage: "修改失败" },
    );
    if (ok) {
      setCurrentPassword("");
      setNewPassword("");
      setConfirmPassword("");
      setPasswordError(null);
    }
  };

  const usernameBusy = updateUsername.isPending;
  const passwordBusy = changePassword.isPending;

  return (
    <Modal
      open={open}
      onClose={handleClose}
      title="账号设置"
      footer={
        <button type="button" className="btn btn-ghost" onClick={handleClose}>
          关闭
        </button>
      }
    >
      <form
        id="account-username-form"
        className={formStyles.form}
        onSubmit={(e) => void handleUsernameSubmit(e)}
      >
        <h3 className={formStyles.sectionTitle}>用户名</h3>
        {usernameError && <p className={formStyles.error}>{usernameError}</p>}
        <label className={formStyles.field}>
          <FieldLabel required>用户名</FieldLabel>
          <input
            value={username}
            onChange={(e) => setUsername(e.target.value)}
            required
            maxLength={64}
            autoComplete="username"
          />
        </label>
        <button
          type="submit"
          className="btn"
          disabled={usernameBusy || !username.trim() || username.trim() === user?.username}
        >
          {usernameBusy ? "保存中…" : "保存用户名"}
        </button>
      </form>

      <form
        id="account-password-form"
        className={formStyles.form}
        onSubmit={(e) => void handlePasswordSubmit(e)}
      >
        <h3 className={formStyles.sectionTitle}>密码</h3>
        {passwordError && <p className={formStyles.error}>{passwordError}</p>}
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
        <button type="submit" className="btn" disabled={passwordBusy}>
          {passwordBusy ? "保存中…" : "保存密码"}
        </button>
      </form>
    </Modal>
  );
}
