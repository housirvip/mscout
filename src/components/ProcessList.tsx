import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useToast } from "./Toast";

interface ProcessInfo {
  pid: number;
  name: string;
}

interface Props {
  onAttach: (pid: number, name: string) => void;
  onClose: () => void;
}

export function ProcessList({ onAttach, onClose }: Props) {
  const [processes, setProcesses] = useState<ProcessInfo[]>([]);
  const [filter, setFilter] = useState("");
  const [loading, setLoading] = useState(true);
  const { showToast } = useToast();

  useEffect(() => {
    invoke<ProcessInfo[]>("list_processes")
      .then(setProcesses)
      .catch((e) => showToast(`Failed to list processes: ${e}`, "error"))
      .finally(() => setLoading(false));
  }, []);

  const filtered = processes.filter(
    (p) =>
      p.name.toLowerCase().includes(filter.toLowerCase()) ||
      String(p.pid).includes(filter)
  );

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <h2>Select Process</h2>
        <input
          className="search-input"
          type="text"
          placeholder="Filter by name or PID..."
          value={filter}
          onChange={(e) => setFilter(e.target.value)}
          autoFocus
        />
        <div className="process-list">
          {loading ? (
            <div style={{ padding: 12, color: "var(--text-secondary)" }}>
              Loading...
            </div>
          ) : filtered.length === 0 ? (
            <div style={{ padding: 12, color: "var(--text-secondary)", textAlign: "center" }}>
              {processes.length === 0
                ? "No processes found."
                : "No processes match the filter."}
            </div>
          ) : (
            filtered.map((p) => (
              <div
                key={p.pid}
                className="process-item"
                onDoubleClick={() => onAttach(p.pid, p.name)}
              >
                <span>{p.name}</span>
                <span style={{ color: "var(--text-secondary)" }}>{p.pid}</span>
              </div>
            ))
          )}
        </div>
      </div>
    </div>
  );
}
