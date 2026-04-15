pub mod dbc;
pub mod xlsx;

use std::path::Path;

use anyhow::Result;
use rayon::prelude::*;

use crate::model::{FileFormat, KMatrix};

/// Trait that every K-Matrix parser must implement.
pub trait KMatrixParser: Send + Sync {
    /// Check if this parser can handle the given file.
    fn can_parse(&self, path: &Path) -> bool;

    /// Parse the file into one or more KMatrix structs.
    fn parse(&self, path: &Path) -> Result<Vec<KMatrix>>;

    /// Supported file formats for this parser.
    fn supported_formats(&self) -> &[FileFormat];
}

/// Registry that manages all available parsers and dispatches files to the correct one.
pub struct ParserRegistry {
    parsers: Vec<Box<dyn KMatrixParser>>,
}

impl ParserRegistry {
    /// Create a new registry with all built-in parsers.
    pub fn new() -> Self {
        Self {
            parsers: vec![
                Box::new(XlsxParserImpl),
                Box::new(DbcParserImpl),
            ],
        }
    }

    /// Parse a single file, dispatching to the correct parser.
    pub fn parse(&self, path: &Path) -> Result<Vec<KMatrix>> {
        for parser in &self.parsers {
            if parser.can_parse(path) {
                return parser.parse(path);
            }
        }
        anyhow::bail!("No parser found for {}", path.display())
    }

    /// Parse all supported files in a directory recursively, in parallel.
    pub fn parse_directory(&self, dir: &Path) -> Result<Vec<KMatrix>> {
        let files = self.find_supported_files(dir)?;

        tracing::info!(
            "Found {} supported files in {}",
            files.len(),
            dir.display()
        );

        let results: Vec<Result<Vec<KMatrix>>> = files
            .par_iter()
            .map(|path| {
                tracing::debug!("Parsing {}", path.display());
                self.parse(path)
            })
            .collect();

        let mut all_matrices = Vec::new();
        let mut errors = Vec::new();

        for result in results {
            match result {
                Ok(matrices) => all_matrices.extend(matrices),
                Err(e) => errors.push(e),
            }
        }

        if !errors.is_empty() {
            tracing::warn!("{} files failed to parse:", errors.len());
            for err in &errors {
                tracing::warn!("  {err:#}");
            }
        }

        tracing::info!(
            "Parsed {} K-Matrices from {} files",
            all_matrices.len(),
            files.len() - errors.len()
        );

        Ok(all_matrices)
    }

    /// Find all files in a directory that match any parser's supported formats.
    fn find_supported_files(&self, dir: &Path) -> Result<Vec<std::path::PathBuf>> {
        use glob::glob;

        let dir_str = dir.to_string_lossy();
        let mut files = Vec::new();

        // Collect all extensions from parsers
        let patterns = [
            format!("{dir_str}/**/*.xlsx"),
            format!("{dir_str}/**/*.dbc"),
        ];

        for pattern in &patterns {
            for entry in glob(pattern)? {
                let path = entry?;
                // Check filename heuristics
                if self.should_parse(&path) {
                    files.push(path);
                }
            }
        }

        Ok(files)
    }

    /// Filename heuristics to decide if a file should be parsed.
    fn should_parse(&self, path: &Path) -> bool {
        let filename = path
            .file_name()
            .map(|f| f.to_string_lossy().to_lowercase())
            .unwrap_or_default();

        // Skip temp files
        if filename.starts_with('~') || filename.starts_with('.') {
            return false;
        }

        // For xlsx: only parse files with "KMatrix" or "kmatrix" in name
        if filename.ends_with(".xlsx") {
            return filename.contains("kmatrix");
        }

        // DBC files are always parsed
        if filename.ends_with(".dbc") {
            return true;
        }

        false
    }
}

impl Default for ParserRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// --- Parser implementations ---

struct XlsxParserImpl;

impl KMatrixParser for XlsxParserImpl {
    fn can_parse(&self, path: &Path) -> bool {
        path.extension()
            .map(|ext| ext.to_string_lossy().to_lowercase() == "xlsx")
            .unwrap_or(false)
    }

    fn parse(&self, path: &Path) -> Result<Vec<KMatrix>> {
        xlsx::parse_xlsx(path)
    }

    fn supported_formats(&self) -> &[FileFormat] {
        &[FileFormat::XLSX]
    }
}

struct DbcParserImpl;

impl KMatrixParser for DbcParserImpl {
    fn can_parse(&self, path: &Path) -> bool {
        path.extension()
            .map(|ext| ext.to_string_lossy().to_lowercase() == "dbc")
            .unwrap_or(false)
    }

    fn parse(&self, path: &Path) -> Result<Vec<KMatrix>> {
        dbc::parse_dbc(path)
    }

    fn supported_formats(&self) -> &[FileFormat] {
        &[FileFormat::DBC]
    }
}
