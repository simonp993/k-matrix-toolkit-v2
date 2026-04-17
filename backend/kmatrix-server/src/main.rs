//! kmatrix-server — Controller layer for the K-Matrix Toolkit.
//!
//! Axum HTTP server exposing a REST API for importing K-Matrices
//! and searching across them. State is persisted to disk.

use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use axum::extract::{DefaultBodyLimit, Multipart, Path as AxumPath, Query, State};
use axum::http::StatusCode;
use axum::response::Json;
use axum::routing::{delete, get, post};
use axum::Router;
use serde::{Deserialize, Serialize};
use tower_http::cors::CorsLayer;
use tracing_subscriber::EnvFilter;

use kmatrix_core::{build_index, search, KMatrix, ParserRegistry, SearchFilter, SearchHit};

/// Persistent data file location.
fn data_path() -> PathBuf {
    let dir = dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("kmatrix-toolkit");
    std::fs::create_dir_all(&dir).ok();
    dir.join("matrices.json")
}

/// Shared application state.
struct AppState {
    matrices: RwLock<Vec<KMatrix>>,
    index: RwLock<Vec<SearchHit>>,
    /// Where to persist state. `None` disables persistence (used in tests).
    persist_path: Option<PathBuf>,
}

impl AppState {
    /// Save current matrices to disk.
    fn persist(&self) {
        let Some(ref path) = self.persist_path else {
            return;
        };
        let matrices = self.matrices.read().unwrap();
        if let Ok(json) = serde_json::to_vec(&*matrices) {
            if let Err(e) = std::fs::write(path, json) {
                tracing::warn!("Failed to persist state: {e}");
            } else {
                tracing::info!("Persisted {} matrices to {}", matrices.len(), path.display());
            }
        }
    }

    /// Load matrices from disk if file exists.
    fn load() -> Vec<KMatrix> {
        let path = data_path();
        if path.exists() {
            match std::fs::read(&path) {
                Ok(data) => match serde_json::from_slice::<Vec<KMatrix>>(&data) {
                    Ok(matrices) => {
                        tracing::info!("Loaded {} matrices from {}", matrices.len(), path.display());
                        return matrices;
                    }
                    Err(e) => tracing::warn!("Failed to parse saved state: {e}"),
                },
                Err(e) => tracing::warn!("Failed to read saved state: {e}"),
            }
        }
        Vec::new()
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("info".parse().unwrap()))
        .init();

    let loaded = AppState::load();
    let index = build_index(&loaded);

    let state = Arc::new(AppState {
        matrices: RwLock::new(loaded),
        index: RwLock::new(index),
        persist_path: Some(data_path()),
    });

    let app = build_router(state);

    let addr = "0.0.0.0:3001";
    tracing::info!("K-Matrix Toolkit server listening on {addr}");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

/// Build the application router with the given shared state.
fn build_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/api/status", get(status))
        .route("/api/import", post(import_directory))
        .route("/api/upload", post(upload_files))
        .route("/api/search", get(search_handler))
        .route("/api/filters", get(filters_handler))
        .route("/api/matrices", get(list_matrices))
        .route("/api/matrices/:id", delete(delete_matrix))
        .route("/api/matrices/clear", post(clear_matrices))
        .layer(DefaultBodyLimit::max(500 * 1024 * 1024)) // 500 MB
        .layer(CorsLayer::permissive())
        .with_state(state)
}

// ── Status ──────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize)]
struct StatusResponse {
    matrix_count: usize,
    signal_count: usize,
}

async fn status(State(state): State<Arc<AppState>>) -> Json<StatusResponse> {
    let matrices = state.matrices.read().unwrap();
    let signal_count: usize = matrices
        .iter()
        .flat_map(|m| &m.messages)
        .map(|msg| msg.signals.len())
        .sum();
    Json(StatusResponse {
        matrix_count: matrices.len(),
        signal_count,
    })
}

// ── Import ──────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct ImportRequest {
    path: String,
}

#[derive(Serialize, Deserialize)]
struct ImportResponse {
    files_imported: usize,
    total_matrices: usize,
    total_signals: usize,
}

async fn import_directory(
    State(state): State<Arc<AppState>>,
    Json(body): Json<ImportRequest>,
) -> Result<Json<ImportResponse>, (StatusCode, String)> {
    let path_str = body.path.trim().trim_matches(|c: char| c == '\'' || c == '"');
    let p = PathBuf::from(path_str);

    if !p.exists() {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("Path does not exist: {}", path_str),
        ));
    }

    let registry = ParserRegistry::new();
    let new_matrices = if p.is_dir() {
        registry
            .parse_directory(&p)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    } else if p.is_file() {
        registry
            .parse(&p)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    } else {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("Not a file or directory: {}", path_str),
        ));
    };

    let mut matrices = state.matrices.write().unwrap();

    // Deduplicate: skip files whose source_file already exists
    let existing_files: std::collections::HashSet<String> = matrices
        .iter()
        .map(|m| m.source_file.clone())
        .collect();
    let deduped: Vec<KMatrix> = new_matrices
        .into_iter()
        .filter(|m| !existing_files.contains(&m.source_file))
        .collect();
    let files_imported = deduped.len();
    matrices.extend(deduped);

    // Rebuild the flat index
    let new_index = build_index(&matrices);
    let total_signals: usize = matrices
        .iter()
        .flat_map(|m| &m.messages)
        .map(|msg| msg.signals.len())
        .sum();
    let total_matrices = matrices.len();
    drop(matrices);

    let mut index = state.index.write().unwrap();
    *index = new_index;

    state.persist();

    Ok(Json(ImportResponse {
        files_imported,
        total_matrices,
        total_signals,
    }))
}

// ── Search ──────────────────────────────────────────────────────────

// ── Upload Files ────────────────────────────────────────────────────

const MAX_UPLOAD_SIZE: usize = 500 * 1024 * 1024; // 500 MB total

fn is_supported_matrix_file(file_name: &str) -> bool {
    let lower = file_name.to_lowercase();
    lower.ends_with(".xlsx") || lower.ends_with(".dbc") || lower.ends_with(".ldf")
}

fn is_supported_upload_file(file_name: &str) -> bool {
    is_supported_matrix_file(file_name) || file_name.to_lowercase().ends_with(".zip")
}

fn add_to_total_size(total_size: &mut usize, additional_bytes: usize) -> Result<(), (StatusCode, String)> {
    *total_size = total_size
        .checked_add(additional_bytes)
        .ok_or_else(|| (StatusCode::PAYLOAD_TOO_LARGE, "Upload exceeds 500 MB limit".to_string()))?;
    if *total_size > MAX_UPLOAD_SIZE {
        return Err((
            StatusCode::PAYLOAD_TOO_LARGE,
            "Upload exceeds 500 MB limit".to_string(),
        ));
    }
    Ok(())
}

fn extract_zip_archive(
    zip_bytes: &[u8],
    dest_root: &Path,
    total_size: &mut usize,
) -> Result<usize, (StatusCode, String)> {
    use std::io::{Cursor, Read};

    let cursor = Cursor::new(zip_bytes);
    let mut archive = zip::ZipArchive::new(cursor)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid zip archive: {e}")))?;

    let mut extracted_count = 0usize;

    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| (StatusCode::BAD_REQUEST, format!("Failed reading zip entry: {e}")))?;

        if entry.is_dir() {
            continue;
        }

        let Some(enclosed_name) = entry.enclosed_name().map(|p| p.to_owned()) else {
            continue;
        };

        let entry_name = enclosed_name.to_string_lossy();
        if !is_supported_matrix_file(&entry_name) {
            continue;
        }

        let entry_size: usize = entry.size().try_into().map_err(|_| {
            (
                StatusCode::PAYLOAD_TOO_LARGE,
                "Zip entry is too large".to_string(),
            )
        })?;
        add_to_total_size(total_size, entry_size)?;

        let dest_path = dest_root.join(enclosed_name);
        if let Some(parent) = dest_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        }

        let mut out_file = std::fs::File::create(&dest_path)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        std::io::copy(&mut entry.by_ref().take(entry_size as u64), &mut out_file)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        extracted_count += 1;
    }

    Ok(extracted_count)
}

async fn upload_files(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Result<Json<ImportResponse>, (StatusCode, String)> {
    let temp_dir = tempfile::tempdir()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let mut total_size: usize = 0;
    let mut upload_item_count: usize = 0;
    let mut supported_file_count: usize = 0;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?
    {
        let file_name = field
            .file_name()
            .map(|s| s.to_string())
            .unwrap_or_default();

        // Only accept supported formats
        if !is_supported_upload_file(&file_name) {
            continue;
        }

        let data = field
            .bytes()
            .await
            .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

        upload_item_count += 1;

        if file_name.to_lowercase().ends_with(".zip") {
            let extracted = extract_zip_archive(&data, temp_dir.path(), &mut total_size)?;
            supported_file_count += extracted;
            continue;
        }
        add_to_total_size(&mut total_size, data.len())?;

        // Sanitize filename: only keep the basename, no path traversal
        let safe_name = std::path::Path::new(&file_name)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| format!("upload_{upload_item_count}"));

        let dest = temp_dir.path().join(&safe_name);
        tokio::fs::write(&dest, &data)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        supported_file_count += 1;
    }

    if supported_file_count == 0 {
        return Err((
            StatusCode::BAD_REQUEST,
            "No supported files uploaded (.xlsx, .dbc, .ldf, .zip)".to_string(),
        ));
    }

    // Parse the uploaded files
    let registry = ParserRegistry::new();
    let new_matrices = registry
        .parse_directory(temp_dir.path())
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let mut matrices = state.matrices.write().unwrap();

    // Deduplicate: skip files whose source_file already exists
    let existing_files: std::collections::HashSet<String> = matrices
        .iter()
        .map(|m| m.source_file.clone())
        .collect();
    let deduped: Vec<KMatrix> = new_matrices
        .into_iter()
        .filter(|m| !existing_files.contains(&m.source_file))
        .collect();
    let files_imported = deduped.len();
    matrices.extend(deduped);

    let new_index = build_index(&matrices);
    let total_signals: usize = matrices
        .iter()
        .flat_map(|m| &m.messages)
        .map(|msg| msg.signals.len())
        .sum();
    let total_matrices = matrices.len();
    drop(matrices);

    let mut index = state.index.write().unwrap();
    *index = new_index;

    state.persist();

    Ok(Json(ImportResponse {
        files_imported,
        total_matrices,
        total_signals,
    }))
}

// ── Search (continued) ─────────────────────────────────────────────

#[derive(Deserialize)]
struct SearchQuery {
    q: Option<String>,
    platform: Option<String>,
    bus: Option<String>,
    bus_name: Option<String>,
    file_type: Option<String>,
    limit: Option<usize>,
    offset: Option<usize>,
}

#[derive(Serialize, Deserialize)]
struct FilterCounts {
    bus_types: std::collections::HashMap<String, usize>,
    platforms: std::collections::HashMap<String, usize>,
    file_types: std::collections::HashMap<String, usize>,
}

#[derive(Serialize, Deserialize)]
struct SearchResponse {
    query: String,
    total: usize,
    offset: usize,
    limit: usize,
    results: Vec<SearchHit>,
    filter_counts: FilterCounts,
    column_values: std::collections::HashMap<String, Vec<String>>,
}

async fn search_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<SearchQuery>,
) -> Json<SearchResponse> {
    let q = params.q.unwrap_or_default();
    let filter = SearchFilter {
        platform: params.platform,
        bus_type: params.bus,
        bus_name: params.bus_name,
        file_type: params.file_type,
    };
    let limit = params.limit.unwrap_or(200).min(1000);
    let offset = params.offset.unwrap_or(0);

    let index = state.index.read().unwrap();
    let all_results = search(&index, &q, &filter);
    let total = all_results.len();

    // Compute filter counts and unique column values from ALL matching results (before pagination)
    let mut bus_type_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut platform_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut file_type_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

    // Collect unique values per column for filter popups
    use std::collections::{BTreeSet, HashMap};
    let mut col_vals: HashMap<&str, BTreeSet<String>> = HashMap::new();
    let col_keys = [
        "signal_name", "message_name", "identifier", "bus_type", "bus_name",
        "file_type", "start_bit", "bit_length", "ecu_sender", "ecu_receivers",
        "comment", "description", "init_value", "error_value", "min_raw",
        "max_raw", "physical_value", "unit", "offset", "scaling", "source_file",
    ];
    for key in &col_keys {
        col_vals.insert(key, BTreeSet::new());
    }

    for hit in &all_results {
        let bt = hit.bus_type.clone();
        if !bt.is_empty() {
            *bus_type_counts.entry(bt.clone()).or_insert(0) += 1;
            col_vals.get_mut("bus_type").unwrap().insert(bt);
        }
        if let Some(ref p) = hit.platform {
            *platform_counts.entry(p.clone()).or_insert(0) += 1;
        }
        if let Some(ext) = hit.source_file.rsplit('.').next() {
            let ext_lower = ext.to_lowercase();
            *file_type_counts.entry(ext_lower.clone()).or_insert(0) += 1;
            col_vals.get_mut("file_type").unwrap().insert(ext_lower);
        }

        // Populate all column values
        col_vals.get_mut("signal_name").unwrap().insert(hit.signal_name.clone());
        col_vals.get_mut("message_name").unwrap().insert(hit.message_name.clone());
        if let Some(ref v) = hit.identifier { col_vals.get_mut("identifier").unwrap().insert(v.clone()); }
        if !hit.bus_name.is_empty() { col_vals.get_mut("bus_name").unwrap().insert(hit.bus_name.clone()); }
        if let Some(v) = hit.start_bit { col_vals.get_mut("start_bit").unwrap().insert(v.to_string()); }
        if let Some(v) = hit.bit_length { col_vals.get_mut("bit_length").unwrap().insert(v.to_string()); }
        if let Some(ref v) = hit.ecu_sender { col_vals.get_mut("ecu_sender").unwrap().insert(v.clone()); }
        if !hit.ecu_receivers.is_empty() { col_vals.get_mut("ecu_receivers").unwrap().insert(hit.ecu_receivers.join(", ")); }
        if let Some(ref v) = hit.comment { if !v.is_empty() { col_vals.get_mut("comment").unwrap().insert(v.clone()); } }
        if let Some(ref v) = hit.description { if !v.is_empty() { col_vals.get_mut("description").unwrap().insert(v.clone()); } }
        if let Some(ref v) = hit.init_value { col_vals.get_mut("init_value").unwrap().insert(v.clone()); }
        if let Some(ref v) = hit.error_value { col_vals.get_mut("error_value").unwrap().insert(v.clone()); }
        if let Some(ref v) = hit.min_raw { col_vals.get_mut("min_raw").unwrap().insert(v.clone()); }
        if let Some(ref v) = hit.max_raw { col_vals.get_mut("max_raw").unwrap().insert(v.clone()); }
        if let Some(ref v) = hit.physical_value { col_vals.get_mut("physical_value").unwrap().insert(v.clone()); }
        if let Some(ref v) = hit.unit { col_vals.get_mut("unit").unwrap().insert(v.clone()); }
        if let Some(ref v) = hit.offset { col_vals.get_mut("offset").unwrap().insert(v.clone()); }
        if let Some(ref v) = hit.scaling { col_vals.get_mut("scaling").unwrap().insert(v.clone()); }
        col_vals.get_mut("source_file").unwrap().insert(hit.source_file.clone());
    }

    let column_values: HashMap<String, Vec<String>> = col_vals
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.into_iter().collect()))
        .collect();

    let page: Vec<SearchHit> = all_results.into_iter().skip(offset).take(limit).collect();

    Json(SearchResponse {
        query: q,
        total,
        offset,
        limit,
        results: page,
        filter_counts: FilterCounts {
            bus_types: bus_type_counts,
            platforms: platform_counts,
            file_types: file_type_counts,
        },
        column_values,
    })
}

// ── Filters ─────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize)]
struct FiltersResponse {
    platforms: Vec<String>,
    bus_types: Vec<String>,
    file_types: Vec<String>,
}

async fn filters_handler(State(state): State<Arc<AppState>>) -> Json<FiltersResponse> {
    let matrices = state.matrices.read().unwrap();
    let mut platforms: Vec<String> = matrices
        .iter()
        .filter_map(|m| m.platform.as_ref().map(|p| p.to_string()))
        .collect();
    platforms.sort();
    platforms.dedup();

    let mut bus_types: Vec<String> = matrices.iter()
        .map(|m| m.bus_type.to_string())
        .filter(|s| !s.is_empty())
        .collect();
    bus_types.sort();
    bus_types.dedup();

    let mut file_types: Vec<String> = matrices
        .iter()
        .filter_map(|m| {
            m.source_file
                .rsplit('.')
                .next()
                .map(|ext| ext.to_lowercase())
        })
        .collect();
    file_types.sort();
    file_types.dedup();

    Json(FiltersResponse {
        platforms,
        bus_types,
        file_types,
    })
}

// ── List Matrices ───────────────────────────────────────────────────

#[derive(Serialize, Deserialize)]
struct MatrixSummary {
    id: String,
    source_file: String,
    platform: Option<String>,
    bus_type: String,
    bus_name: String,
    message_count: usize,
    signal_count: usize,
}

async fn list_matrices(State(state): State<Arc<AppState>>) -> Json<Vec<MatrixSummary>> {
    let matrices = state.matrices.read().unwrap();
    let summaries: Vec<MatrixSummary> = matrices
        .iter()
        .map(|km| MatrixSummary {
            id: km.id.to_string(),
            source_file: km.source_file.clone(),
            platform: km.platform.as_ref().map(|p| p.to_string()),
            bus_type: km.bus_type.to_string(),
            bus_name: km.bus_name.clone(),
            message_count: km.messages.len(),
            signal_count: km.messages.iter().map(|m| m.signals.len()).sum(),
        })
        .collect();
    Json(summaries)
}

// ── Delete Matrix ───────────────────────────────────────────────────

async fn delete_matrix(
    State(state): State<Arc<AppState>>,
    AxumPath(id): AxumPath<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let target_id: uuid::Uuid = id
        .parse()
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid UUID".to_string()))?;

    let mut matrices = state.matrices.write().unwrap();
    let before = matrices.len();
    matrices.retain(|m| m.id != target_id);
    let removed = before - matrices.len();

    if removed == 0 {
        return Err((StatusCode::NOT_FOUND, "Matrix not found".to_string()));
    }

    let new_index = build_index(&matrices);
    drop(matrices);

    let mut index = state.index.write().unwrap();
    *index = new_index;

    state.persist();

    Ok(Json(serde_json::json!({ "removed": removed })))
}

// ── Clear All Matrices ──────────────────────────────────────────────

async fn clear_matrices(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let mut matrices = state.matrices.write().unwrap();
    let count = matrices.len();
    matrices.clear();
    drop(matrices);

    let mut index = state.index.write().unwrap();
    index.clear();

    state.persist();

    Json(serde_json::json!({ "cleared": count }))
}

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    use std::io::Write;

    use axum::body::Body;
    use axum::http::{Request, StatusCode as AxumStatusCode};
    use bytes::Bytes;
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    use chrono::Utc;
    use kmatrix_core::{
        BusType, EcuAssignment, EcuRole, FileFormat, KMatrix, Message, Platform, Signal,
    };

    /// Create a test AppState with no persistence and pre-populated data.
    fn test_state(matrices: Vec<KMatrix>) -> Arc<AppState> {
        let index = build_index(&matrices);
        Arc::new(AppState {
            matrices: RwLock::new(matrices),
            index: RwLock::new(index),
            persist_path: None,
        })
    }

    /// Build a test KMatrix with known data.
    fn make_test_matrix() -> KMatrix {
        KMatrix {
            id: uuid::Uuid::new_v4(),
            source_file: "test_matrix.dbc".into(),
            source_path: "/tmp/test_matrix.dbc".into(),
            platform: Some(Platform::MLBevo2),
            bus_type: BusType::CAN,
            bus_name: "TEST_CAN01".into(),
            format: FileFormat::DBC,
            messages: vec![Message {
                name: "TestMsg_01".into(),
                identifier: Some("0x100".into()),
                signals: vec![
                    Signal {
                        name: "SIG_CRC".into(),
                        comment: Some("CRC checksum".into()),
                        description: None,
                        init_value: Some("0".into()),
                        error_value: None,
                        min_raw: Some("0".into()),
                        max_raw: Some("255".into()),
                        physical_value: None,
                        unit: None,
                        offset: Some("0".into()),
                        scaling: Some("1".into()),
                        raw_value: None,
                        start_bit: Some(0),
                        bit_length: Some(8),
                    },
                    Signal {
                        name: "SIG_Speed".into(),
                        comment: Some("Vehicle speed".into()),
                        description: None,
                        init_value: None,
                        error_value: None,
                        min_raw: Some("0".into()),
                        max_raw: Some("65535".into()),
                        physical_value: None,
                        unit: Some("km/h".into()),
                        offset: Some("0".into()),
                        scaling: Some("0.01".into()),
                        raw_value: None,
                        start_bit: Some(8),
                        bit_length: Some(16),
                    },
                ],
                ecu_assignments: vec![
                    EcuAssignment {
                        ecu_name: "ECU_A".into(),
                        role: EcuRole::Sender,
                    },
                    EcuAssignment {
                        ecu_name: "ECU_B".into(),
                        role: EcuRole::Receiver,
                    },
                ],
            }],
            parsed_at: Utc::now(),
        }
    }

    const MINIMAL_DBC: &str = r#"
VERSION ""

NS_ :

BS_:

BU_: ECU1 ECU2

BO_ 256 TestMsg: 8 ECU1
 SG_ CRC_Signal : 0|8@1+ (1,0) [0|255] "unit" ECU2
 SG_ Counter_Signal : 8|4@1+ (1,0) [0|15] "" ECU1,ECU2

"#;

    fn build_zip_with_files(entries: &[(&str, &str)]) -> Vec<u8> {
        let mut cursor = std::io::Cursor::new(Vec::<u8>::new());
        {
            let mut zip = zip::ZipWriter::new(&mut cursor);
            let options = zip::write::SimpleFileOptions::default();
            for (name, contents) in entries {
                zip.start_file(name, options).unwrap();
                zip.write_all(contents.as_bytes()).unwrap();
            }
            zip.finish().unwrap();
        }
        cursor.into_inner()
    }

    /// Send a GET request and return the response body bytes.
    async fn get_body(app: Router, uri: &str) -> (AxumStatusCode, Bytes) {
        let resp = app
            .oneshot(
                Request::builder()
                    .uri(uri)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = resp.status();
        let body = resp.into_body().collect().await.unwrap().to_bytes();
        (status, body)
    }

    /// Send a POST request with JSON body and return the response.
    async fn post_json(app: Router, uri: &str, json: &str) -> (AxumStatusCode, Bytes) {
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(uri)
                    .header("content-type", "application/json")
                    .body(Body::from(json.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = resp.status();
        let body = resp.into_body().collect().await.unwrap().to_bytes();
        (status, body)
    }

    /// Send a POST request with multipart/form-data body and return the response.
    async fn post_multipart(
        app: Router,
        uri: &str,
        body: Vec<u8>,
        boundary: &str,
    ) -> (AxumStatusCode, Bytes) {
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(uri)
                    .header(
                        "content-type",
                        format!("multipart/form-data; boundary={boundary}"),
                    )
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = resp.status();
        let body = resp.into_body().collect().await.unwrap().to_bytes();
        (status, body)
    }

    /// Send a DELETE request.
    async fn delete_req(app: Router, uri: &str) -> (AxumStatusCode, Bytes) {
        let resp = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(uri)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = resp.status();
        let body = resp.into_body().collect().await.unwrap().to_bytes();
        (status, body)
    }

    // ── GET /api/status ─────────────────────────────────────────────

    #[tokio::test]
    async fn status_empty() {
        let state = test_state(vec![]);
        let app = build_router(state);
        let (status, body) = get_body(app, "/api/status").await;

        assert_eq!(status, AxumStatusCode::OK);
        let resp: StatusResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(resp.matrix_count, 0);
        assert_eq!(resp.signal_count, 0);
    }

    #[tokio::test]
    async fn status_with_data() {
        let state = test_state(vec![make_test_matrix()]);
        let app = build_router(state);
        let (status, body) = get_body(app, "/api/status").await;

        assert_eq!(status, AxumStatusCode::OK);
        let resp: StatusResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(resp.matrix_count, 1);
        assert_eq!(resp.signal_count, 2);
    }

    // ── GET /api/filters ────────────────────────────────────────────

    #[tokio::test]
    async fn filters_empty() {
        let state = test_state(vec![]);
        let app = build_router(state);
        let (status, body) = get_body(app, "/api/filters").await;

        assert_eq!(status, AxumStatusCode::OK);
        let resp: FiltersResponse = serde_json::from_slice(&body).unwrap();
        assert!(resp.platforms.is_empty());
        assert!(resp.bus_types.is_empty());
        assert!(resp.file_types.is_empty());
    }

    #[tokio::test]
    async fn filters_with_data() {
        let state = test_state(vec![make_test_matrix()]);
        let app = build_router(state);
        let (status, body) = get_body(app, "/api/filters").await;

        assert_eq!(status, AxumStatusCode::OK);
        let resp: FiltersResponse = serde_json::from_slice(&body).unwrap();
        assert!(resp.platforms.contains(&"MLBevo 2".to_string()));
        assert!(resp.bus_types.contains(&"CAN".to_string()));
        assert!(resp.file_types.contains(&"dbc".to_string()));
    }

    // ── GET /api/matrices ───────────────────────────────────────────

    #[tokio::test]
    async fn list_matrices_empty() {
        let state = test_state(vec![]);
        let app = build_router(state);
        let (status, body) = get_body(app, "/api/matrices").await;

        assert_eq!(status, AxumStatusCode::OK);
        let resp: Vec<MatrixSummary> = serde_json::from_slice(&body).unwrap();
        assert!(resp.is_empty());
    }

    #[tokio::test]
    async fn list_matrices_with_data() {
        let state = test_state(vec![make_test_matrix()]);
        let app = build_router(state);
        let (status, body) = get_body(app, "/api/matrices").await;

        assert_eq!(status, AxumStatusCode::OK);
        let resp: Vec<MatrixSummary> = serde_json::from_slice(&body).unwrap();
        assert_eq!(resp.len(), 1);
        assert_eq!(resp[0].source_file, "test_matrix.dbc");
        assert_eq!(resp[0].bus_type, "CAN");
        assert_eq!(resp[0].bus_name, "TEST_CAN01");
        assert_eq!(resp[0].message_count, 1);
        assert_eq!(resp[0].signal_count, 2);
    }

    // ── GET /api/search ─────────────────────────────────────────────

    #[tokio::test]
    async fn search_empty_query_returns_all() {
        let state = test_state(vec![make_test_matrix()]);
        let app = build_router(state);
        let (status, body) = get_body(app, "/api/search").await;

        assert_eq!(status, AxumStatusCode::OK);
        let resp: SearchResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(resp.total, 2); // 2 signals
        assert_eq!(resp.results.len(), 2);
    }

    #[tokio::test]
    async fn search_by_signal_name() {
        let state = test_state(vec![make_test_matrix()]);
        let app = build_router(state);
        let (status, body) = get_body(app, "/api/search?q=CRC").await;

        assert_eq!(status, AxumStatusCode::OK);
        let resp: SearchResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(resp.total, 1);
        assert_eq!(resp.results[0].signal_name, "SIG_CRC");
    }

    #[tokio::test]
    async fn search_case_insensitive() {
        let state = test_state(vec![make_test_matrix()]);
        let app = build_router(state);
        let (_, body) = get_body(app, "/api/search?q=speed").await;

        let resp: SearchResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(resp.total, 1);
        assert_eq!(resp.results[0].signal_name, "SIG_Speed");
    }

    #[tokio::test]
    async fn search_pagination() {
        let state = test_state(vec![make_test_matrix()]);
        let app = build_router(state);
        let (_, body) = get_body(app, "/api/search?limit=1&offset=0").await;

        let resp: SearchResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(resp.total, 2); // total is 2
        assert_eq!(resp.results.len(), 1); // but only 1 returned
        assert_eq!(resp.limit, 1);
        assert_eq!(resp.offset, 0);
    }

    #[tokio::test]
    async fn search_pagination_offset() {
        let state = test_state(vec![make_test_matrix()]);
        let app = build_router(state);
        let (_, body) = get_body(app, "/api/search?limit=1&offset=1").await;

        let resp: SearchResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(resp.total, 2);
        assert_eq!(resp.results.len(), 1);
        assert_eq!(resp.offset, 1);
    }

    #[tokio::test]
    async fn search_with_bus_filter() {
        let state = test_state(vec![make_test_matrix()]);
        let app = build_router(state);
        // Filter for LIN — should return nothing (test data is CAN)
        let (_, body) = get_body(app, "/api/search?bus=LIN").await;
        let resp: SearchResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(resp.total, 0);
    }

    #[tokio::test]
    async fn search_filter_counts() {
        let state = test_state(vec![make_test_matrix()]);
        let app = build_router(state);
        let (_, body) = get_body(app, "/api/search").await;

        let resp: SearchResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(*resp.filter_counts.bus_types.get("CAN").unwrap_or(&0), 2);
        assert_eq!(
            *resp.filter_counts.platforms.get("MLBevo 2").unwrap_or(&0),
            2
        );
    }

    #[tokio::test]
    async fn search_no_results() {
        let state = test_state(vec![make_test_matrix()]);
        let app = build_router(state);
        let (_, body) = get_body(app, "/api/search?q=NONEXISTENT_SIGNAL_XYZ").await;

        let resp: SearchResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(resp.total, 0);
        assert!(resp.results.is_empty());
    }

    // ── DELETE /api/matrices/:id ────────────────────────────────────

    #[tokio::test]
    async fn delete_matrix_success() {
        let km = make_test_matrix();
        let km_id = km.id.to_string();
        let state = test_state(vec![km]);
        let app = build_router(state);

        let (status, body) = delete_req(app, &format!("/api/matrices/{km_id}")).await;
        assert_eq!(status, AxumStatusCode::OK);
        let val: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(val["removed"], 1);
    }

    #[tokio::test]
    async fn delete_matrix_not_found() {
        let state = test_state(vec![make_test_matrix()]);
        let app = build_router(state);
        let fake_id = uuid::Uuid::new_v4();

        let (status, _) = delete_req(app, &format!("/api/matrices/{fake_id}")).await;
        assert_eq!(status, AxumStatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn delete_matrix_invalid_uuid() {
        let state = test_state(vec![]);
        let app = build_router(state);

        let (status, _) = delete_req(app, "/api/matrices/not-a-uuid").await;
        assert_eq!(status, AxumStatusCode::BAD_REQUEST);
    }

    // ── POST /api/matrices/clear ────────────────────────────────────

    #[tokio::test]
    async fn clear_matrices_success() {
        let state = test_state(vec![make_test_matrix()]);
        let app = build_router(state);

        let (status, body) = post_json(app, "/api/matrices/clear", "").await;
        assert_eq!(status, AxumStatusCode::OK);
        let val: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(val["cleared"], 1);
    }

    // ── ZIP upload support ───────────────────────────────────────────

    #[test]
    fn extract_zip_archive_extracts_supported_files_only() {
        let zip_data = build_zip_with_files(&[
            ("nested/test_matrix.dbc", MINIMAL_DBC),
            ("nested/readme.txt", "ignore me"),
        ]);
        let temp_dir = tempfile::tempdir().unwrap();
        let mut total_size = 0usize;

        let extracted = extract_zip_archive(&zip_data, temp_dir.path(), &mut total_size).unwrap();
        assert_eq!(extracted, 1);
        assert!(temp_dir.path().join("nested/test_matrix.dbc").exists());
        assert!(!temp_dir.path().join("nested/readme.txt").exists());
        assert_eq!(total_size, MINIMAL_DBC.len());
    }

    #[tokio::test]
    async fn upload_zip_with_supported_files() {
        let state = test_state(vec![]);
        let app = build_router(state);

        let zip_data = build_zip_with_files(&[("folder/test_matrix.dbc", MINIMAL_DBC)]);
        let boundary = "KMATRIX_BOUNDARY";
        let mut multipart_body = Vec::<u8>::new();
        multipart_body.extend_from_slice(
            format!(
                "--{boundary}\r\nContent-Disposition: form-data; name=\"files\"; filename=\"kmatrix.zip\"\r\nContent-Type: application/zip\r\n\r\n"
            )
            .as_bytes(),
        );
        multipart_body.extend_from_slice(&zip_data);
        multipart_body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());

        let (status, body) = post_multipart(app, "/api/upload", multipart_body, boundary).await;
        assert_eq!(status, AxumStatusCode::OK);

        let resp: ImportResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(resp.files_imported, 1);
        assert_eq!(resp.total_matrices, 1);
        assert_eq!(resp.total_signals, 2);
    }

    // ── POST /api/import (error cases) ──────────────────────────────

    #[tokio::test]
    async fn import_nonexistent_path() {
        let state = test_state(vec![]);
        let app = build_router(state);

        let (status, _) = post_json(
            app,
            "/api/import",
            r#"{"path": "/nonexistent/path/to/kmatrix"}"#,
        )
        .await;
        assert_eq!(status, AxumStatusCode::BAD_REQUEST);
    }
}
