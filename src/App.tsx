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
  const { showToast } = useToast();

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
  }, []);

  async function handleSaveTable() {
    try {
      await invoke("export_table");
      showToast("Table saved", "success");
    } catch (e) {
      showToast(`Save failed: ${e}`, "error");
    }
  }

  async function handleLoadTable() {
    try {
      await invoke("import_table");
      showToast("Table loaded", "success");
    } catch (e) {
      showToast(`Load failed: ${e}`, "error");
    }
  }

  const handleViewMemory = useCallback((address: number) => {
    setMemoryViewerAddress(address);
    setShowMemoryViewer(true);
  }, []);

  const handleAddToTable = useCallback((address: string, value: string, valueType: string) => {
    setAddedEntries((prev) => [
      ...prev,
      { address, label: "", value_type: valueType, value, frozen: false },
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
          <ScanPanel attached={attachedPid !== null} />
        </aside>
        <main className="content">
          <div className="results-panel">
            <ResultsTable
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
