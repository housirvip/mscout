import { useState } from "react";
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

  async function handleScan() {
    if (!targetAddress) return;
    setScanning(true);
    try {
      const addr = parseInt(targetAddress, 16);
      const data = await invoke<PointerChain[]>("pointer_scan", {
        targetAddress: addr,
        maxDepth,
        maxOffset,
      });
      setResults(data);
    } catch (e) {
      showToast(`Pointer scan failed: ${e}`, "error");
    } finally {
      setScanning(false);
    }
  }

  return (
    <div className="modal-overlay" onClick={onClose}>
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
              onChange={(e) => setMaxDepth(Number(e.target.value))}
            />
          </div>
          <div className="scan-section" style={{ flex: 1 }}>
            <label>Max Offset</label>
            <input
              type="number"
              min={0}
              value={maxOffset}
              onChange={(e) => setMaxOffset(Number(e.target.value))}
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
                  <th>Base Address</th>
                  <th>Offsets</th>
                  <th>Resolved</th>
                </tr>
              </thead>
              <tbody>
                {results.map((r, i) => (
                  <tr key={i}>
                    <td style={{ fontFamily: "monospace" }}>{r.base_address}</td>
                    <td style={{ fontFamily: "monospace" }}>
                      {r.offsets.map((o) => `+0x${o.toString(16).toUpperCase()}`).join(" → ")}
                    </td>
                    <td style={{ fontFamily: "monospace" }}>{r.resolved_value ?? "?"}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}

        <div style={{ marginTop: 12, textAlign: "right" }}>
          <button onClick={onClose}>Close</button>
        </div>
      </div>
    </div>
  );
}
