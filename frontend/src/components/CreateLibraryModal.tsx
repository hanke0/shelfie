import { useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { useCreateLibrary } from "@/api/generated/libraries/libraries";
import { Modal } from "@/components/ui/Modal";
import formStyles from "@/components/ui/Form.module.css";

interface CreateLibraryModalProps {
  open: boolean;
  onClose: () => void;
}

export function CreateLibraryModal({ open, onClose }: CreateLibraryModalProps) {
  const qc = useQueryClient();
  const createLibrary = useCreateLibrary();
  const [name, setName] = useState("");
  const [slug, setSlug] = useState("");
  const [error, setError] = useState<string | null>(null);

  const reset = () => {
    setName("");
    setSlug("");
    setError(null);
  };

  const handleClose = () => {
    reset();
    onClose();
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    try {
      await createLibrary.mutateAsync({ data: { name, slug } });
      await qc.invalidateQueries({ queryKey: ["/libraries"] });
      handleClose();
    } catch (err) {
      setError(err instanceof Error ? err.message : "创建失败");
    }
  };

  return (
    <Modal
      open={open}
      onClose={handleClose}
      title="创建图书馆"
      footer={
        <>
          <button type="button" className="btn btn-ghost" onClick={handleClose}>
            取消
          </button>
          <button
            type="submit"
            form="create-library-form"
            className="btn"
            disabled={createLibrary.isPending}
          >
            {createLibrary.isPending ? "创建中…" : "创建"}
          </button>
        </>
      }
    >
      <form
        id="create-library-form"
        className={formStyles.form}
        onSubmit={(e) => void handleSubmit(e)}
      >
        <p className={formStyles.hint}>
          仅系统管理员可创建新图书馆。磁盘目录将与图书馆名称一致（支持中文等字符，不可含 / \ : * ? 等路径符号）。
        </p>
        {error && <p className={formStyles.error}>{error}</p>}
        <label className={formStyles.field}>
          名称
          <input value={name} onChange={(e) => setName(e.target.value)} required />
        </label>
        <label className={formStyles.field}>
          Slug（URL 标识，小写字母/数字/连字符）
          <input
            value={slug}
            onChange={(e) => setSlug(e.target.value)}
            pattern="[a-z0-9][a-z0-9_-]*"
            required
          />
        </label>
      </form>
    </Modal>
  );
}
