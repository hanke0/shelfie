/** 从 API / fetch / React Query 抛出的错误中提取可读文案 */
export function formatApiError(err: unknown, fallback = "操作失败"): string {
  if (err instanceof Error && err.message.trim()) {
    return err.message;
  }
  if (typeof err === "string" && err.trim()) {
    return err;
  }
  if (err && typeof err === "object" && "message" in err) {
    const msg = (err as { message?: unknown }).message;
    if (typeof msg === "string" && msg.trim()) {
      return msg;
    }
  }
  return fallback;
}
