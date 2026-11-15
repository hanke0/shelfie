import { useEffect, useState } from "react";
import { fetchCoverBlob } from "@/lib/custom-fetch";

interface CoverState {
  key: string;
  src: string | null;
}

/**
 * 加载服务器封面为 object URL，卸载或参数变化时自动 revoke。
 * `missing` 仅在确认无封面（请求失败）后为 true，加载中为 false，避免提前绘制回退封面。
 * 递增 `version` 可强制重新拉取（如更换封面后）。
 */
export function useBookCover(
  bookId: string,
  { thumbnail = false, version = 0 }: { thumbnail?: boolean; version?: number } = {},
) {
  const key = `${bookId}|${thumbnail}|${version}`;
  const [state, setState] = useState<CoverState | null>(null);

  useEffect(() => {
    if (!bookId) return;
    const requestKey = `${bookId}|${thumbnail}|${version}`;
    let url: string | null = null;
    let cancelled = false;
    fetchCoverBlob(bookId, { thumbnail })
      .then((blobUrl) => {
        if (cancelled) {
          URL.revokeObjectURL(blobUrl);
          return;
        }
        url = blobUrl;
        setState({ key: requestKey, src: blobUrl });
      })
      .catch(() => {
        if (!cancelled) setState({ key: requestKey, src: null });
      });
    return () => {
      cancelled = true;
      if (url) URL.revokeObjectURL(url);
    };
  }, [bookId, thumbnail, version]);

  const current = state?.key === key ? state : null;
  return { src: current?.src ?? null, missing: current !== null && current.src === null };
}
