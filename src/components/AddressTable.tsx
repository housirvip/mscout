import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useToast } from "./Toast";

export interface AddressEntry {
  address: string;
  label: string;
  value_type: string;
  value: string;
  frozen: boolean;
}

interface Props {
  externalEntries?: AddressEntry[];
}

export function AddressTable({ externalEntries }: Props) {
  const [entries, setEntries] = useState<AddressEntry[]>([]);
  const [editingLabel, setEditingLabel] = useState<string | null>(null);
  const [labelDraft, setLabelDraft] = useState("");
  const { showToast } = useToast();

  // Live refresh: poll values every 500ms
  const refreshValues = useCallback(async () => {
    try {
      const data = await invoke<AddressEntry[]>("list_frozen");
      setEntries(data);
    } catch {
      // ignore
    }
  }, []);

  useEffect(() => {
    refreshValues();
    const interval = setInterval(refreshValues, 500);
    return () => clearInterval(interval);
  }, [refreshValues]);

  // Merge external entries added from ResultsTable context menu
  useEffect(() => {
    if (externalEntries && externalEntries.length > 0) {
      setEntries((prev) => {
        const existing = new Set(prev.map((e) => e.address));
        const newOnes = externalEntries.filter((e) => !existing.has(e.address));
        return [...prev, ...newOnes];
      });
    }
  }, [externalEntries]);

  async function toggleFreeze(address: string) {
    try {
      await invoke("toggle_frozen", { address });
    } catch (e) {
      showToast(`Failed to toggle freeze: ${e}`, "error");
    }
  }

  async function handleEditValue(address: string, currentValue: string) {
    const newValue = prompt("Enter new value:", currentValue);
    if (newValue === null) return;
    try {
      await invoke("write_value", { address, value: newValue });
    } catch (e) {
      showToast(`Write failed: ${e}`, "error");
    }
  }

  function startEditLabel(address: string, current: string) {
    setEditingLabel(address);
    setLabelDraft(current);
  }

  function commitLabel() {
    if (editingLabel) {
      setEntries((prev) =>
        prev.map((e) =>
          e.address === editingLabel ? { ...e, label: labelDraft } : e
        )
      );
    }
    setEditingLabel(null);
  }

  return (
    <div>
      <table>
        <thead>
          <tr>
            <th style={{ width: 30 }}>❄️</th>
            <th>Description</th>
            <th>Address</th>
            <th>Type</th>
            <th>Value</th>
          </tr>
        </thead>
        <tbody>
          {entries.map((entry) => (
            <tr key={entry.address}>
              <td>
                <input
                  type="checkbox"
                  checked={entry.frozen}
                  onChange={() => toggleFreeze(entry.address)}
                />
              </td>
              <td onDoubleClick={() => startEditLabel(entry.address, entry.label)}>
                {editingLabel === entry.address ? (
                  <input
                    type="text"
                    value={labelDraft}
                    onChange={(e) => setLabelDraft(e.target.value)}
                    onBlur={commitLabel}
                    onKeyDown={(e) => e.key === "Enter" && commitLabel()}
                    autoFocus
                    style={{ width: "100%" }}
                  />
                ) : (
                  entry.label || "(double-click to name)"
                )}
              </td>
              <td style={{ fontFamily: "monospace" }}>{entry.address}</td>
              <td>{entry.value_type}</td>
              <td
                style={{ fontFamily: "monospace", cursor: "pointer" }}
                onDoubleClick={() => handleEditValue(entry.address, entry.value)}
              >
                {entry.value}
              </td>
            </tr>
          ))}
          {entries.length === 0 && (
            <tr>
              <td
                colSpan={5}
                style={{ textAlign: "center", color: "var(--text-secondary)", padding: 12 }}
              >
                Address table is empty. Add addresses from scan results.
              </td>
            </tr>
          )}
        </tbody>
      </table>
    </div>
  );
}
