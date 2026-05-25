import type { ReactNode } from "react";
import { useLibrary } from "@/context/LibraryContext";
import styles from "./LibraryBootstrap.module.css";

export function LibraryBootstrap({ children }: { children: ReactNode }) {
  const { isReady } = useLibrary();

  if (!isReady) {
    return (
      <div className={`app-shell ${styles.wrap}`}>
        <p className={styles.message} aria-busy="true">
          正在加载图书馆…
        </p>
      </div>
    );
  }

  return <>{children}</>;
}
