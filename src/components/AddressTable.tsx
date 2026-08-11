import { useState, useEffect, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useToast } from "./Toast";

export interface AddressEntry {
  address: number;
  label: string;
  value: unknown;
  enabled: boolean;
}

interface Props {
  externalEntries?: AddressEntry[];
}

function formatAddress(addr: number): string {
  return addr.toString(16).toUpperCase().padStart(8, "0");
}

function formatValue(val: unknown): string {
  if (val === null || val === undefined) return "";
  if (typeof val === "object") {
    const entries = Object.entries(val as Record<string, unknown>);
    if (entries.length === 1) return String(entries[0][1]);
  }
  return String(val);
}

export function getValueType(val: unknown): string {
  if (val === null || val === undefined) return "I32";
  if (typeof val === "object") {
    const keys = Object.keys(val as Record<string, unknown>);
    if (keys.length === 1) return keys[0];
  }
  return "I32";
}

export function AddressTable({ externalEntries }: Props) {
  const [entries, setEntries] = useState<AddressEntry[]>([]);
  const [editingLabel, setEditingLabel] = useState<number | null>(null);
  const [labelDraft, setLabelDraft] = useState("");
  const { showToast } = useToast();
  const failCountRef = useRef(0);

  const refreshValues = useCallback(async () => {
    try {
      const data = await invoke<AddressEntry[]>("list_frozen");
      setEntries(data);
      failCountRef.current = 0;
    } catch (e) {
      failCountRef.current++;
      if (failCountRef.current >= 3) {
        showToast("Lost connection to process", "error");
      }
    }
  }, [showToast]);

  useEffect(() => {
    refreshValues();
    const interval = setInterval(refreshValues, 500);
    return () => clearInterval(interval);
  }, [refreshValues]);

  // Persist external entries to backend so polling doesn't overwrite them
  useEffect(() => {
    if (!externalEntries || externalEntries.length === 0) return;
    const addToBackend = async () => {
      for (const entry of externalEntries) {
        try {
          await invoke("add_frozen", {
            address: entry.address,
            value: entry.value,
            label: entry.label,
          });
        } catch (e) {
          showToast(`Failed to add: ${e}`, "error");
        }
      }
      refreshValues();
    };
    addToBackend();
  }, [externalEntries, showToast, refreshValues]);

  async function toggleFreeze(address: number, currentEnabled: boolean) {
    try {
      await invoke("toggle_frozen", { address, enabled: !currentEnabled });
    } catch (e) {
      showToast(`Failed to toggle freeze: ${e}`, "error");
    }
  }

  async function handleEditValue(address: number, currentValue: unknown) {
    const display = formatValue(currentValue);
    const newValue = prompt("Enter new value:", display);
    if (newValue === null || newValue.trim() === "") return;
    try {
      const vtype = getValueType(currentValue);
      const parsed = vtype.startsWith("F") ? parseFloat(newValue) : parseInt(newValue, 10);
      if (isNaN(parsed)) {
        showToast("Invalid number", "error");
        return;
      }
      const value = { [vtype]: parsed };
      await invoke("write_at", { address, value });
    } catch (e) {
      showToast(`Write failed: ${e}`, "error");
    }
  }

  function startEditLabel(address: number, current: string) {
    setEditingLabel(address);
    setLabelDraft(current);
  }

  function commitLabel() {
    if (editingLabel !== null) {
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
                  checked={entry.enabled}
                  onChange={() => toggleFreeze(entry.address, entry.enabled)}
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
              <td style={{ fontFamily: "monospace" }}>{formatAddress(entry.address)}</td>
              <td>{getValueType(entry.value)}</td>
              <td
                style={{ fontFamily: "monospace", cursor: "pointer" }}
                onDoubleClick={() => handleEditValue(entry.address, entry.value)}
              >
                {formatValue(entry.value)}
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
