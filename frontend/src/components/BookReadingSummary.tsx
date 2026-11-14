import type { BookReadingSummary as Summary } from "@/api/generated/models";
import styles from "./BookReadingSummary.module.css";

export const READING_STATUS_LABELS: Record<string, string> = {
  reading: "阅读中",
  complete: "已读完",
  abandoned: "已放弃",
};

export function formatDuration(seconds: number): string {
  const minutes = Math.round(seconds / 60);
  if (minutes < 60) return `${minutes} 分钟`;
  const hours = Math.floor(minutes / 60);
  const rest = minutes % 60;
  return rest ? `${hours} 小时 ${rest} 分钟` : `${hours} 小时`;
}

function formatDate(unixSeconds: number): string {
  return new Date(unixSeconds * 1000).toLocaleDateString("zh-CN");
}

export function BookReadingSummary({ summary }: { summary?: Summary }) {
  if (!summary) return null;
  const hasStats = summary.last_read_at != null;

  return (
    <dl className={styles.list}>
      <div>
        <dt>状态</dt>
        <dd>
          {summary.status ? (READING_STATUS_LABELS[summary.status] ?? summary.status) : "—"}
          {summary.status_modified ? (
            <span className={styles.muted}>（{summary.status_modified}）</span>
          ) : null}
        </dd>
      </div>
      {hasStats ? (
        <>
          <div>
            <dt>累计时长</dt>
            <dd>{formatDuration(summary.total_seconds)}</dd>
          </div>
          <div>
            <dt>已读页数</dt>
            <dd>
              {summary.pages_read}
              {summary.total_pages ? ` / ${summary.total_pages}` : ""}
            </dd>
          </div>
          <div>
            <dt>阅读天数</dt>
            <dd>{summary.reading_days} 天</dd>
          </div>
          <div>
            <dt>最近阅读</dt>
            <dd>{formatDate(summary.last_read_at!)}</dd>
          </div>
        </>
      ) : (
        <div>
          <dt>统计</dt>
          <dd className={styles.muted}>暂无阅读统计</dd>
        </div>
      )}
    </dl>
  );
}
