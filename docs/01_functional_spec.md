# K-Matrix Toolkit v2 — Functional Specification

> **Version:** 1.0  
> **Date:** April 15, 2026  
> **Status:** Draft

---

## 1. Overview

The K-Matrix Toolkit v2 is a web-based search tool for automotive communication matrices (K-Matrices). It replaces the existing PyQt6 desktop tool with a performant Rust backend + Next.js frontend architecture using the Porsche Design System.

### 1.1 Problem Statement

| Problem (v1) | Solution (v2) |
|---|---|
| Parsing takes 10+ seconds (Python/pandas/openpyxl) | Rust-based parsing with calamine — sub-second performance |
| Results exported as a separate Excel file only | Interactive in-browser table with instant filtering |
| Only .xlsx supported, no .dbc/.ldf | Multi-format parser (xlsx, dbc, ldf, json, xml) |
| Desktop app (.dmg) must be installed | Web-based — access via browser URL |
| ECU routing panel permanently occupies space | Collapsible side panel, hidden by default |
| Rigid UI with absolute pixel positioning | Responsive Porsche Design System UI |

### 1.2 Target Users

Internal developers and engineers who need to search through K-Matrices to find signals, messages, ECU assignments, and routing information.

### 1.3 Deployment Model

- **Self-hosted** on an internal server (Docker container)
- **No authentication** required — internal network only
- **Single-user** primarily, but multi-user capable via server hosting

---

## 2. Data Sources & File Formats

### 2.1 Supported Vehicle Platforms

| Platform | Bus Types | Primary File Formats |
|---|---|---|
| E³ 1.2 (Premium) | CANFD, LIN, Ethernet (VLAN) | .xlsx, .ldf, .json |
| MLBevo 2 | CAN, LIN, FlexRay, Ethernet, MOST | .xlsx, .dbc, .ldf, .vsde |

### 2.2 Supported File Formats

| Format | Description | Parser Priority |
|---|---|---|
| **.xlsx** | Excel K-Matrices (all platforms) | P0 — Core |
| **.dbc** | Vector CAN database (MLBevo 2 CAN buses) | P0 — Core |
| **.ldf** | LIN Description Files (LIN buses) | P1 — Extended |
| **.json** | Service definitions, SwitchConfigs | P1 — Extended |
| **.xml / .xnml** | Proprietary XML formats | P2 — Future |

### 2.3 Automatic Format Detection

The system detects file formats automatically based on:
1. **File extension** (.xlsx, .dbc, .ldf, .json, .xml)
2. **Filename heuristics** (e.g. `*KMatrix*` in .xlsx filenames)
3. **Sheet/content detection** for .xlsx (checks for columns like `Botschaften`, `Signale`, `Wertebereich`)

Files not recognized as K-Matrices (e.g. comparison files, routing tables, NPM files) are automatically skipped.

---

## 3. Data Import

### 3.1 Upload Methods

Users can provide K-Matrix data in three ways:

| Method | Description |
|---|---|
| **Drag & Drop** | Drop folders or files directly into the browser window |
| **Directory path** | Enter a local or network path (like the current tool) |
| **ZIP archive** | Upload a ZIP file that is automatically extracted and parsed |

### 3.2 Recursive Directory Scanning

When a directory is provided, the system recursively scans for supported files. The directory structure is preserved as metadata (platform, bus segment, domain).

### 3.3 Metadata Extraction

The following metadata is derived from directory structure and file contents:

| Metadata | Source |
|---|---|
| **Platform** (E³ 1.2, MLBevo 2) | Directory name / filename |
| **Bus type** (CAN, CANFD, LIN, Ethernet, FlexRay) | Filename / sheet name / .dbc header |
| **Bus name** (e.g. HCP4_CANFD3, Konzern_CAN) | Filename parsing |
| **Domain** (HCP1-5) | Directory structure |
| **Source file** | Full file path |

### 3.4 Persistence

- Parsed data is **persisted at file level** (cache)
- Re-importing an already known file (verified via file size + modification date) uses the cache
- Platform/architecture metadata is stored alongside parsed data
- **Manual re-import** triggered by the user forces re-parsing (cache is invalidated)

---

## 4. Search

### 4.1 Free-Text Search (Main Feature)

- **Instant search** while typing (like VS Code Search)
- Searches across **all loaded K-Matrices**
- Searches all columns: signal name, message name (Botschaft), identifier, comment (Signalkommentar), ECU assignments, value ranges
- Results are grouped by relevance and source file

### 4.2 Search Options

| Option | Description | Default |
|---|---|---|
| **Free-text search** | Substring match across all columns | Enabled |
| **Case insensitive** | Ignore uppercase/lowercase | Enabled |
| **Exact match** | Only exact cell value matches | Disabled |
| **Platform filter** | Restrict results to a specific platform | All |
| **Bus filter** | Restrict results to a specific bus type | All |

### 4.3 Specialized Search (ECU Routing)

Available in the collapsible side panel:
- **Signal ID → Name** resolution (e.g. `0x11F` → `Aero_03`)
- **Signal Name → ID** resolution
- **Sender ECU** identification (which ECU sends the signal)
- **Routing ECU** identification (which ECUs route the signal)
- **Source bus** identification

---

## 5. Results Display

### 5.1 Interactive Results Table

Search results are displayed as an interactive table in the browser:

| Feature | Description |
|---|---|
| **Column sorting** | Every column is sortable (ascending/descending) |
| **Column filtering** | Inline filter per column |
| **Pagination** | For >100 results, configurable page size |
| **Column selection** | User can show/hide columns |
| **Row highlight** | Search term highlighted in results |
| **Source file display** | Each row shows which K-Matrix it originates from |
| **Detail view** | Click on a row to see all columns of the K-Matrix row |

### 5.2 Displayed Columns (Default)

Standard columns shown from K-Matrix Excel files (German domain names preserved):

| Column | Description |
|---|---|
| Signale | Signal name |
| Botschaft / PDU | Message name |
| Identifier [hex] / PDU-ID [hex] | Signal identifier |
| Signalkommentar | Signal description/comment |
| Sender - Empfänger | ECU assignments (sender/receiver) |
| InitWert roh [dez] | Initial raw value |
| FehlerWert roh [dez] | Error value |
| Value range (Physikalische Werte) | Min/Max/Unit/Offset/Scaling |
| Source file | K-Matrix filename |

For **.dbc** files, equivalent fields are mapped:

| DBC Field | Display Name |
|---|---|
| Message name | Botschaft |
| Message ID | Identifier [hex] |
| Signal name | Signale |
| Signal comment | Signalkommentar |
| Min/Max/Factor/Offset/Unit | Value range |
| Transmitter/Receiver | Sender - Empfänger |

### 5.3 No Excel Export (V1)

Results are displayed **only in the browser UI**. Excel/CSV export is planned for a later version but is not a priority.

---

## 6. User Interface

### 6.1 Design System

**Porsche Design System v3** (`@porsche-design-system/components-react`) — native Next.js integration with SSR support.

### 6.2 Language

- **English** — all UI text, labels, placeholders, headings
- K-Matrix column headers remain in their **original German domain language** (e.g. `Botschaft`, `Signale`, `Startbit`, `Signalkommentar`, `Wertebereich`)
- These German terms are automotive domain-specific and should not be translated

### 6.3 Layout Overview

```
┌─────────────────────────────────────────────────┐
│  K-Matrix Toolkit                          [⚙]  │
├─────────────────────────────────────────────────┤
│                                                  │
│  📂 Data Import                                  │
│  ┌──────────────────────────┐  ┌──────────┐     │
│  │ Directory / ZIP path     │  │  Load    │     │
│  └──────────────────────────┘  └──────────┘     │
│  ── or drag & drop here ──                      │
│                                                  │
│  Loaded K-Matrices: 42 files (3 platforms)       │
│                                                  │
├─────────────────────────────────────────────────┤
│                                                  │
│  🔍 Search                                       │
│  ┌──────────────────────────────────────────┐   │
│  │ Enter search term...                      │   │
│  └──────────────────────────────────────────┘   │
│  ☑ Free-text search  ☑ Case insensitive         │
│  Filter: [Platform ▾] [Bus Type ▾]              │
│                                                  │
├─────────────────────────────────────────────────┤
│                                                  │
│  Results (127 hits)                              │
│  ┌──────────────────────────────────────────┐   │
│  │ Signal │ Botschaft │ ID   │ Comment  │…  │   │
│  │────────┼───────────┼──────┼──────────┼─  │   │
│  │ Aero_03│ MSG_AERO  │ 0x1F │ Aero...  │   │   │
│  │ ...    │ ...       │ ...  │ ...      │   │   │
│  └──────────────────────────────────────────┘   │
│  < 1 2 3 ... 13 >                                │
│                                                  │
│  ┌─ ECU Routing ────────────────── [▸ expand] ──┤
│  │  (collapsed)                                  │
│  └──────────────────────────────────────────────┘
└─────────────────────────────────────────────────┘
```

### 6.4 Sections in Detail

#### 6.4.1 Data Import Section (Top)
- Text field for directory path + "Load" button
- Drag & drop zone for folders or ZIP files
- Status display: number of detected files, platforms, parse progress
- Error messages for unsupported files

#### 6.4.2 Search Section (Middle)
- Large text field for search term
- Checkboxes for search options (free-text, case insensitive)
- Dropdown filters for platform and bus type
- Real-time result counter during typing

#### 6.4.3 Results Table (Main Area)
- Interactive table (see section 5.1)
- Takes up all remaining space
- Scales responsively with the browser window

#### 6.4.4 ECU Routing Panel (Collapsible, Right Side)
- **Collapsed by default** — only a narrow strip with "ECU Routing" label visible
- When expanded: side panel (300-400px wide) with:
  - Signal ID input field
  - Signal Name input field
  - K-Matrix dropdown selection
  - "Search" button
  - Result display: Sender ECU, Routing ECUs, Source Bus
- Can also be populated by clicking a row in the results table

---

## 7. Import Flow (User Flow)

```
1. User opens the tool in the browser
2. User selects import method:
   a) Enter a directory path → click "Load"
   b) Drag & drop a folder / ZIP archive
3. System shows progress bar during parsing
4. System shows summary:
   - Number of K-Matrices found
   - Detected platform(s)
   - Detected bus types
   - Skipped / erroneous files (if any)
5. Search section and results table become active
```

## 8. Search Flow (User Flow)

```
1. User types search term into the search field
2. Results appear instantly while typing (debounce ~150ms)
3. Results table shows all hits across all loaded K-Matrices
4. User can:
   a) Sort columns (click column header)
   b) Filter columns (inline filter)
   c) Open result details (click on row)
   d) Change search options (checkboxes, filter dropdowns)
   e) Open ECU routing panel (expand side panel)
```

---

## 9. Non-Functional Requirements

| Requirement | Target |
|---|---|
| **Parse time** (50 xlsx + 40 dbc files) | < 2 seconds |
| **Search latency** (instant search) | < 100ms after input |
| **Time to first result** | < 200ms |
| **Memory usage** (server) | < 500MB for typical dataset |
| **Browser compatibility** | Chrome, Firefox, Edge (current) |
| **Responsive** | Desktop-optimized (≥1280px), functional from 1024px |

---

## 10. Out of Scope (V1)

- Excel/CSV export of search results
- Authentication / login
- Multi-user management
- .arxml parsing (AUTOSAR ECU extracts)
- K-Matrix version comparison
- SOME/IP service search
- Automatic file watching (file watcher)
- Mobile-optimized layout

---

## 11. Glossary

| Term | Description |
|---|---|
| **K-Matrix** | Communication matrix — tabular overview of all signals and messages on a bus segment |
| **Botschaft / PDU** | Protocol Data Unit — a data packet on the bus |
| **Signal** | A single value within a message |
| **ECU** | Electronic Control Unit |
| **Routing** | Forwarding a signal from one bus to another |
| **DBC** | Vector CAN Database — description file for CAN communication |
| **LDF** | LIN Description File — description file for LIN communication |
| **HCP** | High Computing Platform — central computing platform in E³ 1.2 |
| **VLAN** | Virtual LAN — Ethernet segment in the vehicle |
| **NIP** | Network Integration Package |
