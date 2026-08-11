import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useToast } from "./Toast";
import { useI18n } from "../i18n";

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
  const { t } = useI18n();
  const { showToast } = useToast();
  const [step, setStep] = useState<1 | 2>(1);
  const [vms, setVms] = useState<VmInfo[]>([]);
  const [guestProcesses, setGuestProcesses] = useState<GuestProcess[]>([]);
  const [selectedVm, setSelectedVm] = useState<VmInfo | null>(null);
  const [selectedGuest, setSelectedGuest] = useState<number | null>(null);
  const [filter, setFilter] = useState("");
  const [typeFilter, setTypeFilter] = useState<string>("all");
  const [loading, setLoading] = useState(true);
  const [scanningGuest, setScanningGuest] = useState(false);
  const mountedRef = useRef(true);

  useEffect(() => {
    mountedRef.current = true;
    return () => { mountedRef.current = false; };
  }, []);

  useEffect(() => {
    function handleKey(e: KeyboardEvent) {
      if (e.key === "Escape") onClose();
    }
    document.addEventListener("keydown", handleKey);
    return () => document.removeEventListener("keydown", handleKey);
  }, [onClose]);

  // Load VMs on mount
  useEffect(() => {
    loadVms();
  }, []);

  async function loadVms() {
    setLoading(true);
    try {
      const data = await invoke<VmInfo[]>("list_vms");
      if (!mountedRef.current) return;
      setVms(data);
    } catch (e) {
      if (!mountedRef.current) return;
      showToast(String(e), "error");
    } finally {
      if (mountedRef.current) setLoading(false);
    }
  }

  async function handleSelectVm(vm: VmInfo) {
    setSelectedVm(vm);
    setScanningGuest(true);
    setStep(2);
    setFilter("");
    setSelectedGuest(null);

    try {
      const procs = await invoke<GuestProcess[]>("attach_vm", { pid: vm.pid });
      if (!mountedRef.current) return;
      setGuestProcesses(procs);
    } catch (e) {
      if (!mountedRef.current) return;
      showToast(String(e), "error");
      setStep(1);
      setSelectedVm(null);
    } finally {
      if (mountedRef.current) setScanningGuest(false);
    }
  }

  function handleBack() {
    setStep(1);
    setSelectedVm(null);
    setGuestProcesses([]);
    setSelectedGuest(null);
    setFilter("");
  }

  function handleAttachGuest() {
    const proc = guestProcesses.find((p) => p.pid === selectedGuest);
    if (!proc || !selectedVm) return;
    invoke("attach_vm_process", { vmPid: selectedVm.pid, guestPid: proc.pid })
      .then(() => { onAttach(proc.pid, proc.name); })
      .catch((e) => showToast(String(e), "error"));
  }

  const filteredVms = vms.filter((vm) =>
    typeFilter === "all" || vm.vm_type === typeFilter
  );

  const filteredGuests = guestProcesses.filter(
    (p) =>
      p.name.toLowerCase().includes(filter.toLowerCase()) ||
      String(p.pid).includes(filter)
  );

  return (
    <div className="scrim" onClick={onClose}>
      <div
        className="modal"
        role="dialog"
        aria-modal="true"
        onClick={(e) => e.stopPropagation()}
        style={{ width: 640, maxHeight: "82vh" }}
      >
        {/* Header */}
        <div className="modal-head">
          <svg className="icon" aria-hidden="true" viewBox="0 0 24 24" style={{ marginTop: 2 }}>
            <rect x="2.5" y="4.5" width="19" height="12" rx="2" fill="none" stroke="currentColor" strokeWidth="1.5" />
            <path d="M8 20h8M12 16.5V20" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
            <path d="M10 8.6l4 2-4 2z" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinejoin="round" />
          </svg>
          <div>
            <h2>{t("vm.title")}</h2>
            <p>直接读取虚拟机的物理内存，无需在客户机内安装任何组件。</p>
          </div>
          <span className="spacer"></span>
          <button className="btn-close" onClick={onClose}>
            <svg className="icon" aria-hidden="true" viewBox="0 0 24 24">
              <path d="m6 6 12 12M18 6 6 18" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
            </svg>
          </button>
        </div>

        {/* Steps indicator */}
        <div style={{
          display: "flex", alignItems: "center", gap: 8,
          padding: "var(--space-3) var(--space-5)",
          borderBottom: "1px solid var(--border-soft)",
          fontSize: "var(--text-xs)"
        }}>
          <span style={{ display: "flex", alignItems: "center", gap: 4, fontWeight: step === 1 ? 700 : 400, color: step === 1 ? "var(--fg)" : "var(--muted)" }}>
            <span style={{
              width: 20, height: 20, borderRadius: "var(--radius-pill)",
              background: step === 1 ? "var(--fg)" : "var(--border)",
              color: step === 1 ? "var(--surface)" : "var(--muted)",
              display: "inline-grid", placeItems: "center", fontSize: 11, fontWeight: 700
            }}>1</span>
            {t("vm.step1")}
          </span>
          <span style={{ width: 24, height: 1, background: "var(--border)" }}></span>
          <span style={{ display: "flex", alignItems: "center", gap: 4, fontWeight: step === 2 ? 700 : 400, color: step === 2 ? "var(--fg)" : "var(--muted)" }}>
            <span style={{
              width: 20, height: 20, borderRadius: "var(--radius-pill)",
              background: step === 2 ? "var(--fg)" : "var(--border)",
              color: step === 2 ? "var(--surface)" : "var(--muted)",
              display: "inline-grid", placeItems: "center", fontSize: 11, fontWeight: 700
            }}>2</span>
            {t("vm.step2")}
          </span>
        </div>

        {/* Step 1: VM list */}
        {step === 1 && (
          <>
            {/* Type filters */}
            <div style={{ display: "flex", alignItems: "center", gap: 6, padding: "var(--space-2) var(--space-5)" }}>
              {["all", "VMware", "Hyper-V", "QEMU"].map((tp) => (
                <button
                  key={tp}
                  className="btn"
                  aria-pressed={typeFilter === tp}
                  onClick={() => setTypeFilter(tp)}
                  style={{
                    height: 28, fontSize: 11,
                    background: typeFilter === tp ? "var(--fg)" : undefined,
                    color: typeFilter === tp ? "var(--surface)" : undefined,
                    borderColor: typeFilter === tp ? "var(--fg)" : undefined,
                  }}
                >
                  {tp === "all" ? "全部" : tp === "QEMU" ? "QEMU / KVM" : tp}
                </button>
              ))}
              <span className="spacer"></span>
              <button className="btn" onClick={loadVms} style={{ height: 28, fontSize: 11 }}>
                <svg className="icon" aria-hidden="true" viewBox="0 0 24 24" style={{ width: 13, height: 13 }}>
                  <path d="M20 12a8 8 0 1 1-2.6-5.9" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
                  <path d="M20 4v4.6h-4.6" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />
                </svg>
                重新检测
              </button>
            </div>

            {/* VM list body */}
            <div style={{ flex: "1 1 auto", overflow: "auto", minHeight: 160 }}>
              {loading ? (
                <div className="empty" style={{ minHeight: 160 }}>
                  <div className="empty-inner">
                    <div className="spin"></div>
                    <p>正在检测虚拟机…</p>
                  </div>
                </div>
              ) : filteredVms.length === 0 ? (
                <div className="empty" style={{ minHeight: 200 }}>
                  <div className="empty-inner">
                    <svg className="icon" aria-hidden="true" viewBox="0 0 24 24" style={{ width: 48, height: 48, color: "var(--muted)" }}>
                      <rect x="2.5" y="4.5" width="19" height="12" rx="2" fill="none" stroke="currentColor" strokeWidth="1.5" />
                      <path d="M8 20h8M12 16.5V20" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
                    </svg>
                    <h2 style={{ marginTop: 12 }}>{t("vm.noVm")}</h2>
                    <p>{t("vm.noVmHint")}</p>
                    <button className="btn" onClick={loadVms} style={{ marginTop: 12 }}>{t("vm.refresh")}</button>
                  </div>
                </div>
              ) : (
                <div style={{ padding: "var(--space-2) var(--space-5)" }}>
                  {filteredVms.map((vm) => (
                    <div
                      key={vm.id}
                      onClick={() => setSelectedVm(vm)}
                      onDoubleClick={() => handleSelectVm(vm)}
                      style={{
                        display: "flex", alignItems: "center", gap: 10,
                        padding: "10px 12px", borderRadius: "var(--radius-sm)",
                        border: selectedVm?.id === vm.id ? "1px solid var(--fg)" : "1px solid var(--border-soft)",
                        background: selectedVm?.id === vm.id ? "var(--surface-warm)" : "var(--surface)",
                        cursor: "pointer", marginBottom: 6,
                      }}
                    >
                      <svg className="icon" aria-hidden="true" viewBox="0 0 24 24" style={{ flexShrink: 0 }}>
                        <rect x="2.5" y="4.5" width="19" height="12" rx="2" fill="none" stroke="currentColor" strokeWidth="1.5" />
                        <path d="M8 20h8M12 16.5V20" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
                      </svg>
                      <div style={{ flex: 1 }}>
                        <div style={{ fontWeight: 600, fontSize: "var(--text-xs)" }}>{vm.name}</div>
                        <div style={{ fontSize: 11, color: "var(--muted)" }}>PID {vm.pid}</div>
                      </div>
                      <span style={{
                        fontSize: 10, fontWeight: 700, padding: "2px 7px",
                        borderRadius: "var(--radius-pill)",
                        background: "var(--surface-warm)", border: "1px solid var(--border)",
                      }}>{vm.vm_type}</span>
                    </div>
                  ))}
                </div>
              )}
            </div>
          </>
        )}

        {/* Step 2: Guest processes */}
        {step === 2 && (
          <div style={{ display: "flex", flexDirection: "column", flex: "1 1 auto", minHeight: 0 }}>
            {/* Search */}
            <div className="search-wrap">
              <svg className="icon" aria-hidden="true" viewBox="0 0 24 24">
                <circle cx="10.5" cy="10.5" r="6.5" fill="none" stroke="currentColor" strokeWidth="1.5" />
                <path d="m15.4 15.4 4.6 4.6" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
              </svg>
              <input
                className="control search-in"
                value={filter}
                onChange={(e) => setFilter(e.target.value)}
                placeholder="在客户机进程中按名称或 PID 过滤…"
                autoFocus
              />
            </div>

            <div style={{ flex: "1 1 auto", overflow: "auto", minHeight: 120 }}>
              {scanningGuest ? (
                <div className="empty" style={{ minHeight: 160 }}>
                  <div className="empty-inner">
                    <div className="spin"></div>
                    <h2>正在扫描客户机内核…</h2>
                    <p className="mono" style={{ color: "var(--muted)" }}>定位 KPCR 与 EPROCESS 链表</p>
                  </div>
                </div>
              ) : filteredGuests.length === 0 ? (
                <div className="empty" style={{ minHeight: 120 }}>
                  <div className="empty-inner">
                    <p>没有匹配的客户机进程。</p>
                  </div>
                </div>
              ) : (
                <table>
                  <thead>
                    <tr>
                      <th style={{ width: 96 }}>PID</th>
                      <th>进程名</th>
                    </tr>
                  </thead>
                  <tbody>
                    {filteredGuests.map((p) => (
                      <tr
                        key={p.pid}
                        className={selectedGuest === p.pid ? "sel" : undefined}
                        onClick={() => setSelectedGuest(p.pid)}
                        onDoubleClick={() => { setSelectedGuest(p.pid); handleAttachGuest(); }}
                      >
                        <td className="mono">{p.pid}</td>
                        <td>{p.name}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              )}
            </div>
          </div>
        )}

        {/* Footer */}
        <div className="modal-foot">
          <span style={{ fontSize: 11, color: "var(--muted)" }}>
            {step === 1
              ? `已检测到 ${filteredVms.length} 台虚拟机`
              : `${selectedVm?.name} · ${filteredGuests.length} 个进程`
            }
          </span>
          <span className="spacer"></span>
          {step === 2 && (
            <button className="btn" onClick={handleBack}>返回上一步</button>
          )}
          <button className="btn" onClick={onClose}>{t("vm.cancel")}</button>
          {step === 1 ? (
            <button
              className="btn btn-primary"
              disabled={selectedVm === null}
              onClick={() => selectedVm && handleSelectVm(selectedVm)}
            >
              {t("vm.attach")}
            </button>
          ) : (
            <button
              className="btn btn-solid-dark"
              disabled={selectedGuest === null}
              onClick={handleAttachGuest}
            >
              {t("vm.attach")}
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
