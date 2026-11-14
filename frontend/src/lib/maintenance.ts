import type { TaskProgress, TaskStatus } from "@/api/generated/models";

export const TASK_STATUS_LABEL: Record<TaskStatus, string> = {
  running: "运行中",
  completed: "已完成",
  failed: "失败",
  cancelled: "已取消",
  interrupted: "已中断",
};

/** 毫秒 → `1小时2分3秒` / `2分3秒` / `3.2秒` */
export function formatDuration(ms: number): string {
  if (ms < 10_000) return `${(Math.max(ms, 0) / 1000).toFixed(1)}秒`;
  const total = Math.floor(ms / 1000);
  const h = Math.floor(total / 3600);
  const m = Math.floor((total % 3600) / 60);
  const s = total % 60;
  if (h > 0) return `${h}小时${m}分${s}秒`;
  if (m > 0) return `${m}分${s}秒`;
  return `${s}秒`;
}

export function formatProgress(p: TaskProgress): string {
  const count = p.total != null ? `${p.current}/${p.total}` : p.current ? `${p.current}` : "";
  return [count, p.message].filter(Boolean).join(" · ");
}

export function formatDateTime(iso: string): string {
  const d = new Date(iso);
  return Number.isNaN(d.getTime()) ? iso : d.toLocaleString();
}
