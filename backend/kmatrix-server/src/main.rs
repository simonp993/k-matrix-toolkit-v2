//! kmatrix-server — Controller layer for the K-Matrix Toolkit.
//!
//! Axum HTTP server exposing a REST API for importing K-Matrices
//! and searching across them. State is persisted to disk.

use std::path::PathBuf;
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
}

impl AppState {
    /// Save current matrices to disk.
    fn persist(&self) {
        let matrices = self.matrices.read().unwrap();
        let path = data_path();
        if let Ok(json) = serde_json::to_vec(&*matrices) {
            if let Err(e) = std::fs::write(&path, json) {
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
    });

    let app = Router::new()
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
        .with_state(state);

    let addr = "0.0.0.0:3001";
    tracing::info!("K-Matrix Toolkit server listening on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

// ── Status ──────────────────────────────────────────────────────────

#[derive(Serialize)]
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

#[derive(Serialize)]
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

async fn upload_files(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Result<Json<ImportResponse>, (StatusCode, String)> {
    let temp_dir = tempfile::tempdir()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let mut total_size: usize = 0;
    let mut file_count: usize = 0;

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
        let lower = file_name.to_lowercase();
        if !lower.ends_with(".xlsx")
            && !lower.ends_with(".dbc")
            && !lower.ends_with(".ldf")
        {
            continue;
        }

        let data = field
            .bytes()
            .await
            .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

        total_size += data.len();
        if total_size > MAX_UPLOAD_SIZE {
            return Err((
                StatusCode::PAYLOAD_TOO_LARGE,
                "Upload exceeds 500 MB limit".to_string(),
            ));
        }

        // Sanitize filename: only keep the basename, no path traversal
        let safe_name = std::path::Path::new(&file_name)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| format!("upload_{file_count}"));

        let dest = temp_dir.path().join(&safe_name);
        tokio::fs::write(&dest, &data)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        file_count += 1;
    }

    if file_count == 0 {
        return Err((
            StatusCode::BAD_REQUEST,
            "No supported files uploaded (.xlsx, .dbc, .ldf)".to_string(),
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

#[derive(Serialize)]
struct FilterCounts {
    bus_types: std::collections::HashMap<String, usize>,
    platforms: std::collections::HashMap<String, usize>,
    file_types: std::collections::HashMap<String, usize>,
}

#[derive(Serialize)]
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

#[derive(Serialize)]
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

#[derive(Serialize)]
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
