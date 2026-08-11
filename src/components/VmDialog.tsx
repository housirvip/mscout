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
    invoke<VmInfo[]>("list_vms")
      .then(setVms)
      .catch((e) => setError(String(e)))
      .finally(() => setLoading(false));
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
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <h2>VM Scan</h2>

        {error && (
          <div style={{ color: "var(--error)", padding: "8px 0" }}>
            {error}
          </div>
        )}

        {!selectedVm ? (
          <>
            <p style={{ color: "var(--text-secondary)", margin: "8px 0" }}>
              Select a running virtual machine:
            </p>
            <div className="process-list">
              {loading ? (
                <div style={{ padding: 12, color: "var(--text-secondary)" }}>
                  Detecting VMs...
                </div>
              ) : vms.length === 0 ? (
                <div style={{ padding: 12, color: "var(--text-secondary)" }}>
                  No VMs detected. Ensure VMware or Hyper-V is running.
                </div>
              ) : (
                vms.map((vm) => (
                  <div
                    key={vm.id}
                    className="process-item"
                    onDoubleClick={() => handleSelectVm(vm)}
                  >
                    <span>{vm.name}</span>
                    <span style={{ color: "var(--text-secondary)" }}>
                      {vm.vm_type}
                    </span>
                  </div>
                ))
              )}
            </div>
          </>
        ) : (
          <>
            <p style={{ color: "var(--text-secondary)", margin: "8px 0" }}>
              {selectedVm.name} — Select a guest process:
            </p>
            {scanning ? (
              <div style={{ padding: 12, color: "var(--text-secondary)" }}>
                Scanning guest kernel... This may take a few seconds.
              </div>
            ) : (
              <>
                <input
                  className="search-input"
                  type="text"
                  placeholder="Filter by name or PID..."
                  value={filter}
                  onChange={(e) => setFilter(e.target.value)}
                  autoFocus
                />
                <div className="process-list">
                  {filteredProcesses.map((p) => (
                    <div
                      key={p.pid}
                      className="process-item"
                      onDoubleClick={() => handleAttachProcess(p)}
                    >
                      <span>{p.name}</span>
                      <span style={{ color: "var(--text-secondary)" }}>
                        {p.pid}
                      </span>
                    </div>
                  ))}
                </div>
              </>
            )}
            <button
              style={{ marginTop: 8 }}
              onClick={() => {
                setSelectedVm(null);
                setGuestProcesses([]);
                setFilter("");
              }}
            >
              ← Back to VM list
            </button>
          </>
        )}
      </div>
    </div>
  );
}
