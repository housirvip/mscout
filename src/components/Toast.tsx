import { createContext, useContext, useState, useCallback, useRef, useEffect, type ReactNode } from "react";

type ToastKind = "success" | "error" | "info";

interface ToastMessage {
  id: number;
  text: string;
  kind: ToastKind;
  exiting?: boolean;
}

interface ToastContextValue {
  showToast: (message: string, kind?: ToastKind) => void;
}

const ToastContext = createContext<ToastContextValue>({ showToast: () => {} });

export function useToast() {
  return useContext(ToastContext);
}

const ICONS: Record<ToastKind, string> = {
  success: "M8.2 12.2 10.9 14.9 15.9 9.3",
  error: "M12 7.8v5M12 16.1v.1",
  info: "M12 11v5.2M12 7.9v.1",
};

export function ToastProvider({ children }: { children: ReactNode }) {
  const [toasts, setToasts] = useState<ToastMessage[]>([]);
  const nextId = useRef(0);
  const timers = useRef<Map<number, number>>(new Map());

  useEffect(() => {
    return () => {
      for (const tid of timers.current.values()) {
        clearTimeout(tid);
      }
      timers.current.clear();
    };
  }, []);

  const dismiss = useCallback((id: number) => {
    setToasts((prev) => prev.map((t) => (t.id === id ? { ...t, exiting: true } : t)));
    const removeTimer = window.setTimeout(() => {
      setToasts((prev) => prev.filter((t) => t.id !== id));
      timers.current.delete(id);
    }, 250);
    timers.current.set(id, removeTimer);
  }, []);

  const showToast = useCallback((message: string, kind: ToastKind = "info") => {
    const id = nextId.current++;
    setToasts((prev) => [...prev, { id, text: message, kind }]);
    const tid = window.setTimeout(() => {
      dismiss(id);
    }, 4000);
    timers.current.set(id, tid);
  }, [dismiss]);

  return (
    <ToastContext.Provider value={{ showToast }}>
      {children}
      <div className="toasts" aria-live="polite">
        {toasts.map((toast) => (
          <div
            key={toast.id}
            className={`toast${toast.exiting ? " out" : ""}`}
            data-kind={toast.kind}
          >
            <svg className="icon" aria-hidden="true" viewBox="0 0 24 24">
              <circle cx="12" cy="12" r="8.6" fill="none" stroke="currentColor" strokeWidth="1.5" />
              <path d={ICONS[toast.kind]} fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />
            </svg>
            <span className="txt">{toast.text}</span>
          </div>
        ))}
      </div>
    </ToastContext.Provider>
  );
}
