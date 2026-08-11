import { useState, useEffect, useRef } from "react";
import { invoke, Channel } from "@tauri-apps/api/core";
import { useI18n } from "../i18n";
import { useToast } from "./Toast";

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

interface Props {
  attached: boolean;
  hasSession: boolean;
  onFirstScan: (count: number) => void;
  onNextScan: (count: number) => void;
  onUndo: (count: number) => void;
  onValueTypeChange: (vt: string) => void;
  onReset: () => void;
  onPickProcess: () => void;
}

type ValueType = "I8" | "I16" | "I32" | "I64" | "F32" | "F64" | "ByteArray";
type Condition =
  | "Exact"
  | "GreaterThan"
  | "LessThan"
  | "Between"
  | "Unknown"
  | "Changed"
  | "Unchanged"
  | "Increased"
  | "Decreased";

const VALUE_TYPES: { value: ValueType; labelKey: string }[] = [
  { value: "I8", labelKey: "type.i8" },
  { value: "I16", labelKey: "type.i16" },
  { value: "I32", labelKey: "type.i32" },
  { value: "I64", labelKey: "type.i64" },
  { value: "F32", labelKey: "type.f32" },
  { value: "F64", labelKey: "type.f64" },
  { value: "ByteArray", labelKey: "type.aob" },
];

const FIRST_CONDITIONS: { value: Condition; labelKey: string }[] = [
  { value: "Exact", labelKey: "cond.exact" },
  { value: "GreaterThan", labelKey: "cond.gt" },
  { value: "LessThan", labelKey: "cond.lt" },
  { value: "Between", labelKey: "cond.between" },
  { value: "Unknown", labelKey: "cond.unknown" },
];

const NEXT_CONDITIONS: { value: Condition; labelKey: string }[] = [
  { value: "Changed", labelKey: "cond.changed" },
  { value: "Unchanged", labelKey: "cond.unchanged" },
  { value: "Increased", labelKey: "cond.increased" },
  { value: "Decreased", labelKey: "cond.decreased" },
];

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

export function ScanPanel({
  attached,
  hasSession,
  onFirstScan,
  onNextScan,
  onUndo,
  onValueTypeChange,
  onReset: _onReset,
  onPickProcess,
}: Props) {
  const { t } = useI18n();
  const { showToast } = useToast();

  const [valueType, setValueType] = useState<ValueType>("I32");
  const [condition, setCondition] = useState<Condition>("Exact");
  const [value, setValue] = useState("");
  const [value2, setValue2] = useState("");
  const [scanning, setScanning] = useState(false);
  const [progress, setProgress] = useState<number | null>(null);
  const [showOptions, setShowOptions] = useState(false);
  const [regionFilter, setRegionFilter] = useState<RegionFilter>({
    writable_only: true,
    skip_executable: true,
    skip_mapped_files: false,
  });

  const needsValue = !["Unknown", "Changed", "Unchanged", "Increased", "Decreased"].includes(condition);
  const needsSecondValue = condition === "Between";
  const isAob = valueType === "ByteArray";

  // --- Build ScanValue ---
  function buildScanValue(): unknown | undefined {
    if (!needsValue) return null;
    if (isAob) {
      const pattern = parseAobPattern(value);
      if (!pattern || pattern.length === 0) {
        showToast(t("toast.invalidNumber"), "error");
        return undefined;
      }
      return { Pattern: pattern };
    }
    const numVal = valueType.startsWith("F") ? parseFloat(value) : parseInt(value, 10);
    if (value.trim() === "" || isNaN(numVal)) {
      showToast(t("toast.invalidNumber"), "error");
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

  // --- Scan handlers ---
  async function handleFirstScan() {
    if (!attached || scanning) return;
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
      onFirstScan(result.match_count);
    } catch (e) {
      showToast(t("toast.scanFailed", { error: String(e) }), "error");
    } finally {
      setScanning(false);
      setProgress(null);
    }
  }

  async function handleNextScan() {
    if (!attached || scanning) return;
    setScanning(true);
    try {
      const result = await invoke<{ match_count: number }>("next_scan", {
        condition,
        value: needsValue ? buildValue(value) : null,
        value2: needsSecondValue ? buildValue(value2) : null,
      });
      onNextScan(result.match_count);
    } catch (e) {
      showToast(t("toast.scanFailed", { error: String(e) }), "error");
    } finally {
      setScanning(false);
    }
  }

  async function handleUndo() {
    if (!attached) return;
    try {
      const result = await invoke<{ match_count: number }>("undo_scan");
      onUndo(result.match_count);
    } catch (e) {
      showToast(t("toast.scanFailed", { error: String(e) }), "error");
    }
  }

  // --- Keyboard event listeners ---
  const handlersRef = useRef({ handleFirstScan, handleNextScan, handleUndo });
  useEffect(() => {
    handlersRef.current = { handleFirstScan, handleNextScan, handleUndo };
  });

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

  // Notify parent of value type changes
  function handleValueTypeChange(vt: ValueType) {
    setValueType(vt);
    onValueTypeChange(vt);
  }

  // Count active options for summary
  const activeOpts = [regionFilter.writable_only, regionFilter.skip_executable, regionFilter.skip_mapped_files].filter(Boolean).length;

  return (
    <aside className="rail">
      <div className="sec-head">
        <h2>{t("scan.title")}</h2>
        <span className="spacer" />
        <span className="count">
          {hasSession ? t("scan.scanning") : t("scan.notStarted")}
        </span>
      </div>

      <div className="rail-scroll">
        {/* Data type */}
        <div className="field">
          <label htmlFor="dataType">{t("scan.dataType")}</label>
          <select
            className="control"
            id="dataType"
            value={valueType}
            onChange={(e) => handleValueTypeChange(e.target.value as ValueType)}
            disabled={scanning}
          >
            {VALUE_TYPES.map((vt) => (
              <option key={vt.value} value={vt.value}>
                {t(vt.labelKey as keyof typeof t)}
              </option>
            ))}
          </select>
        </div>

        {/* Condition */}
        <div className="field">
          <label htmlFor="condition">{t("scan.condition")}</label>
          <select
            className="control"
            id="condition"
            value={condition}
            onChange={(e) => setCondition(e.target.value as Condition)}
            disabled={scanning}
          >
            {FIRST_CONDITIONS.map((c) => (
              <option key={c.value} value={c.value}>
                {t(c.labelKey as keyof typeof t)}
              </option>
            ))}
            {hasSession &&
              NEXT_CONDITIONS.map((c) => (
                <option key={c.value} value={c.value}>
                  {t(c.labelKey as keyof typeof t)}
                </option>
              ))}
          </select>
          <p className="hint">{t("scan.condHint")}</p>
        </div>

        {/* Value */}
        {needsValue && (
          <div className="field">
            <label htmlFor="val1">{t("scan.value")}</label>
            <div className="two-up" style={needsSecondValue ? undefined : { gridTemplateColumns: "1fr" }}>
              <input
                className="control mono"
                id="val1"
                inputMode="decimal"
                value={value}
                onChange={(e) => setValue(e.target.value)}
                disabled={scanning}
                placeholder={isAob ? "48 8B ?? ?? 90" : ""}
              />
              {needsSecondValue && (
                <>
                  <span className="dash">–</span>
                  <input
                    className="control mono"
                    id="val2"
                    inputMode="decimal"
                    value={value2}
                    onChange={(e) => setValue2(e.target.value)}
                    disabled={scanning}
                  />
                </>
              )}
            </div>
            {isAob && <p className="hint mono">{t("scan.aobHint")}</p>}
          </div>
        )}

        {/* Scan Options */}
        <div>
          <button
            className="opt-toggle"
            aria-expanded={showOptions}
            onClick={() => setShowOptions(!showOptions)}
          >
            <svg
              className="icon chev"
              style={{ width: 13, height: 13 }}
              aria-hidden="true"
              viewBox="0 0 24 24"
            >
              <polyline points="9 18 15 12 9 6" />
            </svg>
            {t("scan.options")}
            <span className="count">{t("scan.optDefault").replace("3", String(activeOpts))}</span>
          </button>
          {showOptions && (
            <div className="opt-body">
              <div className="switch-row">
                {t("scan.writableOnly")}
                <button
                  className="switch"
                  role="switch"
                  aria-checked={regionFilter.writable_only}
                  onClick={() =>
                    setRegionFilter((f) => ({ ...f, writable_only: !f.writable_only }))
                  }
                />
              </div>
              <div className="switch-row">
                {t("scan.skipExec")}
                <button
                  className="switch"
                  role="switch"
                  aria-checked={regionFilter.skip_executable}
                  onClick={() =>
                    setRegionFilter((f) => ({ ...f, skip_executable: !f.skip_executable }))
                  }
                />
              </div>
              <div className="switch-row">
                {t("scan.skipMapped")}
                <button
                  className="switch"
                  role="switch"
                  aria-checked={regionFilter.skip_mapped_files}
                  onClick={() =>
                    setRegionFilter((f) => ({ ...f, skip_mapped_files: !f.skip_mapped_files }))
                  }
                />
              </div>
            </div>
          )}
        </div>
      </div>

      {/* Actions */}
      <div className="rail-actions">
        {/* Progress */}
        {scanning && progress !== null && (
          <div className="progress-wrap" aria-live="polite">
            <div className="progress-meta">
              <span>{t("scan.scanning")}</span>
              <span className="mono">{Math.round(progress * 100)}%</span>
            </div>
            <div className="track">
              <div className="bar" style={{ width: `${Math.round(progress * 100)}%` }} />
            </div>
          </div>
        )}

        <button
          className="btn btn-primary"
          disabled={!attached || scanning}
          onClick={attached ? handleFirstScan : onPickProcess}
        >
          {t("scan.firstScan")}
        </button>

        <div className="pair">
          <button
            className="btn"
            disabled={!attached || !hasSession || scanning}
            onClick={handleNextScan}
          >
            {t("scan.nextScan")} <kbd>F6</kbd>
          </button>
          <button
            className="btn"
            disabled={!attached || !hasSession || scanning}
            onClick={handleUndo}
          >
            <svg className="icon" style={{ width: 14, height: 14 }} aria-hidden="true" viewBox="0 0 24 24">
              <polyline points="1 4 1 10 7 10" />
              <path d="M3.51 15a9 9 0 1 0 2.13-9.36L1 10" />
            </svg>
            {t("scan.undo")}
          </button>
        </div>

        <div className="shortcut-strip">
          <span><kbd>F5</kbd>{t("scan.firstScan")}</span>
          <span><kbd>F6</kbd>{t("scan.nextScan")}</span>
          <span><kbd>⌘Z</kbd>{t("scan.undo")}</span>
          <span><kbd>⌘N</kbd>{t("toolbar.pickProcess")}</span>
          <span><kbd>⌘S</kbd>{t("toolbar.saveTable")}</span>
          <span><kbd>⌘O</kbd>{t("toolbar.loadTable")}</span>
        </div>
      </div>
    </aside>
  );
}
