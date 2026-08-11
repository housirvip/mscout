import { useState, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useToast } from "./Toast";

interface PointerChain {
  base_address: string;
  offsets: number[];
  resolved_value: string | null;
}

interface Props {
  targetAddress: string | null;
  onClose: () => void;
}

export function PointerScanDialog({ targetAddress, onClose }: Props) {
  const [maxDepth, setMaxDepth] = useState(5);
  const [maxOffset, setMaxOffset] = useState(4096);
  const [results, setResults] = useState<PointerChain[]>([]);
  const [scanning, setScanning] = useState(false);
  const { showToast } = useToast();
  const mountedRef = useRef(true);

  async function handleScan() {
    if (!targetAddress || targetAddress.trim() === "") {
      showToast("No target address specified", "error");
      return;
    }
    const addr = parseInt(targetAddress, 16);
    if (isNaN(addr)) {
      showToast("Invalid hex address", "error");
      return;
    }
    setScanning(true);
    try {
      const data = await invoke<PointerChain[]>("pointer_scan", {
        targetAddress: addr,
        maxDepth,
        maxOffset,
      });
      if (mountedRef.current) {
        setResults(data);
      }
    } catch (e) {
      if (mountedRef.current) {
        showToast(`Pointer scan failed: ${e}`, "error");
      }
    } finally {
      if (mountedRef.current) {
        setScanning(false);
      }
    }
  }

  // Cleanup on unmount
  const cleanupRef = useRef(false);
  if (!cleanupRef.current) {
    cleanupRef.current = true;
    // We use a trick: capture the ref for unmount detection
  }

  return (
    <div className="modal-overlay" onClick={() => { mountedRef.current = false; onClose(); }}>
      <div className="modal" onClick={(e) => e.stopPropagation()} style={{ width: 700 }}>
        <h2>Pointer Scanner</h2>
        <div className="scan-section">
          <label>Target Address</label>
          <input type="text" value={targetAddress ?? ""} disabled />
        </div>
        <div style={{ display: "flex", gap: 12 }}>
          <div className="scan-section" style={{ flex: 1 }}>
            <label>Max Depth (1-7)</label>
            <input
              type="number"
              min={1}
              max={7}
              value={maxDepth}
              onChange={(e) => setMaxDepth(Math.max(1, Math.min(7, Number(e.target.value))))}
            />
          </div>
          <div className="scan-section" style={{ flex: 1 }}>
            <label>Max Offset</label>
            <input
              type="number"
              min={1}
              max={65536}
              value={maxOffset}
              onChange={(e) => setMaxOffset(Math.max(1, Math.min(65536, Number(e.target.value))))}
            />
          </div>
        </div>
        <button onClick={handleScan} disabled={scanning || !targetAddress}>
          {scanning ? "Scanning..." : "Scan"}
        </button>

        {results.length > 0 && (
          <div style={{ marginTop: 12, maxHeight: 300, overflow: "auto" }}>
            <table>
              <thead>
                <tr>
                  <th>Base</th>
                  <th>Offsets</th>
                  <th>Value</th>
                </tr>
              </thead>
              <tbody>
                {results.map((chain, i) => (
                  <tr key={i}>
                    <td style={{ fontFamily: "monospace" }}>{chain.base_address}</td>
                    <td style={{ fontFamily: "monospace" }}>
                      {chain.offsets.map((o) => `+0x${o.toString(16).toUpperCase()}`).join(" → ")}
                    </td>
                    <td>{chain.resolved_value ?? "?"}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}

        <div style={{ marginTop: 12, textAlign: "right" }}>
          <button onClick={() => { mountedRef.current = false; onClose(); }}>Close</button>
        </div>
      </div>
    </div>
  );
}
