import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface VmInfo {
  id: string;
  name: string;
  vm_type: string;
  pid: number;
}

interface GuestProcess {
  pid: number;
  name: string;
}

interface Props {
  onAttach: (pid: number, name: string) => void;
  onClose: () => void;
}

export function VmDialog({ onAttach, onClose }: Props) {
  const [vms, setVms] = useState<VmInfo[]>([]);
  const [guestProcesses, setGuestProcesses] = useState<GuestProcess[]>([]);
  const [selectedVm, setSelectedVm] = useState<VmInfo | null>(null);
  const [loading, setLoading] = useState(true);
  const [scanning, setScanning] = useState(false);
  const [filter, setFilter] = useState("");
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let ignore = false;
    invoke<VmInfo[]>("list_vms")
      .then((data) => {
        if (!ignore) setVms(data);
      })
      .catch((e) => {
        if (!ignore) setError(String(e));
      })
      .finally(() => {
        if (!ignore) setLoading(false);
      });
    return () => { ignore = true; };
  }, []);

  const handleSelectVm = async (vm: VmInfo) => {
    setSelectedVm(vm);
    setScanning(true);
    setError(null);
    try {
      const processes = await invoke<GuestProcess[]>("attach_vm", {
        pid: vm.pid,
      });
      setGuestProcesses(processes);
    } catch (e) {
      setError(String(e));
    } finally {
      setScanning(false);
    }
  };

  const handleAttachProcess = async (proc: GuestProcess) => {
    if (!selectedVm) return;
    setError(null);
    try {
      await invoke("attach_vm_process", {
        vmPid: selectedVm.pid,
        guestPid: proc.pid,
      });
      onAttach(proc.pid, `[VM] ${proc.name}`);
    } catch (e) {
      setError(String(e));
    }
  };

  const filteredProcesses = guestProcesses.filter(
    (p) =>
      p.name.toLowerCase().includes(filter.toLowerCase()) ||
      String(p.pid).includes(filter)
  );

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal" onClick={(e) => e.stopPropagation()} style={{ width: 600 }}>
        <h2>Virtual Machine Scanner</h2>

        {error && (
          <div style={{ color: "var(--error)", marginBottom: 8, fontSize: 13 }}>
            {error}
          </div>
        )}

        {!selectedVm ? (
          <>
            <p style={{ color: "var(--text-secondary)", margin: "0 0 8px" }}>
              Select a VM to scan:
            </p>
            <div className="process-list">
              {loading ? (
                <div style={{ padding: 12, color: "var(--text-secondary)" }}>
                  Detecting virtual machines...
                </div>
              ) : vms.length === 0 ? (
                <div style={{ padding: 12, color: "var(--text-secondary)" }}>
                  No virtual machines found.
                </div>
              ) : (
                vms.map((vm) => (
                  <div
                    key={vm.id}
                    className="process-item"
                    onDoubleClick={() => handleSelectVm(vm)}
                  >
                    <span>
                      {vm.name} <span style={{ color: "var(--text-secondary)" }}>({vm.vm_type})</span>
                    </span>
                    <span style={{ color: "var(--text-secondary)" }}>PID {vm.pid}</span>
                  </div>
                ))
              )}
            </div>
          </>
        ) : (
          <>
            <p style={{ color: "var(--text-secondary)", margin: "0 0 8px" }}>
              VM: {selectedVm.name} — Select a guest process:
            </p>
            <input
              className="search-input"
              type="text"
              placeholder="Filter by name or PID..."
              value={filter}
              onChange={(e) => setFilter(e.target.value)}
              autoFocus
            />
            <div className="process-list">
              {scanning ? (
                <div style={{ padding: 12, color: "var(--text-secondary)" }}>
                  Scanning guest processes...
                </div>
              ) : filteredProcesses.length === 0 ? (
                <div style={{ padding: 12, color: "var(--text-secondary)" }}>
                  No guest processes found.
                </div>
              ) : (
                filteredProcesses.map((p) => (
                  <div
                    key={p.pid}
                    className="process-item"
                    onDoubleClick={() => handleAttachProcess(p)}
                  >
                    <span>{p.name}</span>
                    <span style={{ color: "var(--text-secondary)" }}>{p.pid}</span>
                  </div>
                ))
              )}
            </div>
            <div style={{ marginTop: 8 }}>
              <button onClick={() => { setSelectedVm(null); setGuestProcesses([]); setFilter(""); }}>
                ← Back
              </button>
            </div>
          </>
        )}

        <div style={{ marginTop: 12, textAlign: "right" }}>
          <button onClick={onClose}>Close</button>
        </div>
      </div>
    </div>
  );
}
