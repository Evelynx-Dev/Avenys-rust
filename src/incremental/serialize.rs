use super::*;

pub(super) fn encode_cache_db(db: &CacheDb, blob_store: &[u8]) -> Result<Vec<u8>> {
    let mut out = Vec::with_capacity(blob_store.len().saturating_add(4096));
    out.extend_from_slice(CACHE_MAGIC);
    write_u32(&mut out, CACHE_FORMAT_VERSION);
    write_u64(
        &mut out,
        u64::try_from(db.files.len()).map_err(|_| cache_runtime_err("Too many cached files"))?,
    );
    for (key, entry) in &db.files {
        write_string(&mut out, key)?;
        write_u64(&mut out, entry.hash);
        write_u64(&mut out, entry.last_access_epoch_ms);
        write_u64(&mut out, entry.blob_offset);
        write_u64(&mut out, entry.blob_len);
    }

    write_u64(
        &mut out,
        u64::try_from(db.analyses.len())
            .map_err(|_| cache_runtime_err("Too many cached analysis entries"))?,
    );
    for (key, entry) in &db.analyses {
        write_string(&mut out, key)?;
        write_u64(&mut out, entry.fingerprint);
        write_u64(&mut out, entry.last_access_epoch_ms);
        write_u64(&mut out, entry.created_epoch_ms);
        write_u64(&mut out, entry.blob_offset);
        write_u64(&mut out, entry.blob_len);
        write_u32(&mut out, entry.unit_count);
    }

    write_u64(
        &mut out,
        u64::try_from(db.builds.len()).map_err(|_| cache_runtime_err("Too many cached builds"))?,
    );
    for (key, record) in &db.builds {
        write_string(&mut out, key)?;
        write_u64(&mut out, record.last_access_epoch_ms);
        write_build_entry(&mut out, &record.entry)?;
    }

    write_u64(
        &mut out,
        u64::try_from(blob_store.len()).map_err(|_| cache_runtime_err("Blob store too large"))?,
    );
    out.extend_from_slice(blob_store);
    Ok(out)
}

pub(super) fn decode_cache_db(raw: &[u8]) -> Result<(CacheDb, BlobStoreLayout)> {
    let mut cursor = Cursor::new(raw);
    let magic = cursor.read_exact_bytes(CACHE_MAGIC.len())?;
    if magic != CACHE_MAGIC {
        return Err(cache_runtime_err("Invalid incremental cache header"));
    }

    let format_version = cursor.read_u32()?;
    let file_count = usize::try_from(cursor.read_u64()?)
        .map_err(|_| cache_runtime_err("Invalid cached file count"))?;
    let mut files = HashMap::with_capacity(file_count);
    for _ in 0..file_count {
        let key = cursor.read_string()?;
        let hash = cursor.read_u64()?;
        let last_access_epoch_ms = cursor.read_u64()?;
        let blob_offset = cursor.read_u64()?;
        let blob_len = cursor.read_u64()?;
        files.insert(
            key,
            FileCacheEntry {
                hash,
                blob_offset,
                blob_len,
                last_access_epoch_ms,
            },
        );
    }

    let analysis_count = usize::try_from(cursor.read_u64()?)
        .map_err(|_| cache_runtime_err("Invalid cached analysis count"))?;
    let mut analyses = HashMap::with_capacity(analysis_count);
    for _ in 0..analysis_count {
        let key = cursor.read_string()?;
        let fingerprint = cursor.read_u64()?;
        let last_access_epoch_ms = cursor.read_u64()?;
        let created_epoch_ms = cursor.read_u64()?;
        let blob_offset = cursor.read_u64()?;
        let blob_len = cursor.read_u64()?;
        let unit_count = cursor.read_u32()?;
        let entry = AnalysisCacheEntry {
            fingerprint,
            blob_offset,
            blob_len,
            last_access_epoch_ms,
            created_epoch_ms,
            unit_count,
        };
        let entry = if entry.created_epoch_ms == 0 {
            AnalysisCacheEntry {
                created_epoch_ms: entry.last_access_epoch_ms,
                ..entry
            }
        } else {
            entry
        };
        analyses.insert(key, entry);
    }

    let build_count = usize::try_from(cursor.read_u64()?)
        .map_err(|_| cache_runtime_err("Invalid cached build count"))?;
    let mut builds = HashMap::with_capacity(build_count);
    for _ in 0..build_count {
        let key = cursor.read_string()?;
        let last_access_epoch_ms = cursor.read_u64()?;
        let entry = cursor.read_build_entry()?;
        builds.insert(
            key,
            BuildCacheRecord {
                entry,
                last_access_epoch_ms,
            },
        );
    }

    let blob_len = usize::try_from(cursor.read_u64()?)
        .map_err(|_| cache_runtime_err("Invalid blob store length"))?;
    let blob_start = cursor.position();
    cursor.read_exact_bytes(blob_len)?;

    Ok((
        CacheDb {
            format_version,
            files,
            analyses,
            builds,
        },
        BlobStoreLayout {
            start: blob_start,
            len: blob_len,
        },
    ))
}

pub(super) fn append_blob(blob_store: &mut Vec<u8>, blob: &[u8]) -> (u64, u64) {
    let offset = blob_store.len() as u64;
    blob_store.extend_from_slice(blob);
    (offset, blob.len() as u64)
}

pub(super) fn read_blob(blob_store: &[u8], offset: u64, len: u64) -> Result<&[u8]> {
    let start = usize::try_from(offset).map_err(|_| cache_runtime_err("Invalid cache offset"))?;
    let len = usize::try_from(len).map_err(|_| cache_runtime_err("Invalid cache length"))?;
    let end = start
        .checked_add(len)
        .ok_or_else(|| cache_runtime_err("Invalid cache blob range"))?;
    blob_store
        .get(start..end)
        .ok_or_else(|| cache_runtime_err("Cache blob out of bounds"))
}

fn write_build_entry(out: &mut Vec<u8>, entry: &BuildCacheEntry) -> Result<()> {
    write_u64(out, entry.fingerprint);
    let mode_byte: u8 = match entry.mode {
        BuildMode::Debug => 0,
        BuildMode::Release => 1,
    };
    write_u8(out, mode_byte);
    write_u8(out, 0);
    let opt_byte: u8 = match entry.opt_level {
        OptLevel::O0 => 0,
        OptLevel::O1 => 1,
        OptLevel::O2 => 2,
        OptLevel::O3 => 3,
        OptLevel::Os => 4,
        OptLevel::Oz => 5,
    };
    write_u8(out, opt_byte);
    write_bool(out, entry.emit_binary);
    write_bool(out, entry.persist_ir);
    write_path(out, &entry.binary_path)?;
    write_optional_path(out, entry.ir_path.as_ref())?;
    write_optional_path(out, entry.optimized_ir_path.as_ref())?;
    Ok(())
}

fn write_optional_path(out: &mut Vec<u8>, path: Option<&PathBuf>) -> Result<()> {
    match path {
        Some(p) => {
            write_bool(out, true);
            write_path(out, p)?;
        }
        None => write_bool(out, false),
    }
    Ok(())
}

fn write_path(out: &mut Vec<u8>, path: &Path) -> Result<()> {
    write_string(out, &path.to_string_lossy())
}

fn write_string(out: &mut Vec<u8>, value: &str) -> Result<()> {
    write_u64(
        out,
        u64::try_from(value.len()).map_err(|_| cache_runtime_err("String too large"))?,
    );
    out.extend_from_slice(value.as_bytes());
    Ok(())
}

fn write_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn write_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn write_u8(out: &mut Vec<u8>, value: u8) {
    out.push(value);
}

fn write_bool(out: &mut Vec<u8>, value: bool) {
    write_u8(out, u8::from(value));
}

pub(super) fn now_epoch_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

pub(super) fn cache_runtime_err(message: &str) -> MireError {
    MireError::new(ErrorKind::Runtime {
        message: message.to_string(),
    })
}

struct Cursor<'a> {
    raw: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn new(raw: &'a [u8]) -> Self {
        Self { raw, pos: 0 }
    }

    fn position(&self) -> usize {
        self.pos
    }

    fn read_exact_bytes(&mut self, len: usize) -> Result<&'a [u8]> {
        let end = self
            .pos
            .checked_add(len)
            .ok_or_else(|| cache_runtime_err("Cache cursor overflow"))?;
        let slice = self
            .raw
            .get(self.pos..end)
            .ok_or_else(|| cache_runtime_err("Unexpected end of incremental cache"))?;
        self.pos = end;
        Ok(slice)
    }

    fn read_u64(&mut self) -> Result<u64> {
        let bytes = self.read_exact_bytes(8)?;
        let mut array = [0_u8; 8];
        array.copy_from_slice(bytes);
        Ok(u64::from_le_bytes(array))
    }

    fn read_u32(&mut self) -> Result<u32> {
        let bytes = self.read_exact_bytes(4)?;
        let mut array = [0_u8; 4];
        array.copy_from_slice(bytes);
        Ok(u32::from_le_bytes(array))
    }

    fn read_u8(&mut self) -> Result<u8> {
        Ok(*self
            .read_exact_bytes(1)?
            .first()
            .ok_or_else(|| cache_runtime_err("Missing cache byte"))?)
    }

    fn read_bool(&mut self) -> Result<bool> {
        Ok(self.read_u8()? != 0)
    }

    fn read_string(&mut self) -> Result<String> {
        let len = usize::try_from(self.read_u64()?)
            .map_err(|_| cache_runtime_err("Invalid cache string length"))?;
        let bytes = self.read_exact_bytes(len)?;
        String::from_utf8(bytes.to_vec()).map_err(|err| {
            MireError::new(ErrorKind::Runtime {
                message: format!("Invalid UTF-8 in incremental cache: {}", err),
            })
        })
    }

    fn read_path(&mut self) -> Result<PathBuf> {
        Ok(PathBuf::from(self.read_string()?))
    }

    fn read_optional_path(&mut self) -> Result<Option<PathBuf>> {
        if self.read_bool()? {
            Ok(Some(self.read_path()?))
        } else {
            Ok(None)
        }
    }

    fn read_build_entry(&mut self) -> Result<BuildCacheEntry> {
        let fingerprint = self.read_u64()?;
        let mode = match self.read_u8()? {
            0 => BuildMode::Debug,
            1 => BuildMode::Release,
            _ => return Err(cache_runtime_err("Invalid build mode in cache")),
        };
        let import_mode = match self.read_u8()? {
            0 => ImportMode::Reachable,
            1 => ImportMode::Reachable,
            _ => return Err(cache_runtime_err("Invalid import mode in cache")),
        };
        let opt_level = match self.read_u8()? {
            0 => OptLevel::O0,
            1 => OptLevel::O1,
            2 => OptLevel::O2,
            3 => OptLevel::O3,
            4 => OptLevel::Os,
            5 => OptLevel::Oz,
            _ => return Err(cache_runtime_err("Invalid opt level in cache")),
        };
        let emit_binary = self.read_bool()?;
        let persist_ir = self.read_bool()?;
        let binary_path = self.read_path()?;
        let ir_path = self.read_optional_path()?;
        let optimized_ir_path = self.read_optional_path()?;
        Ok(BuildCacheEntry {
            fingerprint,
            mode,
            import_mode,
            opt_level,
            emit_binary,
            persist_ir,
            binary_path,
            ir_path,
            optimized_ir_path,
        })
    }
}

impl From<&MireError> for StoredMireError {
    fn from(value: &MireError) -> Self {
        Self {
            kind: (&value.kind).into(),
            source: value.source().cloned(),
            filename: value.filename().cloned(),
            line: value.line,
            column: value.column,
            explanation: value.explanation().cloned(),
        }
    }
}

impl From<StoredMireError> for MireError {
    fn from(value: StoredMireError) -> Self {
        let mut error = MireError::new(value.kind.into());
        error.set_source(value.source);
        error.set_filename(value.filename);
        error.line = value.line;
        error.column = value.column;
        error.set_explanation(value.explanation);
        error
    }
}

impl From<&ErrorKind> for StoredErrorKind {
    fn from(value: &ErrorKind) -> Self {
        match value {
            ErrorKind::Lexer {
                line,
                column,
                message,
            } => Self::Lexer {
                line: *line,
                column: *column,
                message: message.clone(),
            },
            ErrorKind::DeprecatedSyntax {
                line,
                column,
                message,
            } => Self::DeprecatedSyntax {
                line: *line,
                column: *column,
                message: message.clone(),
            },
            ErrorKind::Parser {
                line,
                column,
                message,
            } => Self::Parser {
                line: *line,
                column: *column,
                message: message.clone(),
            },
            ErrorKind::Backend { message } => Self::Backend {
                message: message.clone(),
            },
            ErrorKind::Runtime { message } => Self::Runtime {
                message: message.clone(),
            },
            ErrorKind::Type {
                line,
                column,
                message,
            } => Self::Type {
                line: *line,
                column: *column,
                message: message.clone(),
            },
            ErrorKind::Ownership { line, column, kind } => Self::Ownership {
                line: *line,
                column: *column,
                kind: kind.into(),
            },
        }
    }
}

impl From<StoredErrorKind> for ErrorKind {
    fn from(value: StoredErrorKind) -> Self {
        match value {
            StoredErrorKind::Lexer {
                line,
                column,
                message,
            } => Self::Lexer {
                line,
                column,
                message,
            },
            StoredErrorKind::DeprecatedSyntax {
                line,
                column,
                message,
            } => Self::DeprecatedSyntax {
                line,
                column,
                message,
            },
            StoredErrorKind::Parser {
                line,
                column,
                message,
            } => Self::Parser {
                line,
                column,
                message,
            },
            StoredErrorKind::Backend { message } => Self::Backend { message },
            StoredErrorKind::Runtime { message } => Self::Runtime { message },
            StoredErrorKind::Type {
                line,
                column,
                message,
            } => Self::Type {
                line,
                column,
                message,
            },
            StoredErrorKind::Ownership { line, column, kind } => Self::Ownership {
                line,
                column,
                kind: kind.into(),
            },
        }
    }
}

impl From<&MssError> for StoredMssError {
    fn from(value: &MssError) -> Self {
        match value {
            MssError::MutationWhileShared => Self::MutationWhileShared,
            MssError::MultipleMutableRefs => Self::MultipleMutableRefs,
            MssError::MoveWhileBorrowed => Self::MoveWhileBorrowed,
            MssError::UseAfterMove => Self::UseAfterMove,
            MssError::DropWhileBorrowed => Self::DropWhileBorrowed,
            MssError::DoubleDrop => Self::DoubleDrop,
            MssError::BorrowOutOfScope => Self::BorrowOutOfScope,
            MssError::InvalidMove => Self::InvalidMove,
            MssError::UnsafeViolation => Self::UnsafeViolation,
        }
    }
}

impl From<StoredMssError> for MssError {
    fn from(value: StoredMssError) -> Self {
        match value {
            StoredMssError::MutationWhileShared => Self::MutationWhileShared,
            StoredMssError::MultipleMutableRefs => Self::MultipleMutableRefs,
            StoredMssError::MoveWhileBorrowed => Self::MoveWhileBorrowed,
            StoredMssError::UseAfterMove => Self::UseAfterMove,
            StoredMssError::DropWhileBorrowed => Self::DropWhileBorrowed,
            StoredMssError::DoubleDrop => Self::DoubleDrop,
            StoredMssError::BorrowOutOfScope => Self::BorrowOutOfScope,
            StoredMssError::InvalidMove => Self::InvalidMove,
            StoredMssError::UnsafeViolation => Self::UnsafeViolation,
        }
    }
}
