"use client";

import { useState, useCallback, useRef, useEffect, useMemo, DragEvent, MouseEvent as RMouseEvent } from "react";
import {
  PHeading,
  PText,
  PButton,
  PBanner,
  PSpinner,
  PTag,
} from "@porsche-design-system/components-react/ssr";
import {
  importFiles,
  listMatrices,
  deleteMatrix,
  clearAllMatrices,
  MatrixSummary,
} from "@/lib/api";
import { SearchIcon, FilterIcon, ColumnFilterPopup } from "@/components/ColumnFilter";

export default function ManageImportsPage() {
  const [matrices, setMatrices] = useState<MatrixSummary[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [progressMsg, setProgressMsg] = useState<string | null>(null);
  const [loadingList, setLoadingList] = useState(true);
  const [dragOver, setDragOver] = useState(false);
  const [deletingId, setDeletingId] = useState<string | null>(null);
  const folderInputRef = useRef<HTMLInputElement>(null);
  const zipInputRef = useRef<HTMLInputElement>(null);

  /* -- Import table columns ------------------------------------------- */

  const IMPORT_COLUMNS: { key: string; label: string; align?: "right"; accessor: (m: MatrixSummary) => string }[] = [
    { key: "source_file", label: "Source File", accessor: (m) => m.source_file },
    { key: "platform", label: "Platform", accessor: (m) => m.platform ?? "" },
    { key: "bus_type", label: "Bus", accessor: (m) => m.bus_type },
    { key: "bus_name", label: "Bus Name", accessor: (m) => m.bus_name },
    { key: "message_count", label: "Messages", align: "right", accessor: (m) => String(m.message_count) },
    { key: "signal_count", label: "Signals", align: "right", accessor: (m) => String(m.signal_count) },
  ];

  /* -- Per-column filters --------------------------------------------- */

  const [columnTextFilters, setColumnTextFilters] = useState<Record<string, string>>({});
  const [columnSetFilters, setColumnSetFilters] = useState<Record<string, Set<string>>>({});
  const [activeFilterCol, setActiveFilterCol] = useState<string | null>(null);
  const [filterAnchorRect, setFilterAnchorRect] = useState<DOMRect | null>(null);

  /* -- Load matrices on mount ----------------------------------------- */

  const refresh = useCallback(async () => {
    try {
      const list = await listMatrices();
      setMatrices(list);
    } catch {
      /* ignore */
    } finally {
      setLoadingList(false);
    }
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  /* -- Import --------------------------------------------------------- */

  const handleFileUpload = useCallback(
    async (files: FileList | File[]) => {
      const arr = Array.from(files);
      const supported = arr.filter(
        (f) =>
          f.name.endsWith(".xlsx") ||
          f.name.endsWith(".dbc") ||
          f.name.endsWith(".ldf") ||
          f.name.endsWith(".zip")
      );
      if (supported.length === 0) {
        setError("No supported files (.xlsx, .dbc, .ldf, .zip).");
        setProgressMsg(null);
        setLoading(false);
        return;
      }
      setLoading(true);
      setError(null);

      const BATCH_SIZE = 10;
      const total = supported.length;
      let uploaded = 0;

      try {
        for (let i = 0; i < total; i += BATCH_SIZE) {
          const batch = supported.slice(i, i + BATCH_SIZE);
          const batchEnd = Math.min(i + BATCH_SIZE, total);
          setProgressMsg(
            `Uploading & parsing file ${uploaded + 1}\u2013${batchEnd} of ${total}\u2026`
          );
          await importFiles(batch);
          uploaded = batchEnd;
        }
        setProgressMsg(null);
        refresh();
      } catch (e) {
        setProgressMsg(null);
        setError(e instanceof Error ? e.message : "Upload failed");
      } finally {
        setLoading(false);
      }
    },
    [refresh]
  );

  /* -- Directory drag-drop via webkitGetAsEntry ----------------------- */

  const scanCountRef = useRef(0);

  const readAllEntries = useCallback(
    async (entry: FileSystemDirectoryEntry): Promise<File[]> => {
      const reader = entry.createReader();
      const files: File[] = [];
      const readBatch = (): Promise<FileSystemEntry[]> =>
        new Promise((resolve, reject) => reader.readEntries(resolve, reject));
      let batch = await readBatch();
      while (batch.length > 0) {
        for (const e of batch) {
          if (e.isFile) {
            const file = await new Promise<File>((resolve, reject) =>
              (e as FileSystemFileEntry).file(resolve, reject)
            );
            files.push(file);
            scanCountRef.current += 1;
            if (scanCountRef.current % 50 === 0) {
              setProgressMsg(`Scanning folder\u2026 ${scanCountRef.current} files found`);
            }
          } else if (e.isDirectory) {
            files.push(
              ...(await readAllEntries(e as FileSystemDirectoryEntry))
            );
          }
        }
        batch = await readBatch();
      }
      return files;
    },
    []
  );

  const onDrop = useCallback(
    async (e: DragEvent) => {
      e.preventDefault();
      setDragOver(false);
      setError(null);

      const items = e.dataTransfer.items;
      if (items && items.length > 0) {
        const allFiles: File[] = [];
        let hasDirectory = false;
        let hasZipFile = false;
        setLoading(true);
        setProgressMsg("Scanning folder\u2026");
        scanCountRef.current = 0;
        for (let i = 0; i < items.length; i++) {
          const entry = items[i].webkitGetAsEntry?.();
          if (!entry) {
            const droppedFile = items[i].getAsFile();
            if (droppedFile && droppedFile.name.toLowerCase().endsWith(".zip")) {
              hasZipFile = true;
              allFiles.push(droppedFile);
            }
            continue;
          }
          try {
            if (entry.isDirectory) {
              hasDirectory = true;
              const dirFiles = await readAllEntries(
                entry as FileSystemDirectoryEntry
              );
              allFiles.push(...dirFiles);
            } else if (entry.isFile) {
              const droppedFile = items[i].getAsFile();
              if (droppedFile && droppedFile.name.toLowerCase().endsWith(".zip")) {
                hasZipFile = true;
                allFiles.push(droppedFile);
              }
            }
          } catch (err) {
            console.warn("Failed to read dropped entry:", entry.name, err);
          }
        }
        setProgressMsg(null);
        if (allFiles.length > 0) {
          handleFileUpload(allFiles);
          return;
        }
        setLoading(false);
        if (hasDirectory) {
          setError(
            "No supported files (.xlsx, .dbc, .ldf, .zip) found in the dropped folder."
          );
        } else if (hasZipFile) {
          setError("No supported files found in dropped .zip archive.");
        } else {
          setError("Please drop a folder or .zip file.");
        }
      }
    },
    [handleFileUpload, readAllEntries]
  );

  /* -- Delete --------------------------------------------------------- */

  const handleDelete = useCallback(
    async (id: string) => {
      setDeletingId(id);
      try {
        await deleteMatrix(id);
        refresh();
      } catch (e) {
        setError(e instanceof Error ? e.message : "Delete failed");
      } finally {
        setDeletingId(null);
      }
    },
    [refresh]
  );

  const handleClearAll = useCallback(async () => {
    setLoading(true);
    try {
      await clearAllMatrices();
      setMatrices([]);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Clear failed");
    } finally {
      setLoading(false);
    }
  }, []);

  /* -- Derived -------------------------------------------------------- */

  const totalSignals = matrices.reduce((s, m) => s + m.signal_count, 0);

  /* -- Unique values per column (computed client-side from all matrices) */

  const uniqueValuesPerColumn = useMemo(() => {
    const map: Record<string, string[]> = {};
    for (const col of IMPORT_COLUMNS) {
      const set = new Set<string>();
      for (const m of matrices) set.add(col.accessor(m));
      map[col.key] = Array.from(set).sort((a, b) => a.localeCompare(b));
    }
    return map;
  }, [matrices]);

  /* -- Filtered matrices ---------------------------------------------- */

  const activeTextFilters = Object.entries(columnTextFilters).filter(([, v]) => v.trim() !== "");
  const activeSetFilters = Object.entries(columnSetFilters).filter(
    ([colKey, checked]) => checked.size < (uniqueValuesPerColumn[colKey]?.length ?? 0)
  );
  const hasAnyFilter = activeTextFilters.length > 0 || activeSetFilters.length > 0;

  const filteredMatrices = useMemo(() => {
    if (!hasAnyFilter) return matrices;
    return matrices.filter((m) => {
      for (const [colKey, fv] of activeTextFilters) {
        const col = IMPORT_COLUMNS.find((c) => c.key === colKey);
        if (!col) continue;
        if (!col.accessor(m).toLowerCase().includes(fv.toLowerCase())) return false;
      }
      for (const [colKey, checked] of activeSetFilters) {
        const col = IMPORT_COLUMNS.find((c) => c.key === colKey);
        if (!col) continue;
        if (!checked.has(col.accessor(m))) return false;
      }
      return true;
    });
  }, [matrices, activeTextFilters, activeSetFilters, hasAnyFilter]);

  /* -- Column filter helpers ------------------------------------------ */

  const toggleColumnFilter = useCallback(
    (colKey: string, e: RMouseEvent<HTMLButtonElement>) => {
      e.stopPropagation();
      if (activeFilterCol === colKey) { setActiveFilterCol(null); return; }
      const rect = (e.target as HTMLElement).getBoundingClientRect();
      setFilterAnchorRect(rect);
      setActiveFilterCol(colKey);
    },
    [activeFilterCol]
  );

  const colHasFilter = useCallback((colKey: string) => {
    if (columnTextFilters[colKey]?.trim()) return true;
    const setFilter = columnSetFilters[colKey];
    if (setFilter && setFilter.size < (uniqueValuesPerColumn[colKey]?.length ?? 0)) return true;
    return false;
  }, [columnTextFilters, columnSetFilters, uniqueValuesPerColumn]);

  const hiddenByColFilter = matrices.length - filteredMatrices.length;

  return (
    <div className="w-full px-8 py-6">
      <PHeading tag="h2" size="large" className="mb-2">
        Manage Imports
      </PHeading>
      <PText color="contrast-medium" className="mb-8">
        Import K-Matrix files and manage loaded data. Files persist across
        browser reloads.
      </PText>

      {/* ── Import zone ──────────────────────────────────────────── */}
      <section className="mb-10">
        <div
          role="button"
          tabIndex={0}
          onDragOver={(e) => {
            e.preventDefault();
            setDragOver(true);
          }}
          onDragLeave={() => setDragOver(false)}
          onDrop={onDrop}
          onKeyDown={(e) => {
            if (e.key === "Enter" || e.key === " ")
              folderInputRef.current?.click();
          }}
          className={`border-2 border-dashed rounded-lg p-6 transition-colors ${
            dragOver
              ? "border-blue-500 bg-blue-50"
              : "border-gray-300 hover:border-gray-400"
          }`}
        >
          {loading ? (
            <div className="flex flex-col items-center gap-3 py-4">
              <PSpinner />
              {progressMsg && (
                <PText color="contrast-medium" size="small">
                  {progressMsg}
                </PText>
              )}
            </div>
          ) : (
          <div className="flex flex-col items-center gap-3">
            <div className="text-3xl">{"\uD83D\uDCC2"}</div>
            <PText weight="bold">
              Drag &amp; drop K-Matrix folders or .zip archives here
            </PText>
            <PText color="contrast-medium" size="small">
              Supported files: .xlsx &middot; .dbc &middot; .ldf &middot; .zip
            </PText>

            <div className="flex flex-wrap items-center justify-center gap-2">
              <PButton
                variant="secondary"
                onClick={() => folderInputRef.current?.click()}
              >
                Browse Folder
              </PButton>
              <PButton
                variant="secondary"
                onClick={() => zipInputRef.current?.click()}
              >
                Upload ZIP
              </PButton>
            </div>
            <input
              ref={folderInputRef}
              type="file"
              className="hidden"
              {...{ webkitdirectory: "", directory: "" } as React.InputHTMLAttributes<HTMLInputElement>}
              onChange={(e) => {
                if (e.target.files && e.target.files.length > 0) {
                  setProgressMsg(`Reading ${e.target.files.length} files from folder\u2026`);
                  // Use setTimeout to let React render the progress message
                  setTimeout(() => handleFileUpload(e.target.files!), 0);
                }
              }}
            />
            <input
              ref={zipInputRef}
              type="file"
              className="hidden"
              accept=".zip,application/zip"
              onChange={(e) => {
                if (e.target.files && e.target.files.length > 0) {
                  setProgressMsg(`Uploading ${e.target.files.length} .zip file(s)…`);
                  setTimeout(() => handleFileUpload(e.target.files!), 0);
                }
              }}
            />
          </div>
          )}
        </div>

        {error && (
          <PBanner
            state="error"
            open={true}
            className="mt-4"
            onDismiss={() => setError(null)}
          >
            <span slot="title">Error</span>
            <span slot="description">{error}</span>
          </PBanner>
        )}
      </section>

      {/* ── Loaded matrices list ─────────────────────────────────── */}
      <section>
        <div className="flex items-center justify-between mb-4">
          <PHeading tag="h3" size="medium">
            Loaded Matrices ({matrices.length})
          </PHeading>
          <div className="flex items-center gap-4">
            <PText color="contrast-medium" size="small">
              {totalSignals.toLocaleString()} signals total
            </PText>
            {matrices.length > 0 && (
              <PButton
                variant="secondary"
                icon="delete"
                onClick={handleClearAll}
                loading={loading}
              >
                Clear All
              </PButton>
            )}
          </div>
        </div>

        {loadingList ? (
          <div className="flex justify-center py-8">
            <PSpinner />
          </div>
        ) : matrices.length === 0 ? (
          <div className="text-center py-12 text-gray-400 border border-dashed border-gray-200 rounded-lg">
            <PText color="contrast-medium">
              No matrices loaded. Import files above to get started.
            </PText>
          </div>
        ) : (
          <>
            {/* Filter status */}
            {(hasAnyFilter || hiddenByColFilter > 0) && (
              <div className="flex items-center gap-4 mb-3">
                {hiddenByColFilter > 0 && (
                  <PText color="contrast-medium" size="small">
                    Showing {filteredMatrices.length} of {matrices.length} matrices
                    ({hiddenByColFilter} hidden by column filters)
                  </PText>
                )}
                {hasAnyFilter && (
                  <button className="text-xs text-blue-600 hover:underline"
                    onClick={() => { setColumnTextFilters({}); setColumnSetFilters({}); }}>
                    Clear all column filters
                  </button>
                )}
              </div>
            )}

            <div className="overflow-x-auto border border-gray-200 rounded-lg">
              <table className="w-full text-sm">
                <thead>
                  <tr className="border-b-2 border-gray-300 text-left bg-gray-50">
                    {IMPORT_COLUMNS.map((col) => (
                      <th key={col.key} className={`px-4 py-3 whitespace-nowrap group ${col.align === "right" ? "text-right" : ""}`}>
                        <span className="flex items-center gap-1">
                          {col.label}
                          <button
                            className={`ml-auto px-1 rounded transition-opacity ${
                              columnTextFilters[col.key]?.trim()
                                ? "opacity-100 text-blue-600 bg-blue-50"
                                : "opacity-0 group-hover:opacity-60 text-gray-400"
                            }`}
                            onClick={(e) => toggleColumnFilter(col.key, e)}
                            title={`Filter ${col.label}`}
                          >
                            <SearchIcon />
                          </button>
                          <button
                            className={`px-1 rounded transition-opacity ${
                              colHasFilter(col.key)
                                ? "opacity-100 text-blue-600 bg-blue-50"
                                : "opacity-0 group-hover:opacity-60 text-gray-400"
                            }`}
                            onClick={(e) => toggleColumnFilter(col.key, e)}
                            title={`Filter ${col.label}`}
                          >
                            <FilterIcon />
                          </button>
                        </span>
                      </th>
                    ))}
                    <th className="px-4 py-3" />
                  </tr>
                </thead>
                <tbody>
                  {filteredMatrices.map((m) => (
                    <tr
                      key={m.id}
                      className="border-b border-gray-100 hover:bg-gray-50"
                    >
                      <td className="px-4 py-3 font-mono text-xs" title={m.source_file}>
                        {m.source_file}
                      </td>
                      <td className="px-4 py-3">
                        {m.platform ? <PTag>{m.platform}</PTag> : "\u2014"}
                      </td>
                      <td className="px-4 py-3">
                        <PTag color="primary">{m.bus_type}</PTag>
                      </td>
                      <td className="px-4 py-3 font-mono text-xs">
                        {m.bus_name}
                      </td>
                      <td className="px-4 py-3 text-right font-mono">
                        {m.message_count}
                      </td>
                      <td className="px-4 py-3 text-right font-mono">
                        {m.signal_count.toLocaleString()}
                      </td>
                      <td className="px-4 py-3 text-right">
                        <PButton
                          variant="tertiary"
                          icon="delete"
                          hideLabel={true}
                          loading={deletingId === m.id}
                          onClick={() => handleDelete(m.id)}
                        >
                          Delete
                        </PButton>
                      </td>
                    </tr>
                  ))}
                  {filteredMatrices.length === 0 && matrices.length > 0 && (
                    <tr>
                      <td colSpan={IMPORT_COLUMNS.length + 1}
                        className="px-4 py-6 text-center text-gray-400">
                        All rows hidden by column filters
                      </td>
                    </tr>
                  )}
                </tbody>
              </table>
            </div>
          </>
        )}

        {/* ── Column filter popup ──────────────────────────────── */}
        {activeFilterCol && filterAnchorRect && (
          <ColumnFilterPopup
            column={IMPORT_COLUMNS.find((c) => c.key === activeFilterCol)!}
            textValue={columnTextFilters[activeFilterCol] ?? ""}
            onTextChange={(val) =>
              setColumnTextFilters((prev) => ({ ...prev, [activeFilterCol]: val }))
            }
            uniqueValues={uniqueValuesPerColumn[activeFilterCol] ?? []}
            checkedValues={
              columnSetFilters[activeFilterCol] ??
              new Set(uniqueValuesPerColumn[activeFilterCol] ?? [])
            }
            onCheckedChange={(next) =>
              setColumnSetFilters((prev) => ({ ...prev, [activeFilterCol]: next }))
            }
            onClose={() => setActiveFilterCol(null)}
            anchorRect={filterAnchorRect}
          />
        )}
      </section>
    </div>
  );
}
