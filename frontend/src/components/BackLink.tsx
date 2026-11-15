import { Link } from "react-router-dom";
import styles from "./BackLink.module.css";

interface BackLinkProps {
  to?: string;
  children?: React.ReactNode;
}

export function BackLink({ to = "/", children = "返回首页" }: BackLinkProps) {
  return (
    <Link to={to} className={styles.back}>
      <span className={styles.arrow} aria-hidden>
        ←
      </span>
      {children}
    </Link>
  );
}
