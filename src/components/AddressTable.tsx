import { useState, useEffect, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useI18n, MessageKey } from "../i18n";
import { useToast } from "./Toast";

export interface AddressEntry {
  address: number;
  label: string;
  value: unknown;
  enabled: boolean;
}

interface FrozenEntry {
  address: number;
  value: Record<string, unknown>;
  enabled: boolean;
  label: string;
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

function getValueType(val: unknown): string {
  if (val === null || val === undefined) return "I32";
  if (typeof val === "object") {
    const keys = Object.keys(val as Record<string, unknown>);
    if (keys.length === 1) return keys[0];
  }
  return "I32";
}

export function AddressTable({ externalEntries }: Props) {
  const { t } = useI18n();
  const { showToast } = useToast();

  const [entries, setEntries] = useState<FrozenEntry[]>([]);
  const [editingLabel, setEditingLabel] = useState<number | null>(null);
  const [labelDraft, setLabelDraft] = useState("");
  const [editingValue, setEditingValue] = useState<number | null>(null);
  const [valueDraft, setValueDraft] = useState("");
  const localLabelsRef = useRef<Map<number, string>>(new Map());
  const failCountRef = useRef(0);
  const pollRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const mountedRef = useRef(true);

  useEffect(() => {
    mountedRef.current = true;
    return () => { mountedRef.current = false; };
  }, []);

  const refreshValues = useCallback(async () => {
    try {
      const data = await invoke<FrozenEntry[]>("list_frozen");
      if (mountedRef.current) {
        setEntries(data.map((e) => ({
          ...e,
          label: localLabelsRef.current.get(e.address) ?? e.label,
        })));
        failCountRef.current = 0;
      }
    } catch {
      failCountRef.current++;
    }
  }, []);

  // Poll with setTimeout to avoid overlapping
  useEffect(() => {
    let active = true;
    async function poll() {
      await refreshValues();
      if (active) {
        const delay = failCountRef.current >= 3 ? 5000 : 500;
        pollRef.current = setTimeout(poll, delay);
      }
    }
    poll();
    return () => {
      active = false;
      clearTimeout(pollRef.current!);
    };
  }, [refreshValues]);

  // Persist external entries to backend
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
          showToast(t("toast.writeFailed", { error: String(e) }), "error");
        }
      }
      refreshValues();
    };
    addToBackend();
  }, [externalEntries, showToast, refreshValues, t]);

  async function toggleFreeze(address: number, currentEnabled: boolean) {
    try {
      await invoke("toggle_frozen", { address, enabled: !currentEnabled });
      refreshValues();
    } catch (e) {
      showToast(t("toast.writeFailed", { error: String(e) }), "error");
    }
  }

  async function handleDelete(address: number) {
    try {
      await invoke("remove_frozen", { address });
      refreshValues();
    } catch (e) {
      showToast(t("toast.writeFailed", { error: String(e) }), "error");
    }
  }

  function startEditLabel(address: number, current: string) {
    setEditingLabel(address);
    setLabelDraft(current);
  }

  function commitLabel() {
    if (editingLabel !== null) {
      localLabelsRef.current.set(editingLabel, labelDraft);
      setEntries((prev) =>
        prev.map((e) =>
          e.address === editingLabel ? { ...e, label: labelDraft } : e
        )
      );
    }
    setEditingLabel(null);
  }

  function startEditValue(address: number, currentValue: unknown) {
    setEditingValue(address);
    setValueDraft(formatValue(currentValue));
  }

  async function commitValue(address: number, currentValue: unknown) {
    setEditingValue(null);
    const vtype = getValueType(currentValue);
    const parsed = vtype.startsWith("F") ? parseFloat(valueDraft) : parseInt(valueDraft, 10);
    if (isNaN(parsed)) {
      showToast(t("toast.invalidNumber"), "error");
      return;
    }
    try {
      const value = { [vtype]: parsed };
      await invoke("write_at", { address, value });
      refreshValues();
    } catch (e) {
      showToast(t("toast.writeFailed", { error: String(e) }), "error");
    }
  }

  return (
    <section className="sec">
      <div className="sec-head">
        <h2>{t("addr.title")}</h2>
        <span className="count">{t("addr.count", { count: entries.length })}</span>
        <span className="spacer" />
        <span className="count">
          <span className="pulse" />
          {t("addr.refreshHint")}
        </span>
      </div>

      <div className="tbl-wrap">
        <table>
          <thead>
            <tr>
              <th style={{ width: 52 }}>{t("addr.colFreeze")}</th>
              <th>{t("addr.colDesc")}</th>
              <th style={{ width: 176 }}>{t("addr.colAddr")}</th>
              <th style={{ width: 92 }}>{t("addr.colType")}</th>
              <th style={{ width: 128 }}>{t("addr.colValue")}</th>
              <th style={{ width: 40 }}>
                <span className="sr">{t("addr.colAction" as MessageKey)}</span>
              </th>
            </tr>
          </thead>
          <tbody>
            {entries.map((entry) => (
              <tr key={entry.address} className={entry.enabled ? "frozen" : ""}>
                <td>
                  <input
                    type="checkbox"
                    className="cb"
                    checked={entry.enabled}
                    onChange={() => toggleFreeze(entry.address, entry.enabled)}
                  />
                </td>
                <td onDoubleClick={() => startEditLabel(entry.address, entry.label)}>
                  {editingLabel === entry.address ? (
                    <input
                      className="edit-in"
                      value={labelDraft}
                      onChange={(e) => setLabelDraft(e.target.value)}
                      onBlur={commitLabel}
                      onKeyDown={(e) => {
                        if (e.key === "Enter") commitLabel();
                        if (e.key === "Escape") setEditingLabel(null);
                      }}
                      autoFocus
                    />
                  ) : (
                    entry.label || "—"
                  )}
                </td>
                <td className="mono">{formatAddress(entry.address)}</td>
                <td>
                  <span className="type-tag">{getValueType(entry.value)}</span>
                </td>
                <td onDoubleClick={() => startEditValue(entry.address, entry.value)}>
                  {editingValue === entry.address ? (
                    <input
                      className="edit-in mono"
                      value={valueDraft}
                      onChange={(e) => setValueDraft(e.target.value)}
                      onBlur={() => commitValue(entry.address, entry.value)}
                      onKeyDown={(e) => {
                        if (e.key === "Enter") e.currentTarget.blur();
                        if (e.key === "Escape") setEditingValue(null);
                      }}
                      autoFocus
                    />
                  ) : (
                    <span className="mono">{formatValue(entry.value)}</span>
                  )}
                </td>
                <td>
                  <button
                    className="row-del"
                    onClick={() => handleDelete(entry.address)}
                    title={t("addr.deleteTitle" as MessageKey)}
                  >
                    <svg className="icon" style={{ width: 14, height: 14 }} viewBox="0 0 24 24" aria-hidden="true">
                      <line x1="18" y1="6" x2="6" y2="18" />
                      <line x1="6" y1="6" x2="18" y2="18" />
                    </svg>
                  </button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>

        {entries.length === 0 && (
          <div className="empty" style={{ minHeight: 120 }}>
            <div className="empty-inner">
              <p>{t("addr.emptyHint")}</p>
            </div>
          </div>
        )}
      </div>
    </section>
  );
}
