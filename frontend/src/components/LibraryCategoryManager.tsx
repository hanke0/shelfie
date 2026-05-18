import { useState } from "react";
import type { FormEvent } from "react";
import { useQueryClient } from "@tanstack/react-query";
import {
  getListCategoriesQueryKey,
  useCreateCategory,
  useDeleteCategory,
  useListCategories,
} from "@/api/generated/categories/categories";
import { useConfirmTwice } from "@/context/ConfirmContext";
import styles from "@/pages/AdminPage.module.css";
import { useApiAction } from "@/hooks/useApiAction";

const DEFAULT_CATEGORY = "未分类";

interface LibraryCategoryManagerProps {
  libraryId: string;
  canEdit: boolean;
}

export function LibraryCategoryManager({ libraryId, canEdit }: LibraryCategoryManagerProps) {
  const qc = useQueryClient();
  const confirmTwice = useConfirmTwice();
  const run = useApiAction();
  const { data: categories, isLoading } = useListCategories(libraryId);
  const createCategory = useCreateCategory();
  const deleteCategory = useDeleteCategory();
  const [newName, setNewName] = useState("");
  const [error, setError] = useState<string | null>(null);

  const invalidate = async () => {
    await qc.invalidateQueries({ queryKey: getListCategoriesQueryKey(libraryId) });
  };

  const handleCreate = async (e: FormEvent) => {
    e.preventDefault();
    const name = newName.trim();
    if (!name) return;
    setError(null);
    await run(
      async () => {
        await createCategory.mutateAsync({ id: libraryId, data: { name } });
        setNewName("");
        await invalidate();
      },
      { successMessage: "分类已创建", errorMessage: "创建失败" },
    );
  };

  const handleDelete = async (name: string) => {
    if (
      !(await confirmTwice(
        `确定删除分类「${name}」？`,
        "再次确认：仅当该分类下无图书且目录为空时可删除。",
      ))
    ) {
      return;
    }
    setError(null);
    await run(
      async () => {
        await deleteCategory.mutateAsync({
          id: libraryId,
          name: encodeURIComponent(name),
        });
        await invalidate();
      },
      { successMessage: "分类已删除", errorMessage: "删除失败" },
    );
  };

  return (
    <div className={styles.card}>
      <h2>图书分类</h2>
      <p className={styles.muted}>
        分类对应磁盘上的子文件夹。名称仅允许字母、数字及常见标点（与文件命名规则一致）；空格会转为下划线。
      </p>
      {error && <p className={styles.deleteError}>{error}</p>}
      {isLoading ? (
        <p className={styles.muted}>加载中…</p>
      ) : (
        <ul className={styles.categoryList}>
          {categories?.map((c) => (
            <li key={c.name} className={styles.categoryRow}>
              <span>{c.name}</span>
              {canEdit && c.name !== DEFAULT_CATEGORY ? (
                <button
                  type="button"
                  className="btn btn-ghost"
                  disabled={deleteCategory.isPending}
                  onClick={() => void handleDelete(c.name)}
                >
                  删除
                </button>
              ) : null}
            </li>
          ))}
        </ul>
      )}
      {canEdit && (
        <form className={styles.categoryForm} onSubmit={(e) => void handleCreate(e)}>
          <input
            value={newName}
            onChange={(e) => setNewName(e.target.value)}
            placeholder="新分类名称"
            maxLength={64}
          />
          <button type="submit" className="btn" disabled={createCategory.isPending || !newName.trim()}>
            {createCategory.isPending ? "添加中…" : "添加分类"}
          </button>
        </form>
      )}
    </div>
  );
}
