import { useEffect, useMemo, useRef, useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { useListLibraries, useListMembers } from "@/api/generated/libraries/libraries";
import { useListCategories } from "@/api/generated/categories/categories";
import { getGetHomeQueryKey } from "@/api/generated/home/home";
import { Modal } from "@/components/ui/Modal";
import { FieldLabel } from "@/components/ui/FieldLabel";
import { CategorySelect } from "@/components/CategorySelect";
import { Dropdown } from "@/components/ui/Dropdown";
import { useLibrary } from "@/context/LibraryContext";
import { getUser } from "@/lib/auth";
import { uploadBookMultipart } from "@/lib/upload";
import {
  metadataFormFromExtract,
  prepareMetadataForUpload,
} from "@/lib/book-upload-state";
import {
  isCalibreImportSupported,
  loadCalibreBooks,
  prepareCalibreBook,
  resolveCategory,
  type CategoryMappingMode,
  type CalibreBookRow,
} from "@/lib/calibre-import";
import formStyles from "@/components/ui/Form.module.css";

interface CalibreImportModalProps {
  open: boolean;
  onClose: () => void;
}

type Phase = "config" | "importing" | "done";

type ImportSummary = {
  success: number;
  failed: number;
  tagFallback: number;
  errors: string[];
};

const DEFAULT_CATEGORY = "未分类";

export function CalibreImportModal({ open, onClose }: CalibreImportModalProps) {
  const qc = useQueryClient();
  const user = getUser();
  const { libraryId: contextLibraryId } = useLibrary();
  const { data: libraries = [] } = useListLibraries({ query: { enabled: open } });

  const [targetLibraryId, setTargetLibraryId] = useState("");
  const [categoryMode, setCategoryMode] = useState<CategoryMappingMode>("fixed");
  const [fixedCategory, setFixedCategory] = useState(DEFAULT_CATEGORY);
  const [fallbackCategory, setFallbackCategory] = useState(DEFAULT_CATEGORY);
  const [phase, setPhase] = useState<Phase>("config");
  const [progress, setProgress] = useState({ current: 0, total: 0, title: "" });
  const [summary, setSummary] = useState<ImportSummary | null>(null);
  const [error, setError] = useState<string | null>(null);
  const abortRef = useRef(false);

  const { data: members } = useListMembers(targetLibraryId, {
    query: { enabled: open && !!targetLibraryId },
  });
  const { data: categories } = useListCategories(targetLibraryId, {
    query: { enabled: open && !!targetLibraryId },
  });

  const categorySet = useMemo(
    () => new Set(categories?.map((c) => c.name) ?? []),
    [categories],
  );

  const isSystemAdmin = user?.role === "system_admin";
  const canEdit = useMemo(() => {
    if (isSystemAdmin) return true;
    if (!user || !members) return false;
    return members.some((m) => m.user_id === user.id && (m.role === "admin" || m.can_edit));
  }, [isSystemAdmin, user, members]);

  const supported = isCalibreImportSupported();

  const libraryOptions = useMemo(
    () => libraries.map((l) => ({ value: l.id, label: l.name })),
    [libraries],
  );

  useEffect(() => {
    if (!open) return;
    setPhase("config");
    setProgress({ current: 0, total: 0, title: "" });
    setSummary(null);
    setError(null);
    abortRef.current = false;
    setTargetLibraryId(contextLibraryId ?? libraries[0]?.id ?? "");
    setCategoryMode("fixed");
    setFixedCategory(DEFAULT_CATEGORY);
    setFallbackCategory(DEFAULT_CATEGORY);
  }, [open, contextLibraryId, libraries]);

  const handleClose = () => {
    if (phase === "importing") return;
    onClose();
  };

  const handleCancel = () => {
    abortRef.current = true;
  };

  const runImport = async () => {
    if (!targetLibraryId || !canEdit) return;
    setError(null);

    let root: FileSystemDirectoryHandle;
    try {
      root = await window.showDirectoryPicker({ mode: "read" });
    } catch (e) {
      if (e instanceof DOMException && e.name === "AbortError") return;
      setError("无法选择目录，请重试");
      return;
    }

    let rows: CalibreBookRow[];
    try {
      rows = await loadCalibreBooks(root);
    } catch {
      setError("未找到有效的 Calibre 图书馆（根目录需包含 metadata.db）");
      return;
    }

    if (rows.length === 0) {
      setError("该 Calibre 图书馆中没有图书");
      return;
    }

    setPhase("importing");
    abortRef.current = false;
    const result: ImportSummary = { success: 0, failed: 0, tagFallback: 0, errors: [] };

    for (let i = 0; i < rows.length; i++) {
      if (abortRef.current) break;
      const row = rows[i];
      setProgress({ current: i + 1, total: rows.length, title: row.title });

      try {
        const prepared = await prepareCalibreBook(root, row);
        if (!prepared) {
          result.failed += 1;
          result.errors.push(`${row.title}：未找到支持的电子书文件`);
          continue;
        }

        const { category, usedFallback } = resolveCategory(
          categoryMode,
          fixedCategory,
          fallbackCategory,
          prepared.firstTag,
          categorySet,
        );
        if (usedFallback) result.tagFallback += 1;

        const metaForm = metadataFormFromExtract(prepared.metadata, prepared.bookFile);
        metaForm.category = category;
        const metadata = prepareMetadataForUpload(metaForm);

        await uploadBookMultipart(
          targetLibraryId,
          category,
          prepared.bookFile,
          metadata,
          prepared.cover ?? null,
        );
        result.success += 1;
      } catch (err) {
        result.failed += 1;
        const msg = err instanceof Error ? err.message : String(err);
        result.errors.push(`${row.title}：${msg}`);
      }
    }

    await qc.invalidateQueries({ queryKey: ["/books"] });
    await qc.invalidateQueries({
      queryKey: getGetHomeQueryKey({ library_id: targetLibraryId, limit: 12 }),
    });
    await qc.invalidateQueries({
      queryKey: [`/libraries/${targetLibraryId}/categories`],
    });

    setSummary(result);
    setPhase("done");
  };

  const footer =
    phase === "config" ? (
      <>
        <button type="button" className="btn btn-ghost" onClick={handleClose}>
          取消
        </button>
        <button
          type="button"
          className="btn"
          disabled={!supported || !targetLibraryId || !canEdit}
          onClick={() => void runImport()}
        >
          选择 Calibre 图书馆并开始导入
        </button>
      </>
    ) : phase === "importing" ? (
      <button type="button" className="btn btn-ghost" onClick={handleCancel}>
        取消导入
      </button>
    ) : (
      <button type="button" className="btn" onClick={handleClose}>
        完成
      </button>
    );

  return (
    <Modal open={open} onClose={handleClose} title="从 Calibre 导入" wide footer={footer}>
      {!supported && (
        <p className={formStyles.error}>
          当前浏览器不支持目录选择，请使用 Chrome 或 Edge 访问本页。
        </p>
      )}

      {phase === "config" && (
        <div className={formStyles.form}>
          <label className={formStyles.field}>
            <FieldLabel required>目标图书馆</FieldLabel>
            <Dropdown
              value={targetLibraryId}
              onChange={setTargetLibraryId}
              options={libraryOptions}
              placeholder="选择图书馆"
            />
          </label>

          {!canEdit && targetLibraryId && (
            <p className={formStyles.error}>
              您对该图书馆没有编辑权限，请在图书馆管理中获取权限后再导入。
            </p>
          )}

          <fieldset className={formStyles.permFieldset}>
            <legend>分类策略</legend>
            <label className={formStyles.checkLabel}>
              <input
                type="radio"
                name="calibre-category-mode"
                checked={categoryMode === "fixed"}
                onChange={() => setCategoryMode("fixed")}
              />
              全部导入到固定分类
            </label>
            <label className={formStyles.checkLabel}>
              <input
                type="radio"
                name="calibre-category-mode"
                checked={categoryMode === "first_tag"}
                onChange={() => setCategoryMode("first_tag")}
              />
              使用 Calibre 每本书的第一个标签作为分类
            </label>
          </fieldset>

          {categoryMode === "fixed" ? (
            <label className={formStyles.field}>
              <FieldLabel required>分类</FieldLabel>
              <CategorySelect
                libraryId={targetLibraryId}
                value={fixedCategory}
                onChange={setFixedCategory}
                required
              />
            </label>
          ) : (
            <label className={formStyles.field}>
              <FieldLabel required>默认分类（标签不存在于图书馆时回退）</FieldLabel>
              <CategorySelect
                libraryId={targetLibraryId}
                value={fallbackCategory}
                onChange={setFallbackCategory}
                required
              />
              <p className="muted-hint">
                仅当 Calibre 第一个标签与图书馆已有分类名称完全一致时才会使用该标签。
              </p>
            </label>
          )}

          {error && <p className={formStyles.error}>{error}</p>}

          <p className="muted-hint">
            将打开目录选择器，请选择 Calibre 图书馆根目录（包含 metadata.db 的文件夹）。导入过程在本地读取文件并逐本上传，大型书库可能耗时较长。
          </p>
        </div>
      )}

      {phase === "importing" && (
        <div className={formStyles.form}>
          <p>
            正在导入 {progress.current} / {progress.total}
          </p>
          {progress.title && <p className="muted-hint">当前：{progress.title}</p>}
        </div>
      )}

      {phase === "done" && summary && (
        <div className={formStyles.form}>
          <p>
            成功 {summary.success} 本，失败 {summary.failed} 本
            {categoryMode === "first_tag" && summary.tagFallback > 0
              ? `，标签回退默认分类 ${summary.tagFallback} 本`
              : ""}
            {abortRef.current ? "（已取消，剩余书目未处理）" : ""}
          </p>
          {summary.errors.length > 0 && (
            <ul className={formStyles.error} style={{ paddingLeft: "1.25rem" }}>
              {summary.errors.slice(0, 20).map((e) => (
                <li key={e}>{e}</li>
              ))}
              {summary.errors.length > 20 && (
                <li>…另有 {summary.errors.length - 20} 条错误未显示</li>
              )}
            </ul>
          )}
        </div>
      )}
    </Modal>
  );
}
