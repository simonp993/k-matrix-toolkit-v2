# K-Matrix Toolkit v2 — Technical & Architecture Specification

> **Version:** 1.0  
> **Date:** April 15, 2026  
> **Status:** Draft  
> **Reference:** [Functional Specification](./01_functional_spec.md)

---

## 1. Architecture Overview — MVC Pattern

The application follows a **classic MVC (Model-View-Controller)** architecture split across two runtimes:

- **Model** — Rust backend: domain types, business logic, data access (parsers, search engine, cache)
- **View** — Next.js frontend: React components with Porsche Design System, presentation logic
- **Controller** — Rust Axum handlers: receive HTTP requests, delegate to model, return JSON responses

```
┌─────────────────────────────────────────────────────────────────────┐
│                                                                     │
│   VIEW  (Next.js + Porsche Design System)                          │
│                                                                     │
│   ┌───────────┐  ┌──────────┐  ┌────────────┐  ┌──────────────┐   │
│   │ Import    │  │ Search   │  │ Results    │  │ ECU Routing  │   │
│   │ Section   │  │ Bar      │  │ Table      │  │ Drawer       │   │
│   └─────┬─────┘  └────┬─────┘  └──────┬─────┘  └──────┬───────┘   │
│         │             │               │               │            │
│   ──────┴─────────────┴───────────────┴───────────────┴──────────  │
│         React Query (TanStack) — client-side state & caching       │
│                                                                     │
└─────────────────────────────┬───────────────────────────────────────┘
                              │ HTTP / JSON
┌─────────────────────────────┼───────────────────────────────────────┐
│                              │                                       │
│   CONTROLLER  (Axum Handlers)│                                       │
│                              │                                       │
│   ┌──────────────────────────┴────────────────────────────────┐     │
│   │  Router                                                   │     │
│   │  POST /api/import  → ImportController::import()           │     │
│   │  GET  /api/search  → SearchController::search()           │     │
│   │  GET  /api/routing → RoutingController::resolve()         │     │
│   │  GET  /api/status  → StatusController::status()           │     │
│   │  GET  /api/matrices→ MatrixController::list()             │     │
│   └──────────────────────────┬────────────────────────────────┘     │
│                              │                                       │
│   MODEL  (Domain + Data Access)                                     │
│                                                                      │
│   ┌────────────────┐  ┌──────────────┐  ┌──────────────────────┐   │
│   │ Domain Types   │  │ Search       │  │ Parser Registry      │   │
│   │                │  │ Engine       │  │                      │   │
│   │ KMatrix        │  │ (In-Memory   │  │ ┌──────────────────┐ │   │
│   │ Message        │  │  Inverted    │  │ │  XLSX Parser     │ │   │
│   │ Signal         │  │  Index)      │  │ │  (calamine)      │ │   │
│   │ EcuAssignment  │  │              │  │ ├──────────────────┤ │   │
│   │ Platform       │  │              │  │ │  DBC Parser      │ │   │
│   │ BusType        │  │              │  │ │  (can-dbc)       │ │   │
│   │                │  │              │  │ ├──────────────────┤ │   │
│   └────────────────┘  └──────────────┘  │ │  LDF Parser      │ │   │
│                                          │ ├──────────────────┤ │   │
│   ┌────────────────────────────────┐    │ │  JSON Parser     │ │   │
│   │ File Cache (SQLite)            │    │ └──────────────────┘ │   │
│   │ Key: path + size + mtime       │    └──────────────────────┘   │
│   └────────────────────────────────┘                                │
│                                                                      │
│   ┌──────────────────────────────────────────────────────────────┐  │
│   │  /data/ (Volume Mount) — uploaded files + cache.db           │  │
│   └──────────────────────────────────────────────────────────────┘  │
│                                                                      │
└──────────────────────────────────────────────────────────────────────┘
```

### 1.1 MVC Responsibilities

| Layer | Location | Responsibility |
|---|---|---|
| **Model** | `backend/kmatrix-core/` | Domain types (`KMatrix`, `Signal`, `Message`), parser trait + implementations, search engine, file cache, ECU routing logic. Zero knowledge of HTTP. |
| **Controller** | `backend/kmatrix-server/src/controllers/` | Receives HTTP requests, validates input, calls model layer, serializes responses as JSON. Thin — no business logic. |
| **View** | `frontend/` | Next.js + PDS React components. Fetches data from controllers via API. Handles presentation, user interaction, client-side filtering/sorting. |

### 1.2 Design Principles

- **Clean MVC separation** — Model has no HTTP dependencies, Controller has no business logic, View has no data access
- **Monorepo** — frontend and backend in a single repository
- **Single binary** — Rust backend compiles to a static binary (with embedded frontend assets)
- **Parser Registry Pattern** — each file format has a dedicated parser with a unified interface
- **In-Memory Search Index** — all parsed data lives in RAM for instant search
- **File-Level Caching** — parsed data is persisted per file in SQLite
- **Dependency injection** — Model components injected into controllers via `AppState`

---

## 2. Project Structure

```
k-matrix-toolkit-v2/
├── Containerfile
├── compose.yaml
├── README.md
│
├── old_source/                       # Original Python/PyQt6 source (reference)
│   ├── K_Matrix_Tool_APP.py
│   ├── requirements.txt
│   ├── README.md
│   ├── src/
│   └── scripts/
│
├── backend/                          # Rust (Cargo workspace)
│   ├── Cargo.toml                    # Workspace manifest
│   ├── Cargo.lock
│   │
│   ├── kmatrix-core/                 # MODEL — Domain types + business logic (lib crate)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs                # Public API surface
│   │       │
│   │       ├── model/                # Domain types
│   │       │   ├── mod.rs
│   │       │   ├── kmatrix.rs        # KMatrix, Message, Signal, EcuAssignment
│   │       │   ├── enums.rs          # Platform, BusType, FileFormat, EcuRole
│   │       │   └── metadata.rs       # Platform/bus metadata extraction from paths
│   │       │
│   │       ├── parser/               # Parser trait + implementations
│   │       │   ├── mod.rs            # KMatrixParser trait + ParserRegistry
│   │       │   ├── xlsx/             # Excel K-Matrix parser
│   │       │   │   ├── mod.rs
│   │       │   │   ├── parser.rs     # calamine-based parsing
│   │       │   │   ├── sheet_detect.rs # Sheet/column detection heuristics
│   │       │   │   └── column_map.rs # Column name → unified field mapping
│   │       │   ├── dbc/              # CAN DBC parser
│   │       │   │   ├── mod.rs
│   │       │   │   └── parser.rs     # can-dbc crate integration
│   │       │   ├── ldf/              # LIN LDF parser
│   │       │   │   ├── mod.rs
│   │       │   │   └── parser.rs
│   │       │   └── json/             # JSON service parser
│   │       │       ├── mod.rs
│   │       │       └── parser.rs
│   │       │
│   │       ├── search/               # Search engine
│   │       │   ├── mod.rs
│   │       │   ├── engine.rs         # In-memory inverted index + search
│   │       │   └── index.rs          # Indexing logic
│   │       │
│   │       ├── routing/              # ECU routing resolution
│   │       │   ├── mod.rs
│   │       │   └── resolver.rs       # Source ECU, routing ECU, source bus lookup
│   │       │
│   │       └── cache/                # File-level cache
│   │           ├── mod.rs
│   │           └── store.rs          # SQLite-backed (path + size + mtime → parsed data)
│   │
│   └── kmatrix-server/               # CONTROLLER — HTTP server (Axum)
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs               # Entry point, server startup, DI wiring
│           ├── config.rs             # Server configuration
│           ├── router.rs             # API route definitions
│           ├── state.rs              # AppState (holds Model references)
│           ├── error.rs              # HTTP error types & responses
│           │
│           └── controllers/          # Request handlers (thin — delegate to Model)
│               ├── mod.rs
│               ├── import.rs         # POST /api/import
│               ├── search.rs         # GET  /api/search?q=...
│               ├── routing.rs        # GET  /api/routing?signal=...
│               ├── status.rs         # GET  /api/status
│               └── matrices.rs       # GET  /api/matrices, GET /api/filters
│
├── frontend/                         # Next.js App
│   ├── package.json
│   ├── tsconfig.json
│   ├── next.config.ts
│   │
│   ├── src/
│   │   ├── app/
│   │   │   ├── layout.tsx            # Root layout (PDS provider)
│   │   │   ├── page.tsx              # Main page
│   │   │   └── globals.css
│   │   │
│   │   ├── components/
│   │   │   ├── ImportSection.tsx      # File upload / path input / drag & drop
│   │   │   ├── SearchBar.tsx          # Search input + options
│   │   │   ├── ResultsTable.tsx       # Interactive data table
│   │   │   ├── EcuRoutingDrawer.tsx   # Collapsible side panel
│   │   │   ├── ImportStatus.tsx       # Parse progress & summary
│   │   │   └── FilterBar.tsx          # Platform & bus-type filters
│   │   │
│   │   ├── hooks/
│   │   │   ├── useSearch.ts           # Debounced search with React Query
│   │   │   ├── useImport.ts           # File upload mutation
│   │   │   └── useRouting.ts          # ECU routing query
│   │   │
│   │   ├── lib/
│   │   │   ├── api.ts                 # API client (fetch wrapper)
│   │   │   └── types.ts              # TypeScript types matching Rust models
│   │   │
│   │   └── providers/
│   │       └── PorscheDesignSystem.tsx # PDS wrapper/provider
│   │
│   └── public/
│
└── docs/
    ├── 01_functional_spec.md
    └── 02_technical_spec.md
```

---

## 3. Backend (Rust) — Model + Controller

The backend is split into two crates following MVC:
- **`kmatrix-core`** (lib) — the **Model**: all domain types, parsers, search engine, routing logic, cache. Has zero HTTP dependencies. Can be used standalone as a library.
- **`kmatrix-server`** (bin) — the **Controller**: Axum HTTP handlers that receive requests, call into `kmatrix-core`, and return JSON. Thin layer with no business logic.

### 3.1 Technology Stack

| Component | Crate / Technology | Version | Purpose |
|---|---|---|---|
| **HTTP Server** | `axum` | 0.7+ | Async web framework |
| **Async Runtime** | `tokio` | 1.x | Async I/O |
| **XLSX Parsing** | `calamine` | 0.26+ | Fast Excel reading (10-50x faster than openpyxl) |
| **DBC Parsing** | `can-dbc` | 7.x | Vector CAN database parsing |
| **JSON** | `serde` + `serde_json` | 1.x | Serialization |
| **Cache** | `rusqlite` | 0.31+ | SQLite for file-level cache |
| **Serialization** | `serde` | 1.x | Struct ↔ JSON |
| **Error Handling** | `thiserror` + `anyhow` | — | Structured error handling |
| **Logging** | `tracing` | 0.1+ | Structured logging |
| **File Upload** | `axum-multipart` / `tower-http` | — | Multipart upload + static files |
| **CORS** | `tower-http` | — | CORS middleware |
| **Parallelism** | `rayon` | 1.x | Parallel file parsing |

### 3.2 Model — Unified Domain Types

All parsers convert their formats into a unified data model. These types live in `kmatrix-core` and have no HTTP dependencies:

```rust
/// A parsed K-Matrix file
pub struct KMatrix {
    pub id: Uuid,
    pub source_file: String,           // Filename
    pub source_path: PathBuf,          // Full path
    pub platform: Option<Platform>,    // E3_1_2, MLBevo2
    pub bus_type: BusType,             // CAN, CANFD, LIN, Ethernet, FlexRay
    pub bus_name: String,              // e.g. "HCP4_CANFD3"
    pub format: FileFormat,            // XLSX, DBC, LDF, JSON
    pub messages: Vec<Message>,
    pub parsed_at: DateTime<Utc>,
}

pub enum Platform {
    E3_1_2,
    MLBevo2,
    Unknown(String),
}

pub enum BusType {
    CAN,
    CANFD,
    LIN,
    Ethernet,
    FlexRay,
    MOST,
    Unknown(String),
}

pub enum FileFormat {
    XLSX,
    DBC,
    LDF,
    JSON,
    XML,
}

/// A message / PDU (Botschaft)
pub struct Message {
    pub name: String,                   // Botschaft / PDU name
    pub identifier: Option<String>,     // Identifier [hex] / CAN-ID
    pub signals: Vec<Signal>,
    pub ecu_assignments: Vec<EcuAssignment>,
}

/// A signal within a message
pub struct Signal {
    pub name: String,                   // Signal name (Signale)
    pub comment: Option<String>,        // Signalkommentar
    pub init_value: Option<String>,     // InitWert roh [dez]
    pub error_value: Option<String>,    // FehlerWert roh [dez]
    pub min_raw: Option<String>,        // Min Rohwert
    pub max_raw: Option<String>,        // Max Rohwert
    pub physical_value: Option<String>, // Physikalischer Wert
    pub unit: Option<String>,           // Einheit
    pub offset: Option<String>,         // Offset
    pub scaling: Option<String>,        // Skalierung
    pub raw_value: Option<String>,      // Rohwert [dez]
}

/// ECU assignment for a message
pub struct EcuAssignment {
    pub ecu_name: String,
    pub role: EcuRole,                  // Sender, Receiver, Router
}

pub enum EcuRole {
    Sender,    // "S" in K-Matrix
    Receiver,  // "E" in K-Matrix
    Router,    // "S*" in K-Matrix
}
```

### 3.3 Model — Parser Registry

```rust
/// Trait that every parser must implement
pub trait KMatrixParser: Send + Sync {
    /// Checks if this file can be processed by this parser
    fn can_parse(&self, path: &Path) -> bool;

    /// Parses the file and returns a list of K-Matrices
    /// (one .xlsx can contain multiple sheets = K-Matrices)
    fn parse(&self, path: &Path) -> Result<Vec<KMatrix>>;

    /// Returns the supported file formats
    fn supported_formats(&self) -> &[FileFormat];
}

/// Registry that manages all parsers
pub struct ParserRegistry {
    parsers: Vec<Box<dyn KMatrixParser>>,
}

impl ParserRegistry {
    pub fn new() -> Self {
        Self {
            parsers: vec![
                Box::new(XlsxParser::new()),
                Box::new(DbcParser::new()),
                Box::new(LdfParser::new()),
                Box::new(JsonParser::new()),
            ],
        }
    }

    /// Finds the appropriate parser for a file
    pub fn parse(&self, path: &Path) -> Result<Vec<KMatrix>> {
        for parser in &self.parsers {
            if parser.can_parse(path) {
                return parser.parse(path);
            }
        }
        Err(anyhow!("No parser found for {:?}", path))
    }
}
```

### 3.4 Model — XLSX Parser Column Detection

The current Python code uses fragile heuristics for column mapping. The Rust parser formalizes this:

```rust
/// Known column patterns in K-Matrix Excel files
pub struct ColumnMapping {
    /// Column name in Excel → Unified field
    mappings: Vec<(ColumnPattern, UnifiedField)>,
}

pub enum ColumnPattern {
    Exact(String),           // e.g. "Signale"
    Contains(String),        // e.g. contains "Identifier"
    Row0Contains(String),    // Value in row 0 contains string
    Row1Contains(String),    // Value in row 1 contains string
}

pub enum UnifiedField {
    SignalName,
    MessageName,
    Identifier,
    SignalComment,
    InitValue,
    ErrorValue,
    SenderReceiver(String),  // ECU name as parameter
    // ... etc.
}
```

### 3.5 Model — Search Engine

In-memory full-text search across all loaded K-Matrices (lives in `kmatrix-core::search`):

```rust
pub struct SearchEngine {
    /// All loaded K-Matrices
    matrices: Vec<KMatrix>,
    /// Inverted index: token → [(matrix_idx, message_idx, signal_idx, field)]
    index: HashMap<String, Vec<SearchHit>>,
}

pub struct SearchHit {
    pub matrix_idx: usize,
    pub message_idx: usize,
    pub signal_idx: Option<usize>,
    pub field: &'static str,
    pub score: f32,
}

impl SearchEngine {
    /// Indexes all K-Matrices for full-text search
    pub fn index(&mut self, matrices: Vec<KMatrix>) { /* ... */ }

    /// Instant search with substring matching
    pub fn search(&self, query: &str, opts: &SearchOptions) -> Vec<SearchResult> {
        // Substring match over the inverted index
        // Filters by platform/bus if specified
        // Returns results sorted by relevance
    }
}

pub struct SearchOptions {
    pub free_text: bool,
    pub case_insensitive: bool,
    pub platform_filter: Option<Platform>,
    pub bus_filter: Option<BusType>,
    pub limit: usize,
    pub offset: usize,
}
```

### 3.6 Model — File-Level Cache

```rust
/// File-level cache — prevents re-parsing of unchanged files
pub struct FileCache {
    db: rusqlite::Connection,  // SQLite DB
}

/// Cache key: combination of file path, size, and modification date
pub struct CacheKey {
    pub path: String,
    pub size: u64,
    pub modified: SystemTime,
}

impl FileCache {
    /// Checks if a file is in the cache and still valid
    pub fn get(&self, key: &CacheKey) -> Option<Vec<KMatrix>>;

    /// Stores parsed data in the cache
    pub fn put(&self, key: &CacheKey, data: &[KMatrix]);

    /// Invalidates cache for a path
    pub fn invalidate(&self, path: &str);

    /// Invalidates entire cache
    pub fn clear(&self);
}
```

### 3.7 Controller — API Endpoints

Controllers in `kmatrix-server/src/controllers/` are thin Axum handlers. Each receives the request, validates input, calls into the Model (`kmatrix-core`), and serializes the response as JSON. No business logic lives here.

| Method | Path | Description | Request | Response |
|---|---|---|---|---|
| `POST` | `/api/import` | Import K-Matrices | Multipart (files/ZIP) or JSON `{path: "..."}` | `ImportResult` |
| `GET` | `/api/search` | Full-text search | `?q=Airbag&free_text=true&case_insensitive=true&platform=E3_1_2&bus=CAN&limit=100&offset=0` | `SearchResponse` |
| `GET` | `/api/routing` | ECU routing resolution | `?signal_id=0x11F&matrix_id=...` or `?signal_name=Aero_03&matrix_id=...` | `RoutingResult` |
| `GET` | `/api/status` | Server/import status | — | `StatusResponse` |
| `POST` | `/api/reimport` | Invalidate cache & re-parse | — | `ImportResult` |
| `GET` | `/api/matrices` | List all loaded K-Matrices | — | `Vec<KMatrixSummary>` |
| `GET` | `/api/filters` | Available filter options | — | `FilterOptions` |

#### Response Types

```typescript
// TypeScript equivalents for API responses

interface ImportResult {
  total_files: number;
  parsed_files: number;
  skipped_files: number;
  errors: ImportError[];
  matrices: KMatrixSummary[];
  duration_ms: number;
}

interface SearchResponse {
  query: string;
  total_hits: number;
  results: SearchResult[];
  duration_ms: number;
}

interface SearchResult {
  matrix_id: string;
  source_file: string;
  platform: string | null;
  bus_type: string;
  bus_name: string;
  message_name: string;
  identifier: string | null;
  signal_name: string;
  signal_comment: string | null;
  init_value: string | null;
  error_value: string | null;
  min_raw: string | null;
  max_raw: string | null;
  physical_value: string | null;
  unit: string | null;
  offset: string | null;
  scaling: string | null;
  ecu_assignments: EcuAssignment[];
  matched_fields: string[];  // Highlight info for the UI
}

interface RoutingResult {
  signal_id: string | null;
  signal_name: string;
  sender_ecus: string[];
  routing_ecus: string[];
  source_buses: string[];
}

interface FilterOptions {
  platforms: string[];
  bus_types: string[];
  bus_names: string[];
}
```

---

## 4. Frontend (Next.js) — View

The View layer is a Next.js application using the Porsche Design System. It has no direct access to the Model — all data flows through the Controller's JSON API via React Query.

### 4.1 Technology Stack

| Component | Package | Purpose |
|---|---|---|
| **Framework** | Next.js 15 (App Router) | React-based web framework |
| **UI Library** | `@porsche-design-system/components-react` v3 | Porsche Design System |
| **Table** | `@tanstack/react-table` v8 | Headless table — sort, filter, paginate |
| **Data Fetching** | `@tanstack/react-query` v5 | Server-state management, caching, debouncing |
| **Language** | TypeScript 5.x | Type safety |
| **Styling** | PDS tokens + CSS Modules | Consistency with design system |

### 4.2 Component Structure

```
page.tsx
├── ImportSection
│   ├── <p-text-field-wrapper>     (Path input)
│   ├── <p-button>                 (Load button)
│   ├── DropZone                   (Drag & drop)
│   └── ImportStatus
│       ├── <p-spinner>            (During parsing)
│       └── <p-banner>             (Result summary)
│
├── SearchBar
│   ├── <p-text-field-wrapper>     (Search field)
│   ├── <p-checkbox-wrapper>       (Free-text, case insensitive)
│   └── FilterBar
│       ├── <p-select-wrapper>     (Platform filter)
│       └── <p-select-wrapper>     (Bus type filter)
│
├── ResultsTable
│   ├── <p-table>                  (PDS Table)
│   ├── @tanstack/react-table      (Logic: sort, filter, paginate)
│   ├── <p-pagination>             (Page navigation)
│   └── ResultDetailModal
│       └── <p-flyout>             (Detail view of a row)
│
└── EcuRoutingDrawer
    ├── <p-flyout>                 (Side panel)
    ├── <p-text-field-wrapper>     (Signal ID / Name)
    ├── <p-select-wrapper>         (K-Matrix selection)
    ├── <p-button>                 (Search)
    └── RoutingResult              (Sender, Router, Bus)
```

### 4.3 Instant Search (Debounce Strategy)

```typescript
// hooks/useSearch.ts
export function useSearch() {
  const [query, setQuery] = useState('');
  const debouncedQuery = useDebounce(query, 150); // 150ms debounce

  const searchResult = useQuery({
    queryKey: ['search', debouncedQuery, filters],
    queryFn: () => api.search(debouncedQuery, filters),
    enabled: debouncedQuery.length >= 2, // Minimum 2 characters
    keepPreviousData: true,              // Show old data while loading
    staleTime: 30_000,                   // 30s cache
  });

  return { query, setQuery, ...searchResult };
}
```

### 4.4 Porsche Design System Integration

```typescript
// providers/PorscheDesignSystem.tsx
import { PorscheDesignSystemProvider } from '@porsche-design-system/components-react/ssr';

export function PdsProvider({ children }: { children: React.ReactNode }) {
  return (
    <PorscheDesignSystemProvider>
      {children}
    </PorscheDesignSystemProvider>
  );
}
```

PDS components used:

| PDS Component | Usage |
|---|---|
| `<p-text-field-wrapper>` | Search field, path input, signal ID/name |
| `<p-button>` | Load, Search, Go |
| `<p-table>` | Results table |
| `<p-pagination>` | Page navigation |
| `<p-checkbox-wrapper>` | Search options |
| `<p-select-wrapper>` | Filter dropdowns, K-Matrix selection |
| `<p-flyout>` | ECU routing side panel, detail view |
| `<p-spinner>` | Loading indicator |
| `<p-banner>` | Status/error messages |
| `<p-tabs>` / `<p-tabs-item>` | Optional: tab navigation |
| `<p-heading>` | Headings |
| `<p-text>` | Description text |
| `<p-tag>` | Platform/bus badges in results |

---

## 5. Deployment

### 5.1 Containerfile (Podman)

```dockerfile
# Multi-stage build
# Stage 1: Frontend build
FROM node:20-alpine AS frontend
WORKDIR /app/frontend
COPY frontend/ .
RUN npm ci && npm run build

# Stage 2: Rust backend build
FROM rust:1.78-alpine AS backend
WORKDIR /app/backend
COPY backend/ .
RUN cargo build --release

# Stage 3: Runtime
FROM alpine:3.19
COPY --from=backend /app/backend/target/release/kmatrix-server /usr/local/bin/
COPY --from=frontend /app/frontend/out/ /app/static/
EXPOSE 3000
CMD ["kmatrix-server", "--static-dir", "/app/static", "--port", "3000"]
```

### 5.2 Podman Compose

```yaml
services:
  kmatrix-toolkit:
    build: .
    ports:
      - "3000:3000"
    volumes:
      - ./data:/app/data          # Persistent cache + uploaded files
    environment:
      - KMATRIX_DATA_DIR=/app/data
      - KMATRIX_MAX_UPLOAD_SIZE=500MB
      - RUST_LOG=info
```

### 5.3 Deployment Flow

```
1. git clone → podman compose up
2. Open browser: http://localhost:3000
3. Import K-Matrix directory
4. Search
```

---

## 6. Performance Targets & Measures

| Target | Measure |
|---|---|
| **Parse < 2s** (50 xlsx + 40 dbc) | calamine (zero-copy xlsx), can-dbc (compiled), parallel parsing with `rayon` |
| **Search < 100ms** | Inverted in-memory index, no disk I/O during search |
| **Instant typing** | 150ms debounce, `keepPreviousData`, response streaming |
| **Minimal RAM** | Only relevant columns in index, lazy-loading of detail data |
| **Fast restart** | SQLite file cache — already parsed files are not re-parsed |

---

## 7. Migration from V1

| Aspect | V1 (Python/PyQt6) | V2 (Rust/Next.js) |
|---|---|---|
| Parsing | pandas/openpyxl (slow) | calamine/can-dbc (10-50x faster) |
| Search | DataFrame iteration, ProcessPoolExecutor | In-memory inverted index |
| UI | Absolute pixel positioning, PyQt6 | Responsive Porsche Design System |
| Results | Export as .xlsx + open in Excel | Interactive table in browser |
| Distribution | .dmg file, macOS-only | Docker → browser, cross-platform |
| File formats | .xlsx only | .xlsx, .dbc, .ldf, .json |
| ECU routing | Permanently visible panel | Collapsible side panel |
| Caching | None — re-parse every time | File-level SQLite cache |

---

## 8. MVC Data Flow Examples

### 8.1 Import Flow

```
View (ImportSection.tsx)
  │  User drops folder / enters path
  │  POST /api/import {path: "/data/K-Matrizen/api_nip_v12"}
  ▼
Controller (import.rs)
  │  Validates input (path exists, not empty)
  │  Calls model: core.import(path)
  ▼
Model (kmatrix-core)
  │  1. Scans directory recursively for supported files
  │  2. Checks file cache (SQLite) for each file
  │  3. Uncached files → ParserRegistry dispatches to correct parser
  │  4. XLSX → XlsxParser, DBC → DbcParser, etc.
  │  5. Parsed KMatrix structs added to SearchEngine index
  │  6. New results written to cache
  │  7. Returns ImportResult
  ▼
Controller (import.rs)
  │  Serializes ImportResult → JSON
  ▼
View (ImportStatus.tsx)
  │  Shows "42 files parsed, 3 platforms detected"
```

### 8.2 Search Flow

```
View (SearchBar.tsx)
  │  User types "Airbag" (debounced 150ms)
  │  GET /api/search?q=Airbag&free_text=true&case_insensitive=true
  ▼
Controller (search.rs)
  │  Parses query params into SearchOptions
  │  Calls model: core.search(query, options)
  ▼
Model (kmatrix-core::search)
  │  1. Lowercase query, tokenize
  │  2. Scan inverted index for substring matches
  │  3. Filter by platform/bus if specified
  │  4. Collect & rank results
  │  5. Apply limit/offset pagination
  │  6. Returns Vec<SearchResult>
  ▼
Controller (search.rs)
  │  Wraps in SearchResponse {total_hits, results, duration_ms}
  ▼
View (ResultsTable.tsx)
  │  Renders interactive table with matched_fields highlighting
```

### 8.3 ECU Routing Flow

```
View (EcuRoutingDrawer.tsx)
  │  User enters signal ID "0x11F", selects K-Matrix
  │  GET /api/routing?signal_id=0x11F&matrix_id=abc-123
  ▼
Controller (routing.rs)
  │  Parses params, calls model: core.resolve_routing(signal, matrix_id)
  ▼
Model (kmatrix-core::routing)
  │  1. Find signal by ID in specified matrix
  │  2. Resolve signal name from ID (or vice versa)
  │  3. Scan all matrices for ECU assignments with role S/S*/E
  │  4. Returns RoutingResult {sender_ecus, routing_ecus, source_buses}
  ▼
Controller (routing.rs)
  │  Serializes RoutingResult → JSON
  ▼
View (EcuRoutingDrawer.tsx)
  │  Displays sender ECUs, routing ECUs, source buses
```

---

## 9. Implementation Phases (Proposed)

| Phase | MVC Layer | Scope | Outcome |
|---|---|---|---|
| **Phase 1** | Model | `kmatrix-core`: unified types + XLSX parser + DBC parser + parser registry | Parseable K-Matrices as JSON, tested against real files |
| **Phase 2** | Model + Controller | Search engine + Axum server + controllers + file cache | Working API: import, search, status |
| **Phase 3** | View | Next.js frontend + PDS + import section + search + results table | Complete UI connected to API |
| **Phase 4** | Model + View | ECU routing resolver + routing drawer + filters | Feature parity+ |
| **Phase 5** | All | LDF/JSON parsers + Docker deployment + polish | Production-ready |
