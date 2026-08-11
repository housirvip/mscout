import { useState, useEffect, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useToast } from "./Toast";
import { useI18n, MessageKey } from "../i18n";

interface ProcessInfo {
  pid: number;
  name: string;
}

interface Props {
  onAttach: (pid: number, name: string) => void;
  onClose: () => void;
}

export function ProcessList({ onAttach, onClose }: Props) {
  const { t } = useI18n();
  const { showToast } = useToast();
  const [processes, setProcesses] = useState<ProcessInfo[]>([]);
  const [filter, setFilter] = useState("");
  const [loading, setLoading] = useState(true);
  const [selected, setSelected] = useState<number | null>(null);
  const mountedRef = useRef(true);

  useEffect(() => {
    mountedRef.current = true;
    invoke<ProcessInfo[]>("list_processes")
      .then((data) => {
        if (mountedRef.current) setProcesses(data);
      })
      .catch((e) => {
        if (mountedRef.current) showToast(String(e), "error");
      })
      .finally(() => {
        if (mountedRef.current) setLoading(false);
      });
    return () => { mountedRef.current = false; };
  }, []);

  useEffect(() => {
    function handleKey(e: KeyboardEvent) {
      if (e.key === "Escape") onClose();
    }
    document.addEventListener("keydown", handleKey);
    return () => document.removeEventListener("keydown", handleKey);
  }, [onClose]);

  const filtered = processes.filter(
    (p) =>
      p.name.toLowerCase().includes(filter.toLowerCase()) ||
      String(p.pid).includes(filter)
  );

  // Reset selection when filter changes
  useEffect(() => { setSelected(null); }, [filter]);

  const handleAttach = useCallback(() => {
    const proc = processes.find((p) => p.pid === selected);
    if (proc) onAttach(proc.pid, proc.name);
  }, [selected, processes, onAttach]);

  return (
    <div className="scrim" onClick={onClose}>
      <div
        className="modal"
        role="dialog"
        aria-modal="true"
        onClick={(e) => e.stopPropagation()}
        style={{ width: 620 }}
      >
        {/* Header */}
        <div className="modal-head">
          <div>
            <h2>{t("proc.title")}</h2>
            <p>{t("proc.doubleClickHint" as MessageKey)}</p>
          </div>
          <span className="spacer"></span>
          <button className="btn-close" onClick={onClose}>
            <svg className="icon" aria-hidden="true" viewBox="0 0 24 24">
              <path d="m6 6 12 12M18 6 6 18" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
            </svg>
          </button>
        </div>

        {/* Search */}
        <div className="search-wrap">
          <svg className="icon" aria-hidden="true" viewBox="0 0 24 24">
            <circle cx="10.5" cy="10.5" r="6.5" fill="none" stroke="currentColor" strokeWidth="1.5" />
            <path d="m15.4 15.4 4.6 4.6" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
          </svg>
          <input
            className="control search-in"
            placeholder={t("proc.search")}
            value={filter}
            onChange={(e) => setFilter(e.target.value)}
            autoFocus
          />
        </div>

        {/* Body */}
        <div className="modal-body">
          {loading ? (
            <div className="empty" style={{ minHeight: 160 }}>
              <div className="empty-inner"><p>{t("proc.loading" as MessageKey)}</p></div>
            </div>
          ) : filtered.length === 0 ? (
            <div className="empty" style={{ minHeight: 120 }}>
              <div className="empty-inner">
                <p>{processes.length === 0 ? t("proc.notFound" as MessageKey) : t("proc.noMatch" as MessageKey)}</p>
              </div>
            </div>
          ) : (
            <table>
              <thead>
                <tr>
                  <th style={{ width: 86 }}>PID</th>
                  <th>{t("proc.colName" as MessageKey)}</th>
                </tr>
              </thead>
              <tbody>
                {filtered.map((p) => (
                  <tr
                    key={p.pid}
                    className={selected === p.pid ? "sel" : undefined}
                    onClick={() => setSelected(p.pid)}
                    onDoubleClick={() => onAttach(p.pid, p.name)}
                  >
                    <td className="mono">{p.pid}</td>
                    <td>{p.name}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </div>

        {/* Footer */}
        <div className="modal-foot">
          <span className="count">{t("proc.count" as MessageKey, { count: filtered.length })}</span>
          <div className="actions">
            <button className="btn" onClick={onClose}>{t("proc.cancel")}</button>
            <button
              className="btn btn-solid-dark"
              disabled={selected === null}
              onClick={handleAttach}
            >
              {t("proc.attach")}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
