import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useI18n } from "../i18n";

interface MemoryRegion {
  base: number;
  size: number;
  readable: boolean;
  writable: boolean;
  executable: boolean;
  info: string;
}

interface Props {
  open: boolean;
  onClose: () => void;
  onViewMemory: (address: number) => void;
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(0)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

function protString(r: MemoryRegion): string {
  return `${r.readable ? "r" : "-"}${r.writable ? "w" : "-"}${r.executable ? "x" : "-"}`;
}

function protClass(r: MemoryRegion): string {
  if (r.writable) return "prot-rw";
  if (r.executable) return "prot-rx";
  return "prot-ro";
}

function barColor(r: MemoryRegion): string {
  if (r.writable) return "var(--success)";
  if (r.executable) return "var(--accent)";
  if (r.readable) return "var(--border)";
  return "var(--surface-warm)";
}

/** Format address as hex string — safe for display even if precision is lost */
function fmtAddr(addr: number): string {
  if (addr <= Number.MAX_SAFE_INTEGER) return addr.toString(16);
  // Fallback: shouldn't happen on current hardware (48-bit user space)
  return addr.toString(16);
}

export function RegionViewer({ open, onClose, onViewMemory }: Props) {
  const { t } = useI18n();
  const [regions, setRegions] = useState<MemoryRegion[]>([]);
  const [loading, setLoading] = useState(false);
  const [filterWritable, setFilterWritable] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const mountedRef = useRef(true);

  useEffect(() => {
    mountedRef.current = true;
    return () => { mountedRef.current = false; };
  }, []);

  async function fetchRegions() {
    setLoading(true);
    setError(null);
    try {
      const data = await invoke<MemoryRegion[]>("list_regions");
      if (mountedRef.current) {
        setRegions(data);
        if (data.length === 0) {
          setError(t("region.noRegions"));
        }
      }
    } catch (e) {
      if (mountedRef.current) setError(String(e));
    } finally {
      if (mountedRef.current) setLoading(false);
    }
  }

  useEffect(() => {
    if (open) fetchRegions();
  }, [open]);

  useEffect(() => {
    function handleKey(e: KeyboardEvent) {
      if (e.key === "Escape") onClose();
    }
    if (open) {
      document.addEventListener("keydown", handleKey);
      return () => document.removeEventListener("keydown", handleKey);
    }
  }, [open, onClose]);

  const displayed = filterWritable ? regions.filter((r) => r.writable) : regions;
  const displayedTotalSize = displayed.reduce((sum, r) => sum + r.size, 0);

  function renderContent() {
    if (loading) {
      return (
        <div className="empty" style={{ flex: 1 }}>
          <div className="empty-inner"><p>{t("region.loading")}</p></div>
        </div>
      );
    }

    if (displayed.length === 0) {
      return (
        <div className="empty" style={{ flex: 1 }}>
          <div className="empty-inner">
            <p>{error || t("region.noProcess")}</p>
            {error && <p style={{ fontSize: 11, color: "var(--muted)", marginTop: 6 }}>{t("region.sudoHint")}</p>}
          </div>
        </div>
      );
    }

    return (
      <div className="tbl-wrap">
        <table>
          <thead>
            <tr>
              <th style={{ width: 160 }}>{t("region.colBase")}</th>
              <th style={{ width: 90 }}>{t("region.colSize")}</th>
              <th style={{ width: 60 }}>{t("region.colProt")}</th>
              <th>{t("region.colInfo")}</th>
            </tr>
          </thead>
          <tbody>
            {displayed.map((r) => (
              <tr
                key={r.base}
                onClick={() => onViewMemory(r.base)}
                style={{ cursor: "pointer" }}
              >
                <td className="mono">0x{fmtAddr(r.base)}</td>
                <td className="num">{formatSize(r.size)}</td>
                <td>
                  <span className={`prot-badge ${protClass(r)}`}>{protString(r)}</span>
                </td>
                <td className="dim" style={{ maxWidth: 200, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                  {r.info || "—"}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    );
  }

  return (
    <aside className="drawer" style={open ? { transform: "none" } : undefined}>
      <div className="drawer-head">
        <svg className="icon" aria-hidden="true" viewBox="0 0 24 24">
          <rect x="3" y="3" width="18" height="18" rx="2" fill="none" stroke="currentColor" strokeWidth="1.5" />
          <line x1="3" y1="9" x2="21" y2="9" stroke="currentColor" strokeWidth="1.5" />
          <line x1="3" y1="15" x2="21" y2="15" stroke="currentColor" strokeWidth="1.5" />
        </svg>
        <h2>{t("region.title")}</h2>
        <span className="count">{t("region.total", { count: displayed.length })}</span>
        <span className="spacer" />
        <button className="btn btn-icon" onClick={onClose}>
          <svg className="icon" aria-hidden="true" viewBox="0 0 24 24">
            <path d="m6 6 12 12M18 6 6 18" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
          </svg>
        </button>
      </div>

      {/* Toolbar */}
      <div className="drawer-bar">
        <label style={{ display: "flex", alignItems: "center", gap: 6, fontSize: 11, fontWeight: 600, cursor: "pointer" }}>
          <input
            type="checkbox"
            checked={filterWritable}
            onChange={() => setFilterWritable(!filterWritable)}
          />
          {t("region.filterWritable")}
        </label>
        <span className="spacer" />
        <button className="btn btn-icon" onClick={fetchRegions} title={t("region.refresh")}>
          <svg className="icon" aria-hidden="true" viewBox="0 0 24 24">
            <polyline points="1 4 1 10 7 10" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />
            <path d="M3.51 15a9 9 0 1 0 2.13-9.36L1 10" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />
          </svg>
        </button>
      </div>

      {/* Memory bar — sizes relative to displayed subset */}
      {displayedTotalSize > 0 && (
        <div className="memory-bar">
          {displayed.map((r) => (
            <span
              key={r.base}
              style={{
                flex: "none",
                width: `${Math.max(0.15, (r.size / displayedTotalSize) * 100)}%`,
                minWidth: 2,
                background: barColor(r),
              }}
              title={`0x${fmtAddr(r.base)} — ${formatSize(r.size)} ${protString(r)}${r.info ? ` ${r.info}` : ""}`}
            />
          ))}
        </div>
      )}

      {renderContent()}
    </aside>
  );
}
