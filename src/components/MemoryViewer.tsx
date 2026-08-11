import { useState, useEffect, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useI18n } from "../i18n";

interface Props {
  address: number | null;
  open: boolean;
  onClose: () => void;
}

export function MemoryViewer({ address, open, onClose }: Props) {
  const { t } = useI18n();
  const [bytes, setBytes] = useState<number[]>([]);
  const [baseAddress, setBaseAddress] = useState(address ?? 0);
  const [gotoInput, setGotoInput] = useState("");
  const mountedRef = useRef(true);
  const timerRef = useRef<number | null>(null);
  const ROWS = 16;
  const COLS = 16;
  const SIZE = ROWS * COLS;

  // Sync base address when prop changes
  useEffect(() => {
    if (address !== null) {
      setBaseAddress(address);
      setGotoInput(address.toString(16).toUpperCase());
    }
  }, [address]);

  const fetchMemory = useCallback(async () => {
    if (!open || address === null) return;
    try {
      const result = await invoke<{ address: number; bytes: number[] }>("read_at", {
        address: baseAddress,
        size: SIZE,
      });
      if (mountedRef.current) {
        setBytes(result.bytes);
      }
    } catch {
      // Silently handle read failures during polling
    }
  }, [baseAddress, open, address, SIZE]);

  // Initial fetch + polling
  useEffect(() => {
    mountedRef.current = true;
    if (!open || address === null) return;

    fetchMemory();

    const schedulePoll = () => {
      timerRef.current = window.setTimeout(async () => {
        if (!mountedRef.current) return;
        await fetchMemory();
        if (mountedRef.current) schedulePoll();
      }, 1000);
    };
    schedulePoll();

    return () => {
      mountedRef.current = false;
      if (timerRef.current !== null) {
        clearTimeout(timerRef.current);
        timerRef.current = null;
      }
    };
  }, [fetchMemory, open, address]);

  // Escape key
  useEffect(() => {
    if (!open) return;
    function handleKey(e: KeyboardEvent) {
      if (e.key === "Escape") onClose();
    }
    document.addEventListener("keydown", handleKey);
    return () => document.removeEventListener("keydown", handleKey);
  }, [open, onClose]);

  function handleGoto() {
    const parsed = parseInt(gotoInput, 16);
    if (!isNaN(parsed)) {
      setBaseAddress(parsed);
    }
  }

  function handleNav(direction: number) {
    setBaseAddress((prev) => Math.max(0, prev + direction * 0x100));
  }

  // Render hex rows
  const rows: React.ReactNode[] = [];
  for (let i = 0; i < ROWS; i++) {
    const offset = baseAddress + i * COLS;
    const rowBytes = bytes.slice(i * COLS, (i + 1) * COLS);

    const hexParts = rowBytes.map((b) => b.toString(16).toUpperCase().padStart(2, "0")).join(" ");
    const asciiParts = rowBytes
      .map((b) => (b >= 0x20 && b < 0x7f ? String.fromCharCode(b) : "."))
      .join("");

    rows.push(
      <div key={i} className="hex-row">
        <span className="off">{offset.toString(16).toUpperCase().padStart(12, "0")}</span>
        <span>{hexParts || ""}</span>
        <span className="ascii">{asciiParts || ""}</span>
      </div>
    );
  }

  return (
    <aside className={`drawer${open ? " open" : ""}`} aria-hidden={!open}>
      {/* Header */}
      <div className="drawer-head">
        <svg className="icon" aria-hidden="true" viewBox="0 0 24 24">
          <rect x="3" y="3" width="7" height="7" rx="1" fill="none" stroke="currentColor" strokeWidth="1.5" />
          <rect x="14" y="3" width="7" height="7" rx="1" fill="none" stroke="currentColor" strokeWidth="1.5" />
          <rect x="3" y="14" width="7" height="7" rx="1" fill="none" stroke="currentColor" strokeWidth="1.5" />
          <rect x="14" y="14" width="7" height="7" rx="1" fill="none" stroke="currentColor" strokeWidth="1.5" />
        </svg>
        <h2>{t("hex.title")}</h2>
        <span className="spacer"></span>
        <button className="btn-close" onClick={onClose}>
          <svg className="icon" aria-hidden="true" viewBox="0 0 24 24">
            <path d="m6 6 12 12M18 6 6 18" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
          </svg>
        </button>
      </div>

      {/* Address bar */}
      <div className="drawer-bar">
        <input
          className="control addr-in mono"
          value={gotoInput}
          onChange={(e) => setGotoInput(e.target.value)}
          onKeyDown={(e) => { if (e.key === "Enter") handleGoto(); }}
          aria-label={t("hex.goto")}
        />
        <button className="btn" onClick={handleGoto} style={{ height: 28, fontSize: 11 }}>
          跳转
        </button>
        <button
          className="btn btn-icon"
          onClick={() => handleNav(-1)}
          title="向上 0x100"
          style={{ height: 28, width: 28 }}
        >
          ↑
        </button>
        <button
          className="btn btn-icon"
          onClick={() => handleNav(1)}
          title="向下 0x100"
          style={{ height: 28, width: 28 }}
        >
          ↓
        </button>
      </div>

      {/* Hex grid */}
      <div className="hex">
        {bytes.length > 0 ? rows : (
          <div className="empty" style={{ minHeight: 160 }}>
            <div className="empty-inner"><p>无数据</p></div>
          </div>
        )}
      </div>

      {/* Footer */}
      <div className="drawer-foot">
        <span><span className="pulse"></span>每 1s 自动刷新</span>
        <span className="mono">
          {baseAddress.toString(16).toUpperCase().padStart(12, "0")}–
          {(baseAddress + SIZE - 1).toString(16).toUpperCase().padStart(12, "0")}
        </span>
      </div>
    </aside>
  );
}
