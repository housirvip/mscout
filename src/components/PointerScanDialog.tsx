import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useToast } from "./Toast";
import { useI18n } from "../i18n";

interface PointerChain {
  base_address: number;
  offsets: number[];
}

interface ResultRow {
  chain: PointerChain;
  resolved: number | null;
  selected: boolean;
}

interface Props {
  targetAddress: string | null;
  onClose: () => void;
  onAddToTable?: (address: number, label: string) => void;
}

export function PointerScanDialog({ targetAddress, onClose, onAddToTable }: Props) {
  const { t } = useI18n();
  const { showToast } = useToast();
  const [target, setTarget] = useState(targetAddress ?? "");
  const [maxDepth, setMaxDepth] = useState(5);
  const [maxOffset, setMaxOffset] = useState(4096);
  const [results, setResults] = useState<ResultRow[]>([]);
  const [scanning, setScanning] = useState(false);
  const [state, setState] = useState<"idle" | "loading" | "done">("idle");
  const mountedRef = useRef(true);

  useEffect(() => {
    mountedRef.current = true;
    return () => { mountedRef.current = false; };
  }, []);

  useEffect(() => {
    function handleKey(e: KeyboardEvent) {
      if (e.key === "Escape") onClose();
    }
    document.addEventListener("keydown", handleKey);
    return () => document.removeEventListener("keydown", handleKey);
  }, [onClose]);

  async function handleScan() {
    const trimmed = target.replace(/^0x/i, "").trim();
    if (!trimmed) {
      showToast("请输入目标地址", "error");
      return;
    }
    const addr = parseInt(trimmed, 16);
    if (isNaN(addr) || addr <= 0) {
      showToast(t("toast.invalidNumber"), "error");
      return;
    }

    setScanning(true);
    setState("loading");
    setResults([]);

    try {
      const chains = await invoke<PointerChain[]>("pointer_scan", {
        targetAddress: addr,
        maxDepth,
        maxOffset,
      });
      if (!mountedRef.current) return;

      // Resolve each chain
      const rows: ResultRow[] = [];
      for (const chain of chains) {
        let resolved: number | null = null;
        try {
          resolved = await invoke<number | null>("resolve_pointer_cmd", { chain });
        } catch { /* ignore resolution failures */ }
        if (!mountedRef.current) return;
        rows.push({ chain, resolved, selected: false });
      }
      setResults(rows);
      setState("done");
    } catch (e) {
      if (!mountedRef.current) return;
      showToast(t("toast.scanFailed", { error: String(e) }), "error");
      setState("idle");
    } finally {
      if (mountedRef.current) setScanning(false);
    }
  }

  function toggleRow(index: number) {
    setResults((prev) =>
      prev.map((r, i) => (i === index ? { ...r, selected: !r.selected } : r))
    );
  }

  function handleAddSelected() {
    const selected = results.filter((r) => r.selected && r.resolved !== null);
    for (const row of selected) {
      const label = `ptr: ${formatHex(row.chain.base_address)} + [${row.chain.offsets.map((o) => "0x" + o.toString(16)).join(", ")}]`;
      onAddToTable?.(row.resolved!, label);
    }
    if (selected.length > 0) {
      showToast(`已添加 ${selected.length} 条指针链`, "success");
    }
  }

  const hasSelected = results.some((r) => r.selected);

  return (
    <div className="scrim" onClick={onClose}>
      <div
        className="modal"
        role="dialog"
        aria-modal="true"
        onClick={(e) => e.stopPropagation()}
        style={{ width: 720, maxHeight: "85vh" }}
      >
        {/* Header */}
        <div className="modal-head">
          <svg className="icon" aria-hidden="true" viewBox="0 0 24 24" style={{ marginTop: 2 }}>
            <path d="M9.6 14.4 14.4 9.6" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
            <path d="M12.6 7.4l1.6-1.6a3.4 3.4 0 0 1 4.8 4.8l-1.6 1.6" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
            <path d="M11.4 16.6l-1.6 1.6a3.4 3.4 0 0 1-4.8-4.8l1.6-1.6" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
          </svg>
          <div>
            <h2>{t("ptr.title")}</h2>
            <p>为动态地址寻找稳定的指针路径。重启后动态地址失效，指针链能长期复用。</p>
          </div>
          <span className="spacer"></span>
          <button className="btn-close" onClick={onClose}>
            <svg className="icon" aria-hidden="true" viewBox="0 0 24 24">
              <path d="m6 6 12 12M18 6 6 18" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
            </svg>
          </button>
        </div>

        {/* Form */}
        <div style={{ padding: "var(--space-4) var(--space-5)", borderBottom: "1px solid var(--border-soft)" }}>
          <div className="field">
            <label htmlFor="ptr-target">{t("ptr.targetAddr")}</label>
            <input
              id="ptr-target"
              className="control mono"
              value={target}
              onChange={(e) => setTarget(e.target.value)}
              placeholder="0x7FF6A1C4E230"
            />
          </div>
          <div className="two-up" style={{ marginTop: "var(--space-3)" }}>
            <div className="field">
              <label htmlFor="ptr-depth">{t("ptr.maxDepth")}</label>
              <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                <input
                  id="ptr-depth"
                  type="range"
                  min={1}
                  max={7}
                  value={maxDepth}
                  onChange={(e) => setMaxDepth(Number(e.target.value))}
                  style={{ flex: 1 }}
                />
                <output style={{ fontWeight: 700, minWidth: 16 }}>{maxDepth}</output>
              </div>
            </div>
            <div className="field">
              <label htmlFor="ptr-offset">{t("ptr.maxOffset")}</label>
              <input
                id="ptr-offset"
                className="control mono"
                value={maxOffset}
                onChange={(e) => setMaxOffset(Number(e.target.value) || 0)}
                inputMode="numeric"
              />
            </div>
          </div>
          <button
            className="btn btn-primary"
            onClick={handleScan}
            disabled={scanning}
            style={{ marginTop: "var(--space-3)" }}
          >
            {scanning ? "扫描中…" : t("ptr.start")}
          </button>
        </div>

        {/* Hint */}
        <p style={{ padding: "var(--space-2) var(--space-5)", margin: 0, fontSize: 11, color: "var(--muted)" }}>
          深度 {maxDepth} · 偏移 {maxOffset} — 深度越大命中越多，但误报也越多。
        </p>

        {/* Body */}
        <div style={{ flex: "1 1 auto", overflow: "auto", minHeight: 160 }}>
          {state === "idle" && (
            <div className="empty" style={{ minHeight: 200 }}>
              <div className="empty-inner">
                <svg className="icon" aria-hidden="true" viewBox="0 0 24 24" style={{ width: 48, height: 48, color: "var(--muted)" }}>
                  <path d="M9.6 14.4 14.4 9.6" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
                  <path d="M12.6 7.4l1.6-1.6a3.4 3.4 0 0 1 4.8 4.8l-1.6 1.6" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
                  <path d="M11.4 16.6l-1.6 1.6a3.4 3.4 0 0 1-4.8-4.8l1.6-1.6" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
                </svg>
                <h2 style={{ marginTop: 12 }}>尚未开始扫描</h2>
                <p>确认目标地址后点击「开始扫描」。</p>
              </div>
            </div>
          )}

          {state === "loading" && (
            <div className="empty" style={{ minHeight: 200 }}>
              <div className="empty-inner">
                <div className="spin"></div>
                <h2 style={{ marginTop: 12 }}>正在遍历模块指针…</h2>
                <p className="mono" style={{ color: "var(--muted)" }}>深度 1/{maxDepth}</p>
              </div>
            </div>
          )}

          {state === "done" && results.length === 0 && (
            <div className="empty" style={{ minHeight: 160 }}>
              <div className="empty-inner">
                <p>未找到指向该地址的指针链。</p>
              </div>
            </div>
          )}

          {state === "done" && results.length > 0 && (
            <table>
              <thead>
                <tr>
                  <th style={{ width: 44 }}><span className="sr">选择</span></th>
                  <th style={{ width: 180 }}>基地址</th>
                  <th>偏移链</th>
                  <th style={{ width: 150 }}>最终解析地址</th>
                  <th style={{ width: 56 }}>深度</th>
                </tr>
              </thead>
              <tbody>
                {results.map((row, i) => (
                  <tr key={i} className={row.selected ? "sel" : undefined}>
                    <td>
                      <input
                        type="checkbox"
                        checked={row.selected}
                        onChange={() => toggleRow(i)}
                      />
                    </td>
                    <td className="mono">{formatHex(row.chain.base_address)}</td>
                    <td className="mono">
                      {row.chain.offsets.map((o) => "+" + o.toString(16).toUpperCase()).join(" → ")}
                    </td>
                    <td className="mono">
                      {row.resolved !== null ? formatHex(row.resolved) : "—"}
                    </td>
                    <td>{row.chain.offsets.length}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </div>

        {/* Footer */}
        <div className="modal-foot">
          <span style={{ fontSize: 11, color: "var(--muted)" }}>
            目标 <b className="mono">{target || "—"}</b> · {t("ptr.results", { count: results.length })}
          </span>
          <span className="spacer"></span>
          <button className="btn" onClick={onClose}>{t("ptr.cancel")}</button>
          <button
            className="btn btn-solid-dark"
            disabled={!hasSelected}
            onClick={handleAddSelected}
          >
            {t("ptr.addSelected")}
          </button>
        </div>
      </div>
    </div>
  );
}

function formatHex(n: number): string {
  return "0x" + n.toString(16).toUpperCase().padStart(12, "0");
}
