import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

interface Props {
  address: number | null;
  onClose: () => void;
}

export function MemoryViewer({ address, onClose }: Props) {
  const [bytes, setBytes] = useState<number[]>([]);
  const [baseAddress, setBaseAddress] = useState(address ?? 0);
  const [gotoInput, setGotoInput] = useState("");
  const ROWS = 16;
  const COLS = 16;
  const SIZE = ROWS * COLS;

  const fetchMemory = useCallback(async () => {
    if (baseAddress === 0) return;
    try {
      const result = await invoke<{ address: number; bytes: number[] }>("read_at", {
        address: baseAddress,
        size: SIZE,
      });
      setBytes(result.bytes);
    } catch {
      setBytes([]);
    }
  }, [baseAddress]);

  useEffect(() => {
    fetchMemory();
  }, [fetchMemory]);

  useEffect(() => {
    const interval = setInterval(fetchMemory, 1000);
    return () => clearInterval(interval);
  }, [fetchMemory]);

  useEffect(() => {
    if (address !== null) {
      setBaseAddress(address);
    }
  }, [address]);

  function handleGoto() {
    const parsed = parseInt(gotoInput, 16);
    if (!isNaN(parsed)) {
      setBaseAddress(parsed);
      setGotoInput("");
    }
  }

  const rows = [];
  for (let i = 0; i < ROWS; i++) {
    const offset = i * COLS;
    const rowBytes = bytes.slice(offset, offset + COLS);
    const addr = (baseAddress + offset).toString(16).padStart(12, "0").toUpperCase();
    const hex = rowBytes
      .map((b) => b.toString(16).padStart(2, "0").toUpperCase())
      .join(" ");
    const ascii = rowBytes
      .map((b) => (b >= 32 && b <= 126 ? String.fromCharCode(b) : "."))
      .join("");
    rows.push({ addr, hex, ascii });
  }

  return (
    <div className="memory-viewer">
      <div className="memory-viewer-header">
        <span>Memory Viewer</span>
        <div style={{ display: "flex", gap: 4, alignItems: "center" }}>
          <input
            type="text"
            className="goto-input"
            value={gotoInput}
            onChange={(e) => setGotoInput(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && handleGoto()}
            placeholder="Go to address..."
            style={{ width: 120, fontSize: 11 }}
          />
          <button onClick={() => setBaseAddress(Math.max(0, baseAddress - SIZE))}>↑</button>
          <button onClick={() => setBaseAddress(baseAddress + SIZE)}>↓</button>
          <button onClick={onClose}>✕</button>
        </div>
      </div>
      <div className="memory-viewer-content">
        <div className="hex-row hex-header">
          <span className="hex-addr">Address</span>
          <span className="hex-bytes">
            {Array.from({ length: COLS }, (_, i) =>
              i.toString(16).toUpperCase().padStart(2, "0")
            ).join(" ")}
          </span>
          <span className="hex-ascii">ASCII</span>
        </div>
        {rows.map((row, i) => (
          <div key={i} className="hex-row">
            <span className="hex-addr">{row.addr}</span>
            <span className="hex-bytes">{row.hex}</span>
            <span className="hex-ascii">{row.ascii}</span>
          </div>
        ))}
        {bytes.length === 0 && (
          <div style={{ padding: 16, textAlign: "center", color: "var(--text-secondary)" }}>
            No data. Select an address to view memory.
          </div>
        )}
      </div>
    </div>
  );
}
