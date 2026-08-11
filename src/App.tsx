import { useState, useEffect, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { save, open } from "@tauri-apps/plugin-dialog";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useI18n, MessageKey } from "./i18n";
import { ToastProvider, useToast } from "./components/Toast";
import { ScanPanel } from "./components/ScanPanel";
import { ResultsTable } from "./components/ResultsTable";
import { AddressTable, AddressEntry } from "./components/AddressTable";
import { MemoryViewer } from "./components/MemoryViewer";
import { ProcessList } from "./components/ProcessList";
import { PointerScanDialog } from "./components/PointerScanDialog";
import { VmDialog } from "./components/VmDialog";
import { RegionViewer } from "./components/RegionViewer";

interface FrozenEntry {
  address: number;
  value: Record<string, unknown>;
  enabled: boolean;
  label: string;
}

interface CheatTable {
  version: number;
  process_name: string;
  entries: Array<{
    label: string;
    address: { Static: number };
    value_type: string;
    frozen: boolean;
    freeze_value: Record<string, unknown> | null;
  }>;
}

function getValueType(val: unknown): string {
  if (val === null || val === undefined) return "I32";
  if (typeof val === "object") {
    const keys = Object.keys(val as Record<string, unknown>);
    if (keys.length === 1) return keys[0];
  }
  return "I32";
}

function AppContent() {
  const { t } = useI18n();
  const { showToast } = useToast();

  const [attachedPid, setAttachedPid] = useState<number | null>(null);
  const [processName, setProcessName] = useState("");
  const [currentValueType, setCurrentValueType] = useState("I32");
  const [showProcessList, setShowProcessList] = useState(false);
  const [showVmDialog, setShowVmDialog] = useState(false);
  const [showPointerScan, setShowPointerScan] = useState(false);
  const [pointerScanTarget, setPointerScanTarget] = useState<string | null>(null);
  const [memViewAddr, setMemViewAddr] = useState<number | null>(null);
  const [showHexDrawer, setShowHexDrawer] = useState(false);
  const [matchCount, setMatchCount] = useState<number>(0);
  const [hasSession, setHasSession] = useState(false);
  const [addedEntries, setAddedEntries] = useState<AddressEntry[]>([]);
  const [alwaysOnTop, setAlwaysOnTop] = useState(false);
  const [showRegionViewer, setShowRegionViewer] = useState(false);

  const attached = attachedPid !== null;

  // --- Save/Load table ---
  const handleSaveTable = useCallback(async () => {
    try {
      const path = await save({
        filters: [{ name: "MScout Table", extensions: ["mst"] }],
      });
      if (!path) return;
      const entries = await invoke<FrozenEntry[]>("list_frozen");
      const table: CheatTable = {
        version: 1,
        process_name: processName,
        entries: entries.map((e) => ({
          label: e.label,
          address: { Static: e.address },
          value_type: getValueType(e.value),
          frozen: e.enabled,
          freeze_value: e.value,
        })),
      };
      await invoke("save_table", { path, table });
      showToast(t("toast.saved"), "success");
    } catch (e) {
      showToast(t("toast.saveFailed" as MessageKey, { error: String(e) }), "error");
    }
  }, [processName, showToast, t]);

  const handleLoadTable = useCallback(async () => {
    try {
      const path = await open({
        filters: [{ name: "MScout Table", extensions: ["mst"] }],
      });
      if (!path) return;
      const table = await invoke<CheatTable>("load_table", { path });
      for (const entry of table.entries) {
        const address = entry.address.Static;
        await invoke("add_frozen", {
          address,
          value: entry.freeze_value ?? { I32: 0 },
          label: entry.label,
        });
      }
      showToast(t("toast.loaded"), "success");
    } catch (e) {
      showToast(t("toast.loadFailed" as MessageKey, { error: String(e) }), "error");
    }
  }, [showToast, t]);

  // --- Always on top ---
  const handleToggleAot = useCallback(async () => {
    try {
      const next = !alwaysOnTop;
      await getCurrentWindow().setAlwaysOnTop(next);
      setAlwaysOnTop(next);
    } catch {
      // ignore if unavailable
    }
  }, [alwaysOnTop]);

  // --- Keyboard shortcuts ---
  const handleSaveRef = useRef(handleSaveTable);
  const handleLoadRef = useRef(handleLoadTable);
  useEffect(() => {
    handleSaveRef.current = handleSaveTable;
    handleLoadRef.current = handleLoadTable;
  });

  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      const mod = e.metaKey || e.ctrlKey;
      if (e.key === "F5") {
        e.preventDefault();
        window.dispatchEvent(new Event("scan:first"));
      } else if (e.key === "F6") {
        e.preventDefault();
        window.dispatchEvent(new Event("scan:next"));
      } else if (mod && e.key === "z") {
        e.preventDefault();
        window.dispatchEvent(new Event("scan:undo"));
      } else if (mod && e.key === "n") {
        e.preventDefault();
        setShowProcessList(true);
      } else if (mod && e.key === "s") {
        e.preventDefault();
        handleSaveRef.current();
      } else if (mod && e.key === "o") {
        e.preventDefault();
        handleLoadRef.current();
      }
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  // --- Handlers ---
  const handleViewMemory = useCallback((address: number) => {
    setMemViewAddr(address);
    setShowHexDrawer(true);
  }, []);

  const handleAddToTable = useCallback(
    (address: string, value: string, valueType: string) => {
      const addr = parseInt(address, 16);
      const numVal = valueType.startsWith("F")
        ? parseFloat(value)
        : parseInt(value, 10);
      const scanValue = isNaN(numVal) ? { I32: 0 } : { [valueType]: numVal };
      setAddedEntries((prev) => [
        ...prev,
        { address: addr, label: "", value: scanValue, enabled: false },
      ]);
    },
    []
  );

  const handlePointerScan = useCallback((address: string) => {
    setPointerScanTarget(address);
    setShowPointerScan(true);
  }, []);

  const handleFirstScanDone = useCallback((count: number) => {
    setMatchCount(count);
    setHasSession(true);
  }, []);

  const handleNextScanDone = useCallback((count: number) => {
    setMatchCount(count);
  }, []);

  const handleUndoDone = useCallback((count: number) => {
    setMatchCount(count);
  }, []);


  return (
    <div className={`app${alwaysOnTop ? " compact" : ""}`}>
      {/* ── toolbar ── */}
      <header className="toolbar">
        <div className="brand">
          <svg className="brand-mark" aria-hidden="true" viewBox="0 0 24 24">
            <rect x="4" y="4" width="16" height="16" rx="2" />
            <line x1="9" y1="1" x2="9" y2="4" />
            <line x1="15" y1="1" x2="15" y2="4" />
            <line x1="9" y1="20" x2="9" y2="23" />
            <line x1="15" y1="20" x2="15" y2="23" />
            <line x1="1" y1="9" x2="4" y2="9" />
            <line x1="1" y1="15" x2="4" y2="15" />
            <line x1="20" y1="9" x2="23" y2="9" />
            <line x1="20" y1="15" x2="23" y2="15" />
          </svg>
          <span className="brand-name">{t("toolbar.brand")}</span>
          <span className="brand-ver">0.4</span>
        </div>

        <div
          className="proc-chip"
          data-state={attached ? "ready" : "empty"}
        >
          <span className="dot" />
          <span className="proc-name">
            {attached ? `${processName} (${attachedPid})` : t("toolbar.notAttached")}
          </span>
          <span className="proc-meta">
            {attached ? `PID ${attachedPid}` : t("toolbar.notAttachedHint")}
          </span>
        </div>

        <button className="btn" onClick={() => setShowProcessList(true)}>
          <svg className="icon" aria-hidden="true" viewBox="0 0 24 24">
            <circle cx="12" cy="12" r="10" />
            <circle cx="12" cy="12" r="3" />
            <line x1="12" y1="2" x2="12" y2="5" />
            <line x1="12" y1="19" x2="12" y2="22" />
            <line x1="2" y1="12" x2="5" y2="12" />
            <line x1="19" y1="12" x2="22" y2="12" />
          </svg>
          <span className="btn-lbl">{t("toolbar.pickProcess")}</span>
          <kbd>⌘N</kbd>
        </button>

        <button className="btn" onClick={() => setShowVmDialog(true)}>
          <svg className="icon" aria-hidden="true" viewBox="0 0 24 24">
            <rect x="2" y="3" width="20" height="14" rx="2" />
            <line x1="8" y1="21" x2="16" y2="21" />
            <line x1="12" y1="17" x2="12" y2="21" />
          </svg>
          <span className="btn-lbl">{t("toolbar.vmScan")}</span>
        </button>
        <button className="btn" onClick={() => setShowRegionViewer(true)} disabled={!attached}>
          <svg className="icon" aria-hidden="true" viewBox="0 0 24 24">
            <rect x="3" y="3" width="18" height="18" rx="2" fill="none" stroke="currentColor" strokeWidth="1.5" />
            <line x1="3" y1="9" x2="21" y2="9" stroke="currentColor" strokeWidth="1.5" />
            <line x1="3" y1="15" x2="21" y2="15" stroke="currentColor" strokeWidth="1.5" />
          </svg>
          <span className="btn-lbl">{t("region.title")}</span>
        </button>

        <span className="spacer" />

        <span className="aot-badge">
          <svg className="icon" style={{ width: 12, height: 12 }} aria-hidden="true" viewBox="0 0 24 24">
            <line x1="12" y1="17" x2="12" y2="3" />
            <line x1="5" y1="10" x2="12" y2="3" />
            <line x1="19" y1="10" x2="12" y2="3" />
            <line x1="5" y1="21" x2="19" y2="21" />
          </svg>
          {t("toolbar.alwaysOnTop")}
        </span>

        <button className="btn" onClick={handleSaveTable}>
          <svg className="icon" aria-hidden="true" viewBox="0 0 24 24">
            <path d="M19 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11l5 5v11a2 2 0 0 1-2 2z" />
            <polyline points="17 21 17 13 7 13 7 21" />
            <polyline points="7 3 7 8 15 8" />
          </svg>
          <span className="btn-lbl">{t("toolbar.saveTable")}</span>
        </button>

        <button className="btn" onClick={handleLoadTable}>
          <svg className="icon" aria-hidden="true" viewBox="0 0 24 24">
            <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" />
          </svg>
          <span className="btn-lbl">{t("toolbar.loadTable")}</span>
        </button>

        <button
          className="btn btn-icon"
          aria-pressed={alwaysOnTop ? "true" : "false"}
          title={t("toolbar.alwaysOnTop")}
          onClick={handleToggleAot}
        >
          <svg className="icon" aria-hidden="true" viewBox="0 0 24 24">
            <line x1="12" y1="17" x2="12" y2="3" />
            <line x1="5" y1="10" x2="12" y2="3" />
            <line x1="19" y1="10" x2="12" y2="3" />
            <line x1="5" y1="21" x2="19" y2="21" />
          </svg>
          <span className="sr">{t("toolbar.alwaysOnTop")}</span>
        </button>
      </header>

      {/* ── body grid ── */}
      <div className="body-grid">
        <ScanPanel
          attached={attached}
          hasSession={hasSession}
          onFirstScan={handleFirstScanDone}
          onNextScan={handleNextScanDone}
          onUndo={handleUndoDone}
          onValueTypeChange={setCurrentValueType}
          onPickProcess={() => setShowProcessList(true)}
        />

        <div className="stack">
          <ResultsTable
            attached={attached}
            valueType={currentValueType}
            hasSession={hasSession}
            matchCount={matchCount}
            onViewMemory={handleViewMemory}
            onAddToTable={handleAddToTable}
            onPointerScan={handlePointerScan}
            onPickProcess={() => setShowProcessList(true)}
          />
          <AddressTable externalEntries={addedEntries} />
        </div>
      </div>

      {/* ── hex drawer ── */}
      {memViewAddr !== null && (
        <MemoryViewer
          address={memViewAddr}
          open={showHexDrawer}
          onClose={() => setShowHexDrawer(false)}
        />
      )}

      {/* ── region viewer drawer ── */}
      {attached && (
        <RegionViewer
          open={showRegionViewer}
          onClose={() => setShowRegionViewer(false)}
          onViewMemory={(addr) => {
            setMemViewAddr(addr);
            setShowHexDrawer(true);
            setShowRegionViewer(false);
          }}
        />
      )}

      {/* ── modals ── */}
      {showProcessList && (
        <ProcessList
          onAttach={async (pid, name) => {
            try {
              await invoke("attach_process", { pid });
              setAttachedPid(pid);
              setProcessName(name);
              setShowProcessList(false);
            } catch (e) {
              showToast(`Attach failed: ${e}`, "error");
            }
          }}
          onClose={() => setShowProcessList(false)}
        />
      )}

      {showPointerScan && (
        <PointerScanDialog
          targetAddress={pointerScanTarget}
          onClose={() => setShowPointerScan(false)}
          onAddToTable={(address, label) => {
            const scanValue = { [currentValueType]: 0 };
            setAddedEntries((prev) => [
              ...prev,
              { address, label, value: scanValue, enabled: false },
            ]);
          }}
        />
      )}

      {showVmDialog && (
        <VmDialog
          onAttach={(pid, name) => {
            setAttachedPid(pid);
            setProcessName(name);
            setShowVmDialog(false);
          }}
          onClose={() => setShowVmDialog(false)}
        />
      )}
    </div>
  );
}

function App() {
  return (
    <ToastProvider>
      <AppContent />
    </ToastProvider>
  );
}

export default App;
