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
    if (address === null) return;
    try {
      const result = await invoke<{ address: number; bytes: number[] }>("read_at", {
        address: baseAddress,
        size: SIZE,
      });
      setBytes(result.bytes);
    } catch (e) {
      console.warn("Failed to read memory:", e);
      setBytes([]);
    }
  }, [baseAddress, address, SIZE]);

  useEffect(() => {
    fetchMemory();
  }, [fetchMemory]);

  useEffect(() => {
    if (address === null) return;
    let mounted = true;
    let timeoutId: number;
    const poll = async () => {
      await fetchMemory();
      if (mounted) {
        timeoutId = window.setTimeout(poll, 1000);
      }
    };
    timeoutId = window.setTimeout(poll, 1000);
    return () => {
      mounted = false;
      clearTimeout(timeoutId);
    };
  }, [fetchMemory, address]);

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
    const rowBytes = bytes.slice(i * COLS, (i + 1) * COLS);
    const addr = baseAddress + i * COLS;
    rows.push(
      <div key={i} className="hex-row">
        <span className="hex-addr">
          {addr.toString(16).toUpperCase().padStart(8, "0")}
        </span>
        <span className="hex-bytes">
          {rowBytes.map((b, j) => (
            <span key={j}>{b.toString(16).toUpperCase().padStart(2, "0")} </span>
          ))}
        </span>
        <span className="hex-ascii">
          {rowBytes.map((b) => (b >= 32 && b <= 126 ? String.fromCharCode(b) : ".")).join("")}
        </span>
      </div>
    );
  }

  return (
    <div className="memory-viewer">
      <div className="memory-viewer-header">
        <span>
          Memory @ 0x{baseAddress.toString(16).toUpperCase().padStart(8, "0")}
        </span>
        <div style={{ display: "flex", gap: 4, alignItems: "center" }}>
          <input
            type="text"
            placeholder="Go to address..."
            value={gotoInput}
            onChange={(e) => setGotoInput(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && handleGoto()}
            style={{ width: 120 }}
          />
          <button onClick={handleGoto}>Go</button>
          <button onClick={onClose}>×</button>
        </div>
      </div>
      <div className="memory-viewer-content">
        <div className="hex-row hex-header">
          <span className="hex-addr">Address</span>
          <span className="hex-bytes">
            {Array.from({ length: COLS }, (_, i) =>
              i.toString(16).toUpperCase().padStart(2, "0")
            ).join(" ")}{" "}
          </span>
          <span className="hex-ascii">ASCII</span>
        </div>
        {rows}
      </div>
    </div>
  );
}
