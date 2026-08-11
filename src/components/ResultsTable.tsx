import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ContextMenu, MenuItem } from "./ContextMenu";

interface ScanResult {
  address: string;
  value: string;
  previous_value: string;
}

interface Props {
  onViewMemory?: (address: number) => void;
  onAddToTable?: (address: string, value: string, valueType: string) => void;
  onPointerScan?: (address: string) => void;
}

export function ResultsTable({ onViewMemory, onAddToTable, onPointerScan }: Props) {
  const [results, setResults] = useState<ScanResult[]>([]);
  const [offset, setOffset] = useState(0);
  const [contextMenu, setContextMenu] = useState<{
    x: number;
    y: number;
    row: ScanResult;
  } | null>(null);
  const pageSize = 100;

  const fetchResults = useCallback(async () => {
    try {
      const data = await invoke<ScanResult[]>("get_scan_results", {
        offset,
        count: pageSize,
      });
      setResults(data);
    } catch {
      // No active scan session
    }
  }, [offset]);

  useEffect(() => {
    fetchResults();
    const interval = setInterval(fetchResults, 1000);
    return () => clearInterval(interval);
  }, [fetchResults]);

  function handleContextMenu(e: React.MouseEvent, row: ScanResult) {
    e.preventDefault();
    setContextMenu({ x: e.clientX, y: e.clientY, row });
  }

  function getMenuItems(row: ScanResult): MenuItem[] {
    return [
      {
        label: "Add to Address Table",
        onClick: () => onAddToTable?.(row.address, row.value, "I32"),
      },
      {
        label: "View in Memory",
        onClick: () => {
          const addr = parseInt(row.address, 16);
          onViewMemory?.(addr);
        },
      },
      {
        label: "Pointer Scan for Address",
        onClick: () => onPointerScan?.(row.address),
      },
      {
        label: "Copy Address",
        onClick: () => {
          navigator.clipboard.writeText(row.address);
        },
      },
    ];
  }

  return (
    <div>
      <table>
        <thead>
          <tr>
            <th>Address</th>
            <th>Value</th>
            <th>Previous</th>
          </tr>
        </thead>
        <tbody>
          {results.map((r) => (
            <tr
              key={r.address}
              onContextMenu={(e) => handleContextMenu(e, r)}
            >
              <td style={{ fontFamily: "monospace" }}>{r.address}</td>
              <td>{r.value}</td>
              <td style={{ color: "var(--text-secondary)" }}>{r.previous_value}</td>
            </tr>
          ))}
          {results.length === 0 && (
            <tr>
              <td
                colSpan={3}
                style={{ textAlign: "center", color: "var(--text-secondary)", padding: 24 }}
              >
                No results. Run a scan to find addresses.
              </td>
            </tr>
          )}
        </tbody>
      </table>
      {results.length === pageSize && (
        <div style={{ padding: 8, display: "flex", gap: 8, justifyContent: "center" }}>
          <button disabled={offset === 0} onClick={() => setOffset(Math.max(0, offset - pageSize))}>
            ← Prev
          </button>
          <button onClick={() => setOffset(offset + pageSize)}>Next →</button>
        </div>
      )}

      {contextMenu && (
        <ContextMenu
          x={contextMenu.x}
          y={contextMenu.y}
          items={getMenuItems(contextMenu.row)}
          onClose={() => setContextMenu(null)}
        />
      )}
    </div>
  );
}
