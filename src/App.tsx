import { useState, useEffect, useCallback } from "react";
import { ProcessList } from "./components/ProcessList";
import { ScanPanel } from "./components/ScanPanel";
import { ResultsTable } from "./components/ResultsTable";
import { AddressTable, AddressEntry } from "./components/AddressTable";
import { MemoryViewer } from "./components/MemoryViewer";
import { PointerScanDialog } from "./components/PointerScanDialog";
import { ToastProvider, useToast } from "./components/Toast";
import { VmDialog } from "./components/VmDialog";
import { invoke } from "@tauri-apps/api/core";
import { save, open } from "@tauri-apps/plugin-dialog";

interface FrozenEntry {
  address: number;
  value: unknown;
  enabled: boolean;
  label: string;
}

interface CheatTable {
  version: number;
  process_name: string;
  entries: Array<{
    label: string;
    address: unknown;
    value_type: string;
    frozen: boolean;
    freeze_value: unknown;
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
  const [attachedPid, setAttachedPid] = useState<number | null>(null);
  const [processName, setProcessName] = useState("");
  const [showProcessList, setShowProcessList] = useState(false);
  const [memoryViewerAddress, setMemoryViewerAddress] = useState<number | null>(null);
  const [showMemoryViewer, setShowMemoryViewer] = useState(false);
  const [pointerScanTarget, setPointerScanTarget] = useState<string | null>(null);
  const [showPointerScan, setShowPointerScan] = useState(false);
  const [showVmDialog, setShowVmDialog] = useState(false);
  const [addedEntries, setAddedEntries] = useState<AddressEntry[]>([]);
  const [currentValueType, setCurrentValueType] = useState("I32");
  const { showToast } = useToast();

  // Save/Load table handlers
  const handleSaveTable = useCallback(async () => {
    try {
      const path = await save({
        filters: [{ name: "MemScanner Table", extensions: ["mst"] }],
      });
      if (!path) return;
      const entries = await invoke<FrozenEntry[]>("list_frozen");
      const table = {
        version: 1,
        process_name: processName,
        entries: entries.map(e => ({
          label: e.label,
          address: { Static: e.address },
          value_type: getValueType(e.value),
          frozen: e.enabled,
          freeze_value: e.value,
        })),
      };
      await invoke("save_table", { path, table });
      showToast("Table saved", "success");
    } catch (e) {
      showToast(`Save failed: ${e}`, "error");
    }
  }, [processName, showToast]);

  const handleLoadTable = useCallback(async () => {
    try {
      const path = await open({
        filters: [{ name: "MemScanner Table", extensions: ["mst"] }],
      });
      if (!path) return;
      const table = await invoke<CheatTable>("load_table", { path });
      for (const entry of table.entries) {
        const address = typeof entry.address === "object" && entry.address !== null && "Static" in (entry.address as Record<string, unknown>)
          ? (entry.address as Record<string, unknown>).Static as number
          : 0;
        await invoke("add_frozen", {
          address,
          value: entry.freeze_value ?? { I32: 0 },
          label: entry.label,
        });
      }
      showToast("Table loaded", "success");
    } catch (e) {
      showToast(`Load failed: ${e}`, "error");
    }
  }, [showToast]);

  // Keyboard shortcuts — dispatch custom events that ScanPanel listens to
  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      const ctrl = e.ctrlKey || e.metaKey;
      if (e.key === "F5") {
        e.preventDefault();
        window.dispatchEvent(new CustomEvent("scan:first"));
      } else if (e.key === "F6") {
        e.preventDefault();
        window.dispatchEvent(new CustomEvent("scan:next"));
      } else if (ctrl && e.key === "z") {
        e.preventDefault();
        window.dispatchEvent(new CustomEvent("scan:undo"));
      } else if (ctrl && e.key === "n") {
        e.preventDefault();
        setShowProcessList(true);
      } else if (ctrl && e.key === "o") {
        e.preventDefault();
        handleLoadTable();
      } else if (ctrl && e.key === "s") {
        e.preventDefault();
        handleSaveTable();
      }
    }
    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [handleSaveTable, handleLoadTable]);

  const handleViewMemory = useCallback((address: number) => {
    setMemoryViewerAddress(address);
    setShowMemoryViewer(true);
  }, []);

  const handleAddToTable = useCallback((address: string, value: string, valueType: string) => {
    const addr = parseInt(address, 16);
    const numVal = valueType.startsWith("F") ? parseFloat(value) : parseInt(value, 10);
    const scanValue = isNaN(numVal) ? { I32: 0 } : { [valueType]: numVal };
    setAddedEntries((prev) => [
      ...prev,
      { address: addr, label: "", value: scanValue, enabled: false },
    ]);
  }, []);

  const handlePointerScan = useCallback((address: string) => {
    setPointerScanTarget(address);
    setShowPointerScan(true);
  }, []);

  return (
    <div className="app">
      <header className="toolbar">
        <button onClick={() => setShowProcessList(true)}>
          {attachedPid ? `${processName} (${attachedPid})` : "Select Process"}
        </button>
        <button onClick={() => setShowVmDialog(true)} title="VM Scan">
          🖥️ VM
        </button>
        <div className="toolbar-spacer" />
        <button onClick={handleSaveTable} title="Save Table (Ctrl+S)">
          💾 Save
        </button>
        <button onClick={handleLoadTable} title="Load Table (Ctrl+O)">
          📂 Open
        </button>
      </header>

      <div className="main-layout">
        <aside className="sidebar">
          <ScanPanel attached={attachedPid !== null} onValueTypeChange={setCurrentValueType} />
        </aside>
        <main className="content">
          <div className="results-panel">
            <ResultsTable
              attached={attachedPid !== null}
              valueType={currentValueType}
              onViewMemory={handleViewMemory}
              onAddToTable={handleAddToTable}
              onPointerScan={handlePointerScan}
            />
          </div>
          <div className="address-panel">
            <AddressTable externalEntries={addedEntries} />
          </div>
          {showMemoryViewer && (
            <div className="memory-viewer-panel">
              <MemoryViewer
                address={memoryViewerAddress}
                onClose={() => setShowMemoryViewer(false)}
              />
            </div>
          )}
        </main>
      </div>

      {showProcessList && (
        <ProcessList
          onAttach={(pid, name) => {
            setAttachedPid(pid);
            setProcessName(name);
            setShowProcessList(false);
          }}
          onClose={() => setShowProcessList(false)}
        />
      )}

      {showPointerScan && (
        <PointerScanDialog
          targetAddress={pointerScanTarget}
          onClose={() => setShowPointerScan(false)}
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
