use crate::avens::{BuildMode, ImportMode, OptLevel, find_project_root};
use crate::error::mss::MssError;
use crate::error::{ErrorKind, MireError, Result};
use crate::parser::Program;
use crate::parser::ast::{
    AssignmentTarget, DataType, EnumVariantDef, Expression, FunctionDef, Identifier, Literal,
    MireValue, QueryBinding, QueryGroup, QueryJoin, QueryOp, Statement, TraitMethodSig, Visibility,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::slice;
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(unix)]
use std::os::fd::AsRawFd;

mod analysis;
mod dependencies;
mod hasher;
mod hashing;

pub use analysis::{
    analysis_child_unit_key, analysis_unit_key, analysis_units_for_program,
    compute_invalidation_report,
};
pub(crate) use dependencies::collect_statement_dependencies;
pub use hasher::FxHasher;
use hashing::stable_statement_hash;
mod serialize;
mod utils;
use serialize::{
    append_blob, cache_runtime_err, decode_cache_db, encode_cache_db, now_epoch_ms, read_blob,
};
use utils::{analysis_cache_key, build_cache_key, manifest_cache_settings, normalize_path_key};
pub use utils::{build_fingerprint, cache_file_path, source_hash, statement_export_name};

const CACHE_DIR_NAME: &str = ".cache";
const CACHE_FILE_NAME: &str = "incremental.bin";
const CACHE_MAGIC: &[u8; 8] = b"MIREINC2";
const CACHE_FORMAT_VERSION: u32 = 6;
const DEFAULT_MAX_UNITS: usize = 256;
const BLOB_COMPACT_THRESHOLD_RATIO: f64 = 0.7;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CacheSettings {
    pub max_units: Option<usize>,
    pub analysis_cache: bool,
    pub compression: bool,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct CacheOverrides {
    pub max_units: Option<usize>,
    pub analysis_cache: Option<bool>,
    pub compression: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedImport {
    pub raw_path: String,
    pub resolved_path: PathBuf,
    pub items: Option<Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct CachedParsedFile {
    pub hash: u64,
    pub program: Program,
    pub exports: Vec<String>,
    pub local_imports: Vec<CachedImport>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredParsedFile {
    program: Program,
    exports: Vec<String>,
    local_imports: Vec<CachedImport>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadedFile {
    pub hash: u64,
    pub direct_dependencies: Vec<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct LoadedProgram {
    pub program: Program,
    pub files: HashMap<PathBuf, LoadedFile>,
    pub statement_origins: Vec<PathBuf>,
    pub sources: HashMap<PathBuf, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildCacheEntry {
    pub fingerprint: u64,
    pub mode: BuildMode,
    pub import_mode: ImportMode,
    pub opt_level: OptLevel,
    pub emit_binary: bool,
    pub persist_ir: bool,
    pub binary_path: PathBuf,
    pub ir_path: Option<PathBuf>,
    pub optimized_ir_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredAnalyzedProgram {
    program: Program,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredAnalysisPayload {
    outcome: StoredAnalysisOutcome,
    units: Vec<AnalysisUnitMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum StoredAnalysisOutcome {
    Success(StoredAnalyzedProgram),
    Error(StoredMireError),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisUnitMetadata {
    pub unit_key: String,
    pub unit_kind: AnalysisUnitKind,
    pub body_hash: u64,
    pub dependencies: Vec<String>,
    pub origin: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnalysisUnitKind {
    Function,
    Type,
    Enum,
    Impl,
    Field,
    Other,
}

#[derive(Debug, Clone)]
pub enum CachedAnalysis {
    Success(Program),
    Error(MireError),
}

#[derive(Debug, Clone)]
pub struct CachedAnalysisSnapshot {
    pub program: Program,
    pub units: Vec<AnalysisUnitMetadata>,
}

#[derive(Debug, Clone, Default)]
pub struct AnalysisInvalidationReport {
    pub changed_units: Vec<String>,
    pub invalidated_units: Vec<String>,
    pub added_units: Vec<String>,
    pub removed_units: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct CacheMetrics {
    pub file_hits: u64,
    pub file_misses: u64,
    pub analysis_hits: u64,
    pub analysis_misses: u64,
    pub build_hits: u64,
    pub build_misses: u64,
    pub evictions: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredMireError {
    kind: StoredErrorKind,
    source: Option<String>,
    filename: Option<String>,
    line: usize,
    column: usize,
    explanation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum StoredErrorKind {
    Lexer {
        line: usize,
        column: usize,
        message: String,
    },
    DeprecatedSyntax {
        line: usize,
        column: usize,
        message: String,
    },
    Parser {
        line: usize,
        column: usize,
        message: String,
    },
    Backend {
        message: String,
    },
    Runtime {
        message: String,
    },
    Type {
        line: usize,
        column: usize,
        message: String,
    },
    Ownership {
        line: usize,
        column: usize,
        kind: StoredMssError,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum StoredMssError {
    MutationWhileShared,
    MultipleMutableRefs,
    MoveWhileBorrowed,
    UseAfterMove,
    DropWhileBorrowed,
    DoubleDrop,
    BorrowOutOfScope,
    InvalidMove,
    UnsafeViolation,
}

#[derive(Debug, Clone, Default)]
struct CacheDb {
    format_version: u32,
    files: HashMap<String, FileCacheEntry>,
    analyses: HashMap<String, AnalysisCacheEntry>,
    builds: HashMap<String, BuildCacheRecord>,
}

#[derive(Debug, Clone)]
struct FileCacheEntry {
    hash: u64,
    blob_offset: u64,
    blob_len: u64,
    last_access_epoch_ms: u64,
}

#[derive(Debug, Clone)]
struct AnalysisCacheEntry {
    fingerprint: u64,
    blob_offset: u64,
    blob_len: u64,
    last_access_epoch_ms: u64,
    created_epoch_ms: u64,
    unit_count: u32,
}

#[derive(Debug, Clone)]
struct BuildCacheRecord {
    entry: BuildCacheEntry,
    last_access_epoch_ms: u64,
}

#[derive(Debug, Clone, Copy)]
struct BlobStoreLayout {
    start: usize,
    len: usize,
}

#[derive(Debug)]
enum BlobStore {
    Owned(Vec<u8>),
    #[cfg(unix)]
    Mapped {
        mapping: MemoryMappedFile,
        layout: BlobStoreLayout,
    },
}

impl Default for BlobStore {
    fn default() -> Self {
        Self::Owned(Vec::new())
    }
}

impl BlobStore {
    fn from_owned(bytes: Vec<u8>) -> Self {
        Self::Owned(bytes)
    }

    #[cfg(unix)]
    fn from_mapped(mapping: MemoryMappedFile, layout: BlobStoreLayout) -> Self {
        Self::Mapped { mapping, layout }
    }

    fn bytes(&self) -> &[u8] {
        match self {
            Self::Owned(bytes) => bytes.as_slice(),
            #[cfg(unix)]
            Self::Mapped { mapping, layout } => {
                let end = layout.start.saturating_add(layout.len);
                &mapping.as_slice()[layout.start..end]
            }
        }
    }

    fn read(&self, offset: u64, len: u64) -> Result<&[u8]> {
        read_blob(self.bytes(), offset, len)
    }

    fn append(&mut self, blob: &[u8]) -> (u64, u64) {
        let store = self.ensure_owned();
        append_blob(store, blob)
    }

    fn ensure_owned(&mut self) -> &mut Vec<u8> {
        if !matches!(self, Self::Owned(_)) {
            let owned = self.bytes().to_vec();
            *self = Self::Owned(owned);
        }

        let Self::Owned(bytes) = self else {
            unreachable!("blob store must be owned after promotion");
        };
        bytes
    }

    #[cfg(test)]
    fn is_memory_mapped(&self) -> bool {
        #[cfg(unix)]
        {
            matches!(self, Self::Mapped { .. })
        }

        #[cfg(not(unix))]
        {
            false
        }
    }
}

#[cfg(unix)]
#[derive(Debug)]
struct MemoryMappedFile {
    ptr: *mut libc::c_void,
    len: usize,
}

#[cfg(unix)]
impl MemoryMappedFile {
    fn map(file: &File) -> Result<Option<Self>> {
        let len = usize::try_from(
            file.metadata()
                .map_err(|err| cache_runtime_err(&format!("Could not stat cache file: {}", err)))?
                .len(),
        )
        .map_err(|_| cache_runtime_err("Incremental cache file too large to map"))?;
        if len == 0 {
            return Ok(None);
        }

        let ptr = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                len,
                libc::PROT_READ,
                libc::MAP_PRIVATE,
                file.as_raw_fd(),
                0,
            )
        };
        if ptr == libc::MAP_FAILED {
            return Err(cache_runtime_err("Could not memory-map incremental cache"));
        }

        Ok(Some(Self { ptr, len }))
    }

    fn as_slice(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(self.ptr.cast::<u8>(), self.len) }
    }
}

#[cfg(unix)]
impl Drop for MemoryMappedFile {
    fn drop(&mut self) {
        if self.len == 0 {
            return;
        }

        unsafe {
            libc::munmap(self.ptr, self.len);
        }
    }
}

pub struct IncrementalCache {
    cache_path: PathBuf,
    settings: CacheSettings,
    db: CacheDb,
    blob_store: BlobStore,
    metrics: CacheMetrics,
}

impl CacheSettings {
    pub fn defaults() -> Self {
        Self {
            max_units: Some(DEFAULT_MAX_UNITS),
            analysis_cache: true,
            compression: false,
        }
    }

    pub fn resolve_for(source_path: &Path, overrides: CacheOverrides) -> Result<Self> {
        let mut resolved = manifest_cache_settings(source_path)?;
        if let Some(max_units) = overrides.max_units {
            resolved.max_units = (max_units != 0).then_some(max_units);
        }
        if let Some(enabled) = overrides.analysis_cache {
            resolved.analysis_cache = enabled;
        }
        if let Some(enabled) = overrides.compression {
            resolved.compression = enabled;
        }
        Ok(resolved)
    }
}

impl IncrementalCache {
    pub fn load_for(source_path: &Path) -> Result<Self> {
        Self::load_with_settings(
            source_path,
            CacheSettings::resolve_for(source_path, CacheOverrides::default())?,
        )
    }

    pub fn load_with_settings(source_path: &Path, settings: CacheSettings) -> Result<Self> {
        let cache_path = cache_file_path(source_path);
        let mut cache = Self {
            cache_path,
            settings,
            db: CacheDb {
                format_version: CACHE_FORMAT_VERSION,
                ..CacheDb::default()
            },
            blob_store: BlobStore::default(),
            metrics: CacheMetrics::default(),
        };
        let mut found_cache_file = false;
        let mut loaded_cache = false;

        if let Ok(file) = File::open(&cache.cache_path) {
            found_cache_file = true;
            #[cfg(unix)]
            {
                if let Ok(Some(mapping)) = MemoryMappedFile::map(&file) {
                    if let Ok((db, layout)) = decode_cache_db(mapping.as_slice())
                        && db.format_version == CACHE_FORMAT_VERSION
                    {
                        cache.db = db;
                        cache.blob_store = BlobStore::from_mapped(mapping, layout);
                        loaded_cache = true;
                    }
                } else if let Ok(raw) = fs::read(&cache.cache_path)
                    && let Ok((db, layout)) = decode_cache_db(&raw)
                    && db.format_version == CACHE_FORMAT_VERSION
                {
                    cache.db = db;
                    cache.blob_store = BlobStore::from_owned(
                        raw[layout.start..layout.start + layout.len].to_vec(),
                    );
                    loaded_cache = true;
                }
            }

            #[cfg(not(unix))]
            {
                if let Ok(raw) = fs::read(&cache.cache_path) {
                    if let Ok((db, layout)) = decode_cache_db(&raw) {
                        if db.format_version == CACHE_FORMAT_VERSION {
                            cache.db = db;
                            cache.blob_store = BlobStore::from_owned(
                                raw[layout.start..layout.start + layout.len].to_vec(),
                            );
                            loaded_cache = true;
                        }
                    }
                }
            }
        }

        if found_cache_file && !loaded_cache {
            // Best-effort self-heal: keep running with an empty cache and remove corrupt/incompatible file.
            let _ = fs::remove_file(&cache.cache_path);
        }

        cache.prune_lru();
        Ok(cache)
    }

    pub fn save(&mut self) -> Result<()> {
        if let Some(parent) = self.cache_path.parent() {
            fs::create_dir_all(parent).map_err(|err| {
                MireError::new(ErrorKind::Runtime {
                    message: format!(
                        "Could not create incremental cache directory '{}': {}",
                        parent.display(),
                        err
                    ),
                })
            })?;
        }

        self.prune_lru();
        let raw = encode_cache_db(&self.db, self.blob_store.bytes())?;
        fs::write(&self.cache_path, raw).map_err(|err| {
            MireError::new(ErrorKind::Runtime {
                message: format!(
                    "Could not write incremental cache '{}': {}",
                    self.cache_path.display(),
                    err
                ),
            })
        })
    }

    pub fn metrics(&self) -> &CacheMetrics {
        &self.metrics
    }

    pub fn record_build_hit(&mut self) {
        self.metrics.build_hits += 1;
    }

    pub fn record_build_miss(&mut self) {
        self.metrics.build_misses += 1;
    }

    pub fn cached_file(&mut self, path: &Path, hash: u64) -> Option<CachedParsedFile> {
        let key = normalize_path_key(path);
        let Some(entry) = self.db.files.get(&key) else {
            self.metrics.file_misses += 1;
            return None;
        };
        if entry.hash != hash {
            self.metrics.file_misses += 1;
            return None;
        }

        let blob = self
            .blob_store
            .read(entry.blob_offset, entry.blob_len)
            .ok()?;
        let stored = serde_json::from_slice::<StoredParsedFile>(blob).ok()?;
        if let Some(entry) = self.db.files.get_mut(&key) {
            entry.last_access_epoch_ms = now_epoch_ms();
        }
        self.metrics.file_hits += 1;
        Some(CachedParsedFile {
            hash,
            program: stored.program,
            exports: stored.exports,
            local_imports: stored.local_imports,
        })
    }

    pub fn store_file(&mut self, path: &Path, entry: CachedParsedFile) -> Result<()> {
        let stored = StoredParsedFile {
            program: entry.program,
            exports: entry.exports,
            local_imports: entry.local_imports,
        };
        let blob = serde_json::to_vec(&stored).map_err(|err| {
            MireError::new(ErrorKind::Runtime {
                message: format!("Could not serialize cached parsed file: {}", err),
            })
        })?;
        let (blob_offset, blob_len) = self.blob_store.append(&blob);
        self.db.files.insert(
            normalize_path_key(path),
            FileCacheEntry {
                hash: entry.hash,
                blob_offset,
                blob_len,
                last_access_epoch_ms: now_epoch_ms(),
            },
        );
        self.prune_lru();
        Ok(())
    }

    pub fn cached_analysis(&mut self, source_path: &Path) -> Option<CachedAnalysis> {
        if !self.settings.analysis_cache {
            return None;
        }

        let key = analysis_cache_key(source_path);
        let entry = self.db.analyses.get(&key)?;
        let blob = self
            .blob_store
            .read(entry.blob_offset, entry.blob_len)
            .ok()?;
        let stored = serde_json::from_slice::<StoredAnalysisPayload>(blob).ok()?;
        if let Some(entry) = self.db.analyses.get_mut(&key) {
            entry.last_access_epoch_ms = now_epoch_ms();
        }
        self.metrics.analysis_hits += 1;
        match stored.outcome {
            StoredAnalysisOutcome::Success(stored) => Some(CachedAnalysis::Success(stored.program)),
            StoredAnalysisOutcome::Error(error) => Some(CachedAnalysis::Error(error.into())),
        }
    }

    pub fn store_analysis(&mut self, source_path: &Path, program: &Program) -> Result<()> {
        if !self.settings.analysis_cache {
            return Ok(());
        }

        let units = analysis_units_for_program(program);
        let stored = StoredAnalysisPayload {
            outcome: StoredAnalysisOutcome::Success(StoredAnalyzedProgram {
                program: program.clone(),
            }),
            units: units.clone(),
        };
        let blob = serde_json::to_vec(&stored).map_err(|err| {
            MireError::new(ErrorKind::Runtime {
                message: format!("Could not serialize analysis cache entry: {}", err),
            })
        })?;
        let (blob_offset, blob_len) = self.blob_store.append(&blob);
        let now = now_epoch_ms();
        self.db.analyses.insert(
            analysis_cache_key(source_path),
            AnalysisCacheEntry {
                fingerprint: 0,
                blob_offset,
                blob_len,
                last_access_epoch_ms: now,
                created_epoch_ms: now,
                unit_count: units.len() as u32,
            },
        );
        self.prune_lru();
        Ok(())
    }

    pub fn store_analysis_error(
        &mut self,
        source_path: &Path,
        program: &Program,
        error: &MireError,
    ) -> Result<()> {
        if !self.settings.analysis_cache {
            return Ok(());
        }

        let units = analysis_units_for_program(program);
        let stored = StoredAnalysisPayload {
            outcome: StoredAnalysisOutcome::Error(error.into()),
            units: units.clone(),
        };
        let blob = serde_json::to_vec(&stored).map_err(|err| {
            MireError::new(ErrorKind::Runtime {
                message: format!("Could not serialize cached analysis error: {}", err),
            })
        })?;
        let (blob_offset, blob_len) = self.blob_store.append(&blob);
        let now = now_epoch_ms();
        self.db.analyses.insert(
            analysis_cache_key(source_path),
            AnalysisCacheEntry {
                fingerprint: 0,
                blob_offset,
                blob_len,
                last_access_epoch_ms: now,
                created_epoch_ms: now,
                unit_count: units.len() as u32,
            },
        );
        self.prune_lru();
        Ok(())
    }

    pub fn analysis_invalidation_report(
        &self,
        source_path: &Path,
        program: &Program,
    ) -> Option<AnalysisInvalidationReport> {
        let current_units = analysis_units_for_program(program);
        let previous_units = self.latest_analysis_units(source_path)?;
        Some(compute_invalidation_report(&previous_units, &current_units))
    }

    pub fn latest_successful_analysis(
        &mut self,
        source_path: &Path,
    ) -> Option<CachedAnalysisSnapshot> {
        let key = analysis_cache_key(source_path);
        let entry = self.db.analyses.get(&key)?;
        let blob = self
            .blob_store
            .read(entry.blob_offset, entry.blob_len)
            .ok()?;
        let stored = serde_json::from_slice::<StoredAnalysisPayload>(blob).ok()?;
        let StoredAnalysisOutcome::Success(stored_program) = stored.outcome else {
            return None;
        };
        if let Some(entry) = self.db.analyses.get_mut(&key) {
            entry.last_access_epoch_ms = now_epoch_ms();
        }
        Some(CachedAnalysisSnapshot {
            program: stored_program.program,
            units: stored.units,
        })
    }

    pub fn build_entry(
        &mut self,
        source_path: &Path,
        mode: BuildMode,
        import_mode: ImportMode,
        emit_binary: bool,
        persist_ir: bool,
    ) -> Option<&BuildCacheEntry> {
        let key = build_cache_key(source_path, mode, import_mode, emit_binary, persist_ir);
        let record = self.db.builds.get_mut(&key)?;
        record.last_access_epoch_ms = now_epoch_ms();
        Some(&record.entry)
    }

    pub fn store_build(&mut self, source_path: &Path, entry: BuildCacheEntry) {
        self.db.builds.insert(
            build_cache_key(
                source_path,
                entry.mode,
                entry.import_mode,
                entry.emit_binary,
                entry.persist_ir,
            ),
            BuildCacheRecord {
                entry,
                last_access_epoch_ms: now_epoch_ms(),
            },
        );
    }

    fn prune_lru(&mut self) {
        let Some(max_units) = self.settings.max_units else {
            self.maybe_compact_blob_store();
            return;
        };

        let current_units = self.db.files.len() + self.db.analyses.len() + self.db.builds.len();
        if current_units <= max_units {
            self.maybe_compact_blob_store();
            return;
        }

        let mut victims = Vec::with_capacity(current_units);
        for key in self.db.files.keys() {
            let last_access = self
                .db
                .files
                .get(key)
                .map(|entry| entry.last_access_epoch_ms)
                .unwrap_or(0);
            victims.push((last_access, CacheVictim::File(key.clone())));
        }
        for key in self.db.analyses.keys() {
            let last_access = self
                .db
                .analyses
                .get(key)
                .map(|entry| entry.last_access_epoch_ms)
                .unwrap_or(0);
            victims.push((last_access, CacheVictim::Analysis(key.clone())));
        }
        for key in self.db.builds.keys() {
            let last_access = self
                .db
                .builds
                .get(key)
                .map(|entry| entry.last_access_epoch_ms)
                .unwrap_or(0);
            victims.push((last_access, CacheVictim::Build(key.clone())));
        }
        victims.sort_by_key(|(last_access, _)| *last_access);

        let to_remove = current_units.saturating_sub(max_units);
        for (_, victim) in victims.into_iter().take(to_remove) {
            match victim {
                CacheVictim::File(key) => {
                    self.db.files.remove(&key);
                    self.metrics.evictions += 1;
                }
                CacheVictim::Analysis(key) => {
                    self.db.analyses.remove(&key);
                    self.metrics.evictions += 1;
                }
                CacheVictim::Build(key) => {
                    self.db.builds.remove(&key);
                    self.metrics.evictions += 1;
                }
            }
        }

        self.maybe_compact_blob_store();
    }

    fn maybe_compact_blob_store(&mut self) {
        let total_len = self.blob_store.bytes().len();
        if total_len < 512 {
            return;
        }

        let mut live_ranges = Vec::with_capacity(self.db.files.len() + self.db.analyses.len());
        for entry in self.db.files.values() {
            live_ranges.push((entry.blob_offset, entry.blob_len));
        }
        for entry in self.db.analyses.values() {
            live_ranges.push((entry.blob_offset, entry.blob_len));
        }

        if live_ranges.is_empty() {
            if total_len > 0 {
                self.blob_store = BlobStore::from_owned(Vec::new());
            }
            return;
        }

        live_ranges.sort_unstable_by_key(|(offset, _)| *offset);
        let mut unique_live_ranges = Vec::with_capacity(live_ranges.len());
        for (offset, len) in live_ranges {
            let start = offset as usize;
            let end = start.saturating_add(len as usize).min(total_len);
            if start >= end {
                continue;
            }
            if let Some((_, last_end)) = unique_live_ranges.last_mut()
                && start <= *last_end
            {
                *last_end = (*last_end).max(end);
                continue;
            }
            unique_live_ranges.push((start, end));
        }

        let live_bytes = unique_live_ranges.iter().fold(0usize, |acc, (start, end)| {
            acc.saturating_add(end.saturating_sub(*start))
        });
        if live_bytes == 0 {
            if total_len > 0 {
                self.blob_store = BlobStore::from_owned(Vec::new());
            }
            return;
        }

        let ratio = (live_bytes as f64) / (total_len as f64);
        if ratio >= BLOB_COMPACT_THRESHOLD_RATIO {
            return;
        }

        let old_blob = self.blob_store.bytes();
        let mut compacted = Vec::with_capacity(live_bytes);
        let mut relocated_ranges = Vec::with_capacity(unique_live_ranges.len());

        for (old_start, old_end) in unique_live_ranges {
            let new_offset = compacted.len() as u64;
            compacted.extend_from_slice(&old_blob[old_start..old_end]);
            relocated_ranges.push((old_start as u64, old_end as u64, new_offset));
        }

        for entry in self.db.files.values_mut() {
            if let Some((old_start, _old_end, new_start)) =
                relocated_ranges.iter().find(|(old_start, old_end, _)| {
                    entry.blob_offset >= *old_start
                        && entry.blob_offset.saturating_add(entry.blob_len) <= *old_end
                })
            {
                entry.blob_offset = new_start.saturating_add(entry.blob_offset - *old_start);
            }
        }
        for entry in self.db.analyses.values_mut() {
            if let Some((old_start, _old_end, new_start)) =
                relocated_ranges.iter().find(|(old_start, old_end, _)| {
                    entry.blob_offset >= *old_start
                        && entry.blob_offset.saturating_add(entry.blob_len) <= *old_end
                })
            {
                entry.blob_offset = new_start.saturating_add(entry.blob_offset - *old_start);
            }
        }

        self.blob_store = BlobStore::from_owned(compacted);
    }

    fn latest_analysis_units(&self, source_path: &Path) -> Option<Vec<AnalysisUnitMetadata>> {
        let key = analysis_cache_key(source_path);
        let entry = self.db.analyses.get(&key)?;
        let blob = self
            .blob_store
            .read(entry.blob_offset, entry.blob_len)
            .ok()?;
        let stored = serde_json::from_slice::<StoredAnalysisPayload>(blob).ok()?;
        Some(stored.units)
    }
}

enum CacheVictim {
    File(String),
    Analysis(String),
    Build(String),
}
