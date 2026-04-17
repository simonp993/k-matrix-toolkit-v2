const API_BASE = process.env.NEXT_PUBLIC_API_URL ?? "http://localhost:3001";

export interface StatusResponse {
  matrix_count: number;
  signal_count: number;
}

export interface ImportResponse {
  files_imported: number;
  total_matrices: number;
  total_signals: number;
}

export interface SearchHit {
  matrix_id: string;
  source_file: string;
  platform: string | null;
  bus_type: string;
  bus_name: string;
  message_name: string;
  identifier: string | null;
  signal_name: string;
  comment: string | null;
  description: string | null;
  init_value: string | null;
  error_value: string | null;
  min_raw: string | null;
  max_raw: string | null;
  physical_value: string | null;
  start_bit: number | null;
  bit_length: number | null;
  unit: string | null;
  offset: string | null;
  scaling: string | null;
  ecu_sender: string | null;
  ecu_receivers: string[];
}

export interface FilterCounts {
  bus_types: Record<string, number>;
  platforms: Record<string, number>;
  file_types: Record<string, number>;
}

export interface SearchResponse {
  query: string;
  total: number;
  offset: number;
  limit: number;
  results: SearchHit[];
  filter_counts: FilterCounts;
  column_values: Record<string, string[]>;
}

export interface MatrixSummary {
  id: string;
  source_file: string;
  platform: string | null;
  bus_type: string;
  bus_name: string;
  message_count: number;
  signal_count: number;
}

export async function getStatus(): Promise<StatusResponse> {
  const res = await fetch(`${API_BASE}/api/status`);
  if (!res.ok) throw new Error(`Status failed: ${res.statusText}`);
  return res.json();
}

export async function importDirectory(
  path: string
): Promise<ImportResponse> {
  const res = await fetch(`${API_BASE}/api/import`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ path }),
  });
  if (!res.ok) {
    const text = await res.text();
    throw new Error(text || res.statusText);
  }
  return res.json();
}

export async function importFiles(files: File[]): Promise<ImportResponse> {
  const formData = new FormData();
  for (const file of files) {
    formData.append("files", file);
  }
  const res = await fetch(`${API_BASE}/api/upload`, {
    method: "POST",
    body: formData,
  });
  if (!res.ok) {
    const text = await res.text();
    throw new Error(text || res.statusText);
  }
  return res.json();
}

export async function searchSignals(
  query: string,
  filters?: { platform?: string; bus?: string; bus_name?: string; file_type?: string },
  pagination?: { limit?: number; offset?: number },
  columnFilters?: { textFilters?: Record<string, string>; setFilters?: Record<string, string[]> }
): Promise<SearchResponse> {
  const params = new URLSearchParams();
  if (query) params.set("q", query);
  if (filters?.platform) params.set("platform", filters.platform);
  if (filters?.bus) params.set("bus", filters.bus);
  if (filters?.bus_name) params.set("bus_name", filters.bus_name);
  if (filters?.file_type) params.set("file_type", filters.file_type);
  if (pagination?.limit) params.set("limit", String(pagination.limit));
  if (pagination?.offset) params.set("offset", String(pagination.offset));
  if (columnFilters?.textFilters && Object.keys(columnFilters.textFilters).length > 0) {
    params.set("col_text_filters", JSON.stringify(columnFilters.textFilters));
  }
  if (columnFilters?.setFilters && Object.keys(columnFilters.setFilters).length > 0) {
    params.set("col_set_filters", JSON.stringify(columnFilters.setFilters));
  }

  const res = await fetch(`${API_BASE}/api/search?${params}`);
  if (!res.ok) throw new Error(`Search failed: ${res.statusText}`);
  return res.json();
}

export interface FiltersResponse {
  platforms: string[];
  bus_types: string[];
  file_types: string[];
}

export async function getFilters(): Promise<FiltersResponse> {
  const res = await fetch(`${API_BASE}/api/filters`);
  if (!res.ok) throw new Error(`Filters failed: ${res.statusText}`);
  return res.json();
}

export async function listMatrices(): Promise<MatrixSummary[]> {
  const res = await fetch(`${API_BASE}/api/matrices`);
  if (!res.ok) throw new Error(`Matrices failed: ${res.statusText}`);
  return res.json();
}

export async function deleteMatrix(id: string): Promise<void> {
  const res = await fetch(`${API_BASE}/api/matrices/${encodeURIComponent(id)}`, {
    method: "DELETE",
  });
  if (!res.ok) {
    const text = await res.text();
    throw new Error(text || res.statusText);
  }
}

export async function clearAllMatrices(): Promise<void> {
  const res = await fetch(`${API_BASE}/api/matrices/clear`, {
    method: "POST",
  });
  if (!res.ok) {
    const text = await res.text();
    throw new Error(text || res.statusText);
  }
}
