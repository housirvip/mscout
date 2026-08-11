import { useState, useEffect, useRef } from "react";
import { invoke, Channel } from "@tauri-apps/api/core";
import { useToast } from "./Toast";

interface Props {
  attached: boolean;
  onFirstScan?: () => void;
  onNextScan?: () => void;
  onUndo?: () => void;
  onValueTypeChange?: (vt: string) => void;
}

interface ScanProgress {
  scanned_bytes: number;
  total_bytes: number;
  regions_done: number;
  total_regions: number;
}

interface RegionFilter {
  writable_only: boolean;
  skip_executable: boolean;
  skip_mapped_files: boolean;
}

function parseAobPattern(input: string): (number | null)[] | null {
  const tokens = input.trim().split(/\s+/);
  if (tokens.length === 0 || (tokens.length === 1 && tokens[0] === "")) return null;
  const result: (number | null)[] = [];
  for (const t of tokens) {
    if (t === "??" || t === "?") {
      result.push(null);
    } else {
      const val = parseInt(t, 16);
      if (isNaN(val) || val < 0 || val > 255) return null;
      result.push(val);
    }
  }
  return result;
}

export function ScanPanel({ attached, onFirstScan, onNextScan, onUndo, onValueTypeChange }: Props) {
  const [valueType, setValueType] = useState("I32");
  const [condition, setCondition] = useState("Exact");
  const [value, setValue] = useState("");
  const [value2, setValue2] = useState("");
  const [matchCount, setMatchCount] = useState<number | null>(null);
  const [scanning, setScanning] = useState(false);
  const [progress, setProgress] = useState<number | null>(null);
  const [showOptions, setShowOptions] = useState(false);
  const [regionFilter, setRegionFilter] = useState<RegionFilter>({
    writable_only: true,
    skip_executable: false,
    skip_mapped_files: true,
  });
  const { showToast } = useToast();

  const handlersRef = useRef({ handleFirstScan, handleNextScan, handleUndo });
  useEffect(() => { handlersRef.current = { handleFirstScan, handleNextScan, handleUndo }; });

  useEffect(() => {
    const doFirst = () => handlersRef.current.handleFirstScan();
    const doNext = () => handlersRef.current.handleNextScan();
    const doUndo = () => handlersRef.current.handleUndo();
    window.addEventListener("scan:first", doFirst);
    window.addEventListener("scan:next", doNext);
    window.addEventListener("scan:undo", doUndo);
    return () => {
      window.removeEventListener("scan:first", doFirst);
      window.removeEventListener("scan:next", doNext);
      window.removeEventListener("scan:undo", doUndo);
    };
  }, []);

  const needsValue = !["Unknown", "Changed", "Unchanged", "Increased", "Decreased"].includes(condition);
  const needsSecondValue = condition === "Between";
  const isAob = valueType === "ByteArray";

  function buildScanValue(): unknown {
    if (!needsValue) return null;
    if (isAob) {
      const pattern = parseAobPattern(value);
      if (!pattern || pattern.length === 0) {
        showToast("Invalid AOB pattern. Use format: 48 8B ?? ?? 90", "error");
        return undefined;
      }
      return { Pattern: pattern };
    }
    const numVal = valueType.startsWith("F") ? parseFloat(value) : parseInt(value, 10);
    if (value.trim() === "" || isNaN(numVal)) {
      showToast("Please enter a valid number", "error");
      return undefined;
    }
    return { [valueType]: numVal };
  }

  function buildValue(input: string): unknown {
    if (!input || input.trim() === "") return null;
    const numVal = valueType.startsWith("F") ? parseFloat(input) : parseInt(input, 10);
    if (isNaN(numVal)) return null;
    return { [valueType]: numVal };
  }

  async function handleFirstScan() {
    if (!attached) return;
    const scanValue = buildScanValue();
    if (scanValue === undefined) return;
    setScanning(true);
    setProgress(0);
    try {
      const onProgress = new Channel<ScanProgress>();
      onProgress.onmessage = (p) => {
        if (p.total_bytes > 0) {
          setProgress(p.scanned_bytes / p.total_bytes);
        }
      };
      const result = await invoke<{ match_count: number }>("first_scan", {
        valueType,
        condition,
        value: scanValue,
        value2: needsSecondValue ? buildValue(value2) : null,
        regionFilter,
        channel: onProgress,
      });
      setMatchCount(result.match_count);
      onFirstScan?.();
    } catch (e) {
      showToast(`Scan failed: ${e}`, "error");
    } finally {
      setScanning(false);
      setProgress(null);
    }
  }

  async function handleNextScan() {
    if (!attached) return;
    setScanning(true);
    try {
      const result = await invoke<{ match_count: number }>("next_scan", {
        condition,
        value: needsValue ? buildValue(value) : null,
        value2: needsSecondValue ? buildValue(value2) : null,
      });
      setMatchCount(result.match_count);
      onNextScan?.();
    } catch (e) {
      showToast(`Next scan failed: ${e}`, "error");
    } finally {
      setScanning(false);
    }
  }

  async function handleUndo() {
    if (!attached) return;
    try {
      const result = await invoke<{ match_count: number }>("undo_scan");
      setMatchCount(result.match_count);
      onUndo?.();
    } catch (e) {
      showToast(`Undo failed: ${e}`, "error");
    }
  }

  function handleReset() {
    setMatchCount(null);
    setValue("");
    setValue2("");
    onFirstScan?.();
  }

  return (
    <div>
      <div className="scan-section">
        <label>Value Type</label>
        <select value={valueType} onChange={(e) => { setValueType(e.target.value); onValueTypeChange?.(e.target.value); }}>
          <option value="I8">Int8</option>
          <option value="I16">Int16</option>
          <option value="I32">Int32</option>
          <option value="I64">Int64</option>
          <option value="U8">UInt8</option>
          <option value="U16">UInt16</option>
          <option value="U32">UInt32</option>
          <option value="U64">UInt64</option>
          <option value="F32">Float</option>
          <option value="F64">Double</option>
          <option value="ByteArray">Byte Array</option>
        </select>
      </div>

      <div className="scan-section">
        <label>Scan Condition</label>
        <select value={condition} onChange={(e) => setCondition(e.target.value)}>
          <option value="Exact">Exact Value</option>
          <option value="GreaterThan">Greater Than</option>
          <option value="LessThan">Less Than</option>
          <option value="Between">Between</option>
          <option value="Unknown">Unknown Initial</option>
          <option value="Changed">Changed</option>
          <option value="Unchanged">Unchanged</option>
          <option value="Increased">Increased</option>
          <option value="Decreased">Decreased</option>
          <option value="IncreasedBy">Increased By</option>
          <option value="DecreasedBy">Decreased By</option>
        </select>
      </div>

      {needsValue && (
        <div className="scan-section">
          <label>{isAob ? "AOB Pattern" : "Value"}</label>
          <input
            type="text"
            value={value}
            onChange={(e) => setValue(e.target.value)}
            placeholder={isAob ? "48 8B ?? ?? 90 E8" : "Enter value..."}
          />
          {isAob && (
            <span className="hint">Hex bytes, ?? for wildcard</span>
          )}
        </div>
      )}

      {needsSecondValue && (
        <div className="scan-section">
          <label>Value 2 (max)</label>
          <input
            type="text"
            value={value2}
            onChange={(e) => setValue2(e.target.value)}
            placeholder="Enter max value..."
          />
        </div>
      )}

      {/* Region filter options */}
      <div className="scan-options">
        <div
          className="scan-options-header"
          onClick={() => setShowOptions(!showOptions)}
        >
          <span>{showOptions ? "▾" : "▸"} Scan Options</span>
        </div>
        {showOptions && (
          <div className="scan-options-body">
            <label className="checkbox-label">
              <input
                type="checkbox"
                checked={regionFilter.writable_only}
                onChange={(e) =>
                  setRegionFilter({ ...regionFilter, writable_only: e.target.checked })
                }
              />
              Writable Only
            </label>
            <label className="checkbox-label">
              <input
                type="checkbox"
                checked={regionFilter.skip_executable}
                onChange={(e) =>
                  setRegionFilter({ ...regionFilter, skip_executable: e.target.checked })
                }
              />
              Skip Executable
            </label>
            <label className="checkbox-label">
              <input
                type="checkbox"
                checked={regionFilter.skip_mapped_files}
                onChange={(e) =>
                  setRegionFilter({ ...regionFilter, skip_mapped_files: e.target.checked })
                }
              />
              Skip Mapped Files
            </label>
          </div>
        )}
      </div>

      {/* Progress bar */}
      {progress !== null && (
        <div className="progress-bar">
          <div className="progress-fill" style={{ width: `${Math.round(progress * 100)}%` }} />
          <span className="progress-text">{Math.round(progress * 100)}%</span>
        </div>
      )}

      <div className="scan-buttons">
        <button onClick={handleFirstScan} disabled={!attached || scanning}>
          {scanning ? "Scanning..." : "First Scan"}
        </button>
        <button onClick={handleNextScan} disabled={!attached || scanning || matchCount === null}>
          Next Scan
        </button>
        <button onClick={handleUndo} disabled={matchCount === null}>
          Undo
        </button>
        <button onClick={handleReset} disabled={matchCount === null}>
          Reset
        </button>
      </div>

      {matchCount !== null && (
        <div className="match-count">Found: {matchCount.toLocaleString()} matches</div>
      )}
    </div>
  );
}
