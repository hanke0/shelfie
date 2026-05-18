import {
  createContext,
  useCallback,
  useContext,
  useState,
  type ReactNode,
} from "react";
import { Modal } from "@/components/ui/Modal";
import styles from "./ConfirmContext.module.css";

const DEFAULT_SECOND = "请再次确认。此操作无法撤销，确定继续吗？";

type PendingConfirm = {
  firstMessage: string;
  secondMessage: string;
  step: 1 | 2;
  resolve: (confirmed: boolean) => void;
};

interface ConfirmContextValue {
  confirmTwice: (firstMessage: string, secondMessage?: string) => Promise<boolean>;
}

const ConfirmContext = createContext<ConfirmContextValue | null>(null);

export function ConfirmProvider({ children }: { children: ReactNode }) {
  const [pending, setPending] = useState<PendingConfirm | null>(null);

  const confirmTwice = useCallback(
    (firstMessage: string, secondMessage: string = DEFAULT_SECOND) =>
      new Promise<boolean>((resolve) => {
        setPending({
          firstMessage,
          secondMessage,
          step: 1,
          resolve,
        });
      }),
    [],
  );

  const finish = (confirmed: boolean) => {
    pending?.resolve(confirmed);
    setPending(null);
  };

  const handlePrimary = () => {
    if (!pending) return;
    if (pending.step === 1) {
      setPending({ ...pending, step: 2 });
      return;
    }
    finish(true);
  };

  const handleCancel = () => {
    if (!pending) return;
    if (pending.step === 2) {
      setPending({ ...pending, step: 1 });
      return;
    }
    finish(false);
  };

  const title = pending?.step === 1 ? "确认操作" : "再次确认";
  const body = pending?.step === 1 ? pending.firstMessage : pending?.secondMessage;

  return (
    <ConfirmContext.Provider value={{ confirmTwice }}>
      {children}
      <Modal
        open={pending !== null}
        onClose={() => finish(false)}
        title={title}
        footer={
          <>
            <button type="button" className="btn btn-ghost" onClick={handleCancel}>
              {pending?.step === 2 ? "返回上一步" : "取消"}
            </button>
            <button
              type="button"
              className={pending?.step === 2 ? `btn ${styles.dangerBtn}` : "btn"}
              onClick={handlePrimary}
            >
              {pending?.step === 1 ? "继续" : "确定"}
            </button>
          </>
        }
      >
        <p className={styles.message}>{body}</p>
        {pending?.step === 2 ? (
          <p className={styles.warn}>此操作无法撤销，请谨慎操作。</p>
        ) : null}
      </Modal>
    </ConfirmContext.Provider>
  );
}

export function useConfirmTwice() {
  const ctx = useContext(ConfirmContext);
  if (!ctx) {
    throw new Error("useConfirmTwice must be used within ConfirmProvider");
  }
  return ctx.confirmTwice;
}
