import type { SelectHTMLAttributes } from "react";
import styles from "./Select.module.css";

export function Select({
  className,
  children,
  ...props
}: SelectHTMLAttributes<HTMLSelectElement>) {
  return (
    <div className={styles.wrap}>
      <select className={[styles.select, className].filter(Boolean).join(" ")} {...props}>
        {children}
      </select>
      <span className={styles.chevron} aria-hidden>
        ▾
      </span>
    </div>
  );
}
