import { createContext, useContext, useState, useCallback, useRef, useEffect } from "react";

interface ToastMessage {
  id: number;
  text: string;
  type: "error" | "success" | "info";
}

interface ToastContextValue {
  showToast: (text: string, type?: "error" | "success" | "info") => void;
}

const ToastContext = createContext<ToastContextValue>({
  showToast: () => {},
});

export function useToast() {
  return useContext(ToastContext);
}

export function ToastProvider({ children }: { children: React.ReactNode }) {
  const [toasts, setToasts] = useState<ToastMessage[]>([]);
  const nextId = useRef(0);
  const timeoutIds = useRef<Map<number, number>>(new Map());

  useEffect(() => {
    return () => {
      // Clear all pending timeouts on unmount
      for (const tid of timeoutIds.current.values()) {
        clearTimeout(tid);
      }
      timeoutIds.current.clear();
    };
  }, []);

  const showToast = useCallback((text: string, type: "error" | "success" | "info" = "error") => {
    const id = nextId.current++;
    setToasts((prev) => [...prev, { id, text, type }]);
    const tid = window.setTimeout(() => {
      setToasts((prev) => prev.filter((t) => t.id !== id));
      timeoutIds.current.delete(id);
    }, 4000);
    timeoutIds.current.set(id, tid);
  }, []);

  return (
    <ToastContext.Provider value={{ showToast }}>
      {children}
      <div className="toast-container">
        {toasts.map((toast) => (
          <div key={toast.id} className={`toast toast-${toast.type}`}>
            {toast.text}
          </div>
        ))}
      </div>
    </ToastContext.Provider>
  );
}
