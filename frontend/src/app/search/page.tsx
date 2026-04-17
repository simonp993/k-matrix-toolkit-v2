"use client";

import {
  useState,
  useCallback,
  useRef,
  useEffect,
  DragEvent,
  MouseEvent as RMouseEvent,
} from "react";
import {
  PHeading,
  PText,
  PTextFieldWrapper,
  PButton,
  PSpinner,
  PTag,
  PMultiSelect,
  PMultiSelectOption,
  PSwitch,
} from "@porsche-design-system/components-react/ssr";
import {
  searchSignals,
  getStatus,
  SearchHit,
} from "@/lib/api";
import { SearchIcon, FilterIcon, ColumnFilterPopup } from "@/components/ColumnFilter";

/* ================================================================
   Column definitions — ALL fields from SearchHit
   ================================================================ */

interface ColumnDef {
  key: string;
  label: string;
  minWidth: number;
  defaultWidth: number;
  defaultVisible: boolean;
  accessor: (hit: SearchHit) => string;
  render: (hit: SearchHit) => React.ReactNode;
}

function fileTypeFromSource(src: string): string {
  const dot = src.lastIndexOf(".");
  return dot >= 0 ? src.slice(dot + 1).toLowerCase() : "";
}

const ALL_COLUMNS: ColumnDef[] = [
  {
    key: "message_name",
    label: "Message (Botschaft)",
    minWidth: 130,
    defaultWidth: 180,
    defaultVisible: true,
    accessor: (h) => h.message_name,
    render: (h) => <span className="font-mono">{h.message_name}</span>,
  },
  {
    key: "signal_name",
    label: "Signal",
    minWidth: 120,
    defaultWidth: 200,
    defaultVisible: true,
    accessor: (h) => h.signal_name,
    render: (h) => <span className="font-mono font-medium">{h.signal_name}</span>,
  },
  {
    key: "identifier",
    label: "ID",
    minWidth: 80,
    defaultWidth: 80,
    defaultVisible: true,
    accessor: (h) => h.identifier ?? "",
    render: (h) => <span className="font-mono">{h.identifier ?? "\u2014"}</span>,
  },
  {
    key: "bus_type",
    label: "Bus",
    minWidth: 80,
    defaultWidth: 80,
    defaultVisible: true,
    accessor: (h) => h.bus_type,
    render: (h) => <PTag color="primary">{h.bus_type}</PTag>,
  },
  {
    key: "bus_name",
    label: "Bus Name",
    minWidth: 130,
    defaultWidth: 130,
    defaultVisible: false,
    accessor: (h) => h.bus_name,
    render: (h) => <span className="font-mono text-xs">{h.bus_name}</span>,
  },
  {
    key: "file_type",
    label: "File Type",
    minWidth: 120,
    defaultWidth: 120,
    defaultVisible: false,
    accessor: (h) => fileTypeFromSource(h.source_file),
    render: (h) => {
      const ft = fileTypeFromSource(h.source_file);
      return ft ? <PTag>{`.${ft}`}</PTag> : <>{"\u2014"}</>;
    },
  },
  {
    key: "start_bit",
    label: "Start Bit",
    minWidth: 120,
    defaultWidth: 120,
    defaultVisible: false,
    accessor: (h) => h.start_bit?.toString() ?? "",
    render: (h) => <span className="font-mono text-right block">{h.start_bit ?? "\u2014"}</span>,
  },
  {
    key: "bit_length",
    label: "Length",
    minWidth: 110,
    defaultWidth: 110,
    defaultVisible: false,
    accessor: (h) => h.bit_length?.toString() ?? "",
    render: (h) => <span className="font-mono text-right block">{h.bit_length ?? "\u2014"}</span>,
  },
  {
    key: "ecu_sender",
    label: "Sender",
    minWidth: 110,
    defaultWidth: 120,
    defaultVisible: false,
    accessor: (h) => h.ecu_sender ?? "",
    render: (h) => <>{h.ecu_sender ?? "\u2014"}</>,
  },
  {
    key: "ecu_receivers",
    label: "Receivers",
    minWidth: 130,
    defaultWidth: 130,
    defaultVisible: false,
    accessor: (h) => h.ecu_receivers.join(", "),
    render: (h) => <>{h.ecu_receivers.join(", ") || "\u2014"}</>,
  },
  {
    key: "comment",
    label: "Comment",
    minWidth: 130,
    defaultWidth: 200,
    defaultVisible: false,
    accessor: (h) => h.comment ?? "",
    render: (h) => <span className="text-xs">{h.comment ?? "\u2014"}</span>,
  },
  {
    key: "description",
    label: "Description",
    minWidth: 140,
    defaultWidth: 200,
    defaultVisible: true,
    accessor: (h) => h.description ?? "",
    render: (h) => <span className="text-xs">{h.description ?? "\u2014"}</span>,
  },
  {
    key: "init_value",
    label: "Init Value",
    minWidth: 125,
    defaultWidth: 125,
    defaultVisible: false,
    accessor: (h) => h.init_value ?? "",
    render: (h) => <span className="font-mono text-xs">{h.init_value ?? "\u2014"}</span>,
  },
  {
    key: "error_value",
    label: "Error Value",
    minWidth: 135,
    defaultWidth: 135,
    defaultVisible: false,
    accessor: (h) => h.error_value ?? "",
    render: (h) => <span className="font-mono text-xs">{h.error_value ?? "\u2014"}</span>,
  },
  {
    key: "min_raw",
    label: "Min Raw",
    minWidth: 115,
    defaultWidth: 115,
    defaultVisible: false,
    accessor: (h) => h.min_raw ?? "",
    render: (h) => <span className="font-mono text-xs">{h.min_raw ?? "\u2014"}</span>,
  },
  {
    key: "max_raw",
    label: "Max Raw",
    minWidth: 120,
    defaultWidth: 120,
    defaultVisible: false,
    accessor: (h) => h.max_raw ?? "",
    render: (h) => <span className="font-mono text-xs">{h.max_raw ?? "\u2014"}</span>,
  },
  {
    key: "physical_value",
    label: "Physical",
    minWidth: 120,
    defaultWidth: 120,
    defaultVisible: false,
    accessor: (h) => h.physical_value ?? "",
    render: (h) => <span className="font-mono text-xs">{h.physical_value ?? "\u2014"}</span>,
  },
  {
    key: "unit",
    label: "Unit",
    minWidth: 80,
    defaultWidth: 80,
    defaultVisible: false,
    accessor: (h) => h.unit ?? "",
    render: (h) => <>{h.unit ?? "\u2014"}</>,
  },
  {
    key: "offset",
    label: "Offset",
    minWidth: 100,
    defaultWidth: 100,
    defaultVisible: false,
    accessor: (h) => h.offset ?? "",
    render: (h) => <span className="font-mono text-xs">{h.offset ?? "\u2014"}</span>,
  },
  {
    key: "scaling",
    label: "Scaling",
    minWidth: 110,
    defaultWidth: 110,
    defaultVisible: false,
    accessor: (h) => h.scaling ?? "",
    render: (h) => <span className="font-mono text-xs">{h.scaling ?? "\u2014"}</span>,
  },
  {
    key: "source_file",
    label: "Source",
    minWidth: 110,
    defaultWidth: 200,
    defaultVisible: false,
    accessor: (h) => h.source_file,
    render: (h) => <span className="text-xs">{h.source_file}</span>,
  },
];

/* ================================================================
   Expandable Table Cell — click to toggle wrap
   ================================================================ */

function ExpandableCell({
  children,
  width,
  forceExpand,
}: {
  children: React.ReactNode;
  width: number;
  forceExpand: boolean;
}) {
  const [expanded, setExpanded] = useState(false);
  const isExpanded = forceExpand || expanded;

  return (
    <td
      className={`px-3 py-2 cursor-pointer break-all ${
        isExpanded
          ? "whitespace-pre-wrap"
          : "overflow-hidden text-ellipsis whitespace-nowrap"
      }`}
      style={{ maxWidth: isExpanded ? undefined : width }}
      onClick={() => setExpanded((p) => !p)}
    >
      {children}
    </td>
  );
}

/* ================================================================
   Main Search Page
   ================================================================ */

const PAGE_SIZE = 200;

export default function SearchPage() {
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<SearchHit[]>([]);
  const [totalResults, setTotalResults] = useState(0);
  const [offset, setOffset] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const [searching, setSearching] = useState(false);

  // Status (loaded data)
  const [matrixCount, setMatrixCount] = useState(0);
  const [signalCount, setSignalCount] = useState(0);

  // Expand all rows — persisted to localStorage
  const [expandAll, setExpandAll] = useState<boolean>(() => {
    if (typeof window !== "undefined") {
      try {
        return localStorage.getItem("kmatrix-expand-all") === "true";
      } catch { /* ignore */ }
    }
    return false;
  });

  // Column state — persist visible columns to localStorage
  const [visibleCols, setVisibleCols] = useState<string[]>(() => {
    if (typeof window !== "undefined") {
      try {
        const saved = localStorage.getItem("kmatrix-visible-columns");
        if (saved) {
          const parsed = JSON.parse(saved);
          if (Array.isArray(parsed) && parsed.length > 0) return parsed;
        }
      } catch { /* ignore */ }
    }
    return ALL_COLUMNS.filter((c) => c.defaultVisible).map((c) => c.key);
  });
  const [columnOrder, setColumnOrder] = useState<string[]>(
    ALL_COLUMNS.map((c) => c.key)
  );
  const [columnWidths, setColumnWidths] = useState<Record<string, number>>(
    () => Object.fromEntries(ALL_COLUMNS.map((c) => [c.key, c.defaultWidth]))
  );

  // Per-column text filters
  const [columnTextFilters, setColumnTextFilters] = useState<Record<string, string>>({});
  // Per-column multi-select filters: key → Set of checked values (all checked = no filter)
  const [columnSetFilters, setColumnSetFilters] = useState<Record<string, Set<string>>>({});

  const [activeFilterCol, setActiveFilterCol] = useState<string | null>(null);
  const [filterAnchorRect, setFilterAnchorRect] = useState<DOMRect | null>(null);

  // Unique values per column — from ALL results (returned by backend), NOT just the paginated page
  const [allColumnValues, setAllColumnValues] = useState<Record<string, string[]>>({});

  const debounceTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const filterDebounceTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const filterMounted = useRef(false);
  const searchVersion = useRef(0);

  /* -- Build column filter params for backend ------------------------- */

  const buildColumnFilterParams = useCallback(() => {
    const textFilters: Record<string, string> = {};
    for (const [k, v] of Object.entries(columnTextFilters)) {
      if (v.trim()) textFilters[k] = v;
    }
    const setFilters: Record<string, string[]> = {};
    for (const [k, v] of Object.entries(columnSetFilters)) {
      // Only send if not all values are selected (i.e. it's actually filtering)
      const allVals = allColumnValues[k];
      if (allVals && v.size < allVals.length) {
        setFilters[k] = Array.from(v);
      }
    }
    const hasAny = Object.keys(textFilters).length > 0 || Object.keys(setFilters).length > 0;
    return hasAny ? { textFilters, setFilters } : undefined;
  }, [columnTextFilters, columnSetFilters, allColumnValues]);

  /* -- Search --------------------------------------------------------- */

  const doSearch = useCallback(
    async (q: string, newOffset: number, colFilters?: { textFilters?: Record<string, string>; setFilters?: Record<string, string[]> }) => {
      const version = ++searchVersion.current;
      setSearching(true);
      try {
        const res = await searchSignals(q.trim(), {}, { limit: PAGE_SIZE, offset: newOffset }, colFilters);
        if (version === searchVersion.current) {
          setResults(res.results);
          setTotalResults(res.total);
          setOffset(newOffset);
          setAllColumnValues(res.column_values ?? {});
          setError(null);
        }
      } catch (e) {
        if (version === searchVersion.current)
          setError(e instanceof Error ? e.message : "Search failed");
      } finally {
        if (version === searchVersion.current) setSearching(false);
      }
    },
    []
  );

  /* -- Load status on mount & trigger initial search ------------------- */

  useEffect(() => {
    getStatus().then((s) => {
      setMatrixCount(s.matrix_count);
      setSignalCount(s.signal_count);
    }).catch(() => {});
    // Load all signals on mount (empty query returns everything)
    doSearch("", 0);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const onQueryChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      const val = e.target.value;
      setQuery(val);
      if (debounceTimer.current) clearTimeout(debounceTimer.current);
      debounceTimer.current = setTimeout(() => doSearch(val, 0, buildColumnFilterParams()), 300);
    },
    [doSearch, buildColumnFilterParams]
  );

  /* -- Re-search when column filters change (reset to page 1) -------- */

  useEffect(() => {
    // Skip the initial render (doSearch is already called on mount)
    if (!filterMounted.current) {
      filterMounted.current = true;
      return;
    }
    if (filterDebounceTimer.current !== null) {
      clearTimeout(filterDebounceTimer.current);
    }
    filterDebounceTimer.current = setTimeout(() => {
      doSearch(query, 0, buildColumnFilterParams());
    }, 300);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [columnTextFilters, columnSetFilters]);

  /* -- Unique values per column from ALL results (backend) ------------ */

  const uniqueValuesPerColumn = allColumnValues;

  /* -- Column filtering (now handled server-side) --------------------- */

  const activeTextFilters = Object.entries(columnTextFilters).filter(([, v]) => v.trim() !== "");
  const activeSetFilters = Object.entries(columnSetFilters).filter(
    ([colKey, checked]) => checked.size < (uniqueValuesPerColumn[colKey]?.length ?? 0)
  );

  const hasAnyFilter = activeTextFilters.length > 0 || activeSetFilters.length > 0;

  /* -- Column visibility handler -------------------------------------- */

  const handleVisibleColsChange = useCallback((e: Event) => {
    const detail = (e as CustomEvent).detail;
    if (detail && Array.isArray(detail.value)) {
      const newCols = detail.value as string[];
      setVisibleCols(newCols);
      try {
        localStorage.setItem("kmatrix-visible-columns", JSON.stringify(newCols));
      } catch { /* ignore */ }
    }
  }, []);

  /* -- Column resize -------------------------------------------------- */

  const onResizeStart = useCallback(
    (colKey: string, e: RMouseEvent<HTMLSpanElement>) => {
      e.preventDefault();
      e.stopPropagation();
      const startX = e.clientX;
      const startW = columnWidths[colKey];
      const col = ALL_COLUMNS.find((c) => c.key === colKey)!;
      const onMove = (ev: globalThis.MouseEvent) => {
        const w = Math.max(col.minWidth, startW + ev.clientX - startX);
        setColumnWidths((p) => ({ ...p, [colKey]: w }));
      };
      const onUp = () => {
        document.removeEventListener("mousemove", onMove);
        document.removeEventListener("mouseup", onUp);
      };
      document.addEventListener("mousemove", onMove);
      document.addEventListener("mouseup", onUp);
    },
    [columnWidths]
  );

  /* -- Column reorder ------------------------------------------------- */

  const dragCol = useRef<string | null>(null);
  const onColDragStart = useCallback((k: string) => { dragCol.current = k; }, []);
  const onColDragOver = useCallback((e: DragEvent) => { e.preventDefault(); }, []);
  const onColDrop = useCallback((targetKey: string) => {
    if (!dragCol.current || dragCol.current === targetKey) return;
    setColumnOrder((prev) => {
      const o = [...prev];
      const from = o.indexOf(dragCol.current!);
      const to = o.indexOf(targetKey);
      o.splice(from, 1);
      o.splice(to, 0, dragCol.current!);
      return o;
    });
    dragCol.current = null;
  }, []);

  /* -- Column filter toggle ------------------------------------------- */

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

  /* -- Check if a column has an active filter ------------------------- */

  const colHasFilter = useCallback((colKey: string) => {
    if (columnTextFilters[colKey]?.trim()) return true;
    const setFilter = columnSetFilters[colKey];
    if (setFilter && setFilter.size < (uniqueValuesPerColumn[colKey]?.length ?? 0)) return true;
    return false;
  }, [columnTextFilters, columnSetFilters, uniqueValuesPerColumn]);

  /* -- Derived -------------------------------------------------------- */

  const orderedVisibleColumns = columnOrder
    .filter((k) => visibleCols.includes(k))
    .map((k) => ALL_COLUMNS.find((c) => c.key === k)!)
    .filter(Boolean);

  const totalPages = Math.ceil(totalResults / PAGE_SIZE);
  const currentPage = Math.floor(offset / PAGE_SIZE) + 1;

  // Table: fill available space, but scroll horizontally when columns exceed viewport
  const totalColWidth = orderedVisibleColumns.reduce((sum, col) => sum + columnWidths[col.key], 0);

  return (
    <div className="w-full px-8 py-6">
      {/* Header */}
      <div className="flex items-center justify-between mb-6">
        <PHeading tag="h2" size="large">
          Signal Search
        </PHeading>
        <PText color="contrast-medium">
          {matrixCount} matrices &middot; {signalCount.toLocaleString()} signals
        </PText>
      </div>

      {/* ── Search bar + controls ──────────────────────────────── */}
      <section className="mb-6">
        <div className="flex gap-4 items-end flex-wrap">
          <div className="flex-1 min-w-[300px]">
            <PTextFieldWrapper label="Search signals, messages, ECUs...">
              <input
                type="search"
                value={query}
                onChange={onQueryChange}
                placeholder="e.g. CIMU_01_CRC, Sitzheizung, 0x66..."
              />
            </PTextFieldWrapper>
          </div>

          <div className="w-[300px]">
            <PMultiSelect
              name="columns"
              label="Visible Columns"
              value={visibleCols}
              onUpdate={handleVisibleColsChange}
            >
              {ALL_COLUMNS.map((col) => (
                <PMultiSelectOption key={col.key} value={col.key}>
                  {col.label}
                </PMultiSelectOption>
              ))}
            </PMultiSelect>
          </div>

          <div className="flex items-end">
            <PSwitch
              checked={expandAll}
              alignLabel="left"
              onUpdate={(e) => {
                const val = (e as CustomEvent).detail.checked;
                setExpandAll(val);
                try { localStorage.setItem("kmatrix-expand-all", String(val)); } catch {}
              }}
            >
              Expand rows
            </PSwitch>
          </div>
        </div>

        {/* Result count + reset columns */}
        <div className="flex items-center gap-4 mt-3 flex-wrap">
          {totalResults > 0 && (
            <PText color="contrast-medium">
              {totalResults.toLocaleString()} result{totalResults === 1 ? "" : "s"}
              {totalResults > PAGE_SIZE &&
                ` \u2014 showing ${offset + 1}\u2013${Math.min(offset + PAGE_SIZE, totalResults)}`}
              {searching && " \u00b7 searching\u2026"}
            </PText>
          )}

          {hasAnyFilter && (
            <button className="text-xs text-blue-600 hover:underline"
              onClick={() => { setColumnTextFilters({}); setColumnSetFilters({}); }}>
              Clear all column filters
            </button>
          )}

          <button
            className="ml-auto text-xs text-blue-600 hover:underline"
            onClick={() => {
              const defaults = ALL_COLUMNS.filter((c) => c.defaultVisible).map((c) => c.key);
              setVisibleCols(defaults);
              try { localStorage.setItem("kmatrix-visible-columns", JSON.stringify(defaults)); } catch {}
            }}
          >
            Reset to default columns
          </button>
        </div>
      </section>

      {/* Error */}
      {error && (
        <div className="mb-4 p-3 bg-red-50 border border-red-200 rounded text-sm text-red-700">
          {error}
          <button className="ml-3 underline" onClick={() => setError(null)}>dismiss</button>
        </div>
      )}

      {searching && results.length === 0 && (
        <div className="flex justify-center py-8"><PSpinner /></div>
      )}

      {/* ── Results Table ──────────────────────────────────────── */}
      {results.length > 0 && (
        <>
          <div className="overflow-x-auto border border-gray-200 rounded-lg">
            <table className="text-sm border-collapse" style={{ tableLayout: "fixed", width: "100%", minWidth: totalColWidth }}>
              <colgroup>
                {orderedVisibleColumns.map((col) => (
                  <col key={col.key} style={{ width: columnWidths[col.key] }} />
                ))}
              </colgroup>
              <thead>
                <tr className="border-b-2 border-gray-300 text-left bg-gray-50">
                  {orderedVisibleColumns.map((col) => (
                    <th
                      key={col.key}
                      className="px-3 py-2 whitespace-nowrap select-none relative group"
                      draggable
                      onDragStart={() => onColDragStart(col.key)}
                      onDragOver={onColDragOver}
                      onDrop={() => onColDrop(col.key)}
                      style={{ width: columnWidths[col.key], cursor: "grab" }}
                    >
                      <span className="flex items-center gap-1">
                        {col.label}
                        {/* Search icon (text search) */}
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
                        {/* Filter icon (multi-select) */}
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
                      <span
                        role="separator"
                        aria-orientation="vertical"
                        tabIndex={-1}
                        className="absolute right-0 top-0 bottom-0 w-[5px] cursor-col-resize
                                   hover:bg-blue-400 opacity-0 group-hover:opacity-50"
                        onMouseDown={(e) => onResizeStart(col.key, e)}
                      />
                    </th>
                  ))}
                </tr>
              </thead>
              <tbody>
                {results.map((hit, idx) => (
                  <tr key={`${hit.matrix_id}-${hit.signal_name}-${idx}`}
                    className="border-b border-gray-100 hover:bg-gray-50">
                    {orderedVisibleColumns.map((col) => (
                      <ExpandableCell key={col.key} width={columnWidths[col.key]} forceExpand={expandAll}>
                        {col.render(hit)}
                      </ExpandableCell>
                    ))}
                  </tr>
                ))}
              </tbody>
            </table>
          </div>

          {/* Pagination */}
          {totalPages > 1 && (
            <div className="flex items-center justify-center gap-4 mt-4">
              <PButton variant="tertiary" disabled={currentPage <= 1}
                onClick={() => doSearch(query, offset - PAGE_SIZE, buildColumnFilterParams())}>
                &larr; Previous
              </PButton>
              <PText>Page {currentPage} of {totalPages}</PText>
              <PButton variant="tertiary" disabled={currentPage >= totalPages}
                onClick={() => doSearch(query, offset + PAGE_SIZE, buildColumnFilterParams())}>
                Next &rarr;
              </PButton>
            </div>
          )}
        </>
      )}

      {/* ── Column filter popup ────────────────────────────────── */}
      {activeFilterCol && filterAnchorRect && (
        <ColumnFilterPopup
          column={ALL_COLUMNS.find((c) => c.key === activeFilterCol)!}
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
    </div>
  );
}
