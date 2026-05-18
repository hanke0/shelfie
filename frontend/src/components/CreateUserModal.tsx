import { useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { useRegister } from "@/api/generated/auth/auth";
import { useListLibraries } from "@/api/generated/libraries/libraries";
import { Modal } from "@/components/ui/Modal";
import { Select } from "@/components/ui/Select";
import { FieldLabel } from "@/components/ui/FieldLabel";
import formStyles from "@/components/ui/Form.module.css";

interface CreateUserModalProps {
  open: boolean;
  onClose: () => void;
}

export function CreateUserModal({ open, onClose }: CreateUserModalProps) {
  const qc = useQueryClient();
  const register = useRegister();
  const { data: libraries } = useListLibraries({ query: { enabled: open } });

  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [role, setRole] = useState("user");
  const [libraryId, setLibraryId] = useState("");
  const [libraryRole, setLibraryRole] = useState("member");
  const [canView, setCanView] = useState(true);
  const [canEdit, setCanEdit] = useState(false);
  const [canDelete, setCanDelete] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const isRegularUser = role === "user";
  const isLibraryAdmin = libraryRole === "admin";

  const reset = () => {
    setUsername("");
    setPassword("");
    setRole("user");
    setLibraryId("");
    setLibraryRole("member");
    setCanView(true);
    setCanEdit(false);
    setCanDelete(false);
    setError(null);
  };

  const handleClose = () => {
    reset();
    onClose();
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    if (isRegularUser && !libraryId) {
      setError("请为普通用户选择所属图书馆");
      return;
    }
    try {
      await register.mutateAsync({
        data: {
          username,
          password,
          role,
          library_id: isRegularUser ? libraryId : undefined,
          library_role: isRegularUser ? libraryRole : undefined,
          can_view: isRegularUser && !isLibraryAdmin ? canView : true,
          can_edit: isRegularUser && !isLibraryAdmin ? canEdit : false,
          can_delete: isRegularUser && !isLibraryAdmin ? canDelete : false,
        },
      });
      await qc.invalidateQueries({ queryKey: ["/users"] });
      handleClose();
    } catch (err) {
      setError(err instanceof Error ? err.message : "创建失败");
    }
  };

  return (
    <Modal
      open={open}
      onClose={handleClose}
      title="创建用户"
      wide
      footer={
        <>
          <button type="button" className="btn btn-ghost" onClick={handleClose}>
            取消
          </button>
          <button
            type="submit"
            form="create-user-form"
            className="btn"
            disabled={register.isPending}
          >
            {register.isPending ? "创建中…" : "创建"}
          </button>
        </>
      }
    >
      <form id="create-user-form" className={formStyles.form} onSubmit={(e) => void handleSubmit(e)}>
        {error && <p className={formStyles.error}>{error}</p>}
        <label className={formStyles.field}>
          <FieldLabel required>用户名</FieldLabel>
          <input value={username} onChange={(e) => setUsername(e.target.value)} required />
        </label>
        <label className={formStyles.field}>
          <FieldLabel required>密码</FieldLabel>
          <input
            type="password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            required
          />
        </label>
        <label className={formStyles.field}>
          <FieldLabel>系统角色</FieldLabel>
          <Select value={role} onChange={(e) => setRole(e.target.value)}>
            <option value="user">普通用户</option>
            <option value="system_admin">系统管理员</option>
          </Select>
        </label>
        {isRegularUser ? (
          <>
            <label className={formStyles.field}>
              <FieldLabel required>所属图书馆</FieldLabel>
              <Select
                value={libraryId}
                onChange={(e) => setLibraryId(e.target.value)}
                required
              >
                <option value="">选择图书馆…</option>
                {libraries?.map((lib) => (
                  <option key={lib.id} value={lib.id}>
                    {lib.name}
                  </option>
                ))}
              </Select>
            </label>
            <label className={formStyles.field}>
              <FieldLabel>馆内角色</FieldLabel>
              <Select value={libraryRole} onChange={(e) => setLibraryRole(e.target.value)}>
                <option value="member">成员</option>
                <option value="admin">馆管理员</option>
              </Select>
            </label>
            {isLibraryAdmin ? (
              <p className={formStyles.hint}>
                馆管理员拥有该馆全部权限（查看、编辑、删除、成员管理）。
              </p>
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
          </>
        ) : (
          <p className={formStyles.hint}>系统管理员可访问所有图书馆，无需单独分配馆内权限。</p>
        )}
      </form>
    </Modal>
  );
}
