import { useCallback } from "react";
import { useToast } from "@/context/ToastContext";
import { formatApiError } from "@/lib/api-error";

interface ApiActionOptions {
  successMessage?: string;
  errorMessage?: string;
}

/**
 * 包装异步 API 调用：失败时弹出 Toast，成功可选提示。
 * @returns 是否成功
 */
export function useApiAction() {
  const toast = useToast();

  return useCallback(
    async <T>(fn: () => Promise<T>, options?: ApiActionOptions): Promise<boolean> => {
      try {
        await fn();
        if (options?.successMessage) {
          toast.success(options.successMessage);
        }
        return true;
      } catch (err) {
        toast.error(formatApiError(err, options?.errorMessage));
        return false;
      }
    },
    [toast],
  );
}
