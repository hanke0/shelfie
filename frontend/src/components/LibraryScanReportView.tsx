import { useState, type ReactNode } from "react";
import type { LibraryScanReport, MaintenanceTool } from "@/api/generated/models";
import styles from "@/pages/AdminMaintenancePage.module.css";

interface Action {
  label: string;
  tool: MaintenanceTool;
  danger?: boolean;
}

interface SectionProps<T> {
  title: string;
  hint: string;
  rows: T[];
  keyOf: (row: T) => string;
  render: (row: T) => ReactNode;
  actions: Action[];
  disabled: boolean;
  onRun: (tool: MaintenanceTool, items: string[]) => void;
}

function SelectableSection<T>({
  title,
  hint,
  rows,
  keyOf,
  render,
  actions,
  disabled,
  onRun,
}: SectionProps<T>) {
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const allSelected = rows.length > 0 && selected.size === rows.length;

  const toggle = (key: string) => {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      return next;
    });
  };

  return (
    <section className={styles.reportSection}>
      <div className={styles.reportHead}>
        <h3>
          {title}（{rows.length}）
        </h3>
        {rows.length > 0 && (
          <div className={styles.rowActions}>
            {actions.map((a) => (
              <button
                key={a.tool}
                type="button"
                className={a.danger ? `btn ${styles.dangerBtn}` : "btn"}
                disabled={disabled || selected.size === 0}
                onClick={() => onRun(a.tool, [...selected])}
              >
                {a.label}（{selected.size}）
              </button>
            ))}
          </div>
        )}
      </div>
      <p className={styles.muted}>{hint}</p>
      {rows.length > 0 && (
        <ul className={styles.itemList}>
          <li>
            <label>
              <input
                type="checkbox"
                checked={allSelected}
                onChange={() => setSelected(allSelected ? new Set() : new Set(rows.map(keyOf)))}
              />
              <span className={styles.muted}>全选</span>
            </label>
          </li>
          {rows.map((row) => {
            const key = keyOf(row);
            return (
              <li key={key}>
                <label>
                  <input type="checkbox" checked={selected.has(key)} onChange={() => toggle(key)} />
                  <span>{render(row)}</span>
                </label>
              </li>
            );
          })}
        </ul>
      )}
    </section>
  );
}

const formatSize = (bytes: number) =>
  bytes >= 1 << 20 ? `${(bytes / (1 << 20)).toFixed(1)} MB` : `${Math.ceil(bytes / 1024)} KB`;

interface Props {
  report: LibraryScanReport;
  /** 同一图书馆已有任务在运行时禁用后续操作 */
  disabled: boolean;
  onRun: (tool: MaintenanceTool, items: string[]) => void;
}

export function LibraryScanReportView({ report, disabled, onRun }: Props) {
  return (
    <div>
      <p className={styles.muted}>
        磁盘图书文件 {report.scanned_files} 个，数据库记录 {report.db_books} 条，其中一致{" "}
        {report.in_sync} 本。
      </p>
      {report.errors.length > 0 && (
        <ul className={styles.errorList}>
          {report.errors.map((e) => (
            <li key={e}>{e}</li>
          ))}
        </ul>
      )}

      <SelectableSection
        title="数据库有、文件缺失"
        hint="这些图书的文件已不在磁盘上。删除记录会同时删除其阅读进度等关联数据。"
        rows={report.missing_files}
        keyOf={(b) => b.book_id}
        render={(b) => (
          <>
            <strong>{b.title || "(无标题)"}</strong>
            {b.author && ` — ${b.author}`} <span className={styles.muted}>[{b.category}]</span>
            <code className={styles.path}>{b.book_file_path}</code>
          </>
        )}
        actions={[{ label: "删除选中记录", tool: "library_purge_missing", danger: true }]}
        disabled={disabled}
        onRun={onRun}
      />

      <SelectableSection
        title="文件存在、未入库"
        hint="导入时读取同名 metadata，没有则按文件名生成书名。"
        rows={report.untracked_files}
        keyOf={(f) => f.path}
        render={(f) => (
          <>
            {f.title && <strong>{f.title} </strong>}
            <span className={styles.muted}>
              [{f.category}] {formatSize(f.size)}
            </span>
            <code className={styles.path}>{f.path}</code>
          </>
        )}
        actions={[
          { label: "导入选中", tool: "library_import_untracked" },
          { label: "删除选中文件", tool: "library_delete_untracked", danger: true },
        ]}
        disabled={disabled}
        onRun={onRun}
      />

      <SelectableSection
        title="多余附属文件"
        hint="没有对应图书文件的 metadata / 封面 / 缩略图。"
        rows={report.dangling_files}
        keyOf={(p) => p}
        render={(p) => <code className={styles.path}>{p}</code>}
        actions={[{ label: "删除选中文件", tool: "library_clean_dangling", danger: true }]}
        disabled={disabled}
        onRun={onRun}
      />
    </div>
  );
}
