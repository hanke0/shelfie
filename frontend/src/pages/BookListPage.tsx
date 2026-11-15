import { useMemo, useState } from "react";
import { useSearchParams, Link } from "react-router-dom";
import { useInfiniteQuery, useQueryClient } from "@tanstack/react-query";
import {
  listBooks,
  getListBooksQueryKey,
  useBatchMoveCategory,
} from "@/api/generated/books/books";
import { useLibrary } from "@/context/LibraryContext";
import { BackLink } from "@/components/BackLink";
import { BookCard } from "@/components/BookCard";
import { CategorySelect } from "@/components/CategorySelect";
import { useApiAction } from "@/hooks/useApiAction";
import styles from "./BookListPage.module.css";

const PAGE_SIZE = 50;

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

  const listParams = useMemo(
    () => ({
      sort: sort ?? undefined,
      library_id: activeLibraryId,
      category: category || undefined,
      limit: PAGE_SIZE,
    }),
    [sort, activeLibraryId, category],
  );

  const [selectionScope, setSelectionScope] = useState(listParams);
  if (selectionScope !== listParams) {
    setSelectionScope(listParams);
    setSelectedIds(new Set());
  }

  const { data, isLoading, hasNextPage, fetchNextPage, isFetchingNextPage } = useInfiniteQuery({
    queryKey: [...getListBooksQueryKey(listParams), "infinite"],
    queryFn: ({ pageParam, signal }) =>
      listBooks({ ...listParams, offset: pageParam }, undefined, signal),
    initialPageParam: 0,
    getNextPageParam: (lastPage, allPages) =>
      lastPage.length === PAGE_SIZE ? allPages.length * PAGE_SIZE : undefined,
    enabled: !!activeLibraryId,
  });

  // 分页期间若有图书增删，offset 会错位产生重复项
  const books = useMemo(
    () => [...new Map((data?.pages.flat() ?? []).map((b) => [b.id, b])).values()],
    [data],
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
    if (!books.length) return;
    setSelectedIds(new Set(books.map((b) => b.id)));
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
      <BackLink />
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
            <div className={styles.toolbarActions}>
              <Link to="/duplicates" className="btn btn-ghost">
                查重
              </Link>
              <button
                type="button"
                className="btn btn-ghost"
                onClick={() => setSelectMode(true)}
              >
                批量移动
              </button>
            </div>
          ) : (
            <div className={styles.batchBar}>
              <span className={styles.batchHint}>已选 {selectedIds.size} 本</span>
              <button type="button" className="btn btn-ghost" onClick={selectAllVisible}>
                全选当前列表
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
      {!isLoading && activeLibraryId && books.length === 0 && (
        <p className={styles.empty}>该分类下暂无图书</p>
      )}
      {!isLoading && books.length > 0 && (
        <p className={styles.count}>
          已显示 {books.length} 本
          {category ? `（分类：${category}）` : ""}
        </p>
      )}
      <div className="book-scroll">
        {books.map((book) => (
          <BookCard
            key={book.id}
            book={book}
            selectable={selectMode}
            selected={selectedIds.has(book.id)}
            onSelectToggle={() => toggleSelect(book.id)}
          />
        ))}
      </div>
      {hasNextPage && !isLoading && (
        <div className={styles.loadMore}>
          <button
            type="button"
            className="btn btn-ghost"
            disabled={isFetchingNextPage}
            onClick={() => void fetchNextPage()}
          >
            {isFetchingNextPage ? "加载中…" : "加载更多"}
          </button>
        </div>
      )}
    </div>
  );
}
