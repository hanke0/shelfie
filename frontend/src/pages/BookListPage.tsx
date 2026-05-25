import { useMemo, useState } from "react";
import { useSearchParams, Link } from "react-router-dom";
import { useQueryClient } from "@tanstack/react-query";
import {
  useListBooks,
  getListBooksQueryKey,
  useBatchMoveCategory,
} from "@/api/generated/books/books";
import { useLibrary } from "@/context/LibraryContext";
import { BookCard } from "@/components/BookCard";
import { CategorySelect } from "@/components/CategorySelect";
import { useApiAction } from "@/hooks/useApiAction";
import styles from "./BookListPage.module.css";

export function BookListPage() {
  const [params, setParams] = useSearchParams();
  const sort = params.get("sort") ?? undefined;
  const category = params.get("category") ?? "";
  const { libraryId } = useLibrary();
  const libFromUrl = params.get("library_id");
  const activeLibraryId = libFromUrl ?? libraryId ?? undefined;

  const [selectMode, setSelectMode] = useState(false);
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());
  const [targetCategory, setTargetCategory] = useState("");

  const qc = useQueryClient();
  const run = useApiAction();
  const batchMove = useBatchMoveCategory();

  const { data, isLoading } = useListBooks(
    {
      sort: sort ?? undefined,
      library_id: activeLibraryId,
      category: category || undefined,
      limit: 50,
    },
    { query: { enabled: !!activeLibraryId } },
  );

  const baseTitle =
    sort === "recent" ? "最近阅读" : sort === "new" ? "新书速递" : "全部图书";

  const title = useMemo(() => {
    if (!category) return baseTitle;
    return `${baseTitle} · ${category}`;
  }, [baseTitle, category]);

  const setCategory = (name: string) => {
    const next = new URLSearchParams(params);
    if (name) {
      next.set("category", name);
    } else {
      next.delete("category");
    }
    setParams(next, { replace: true });
  };

  const toggleSelect = (id: string) => {
    setSelectedIds((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const exitSelectMode = () => {
    setSelectMode(false);
    setSelectedIds(new Set());
    setTargetCategory("");
  };

  const selectAllVisible = () => {
    if (!data?.length) return;
    setSelectedIds(new Set(data.map((b) => b.id)));
  };

  const handleBatchMove = async () => {
    if (!activeLibraryId || selectedIds.size === 0 || !targetCategory.trim()) {
      await run(async () => {
        throw new Error("请选择图书和目标分类");
      });
      return;
    }

    await run(
      async () => {
        const res = await batchMove.mutateAsync({
          data: {
            library_id: activeLibraryId,
            book_ids: [...selectedIds],
            category: targetCategory.trim(),
          },
        });
        await qc.invalidateQueries({ queryKey: getListBooksQueryKey() });
        await qc.invalidateQueries({ queryKey: ["/home"] });

        if (res.failures.length > 0) {
          throw new Error(
            `已移动 ${res.moved} 本，${res.failures.length} 本失败：${res.failures[0]?.message ?? ""}`,
          );
        }
        exitSelectMode();
        if (category && category !== targetCategory.trim()) {
          const next = new URLSearchParams(params);
          next.delete("category");
          setParams(next, { replace: true });
        }
      },
      {
        successMessage: `已将 ${selectedIds.size} 本图书移至「${targetCategory.trim()}」`,
        errorMessage: "批量移动失败",
      },
    );
  };

  return (
    <div className="app-shell">
      <Link to="/" className={styles.back}>
        ← 返回首页
      </Link>
      <h1 className={styles.title}>{title}</h1>

      {!activeLibraryId && <p>请先在顶栏选择图书馆</p>}

      {activeLibraryId && (
        <div className={styles.toolbar}>
          <label className={styles.filter}>
            <span className={styles.filterLabel}>分类</span>
            <CategorySelect
              libraryId={activeLibraryId}
              value={category}
              onChange={setCategory}
              includeAllOption
            />
          </label>

          {!selectMode ? (
            <button
              type="button"
              className="btn btn-ghost"
              onClick={() => setSelectMode(true)}
            >
              批量移动
            </button>
          ) : (
            <div className={styles.batchBar}>
              <span className={styles.batchHint}>已选 {selectedIds.size} 本</span>
              <button type="button" className="btn btn-ghost" onClick={selectAllVisible}>
                全选本页
              </button>
              <button
                type="button"
                className="btn btn-ghost"
                onClick={() => setSelectedIds(new Set())}
              >
                清空
              </button>
              <label className={styles.batchTarget}>
                <span className={styles.filterLabel}>移至</span>
                <CategorySelect
                  libraryId={activeLibraryId}
                  value={targetCategory}
                  onChange={setTargetCategory}
                  required
                />
              </label>
              <button
                type="button"
                className="btn"
                disabled={selectedIds.size === 0 || !targetCategory || batchMove.isPending}
                onClick={() => void handleBatchMove()}
              >
                {batchMove.isPending ? "移动中…" : "确认移动"}
              </button>
              <button type="button" className="btn btn-ghost" onClick={exitSelectMode}>
                取消
              </button>
            </div>
          )}
        </div>
      )}

      {isLoading && activeLibraryId && <p>加载中…</p>}
      {!isLoading && activeLibraryId && data?.length === 0 && (
        <p className={styles.empty}>该分类下暂无图书</p>
      )}
      <div className="book-scroll">
        {data?.map((book) => (
          <BookCard
            key={book.id}
            book={book}
            selectable={selectMode}
            selected={selectedIds.has(book.id)}
            onSelectToggle={() => toggleSelect(book.id)}
          />
        ))}
      </div>
    </div>
  );
}
