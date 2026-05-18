import { useEffect, useState } from "react";
import { generateTitleCoverBlobUrl } from "@/lib/title-cover";

interface TitleCoverImageProps {
  title: string;
  author?: string;
  alt?: string;
  className?: string;
}

/** 基于书名/作者绘制的封面图（无服务器封面时的回退） */
export function TitleCoverImage({ title, author, alt, className }: TitleCoverImageProps) {
  const [src, setSrc] = useState<string | null>(null);

  useEffect(() => {
    let url: string | null = null;
    let cancelled = false;

    generateTitleCoverBlobUrl(title, author)
      .then((blobUrl) => {
        if (!cancelled) {
          url = blobUrl;
          setSrc(blobUrl);
        } else {
          URL.revokeObjectURL(blobUrl);
        }
      })
      .catch(() => setSrc(null));

    return () => {
      cancelled = true;
      if (url) URL.revokeObjectURL(url);
    };
  }, [title, author]);

  if (!src) {
    return (
      <div
        className={className}
        aria-hidden
        style={{
          display: "grid",
          placeItems: "center",
          background: "var(--accent-soft)",
          color: "var(--accent)",
          fontFamily: "var(--font-display)",
          fontSize: "2rem",
        }}
      >
        {title.trim().slice(0, 1) || "?"}
      </div>
    );
  }

  return <img src={src} alt={alt ?? title} className={className} />;
}
