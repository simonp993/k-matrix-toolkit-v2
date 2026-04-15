# K-Matrix Toolkit v2 — Testing Strategy

> **Version:** 1.0  
> **Date:** April 15, 2026

---

## 1. Testing Pyramid

```
          ┌──────────┐
          │   E2E    │  Playwright — full browser against real API
          │  (few)   │  "import dir → search → verify results"
         ─┼──────────┼─
         │ Integration │  Rust integration tests + API-level tests
         │  (medium)   │  "parse real file → assert model" / "HTTP request → JSON"
        ─┼─────────────┼─
        │   Unit Tests   │  Rust #[test] functions
        │   (many)       │  column_map, metadata, sheet_detect, search engine
        └────────────────┘
```

---

## 2. Ground Truth — Golden Test Fixtures

The core challenge: **what are the correct search results for a given file + query?**

We solve this by defining **golden fixtures** — hand-verified expected outputs for specific real K-Matrix files. These act as regression anchors.

### 2.1 Fixture: LIN XLSX — `MLBevo_SitzMem_LIN1`

**Source:** `K-Matrizen/MLBevo 2/K-Matrix/LIN/MLBevo_Konzern_MLBevo_SitzMem_LIN1_KMatrix_V8.26.00F.xlsx`

| Property | Expected Value |
|---|---|
| Platform | `MLBevo2` |
| Bus type | `LIN` |
| Sheet used | `MLBevo_SitzMem_LIN1 ` (trailing space stripped) |
| Total messages | **6** unique |
| Total data rows (signals) | **114** |
| Format | `XLSX` |

**Known signals (ground truth):**

| Message | Signal | Identifier | StartBit | BitLength | InitValue | ECU Sender | ECU Receiver | Comment |
|---|---|---|---|---|---|---|---|---|
| `IVB1s_01` | `IVB1_SSG_01_ResponseError` | `0x003` | 0 | 1 | `0` | `IVB1_FS` (S) | `SitzMem_FS` (E) | `IVB Kommunikationsfehler übertragen` |
| `IVB1s_01` | `IVB1_SSG_01_Fehlerstatus` | `0x003` | 1 | 15 | `0` | `IVB1_FS` (S) | `SitzMem_FS` (E) | `IVB Fehlerstati Bitfeld übertragen...` |
| `IVB1s_01` | `IVB1_SSG_01_PWM` | `0x003` | 0 | 7 | — | `IVB1_FS` (S) | `SitzMem_FS` (E) | `IVB Kompressor-PWM anfordern` |

### 2.2 Fixture: CAN DBC — `EFP_SUBCAN02`

**Source:** `K-Matrizen/MLBevo 2/K-Matrix/CAN/MLBevo_Gen2_MLBevo_EFP_SUBCAN02_KMatrix_V8.22.00F_20210601_SEn.dbc`

| Property | Expected Value |
|---|---|
| Platform | `MLBevo2` |
| Bus type | `CAN` |
| Total messages | **2** |
| Format | `DBC` |

**Known messages/signals (ground truth):**

| Message | ID (hex) | Transmitter | Signal | StartBit | BitLength | Factor | Offset | Receivers |
|---|---|---|---|---|---|---|---|---|
| `CIMU_01` | `0x66` | `CIMU` | `CIMU_01_CRC` | 0 | 8 | 1 | 0 | `EFP_PAG` |
| `CIMU_01` | `0x66` | `CIMU` | `CIMU_01_BZ` | 8 | 4 | 1 | 0 | `EFP_PAG` |
| `CIMU_Slave_Ident_01` | `0x96A9554C` | `CIMU` | `CIMU_Ident_01_MUX` | 0 | 4 | 1 | 0 | `EFP_PAG` |

---

## 3. Test Levels

### 3.1 Unit Tests (Rust — `#[test]`)

Located in-module or in `tests/` within `kmatrix-core`.

| Module | What is tested | Example assertion |
|---|---|---|
| `model::metadata` | Platform/bus detection from paths | `"MLBevo 2/K-Matrix/CAN/..."` → `Platform::MLBevo2, BusType::CAN` |
| `parser::xlsx::sheet_detect` | Sheet filtering logic | `["Deckblatt", "Inhalt", "MLBevo_KCAN "]` → `["MLBevo_KCAN"]` |
| `parser::xlsx::column_map` | Header row → field mapping | Row with `"Signale"` → `MappedField::SignalName` at correct index |
| `search::engine` | Index build + query | Index 3 signals → search `"CRC"` → returns `CIMU_01_CRC` |
| `search::engine` | Case insensitivity | Search `"cimu"` → matches `CIMU_01` |
| `search::engine` | Exact match mode | Search `"01"` exact → matches `CIMU_01` only if full name match |
| `routing::resolver` | ECU routing chain | Given signal + matrices → sender ECU, routing ECU, source bus |

### 3.2 Integration Tests (Rust — real files)

Located in `kmatrix-core/tests/`. Parse real K-Matrix files and assert against golden fixtures.

| Test | Input | Assertion |
|---|---|---|
| `xlsx_parse_lin_kmatrix` | LIN XLSX fixture | 6 messages, 114+ signals, `IVB1s_01` first message |
| `xlsx_golden_signal_values` | LIN XLSX fixture | `IVB1_SSG_01_ResponseError`: start_bit=0, bit_length=1, init=`"0"`, sender=`IVB1_FS`, receiver=`SitzMem_FS` |
| `xlsx_ecu_assignments` | LIN XLSX fixture | First message has IVB1_FS as Sender, SitzMem_FS as Receiver |
| `dbc_parse_can_kmatrix` | DBC fixture | 2 messages, `CIMU_01` first message |
| `dbc_golden_signal_values` | DBC fixture | `CIMU_01_CRC`: start_bit=0, bit_length=8, scaling=`"1"`, receiver=`EFP_PAG` |
| `dbc_message_identifier` | DBC fixture | `CIMU_01` → `"0x66"`, `CIMU_Slave_Ident_01` → `"0x96A9554C"` |
| `registry_parse_directory` | CAN directory | Produces multiple KMatrix results |

### 3.3 Search Integration Tests

These are the key tests for ground truth: **"given imported files + search query → expected results"**.

| Test | Setup | Query | Expected Results |
|---|---|---|---|
| `search_signal_name` | Import LIN XLSX | `"IVB1_SSG_01_PWM"` | Exactly 1 hit, signal name matches |
| `search_message_name` | Import LIN XLSX | `"IVB1s_01"` | 3+ signals (all signals in that message) |
| `search_ecu_name` | Import LIN XLSX | `"SitzMem_FS"` | All signals where SitzMem_FS is S/E |
| `search_identifier` | Import LIN XLSX | `"0x003"` | All signals of message `IVB1s_01` |
| `search_comment_substring` | Import LIN XLSX | `"Kompressor"` | Hit on `IVB1_SSG_01_PWM` (comment contains "Kompressor-PWM") |
| `search_case_insensitive` | Import LIN XLSX | `"ivb1"` | Same results as `"IVB1"` |
| `search_cross_format` | Import LIN XLSX + DBC | `"CRC"` | Hit on `CIMU_01_CRC` from DBC |
| `search_no_results` | Import LIN XLSX | `"NONEXISTENT_SIGNAL_XYZ"` | 0 hits |
| `search_platform_filter` | Import both fixtures | `"CIMU"` (platform=`MLBevo2`) | Only DBC results |
| `search_bus_filter` | Import both fixtures | `"01"` (bus=`LIN`) | Only XLSX results |

### 3.4 API Tests (Controller layer)

HTTP-level tests against the Axum server. Uses `axum::test` or `reqwest` with a test server.

| Test | Method | Endpoint | Assertion |
|---|---|---|---|
| `api_status` | GET | `/api/status` | 200, JSON with `matrix_count`, `signal_count` |
| `api_import_directory` | POST | `/api/import` | 200, returns summary with file count |
| `api_search_basic` | GET | `/api/search?q=IVB1` | 200, results array has items from LIN XLSX |
| `api_search_empty` | GET | `/api/search?q=` | 200, empty results or all results |
| `api_search_with_filter` | GET | `/api/search?q=01&bus=CAN` | 200, only CAN bus results |
| `api_matrices_list` | GET | `/api/matrices` | 200, lists all imported K-Matrices with metadata |

### 3.5 E2E Tests (Playwright)

Browser-based tests running against the full stack (Podman Compose).

| Test | Steps | Assertion |
|---|---|---|
| `e2e_import_and_search` | 1. Navigate to `/` 2. Enter fixture directory path 3. Click Import 4. Type `"CIMU"` in search 5. Verify results table | Table shows `CIMU_01_CRC`, `CIMU_01_BZ`, etc. |
| `e2e_empty_search` | 1. Import fixtures 2. Clear search | Results table is empty or shows "enter search term" |
| `e2e_column_sorting` | 1. Import 2. Search `"IVB1"` 3. Click "Signal" column header | Results sorted alphabetically by signal name |
| `e2e_ecu_routing` | 1. Import 2. Search `"IVB1_SSG_01_PWM"` 3. Open routing drawer | Drawer shows sender=`IVB1_FS`, receiver=`SitzMem_FS` |
| `e2e_platform_filter` | 1. Import MLBevo2 files 2. Select platform filter 3. Search | Only matching platform results shown |

---

## 4. Test Data Management

### 4.1 Fixture Files

**Real files** from `K-Matrizen/` are used as fixtures. They are NOT committed to the repo (too large, proprietary). Tests gracefully skip if fixtures are missing:

```rust
if !path.exists() {
    eprintln!("SKIP: fixture not found at {}", path.display());
    return;
}
```

### 4.2 Synthetic Fixtures (for CI)

For CI/CD where real K-Matrix files are unavailable, we maintain small synthetic fixtures:

```
backend/kmatrix-core/tests/fixtures/
├── synthetic_lin.xlsx     # 3 messages, 10 signals, known values
├── synthetic_can.dbc      # 2 messages, 6 signals, known values
└── README.md              # Documents the expected parse output
```

These are manually crafted files with deterministic content that CI can always run.

### 4.3 Fixture Path Configuration

```rust
/// Real fixture directory (for local development)
const REAL_FIXTURES: &str = "/Users/SP8PTW8/Code_Development/K-matrix-search-tool/K-Matrizen";

/// Synthetic fixture directory (for CI)
const SYNTHETIC_FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");

fn fixture_dir() -> &'static Path {
    let real = Path::new(REAL_FIXTURES);
    if real.exists() { real } else { Path::new(SYNTHETIC_FIXTURES) }
}
```

---

## 5. Running Tests

```bash
# Unit + integration tests (Rust)
cd backend && cargo test

# Only integration tests against real files
cd backend && cargo test --test integration_test

# API tests (requires server running or uses test harness)
cd backend && cargo test --test api_test

# E2E tests (requires full stack via Podman Compose)
cd frontend && npx playwright test

# All tests
./scripts/test_all.sh
```

---

## 6. Test Coverage Goals

| Layer | Target | Current |
|---|---|---|
| Model (parsers) | 90%+ | ~70% (Phase 1) |
| Model (search) | 95%+ | 0% (Phase 2) |
| Controller (API) | 80%+ | 0% (Phase 2) |
| View (components) | 60%+ | 0% (Phase 3) |
| E2E | Key flows | 0% (Phase 3) |
