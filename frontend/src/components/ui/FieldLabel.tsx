import type { ReactNode } from "react";
import styles from "./Form.module.css";

export function FieldLabel({
  required,
  children,
}: {
  required?: boolean;
  children: ReactNode;
}) {
  return (
    <span className={styles.labelLine}>
      {required ? (
        <span className={styles.req} aria-hidden="true">
          *{" "}
        </span>
      ) : null}
      {children}
    </span>
  );
}
