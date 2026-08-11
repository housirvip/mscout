import { useState, useEffect, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useI18n, MessageKey } from "../i18n";
import { ContextMenu, MenuItemDef } from "./ContextMenu";

interface ScanResult {
  address: string;
  value: string;
  previous_value: string;
}

interface Props {
  attached: boolean;
  valueType: string;
  hasSession: boolean;
  matchCount: number;
  onViewMemory: (address: number) => void;
  onAddToTable: (address: string, value: string, valueType: string) => void;
  onPointerScan: (address: string) => void;
  onPickProcess: () => void;
}

const PAGE_SIZE = 100;

export function ResultsTable({
  attached,
  valueType,
  hasSession,
  matchCount,
  onViewMemory,
  onAddToTable,
  onPointerScan,
  onPickProcess,
}: Props) {
  const { t } = useI18n();
  const [results, setResults] = useState<ScanResult[]>([]);
  const [page, setPage] = useState(1);
  const [pageInput, setPageInput] = useState("1");
  const [contextMenu, setContextMenu] = useState<{
    x: number;
    y: number;
    row: ScanResult;
  } | null>(null);

  const totalPages = Math.max(1, Math.ceil(matchCount / PAGE_SIZE));
  const offset = (page - 1) * PAGE_SIZE;
  const pollRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const mountedRef = useRef(true);

  useEffect(() => {
    mountedRef.current = true;
    return () => { mountedRef.current = false; };
  }, []);

  useEffect(() => { setPageInput(String(page)); }, [page]);

  const fetchResults = useCallback(async () => {
    if (!attached || !hasSession) return;
    try {
      const data = await invoke<ScanResult[]>("get_scan_results", {
        offset,
        count: PAGE_SIZE,
      });
      if (mountedRef.current) {
        setResults(data);
      }
    } catch {
      // ignore transient errors
    }
  }, [attached, hasSession, offset]);

  // Poll via setTimeout to avoid overlapping requests
  useEffect(() => {
    if (!attached || !hasSession) {
      setResults([]);
      return;
    }

    let active = true;
    async function poll() {
      await fetchResults();
      if (active) {
        pollRef.current = setTimeout(poll, 1000);
      }
    }
    poll();

    return () => {
      active = false;
      if (pollRef.current) clearTimeout(pollRef.current);
    };
  }, [fetchResults, attached, hasSession]);

  // Reset page when match count changes
  useEffect(() => {
    setPage(1);
  }, [matchCount]);

  function handleContextMenu(e: React.MouseEvent, row: ScanResult) {
    e.preventDefault();
    setContextMenu({ x: e.clientX, y: e.clientY, row });
  }

  function getMenuItems(row: ScanResult): MenuItemDef[] {
    return [
      {
        label: t("results.ctxAdd"),
        onClick: () => onAddToTable(row.address, row.value, valueType),
      },
      {
        label: t("results.ctxView"),
        onClick: () => onViewMemory(parseInt(row.address, 16)),
      },
      {
        label: t("results.ctxPointer"),
        onClick: () => onPointerScan(row.address),
      },
      {
        label: t("results.ctxCopy"),
        onClick: () => navigator.clipboard.writeText(row.address),
      },
    ];
  }

  function goToPage(p: number) {
    const clamped = Math.max(1, Math.min(totalPages, p));
    setPage(clamped);
  }

  // --- Not attached: empty state ---
  if (!attached) {
    return (
      <section className="sec">
        <div className="sec-head">
          <h2>{t("results.title")}</h2>
          <span className="count">{t("results.noMatch")}</span>
          <span className="spacer" />
        </div>
        <div className="empty">
          <div className="empty-inner">
            <svg aria-hidden="true" viewBox="0 0 24 24">
              <circle cx="12" cy="12" r="10" />
              <circle cx="12" cy="12" r="3" />
              <line x1="12" y1="2" x2="12" y2="5" />
              <line x1="12" y1="19" x2="12" y2="22" />
              <line x1="2" y1="12" x2="5" y2="12" />
              <line x1="19" y1="12" x2="22" y2="12" />
            </svg>
            <h3>{t("results.emptyTitle")}</h3>
            <p>{t("results.emptyDesc")}</p>
            <button className="btn btn-primary" onClick={onPickProcess}>
              {t("results.attachBtn")}
            </button>
          </div>
        </div>
      </section>
    );
  }

  return (
    <section className="sec">
      <div className="sec-head">
        <h2>{t("results.title")}</h2>
        <span className="count">
          {matchCount > 0
            ? t("results.matches", { count: matchCount })
            : t("results.noMatch")}
        </span>
        <span className="spacer" />
      </div>

      {!hasSession ? (
        <div className="empty">
          <div className="empty-inner">
            <p style={{ fontWeight: 600 }}>{t("results.emptyRunScan" as MessageKey)}</p>
            <p style={{ color: "var(--muted)" }}>{t("results.emptyRunScanDesc" as MessageKey)}</p>
          </div>
        </div>
      ) : results.length === 0 ? (
        <div className="empty">
          <div className="empty-inner">
            <p style={{ color: "var(--muted)" }}>{t("results.noMatch")}</p>
          </div>
        </div>
      ) : (
        <>
          <div className="tbl-wrap">
            <table>
              <thead>
                <tr>
                  <th style={{ width: 196 }}>{t("addr.colAddr")}</th>
                  <th style={{ width: 150 }}>{t("addr.colValue")}</th>
                  <th style={{ width: 150 }}>{t("results.colPrevious" as MessageKey)}</th>
                  <th>{t("addr.colType")}</th>
                </tr>
              </thead>
              <tbody>
                {results.map((row) => (
                  <tr
                    key={row.address}
                    onContextMenu={(e) => handleContextMenu(e, row)}
                  >
                    <td className="mono">{row.address}</td>
                    <td className={row.value !== row.previous_value ? "changed" : ""}>
                      {row.value}
                    </td>
                    <td className="dim">{row.previous_value}</td>
                    <td>
                      <span className="type-tag">{valueType}</span>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>

          {/* Pager */}
          <div className="pager">
            <span className="info">
              {offset + 1}–{Math.min(offset + PAGE_SIZE, matchCount)} / {matchCount}
            </span>
            <span className="spacer" />
            <button
              className="btn btn-icon"
              title={t("results.pageFirst" as MessageKey)}
              disabled={page <= 1}
              onClick={() => goToPage(1)}
            >
              «
            </button>
            <button
              className="btn btn-icon"
              title={t("results.pagePrev" as MessageKey)}
              disabled={page <= 1}
              onClick={() => goToPage(page - 1)}
            >
              ‹
            </button>
            <input
              className="control page-in mono"
              type="number"
              min={1}
              max={totalPages}
              value={pageInput}
              onChange={(e) => setPageInput(e.target.value)}
              onBlur={() => {
                const n = parseInt(pageInput, 10);
                if (!isNaN(n)) goToPage(n);
                else setPageInput(String(page));
              }}
              onKeyDown={(e) => {
                if (e.key === "Enter") (e.target as HTMLInputElement).blur();
              }}
              aria-label={t("results.pageGo" as MessageKey)}
            />
            <span className="info">/ {totalPages}</span>
            <button
              className="btn btn-icon"
              title={t("results.pageNext" as MessageKey)}
              disabled={page >= totalPages}
              onClick={() => goToPage(page + 1)}
            >
              ›
            </button>
            <button
              className="btn btn-icon"
              title={t("results.pageLast" as MessageKey)}
              disabled={page >= totalPages}
              onClick={() => goToPage(totalPages)}
            >
              »
            </button>
          </div>
        </>
      )}

      {/* Context menu */}
      {contextMenu && (
        <ContextMenu
          x={contextMenu.x}
          y={contextMenu.y}
          items={getMenuItems(contextMenu.row)}
          onClose={() => setContextMenu(null)}
        />
      )}
    </section>
  );
}
