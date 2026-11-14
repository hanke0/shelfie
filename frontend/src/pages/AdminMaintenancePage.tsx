import { useMemo, useState } from "react";
import { Link } from "react-router-dom";
import { useQueryClient } from "@tanstack/react-query";
import { useListLibraries } from "@/api/generated/libraries/libraries";
import {
  useCancelMaintenanceTask,
  useGetMaintenanceTask,
  useListMaintenanceTasks,
  useListMaintenanceTools,
  useStartMaintenanceTask,
} from "@/api/generated/maintenance/maintenance";
import type {
  LibraryScanReport,
  MaintenanceActionReport,
  MaintenanceTool,
  TaskDto,
} from "@/api/generated/models";
import { LibraryScanReportView } from "@/components/LibraryScanReportView";
import { Dropdown } from "@/components/ui/Dropdown";
import { useConfirmTwice } from "@/context/ConfirmContext";
import { useLibrary } from "@/context/LibraryContext";
import { useApiAction } from "@/hooks/useApiAction";
import {
  TASK_STATUS_LABEL,
  formatDateTime,
  formatDuration,
  formatProgress,
} from "@/lib/maintenance";
import adminStyles from "./AdminPage.module.css";
import styles from "./AdminMaintenancePage.module.css";

const TASKS_KEY = ["/maintenance/tasks"];

function ActionReportView({ report }: { report: MaintenanceActionReport }) {
  const groups: [string, string[]][] = [
    ["已处理", report.processed],
    ["已跳过（当前已不符合条件）", report.skipped],
    ["错误", report.errors],
  ];
  return (
    <div>
      {groups.map(([label, items]) => (
        <details key={label} className={styles.reportSection} open={label === "错误"}>
          <summary>
            {label}（{items.length}）
          </summary>
          {items.length > 0 && (
            <ul className={styles.itemList}>
              {items.map((i) => (
                <li key={i}>
                  <code className={styles.path}>{i}</code>
                </li>
              ))}
            </ul>
          )}
        </details>
      ))}
    </div>
  );
}

function ProgressBar({ task }: { task: TaskDto }) {
  const { current, total } = task.progress;
  const pct = total ? Math.min(100, (current / total) * 100) : null;
  return (
    <div className={styles.progressBar} aria-label="进度">
      <div
        className={pct == null ? styles.progressIndeterminate : styles.progressFill}
        style={pct == null ? undefined : { width: `${pct}%` }}
      />
    </div>
  );
}

export function AdminMaintenancePage() {
  const qc = useQueryClient();
  const { libraryId: contextLibraryId, setLibraryId } = useLibrary();
  const { data: libraries } = useListLibraries();
  const { data: tools } = useListMaintenanceTools();
  const [libraryId, setSelectedLibrary] = useState(contextLibraryId ?? "");
  const [selectedTaskId, setSelectedTaskId] = useState<string | null>(null);

  const { data: tasks } = useListMaintenanceTasks(undefined, {
    query: {
      refetchInterval: (q) => (q.state.data?.some((t) => t.status === "running") ? 1500 : 5000),
    },
  });
  const { data: selectedTask } = useGetMaintenanceTask(selectedTaskId ?? "", {
    query: {
      enabled: !!selectedTaskId,
      refetchInterval: (q) => (q.state.data?.status === "running" ? 1000 : false),
    },
  });

  const startTask = useStartMaintenanceTask();
  const cancelTask = useCancelMaintenanceTask();
  const confirmTwice = useConfirmTwice();
  const run = useApiAction();

  const toolById = useMemo(() => new Map(tools?.map((t) => [t.id, t]) ?? []), [tools]);
  const busyLibraries = useMemo(
    () =>
      new Set(tasks?.filter((t) => t.status === "running").map((t) => t.library_id ?? "") ?? []),
    [tasks],
  );
  const toolName = (id: string) => toolById.get(id as MaintenanceTool)?.name ?? id;

  const launch = async (tool: MaintenanceTool, targetLibrary: string, items?: string[]) => {
    const spec = toolById.get(tool);
    if (spec?.destructive) {
      const scope = items ? `选中的 ${items.length} 项` : "扫描到的全部项";
      if (
        !(await confirmTwice(
          `确定运行「${spec.name}」？将处理${scope}。`,
          "再次确认：该操作会删除或覆盖数据，且无法撤销。",
        ))
      ) {
        return;
      }
    }
    await run(
      async () => {
        const task = await startTask.mutateAsync({
          data: {
            tool,
            params: items ? { library_id: targetLibrary, items } : { library_id: targetLibrary },
          },
        });
        setSelectedTaskId(task.id);
        await qc.invalidateQueries({ queryKey: TASKS_KEY });
      },
      { successMessage: "任务已启动", errorMessage: "启动失败" },
    );
  };

  const handleCancel = async (id: string) => {
    await run(
      async () => {
        await cancelTask.mutateAsync({ id });
        await qc.invalidateQueries({ queryKey: TASKS_KEY });
        await qc.invalidateQueries({ queryKey: [`/maintenance/tasks/${id}`] });
      },
      { successMessage: "已请求取消", errorMessage: "取消失败" },
    );
  };

  const renderResult = (task: TaskDto) => {
    if (!task.result) return null;
    if (task.tool === "library_scan") {
      return (
        <LibraryScanReportView
          key={task.id}
          report={task.result as unknown as LibraryScanReport}
          disabled={busyLibraries.has(task.library_id ?? "")}
          onRun={(tool, items) => void launch(tool, task.library_id ?? "", items)}
        />
      );
    }
    return <ActionReportView report={task.result as unknown as MaintenanceActionReport} />;
  };

  return (
    <div className={`app-shell ${adminStyles.page}`}>
      <Link to="/" className={adminStyles.back}>
        ← 返回首页
      </Link>
      <h1>维护工具</h1>
      <p className={adminStyles.muted}>
        维护工具在后台运行，可离开本页；同一图书馆同一时间只能运行一个维护任务。
      </p>

      <div className={adminStyles.card}>
        <div className={adminStyles.cardHeader}>
          <h2>工具列表</h2>
          <div className={styles.libraryPicker}>
            <Dropdown
              value={libraryId}
              onChange={(id) => {
                setSelectedLibrary(id);
                if (id) setLibraryId(id);
              }}
              placeholder="选择图书馆"
              options={libraries?.map((lib) => ({ value: lib.id, label: lib.name })) ?? []}
              aria-label="目标图书馆"
            />
          </div>
        </div>
        <ul className={styles.toolList}>
          {tools?.map((tool) => (
            <li key={tool.id} className={styles.toolItem}>
              <div>
                <div className={styles.toolTitle}>
                  <strong>{tool.name}</strong>
                  {tool.destructive && <span className={styles.tagDanger}>破坏性</span>}
                  <span className={styles.tag}>
                    需要{tool.required_permission === "delete" ? "删除" : "编辑"}权限
                  </span>
                </div>
                <p className={styles.muted}>{tool.description}</p>
              </div>
              <button
                type="button"
                className={tool.destructive ? `btn ${styles.dangerBtn}` : "btn"}
                disabled={!libraryId || busyLibraries.has(libraryId)}
                onClick={() => void launch(tool.id, libraryId)}
              >
                运行
              </button>
            </li>
          ))}
        </ul>
        {libraryId && busyLibraries.has(libraryId) && (
          <p className={styles.muted}>该图书馆有任务正在运行，结束后才能启动新任务。</p>
        )}
      </div>

      <div className={adminStyles.card}>
        <h2>后台任务</h2>
        {tasks?.length ? (
          <div className={styles.tableWrap}>
            <table className={styles.table}>
              <thead>
                <tr>
                  <th>工具</th>
                  <th>图书馆</th>
                  <th>状态</th>
                  <th>进度</th>
                  <th>启动时间</th>
                  <th>运行时长</th>
                  <th>发起人</th>
                  <th></th>
                </tr>
              </thead>
              <tbody>
                {tasks.map((t) => (
                  <tr key={t.id} className={t.id === selectedTaskId ? styles.selectedRow : ""}>
                    <td>{toolName(t.tool)}</td>
                    <td>{t.library_name ?? "—"}</td>
                    <td>
                      <span className={styles[`status_${t.status}`]}>
                        {t.cancel_requested && t.status === "running"
                          ? "取消中"
                          : TASK_STATUS_LABEL[t.status]}
                      </span>
                    </td>
                    <td>
                      <div className={styles.progressCell} title={formatProgress(t.progress)}>
                        {formatProgress(t.progress)}
                      </div>
                    </td>
                    <td>{formatDateTime(t.started_at)}</td>
                    <td>{formatDuration(t.elapsed_ms)}</td>
                    <td>{t.created_by ?? "—"}</td>
                    <td className={styles.rowActions}>
                      <button
                        type="button"
                        className="btn btn-ghost"
                        onClick={() => setSelectedTaskId(t.id)}
                      >
                        详情
                      </button>
                      {t.status === "running" && !t.cancel_requested && (
                        <button
                          type="button"
                          className="btn btn-ghost"
                          onClick={() => void handleCancel(t.id)}
                        >
                          取消
                        </button>
                      )}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        ) : (
          <p className={adminStyles.muted}>暂无任务</p>
        )}
      </div>

      {selectedTask && (
        <div className={adminStyles.card}>
          <div className={adminStyles.cardHeader}>
            <h2>
              {toolName(selectedTask.tool)}
              {selectedTask.library_name && ` · ${selectedTask.library_name}`}
            </h2>
            <div className={styles.rowActions}>
              {selectedTask.status === "running" && !selectedTask.cancel_requested && (
                <button
                  type="button"
                  className="btn btn-ghost"
                  onClick={() => void handleCancel(selectedTask.id)}
                >
                  取消任务
                </button>
              )}
              {selectedTask.status !== "running" && selectedTask.library_id && (
                <button
                  type="button"
                  className="btn"
                  disabled={busyLibraries.has(selectedTask.library_id)}
                  onClick={() => void launch("library_scan", selectedTask.library_id ?? "")}
                >
                  重新扫描
                </button>
              )}
              <button
                type="button"
                className="btn btn-ghost"
                onClick={() => setSelectedTaskId(null)}
              >
                关闭
              </button>
            </div>
          </div>
          <p className={adminStyles.muted}>
            {TASK_STATUS_LABEL[selectedTask.status]} · 启动于{" "}
            {formatDateTime(selectedTask.started_at)} ·{" "}
            {selectedTask.status === "running" ? "已运行" : "耗时"}{" "}
            {formatDuration(selectedTask.elapsed_ms)}
            {selectedTask.created_by && ` · ${selectedTask.created_by}`}
          </p>
          {selectedTask.status === "running" && (
            <>
              <ProgressBar task={selectedTask} />
              <p className={adminStyles.muted}>{formatProgress(selectedTask.progress)}</p>
            </>
          )}
          {selectedTask.status === "cancelled" && selectedTask.result != null && (
            <p className={adminStyles.muted}>任务被取消，以下为取消前的部分结果。</p>
          )}
          {selectedTask.error && <p className={adminStyles.error}>{selectedTask.error}</p>}
          {renderResult(selectedTask)}
        </div>
      )}
    </div>
  );
}
