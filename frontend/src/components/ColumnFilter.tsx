"use client";

import { useState, useEffect, useRef } from "react";

/* ================================================================
   SVG icons
   ================================================================ */

export function SearchIcon({ className }: { className?: string }) {
  return (
    <svg className={className} width="14" height="14" viewBox="0 0 24 24"
      fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <circle cx="11" cy="11" r="8" />
      <line x1="21" y1="21" x2="16.65" y2="16.65" />
    </svg>
  );
}

export function FilterIcon({ className }: { className?: string }) {
  return (
    <svg className={className} width="14" height="14" viewBox="0 0 24 24"
      fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <polygon points="22 3 2 3 10 12.46 10 19 14 21 14 12.46 22 3" />
    </svg>
  );
}

/* ================================================================
   Column Filter Popup — text search + multi-select checkboxes
   ================================================================ */

export interface ColumnInfo {
  key: string;
  label: string;
}

export function ColumnFilterPopup({
  column,
  textValue,
  onTextChange,
  uniqueValues,
  checkedValues,
  onCheckedChange,
  onClose,
  anchorRect,
}: Readonly<{
  column: ColumnInfo;
  textValue: string;
  onTextChange: (val: string) => void;
  uniqueValues: string[];
  checkedValues: Set<string>;
  onCheckedChange: (next: Set<string>) => void;
  onClose: () => void;
  anchorRect: DOMRect;
}>) {
  const ref = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const [localSearch, setLocalSearch] = useState("");

  useEffect(() => {
    const handler = (e: globalThis.MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) onClose();
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, [onClose]);

  useEffect(() => { inputRef.current?.focus(); }, []);

  const allChecked = checkedValues.size === uniqueValues.length;

  const toggleAll = () => {
    if (allChecked) {
      onCheckedChange(new Set());
    } else {
      onCheckedChange(new Set(uniqueValues));
    }
  };

  const toggleOne = (val: string) => {
    const next = new Set(checkedValues);
    if (next.has(val)) next.delete(val); else next.add(val);
    onCheckedChange(next);
  };

  const filtered = localSearch
    ? uniqueValues.filter((v) => v.toLowerCase().includes(localSearch.toLowerCase()))
    : uniqueValues;

  const left = Math.min(anchorRect.left, window.innerWidth - 280);

  return (
    <div
      ref={ref}
      className="fixed bg-white border border-gray-300 rounded-lg shadow-xl p-3 z-50"
      style={{ top: anchorRect.bottom + 4, left, minWidth: 260, maxWidth: 320 }}
    >
      <div className="text-xs font-semibold text-gray-500 mb-2">Filter: {column.label}</div>

      <div className="relative mb-2">
        <SearchIcon className="absolute left-2 top-1/2 -translate-y-1/2 text-gray-400" />
        <input
          ref={inputRef}
          type="text"
          className="w-full border border-gray-300 rounded pl-7 pr-2 py-1.5 text-sm focus:outline-none focus:border-blue-500"
          placeholder={`Search in ${column.label}...`}
          value={textValue}
          onChange={(e) => onTextChange(e.target.value)}
          onKeyDown={(e) => { if (e.key === "Escape") onClose(); }}
        />
      </div>

      {uniqueValues.length > 0 && (
        <>
          <div className="border-t border-gray-200 pt-2 mt-1">
            {uniqueValues.length > 8 && (
              <input
                type="text"
                className="w-full border border-gray-200 rounded px-2 py-1 text-xs mb-2 focus:outline-none focus:border-blue-400"
                placeholder="Search options..."
                value={localSearch}
                onChange={(e) => setLocalSearch(e.target.value)}
              />
            )}

            <label className="flex items-center gap-2 py-1 px-1 rounded hover:bg-gray-50 cursor-pointer text-xs font-semibold text-gray-700 border-b border-gray-100 mb-1 pb-1">
              <input
                type="checkbox"
                checked={allChecked}
                onChange={toggleAll}
                className="accent-black"
              />
              Select All ({uniqueValues.length})
            </label>

            <div className="max-h-[200px] overflow-y-auto">
              {filtered.map((val) => (
                <label key={val} className="flex items-center gap-2 py-0.5 px-1 rounded hover:bg-gray-50 cursor-pointer text-xs">
                  <input
                    type="checkbox"
                    checked={checkedValues.has(val)}
                    onChange={() => toggleOne(val)}
                    className="accent-black"
                  />
                  <span className="truncate">{val || "(empty)"}</span>
                </label>
              ))}
              {filtered.length === 0 && (
                <div className="text-xs text-gray-400 py-1 px-1">No matches</div>
              )}
            </div>
          </div>
        </>
      )}

      {(textValue || checkedValues.size < uniqueValues.length) && (
        <button className="mt-2 text-xs text-blue-600 hover:underline"
          onClick={() => { onTextChange(""); onCheckedChange(new Set(uniqueValues)); onClose(); }}>
          Clear filter
        </button>
      )}
    </div>
  );
}
