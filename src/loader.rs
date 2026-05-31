use crate::avens::{ImportMode, MireImportEntry, find_project_root, load_manifest_imports};
use crate::error::{ErrorKind, MireError, Result};
use crate::incremental::{
    CacheSettings, CachedImport, CachedParsedFile, IncrementalCache, LoadedFile, LoadedProgram,
    collect_statement_dependencies, source_hash, statement_export_name,
};
use crate::parser::ast::Statement;
use crate::parser::{Program, parse};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

pub fn load_program_from_file(path: &Path) -> Result<Program> {
    Ok(load_program_with_metadata(path)?.program)
}

pub fn load_program_with_metadata(path: &Path) -> Result<LoadedProgram> {
    let settings = CacheSettings::resolve_for(path, Default::default())?;
    load_program_with_metadata_with_settings(path, settings, ImportMode::Legacy)
}

pub fn load_program_with_metadata_with_settings(
    path: &Path,
    settings: CacheSettings,
    import_mode: ImportMode,
) -> Result<LoadedProgram> {
    let canonical = path.canonicalize().map_err(|err| {
        MireError::new(ErrorKind::Runtime {
            message: format!("Could not resolve '{}': {}", path.display(), err),
        })
    })?;

    let Some(project_root) =
        find_project_root(canonical.parent().unwrap_or_else(|| Path::new(".")))
    else {
        return load_shallow_program(&canonical);
    };

    let manifest_imports = load_manifest_imports(&project_root).unwrap_or_default();
    let mut resolver = ImportResolver::new(
        project_root,
        IncrementalCache::load_with_settings(&canonical, settings)?,
        import_mode,
        manifest_imports,
    );
    let statements = resolver.load_file(&canonical)?;
    resolver.cache.save()?;
    let statement_origins = statements.iter().map(|stmt| stmt.origin.clone()).collect();
    let program_statements = statements.into_iter().map(|stmt| stmt.statement).collect();
    Ok(LoadedProgram {
        program: Program {
            statements: program_statements,
        },
        files: resolver.files,
        statement_origins,
        sources: resolver.sources,
    })
}

fn load_shallow_program(path: &Path) -> Result<LoadedProgram> {
    let source = read_source_file(path)?;
    let hash = source_hash(&source);
    let program = parse(&source).map_err(|err| {
        err.with_source(source.clone())
            .with_filename(path.display().to_string())
    })?;
    if contains_local_import(&program.statements) {
        return Err(MireError::new(ErrorKind::Runtime {
            message: format!(
                "Local import statements require a Mire project root with owl.toml: '{}'",
                path.display()
            ),
        }));
    }
    let mut files = HashMap::new();
    files.insert(
        path.to_path_buf(),
        LoadedFile {
            hash,
            direct_dependencies: Vec::new(),
        },
    );
    let statement_origins = vec![path.to_path_buf(); program.statements.len()];
    let mut sources = HashMap::new();
    sources.insert(path.to_path_buf(), source);
    Ok(LoadedProgram {
        program,
        files,
        statement_origins,
        sources,
    })
}

struct ImportResolver {
    project_root: PathBuf,
    cache: IncrementalCache,
    expanded_cache: HashMap<PathBuf, Vec<ExpandedStatement>>,
    active_stack: HashSet<PathBuf>,
    files: HashMap<PathBuf, LoadedFile>,
    sources: HashMap<PathBuf, String>,
    import_mode: ImportMode,
    manifest_imports: HashMap<String, MireImportEntry>,
}

impl ImportResolver {
    fn new(
        project_root: PathBuf,
        cache: IncrementalCache,
        import_mode: ImportMode,
        manifest_imports: HashMap<String, MireImportEntry>,
    ) -> Self {
        Self {
            project_root,
            cache,
            expanded_cache: HashMap::new(),
            active_stack: HashSet::new(),
            files: HashMap::new(),
            sources: HashMap::new(),
            import_mode,
            manifest_imports,
        }
    }

    fn load_file(&mut self, path: &Path) -> Result<Vec<ExpandedStatement>> {
        let canonical = path.canonicalize().map_err(|err| {
            MireError::new(ErrorKind::Runtime {
                message: format!("Could not resolve '{}': {}", path.display(), err),
            })
        })?;

        if let Some(cached) = self.expanded_cache.get(&canonical) {
            return Ok(cached.clone());
        }

        if !self.active_stack.insert(canonical.clone()) {
            return Err(MireError::new(ErrorKind::Runtime {
                message: format!("Cyclic local import detected at '{}'", canonical.display()),
            }));
        }

        let parsed = self.load_or_parse_file(&canonical)?;
        let imported_symbol_candidates = collect_program_dependency_candidates(&parsed.program);
        let mut expanded = Vec::new();
        let mut direct_dependencies = Vec::new();
        for statement in parsed.program.statements {
            match statement {
                Statement::Use {
                    path,
                    alias: _,
                    items,
                    is_local: false,
                } if path == "std" => {
                    let imported_path = self.std_entry_path()?;
                    let selected = if items.is_some() {
                        items
                    } else if matches!(self.import_mode, ImportMode::Reachable) {
                        self.infer_reachable_import_items(
                            &imported_path,
                            None,
                            &imported_symbol_candidates,
                        )?
                    } else {
                        None
                    };
                    let imported = if selected.is_some() {
                        self.load_selected_imports(&imported_path, selected.as_deref())?
                    } else {
                        self.load_file(&imported_path)?
                    };
                    direct_dependencies.push(imported_path.clone());
                    expanded.extend(select_imported_statements(
                        &imported,
                        selected.as_deref(),
                        &imported_path,
                    )?);
                }
                Statement::Use {
                    path,
                    alias,
                    items,
                    is_local,
                } if is_local => {
                    if alias.is_some() {
                        self.active_stack.remove(&canonical);
                        return Err(MireError::new(ErrorKind::Runtime {
                            message: "Local import statements do not support aliasing".to_string(),
                        }));
                    }
                    let imported_path = match parsed
                        .local_imports
                        .iter()
                        .find(|import| import.raw_path == path && import.items == items)
                    {
                        Some(import) => import.resolved_path.clone(),
                        None => {
                            self.active_stack.remove(&canonical);
                            return Err(MireError::new(ErrorKind::Runtime {
                                message: format!(
                                    "Incremental cache is missing local import metadata for '{}'",
                                    path
                                ),
                            }));
                        }
                    };
                    let selected = if items.is_some() {
                        items
                    } else if matches!(self.import_mode, ImportMode::Reachable) {
                        self.infer_reachable_import_items(
                            &imported_path,
                            None,
                            &imported_symbol_candidates,
                        )?
                    } else {
                        None
                    };
                    let imported = if selected.is_some() {
                        self.load_selected_imports(&imported_path, selected.as_deref())?
                    } else {
                        self.load_file(&imported_path)?
                    };
                    direct_dependencies.push(imported_path.clone());
                    expanded.extend(select_imported_statements(
                        &imported,
                        selected.as_deref(),
                        &imported_path,
                    )?);
                }
                Statement::Use {
                    path,
                    alias: _,
                    items,
                    is_local: false,
                } if path != "std"
                    && !path.starts_with("__")
                    && !path.starts_with("stdall:")
                    && !path.starts_with("stdselect:")
                    && !path.starts_with("stdalias:") =>
                {
                    if let Some(submodules) = items {
                        let module_dir = self.resolve_module_dir(&path)?;
                        let imported = self.load_all_modules(&path, &module_dir, &submodules)?;
                        direct_dependencies.extend(imported.iter().map(|e| e.origin.clone()));
                        expanded.extend(imported);
                    } else {
                        let module_path = self.resolve_module_path(&path)?;
                        let selected = if matches!(self.import_mode, ImportMode::Reachable) {
                            self.infer_reachable_import_items(
                                &module_path,
                                Some(path.as_str()),
                                &imported_symbol_candidates,
                            )?
                        } else {
                            None
                        };
                        let imported = if selected.is_some() {
                            self.load_module_selected(&path, &module_path, selected.as_deref())?
                        } else {
                            self.load_module(&path, &module_path)?
                        };
                        direct_dependencies.push(module_path);
                        expanded.extend(imported);
                    }
                }
                other => expanded.push(ExpandedStatement {
                    statement: other,
                    origin: canonical.clone(),
                }),
            }
        }

        self.active_stack.remove(&canonical);
        self.files.insert(
            canonical.clone(),
            LoadedFile {
                hash: parsed.hash,
                direct_dependencies,
            },
        );
        self.expanded_cache
            .insert(canonical.clone(), expanded.clone());
        Ok(expanded)
    }

    fn infer_reachable_import_items(
        &mut self,
        path: &Path,
        module_prefix: Option<&str>,
        candidates: &HashSet<String>,
    ) -> Result<Option<Vec<String>>> {
        let parsed = self.load_or_parse_file(path)?;
        if parsed.exports.is_empty() {
            return Ok(None);
        }

        let mut selected = Vec::new();
        for export in &parsed.exports {
            let export_tail = export
                .rsplit_once('.')
                .map_or(export.as_str(), |(_, tail)| tail);
            let prefixed = module_prefix.map(|prefix| format!("{prefix}.{export_tail}"));
            let prefixed_double_colon =
                module_prefix.map(|prefix| format!("{prefix}::{export_tail}"));
            if candidates.contains(export)
                || candidates.contains(export_tail)
                || prefixed
                    .as_ref()
                    .is_some_and(|value| candidates.contains(value))
                || prefixed_double_colon
                    .as_ref()
                    .is_some_and(|value| candidates.contains(value))
            {
                selected.push(export_tail.to_string());
            }
        }

        if selected.is_empty() {
            return Ok(None);
        }
        selected.sort();
        selected.dedup();
        Ok(Some(selected))
    }

    fn load_or_parse_file(&mut self, path: &Path) -> Result<ResolvedFile> {
        let source = read_source_file(path)?;
        self.sources.insert(path.to_path_buf(), source.clone());
        let hash = source_hash(&source);
        if let Some(cached) = self.cache.cached_file(path, hash) {
            return Ok(ResolvedFile::from_cached(cached, source));
        }

        let program = parse(&source).map_err(|err| {
            err.with_source(source.clone())
                .with_filename(path.display().to_string())
        })?;
        let mut local_imports = Vec::new();
        for statement in &program.statements {
            if let Statement::Use {
                path: import_path,
                items,
                is_local,
                ..
            } = statement
                && *is_local
            {
                local_imports.push(CachedImport {
                    raw_path: import_path.clone(),
                    resolved_path: self.resolve_local_import(import_path, path)?,
                    items: items.clone(),
                });
            }
        }
        let exports: Vec<String> = program
            .statements
            .iter()
            .filter_map(statement_export_name)
            .map(ToString::to_string)
            .collect();
        self.cache.store_file(
            path,
            CachedParsedFile {
                hash,
                exports: exports.clone(),
                local_imports: local_imports.clone(),
                program: program.clone(),
            },
        )?;
        Ok(ResolvedFile {
            hash,
            program,
            exports,
            local_imports,
        })
    }

    fn load_selected_imports(
        &mut self,
        path: &Path,
        items: Option<&[String]>,
    ) -> Result<Vec<ExpandedStatement>> {
        let parsed = self.load_or_parse_file(path)?;
        if !parsed.local_imports.is_empty() {
            return self.load_file(path);
        }
        self.files.insert(
            path.to_path_buf(),
            LoadedFile {
                hash: parsed.hash,
                direct_dependencies: Vec::new(),
            },
        );
        let expanded: Vec<ExpandedStatement> = parsed
            .program
            .statements
            .into_iter()
            .map(|statement| ExpandedStatement {
                statement,
                origin: path.to_path_buf(),
            })
            .collect();
        select_imported_statements(&expanded, items, path)
    }

    fn resolve_local_import(&self, raw_path: &str, importer_path: &Path) -> Result<PathBuf> {
        if !raw_path.starts_with("./") {
            return Err(MireError::new(ErrorKind::Runtime {
                message: format!("Local import '{}' must start with './'", raw_path),
            }));
        }

        let relative = &raw_path[2..];
        let importer_dir = importer_path
            .parent()
            .unwrap_or(self.project_root.as_path());

        // First try ./<name>.mire
        let mut candidate = importer_dir.join(relative);
        if candidate.extension().is_none() {
            candidate.set_extension("mire");
        }
        if let Ok(canonical) = candidate.canonicalize() {
            return Ok(canonical);
        }

        // Then try ./<name>/mod.mire (directory module)
        let dir_candidate = importer_dir.join(relative).join("mod.mire");
        let canonical = dir_candidate.canonicalize().map_err(|err| {
            MireError::new(ErrorKind::Runtime {
                message: format!("Could not resolve local import '{}': {}", raw_path, err),
            })
        })?;

        if importer_path.starts_with(&self.project_root) {
            if !canonical.starts_with(&self.project_root) {
                return Err(MireError::new(ErrorKind::Runtime {
                    message: format!(
                        "Local import '{}' escapes the project root '{}'",
                        raw_path,
                        self.project_root.display()
                    ),
                }));
            }
        } else {
            let bundled_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/modules");
            if !canonical.starts_with(&bundled_root) {
                return Err(MireError::new(ErrorKind::Runtime {
                    message: format!(
                        "Local import '{}' escapes bundled modules root '{}'",
                        raw_path,
                        bundled_root.display()
                    ),
                }));
            }
        }

        Ok(canonical)
    }

    fn std_entry_path(&self) -> Result<PathBuf> {
        let project_candidate = self.project_root.join("src/modules/kioto/mod.mire");
        if let Ok(path) = project_candidate.canonicalize() {
            return Ok(path);
        }

        let bundled_candidate =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/modules/kioto/mod.mire");
        bundled_candidate.canonicalize().map_err(|err| {
            MireError::new(ErrorKind::Runtime {
                message: format!(
                    "Could not resolve std module entry '{}' nor bundled '{}': {}",
                    project_candidate.display(),
                    bundled_candidate.display(),
                    err
                ),
            })
        })
    }

    fn owl_home_modules(&self) -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| "~".to_string());
        PathBuf::from(home).join(".owl").join("modules")
    }

    fn resolve_import_from_manifest(&self, name: &str) -> Option<PathBuf> {
        self.manifest_imports.get(name).and_then(|entry| {
            match entry {
                MireImportEntry::Simple { version: _v } => {
                    // version-based: check owl home
                    let owl = self.owl_home_modules().join(name).join("lib.mire");
                    if owl.exists() { Some(owl) } else { None }
                }
                MireImportEntry::PathOnly { path } | MireImportEntry::WithPath { path, .. } => {
                    let p = PathBuf::from(path);
                    let candidate = if p.is_absolute() {
                        p
                    } else {
                        self.project_root.join(&p)
                    };
                    // Try direct file, then <path>/lib.mire
                    if candidate.exists() && candidate.extension().is_some() {
                        Some(candidate)
                    } else {
                        let lib = candidate.join("lib.mire");
                        if lib.exists() { Some(lib) } else { None }
                    }
                }
            }
        })
    }

    fn resolve_module_path(&self, name: &str) -> Result<PathBuf> {
        // 0. Check manifest imports first
        if let Some(path) = self.resolve_import_from_manifest(name) {
            return Ok(path);
        }
        // 1. Project local: ./<name>/lib.mire
        let project_candidate = self.project_root.join(name).join("lib.mire");
        if let Ok(path) = project_candidate.canonicalize() {
            return Ok(path);
        }
        // 2. Project modules dir: ./modules/<name>/lib.mire
        let local_modules_candidate = self
            .project_root
            .join("modules")
            .join(name)
            .join("lib.mire");
        if let Ok(path) = local_modules_candidate.canonicalize() {
            return Ok(path);
        }
        // 3. Global owl modules: ~/.owl/modules/<name>/lib.mire
        let owl_candidate = self.owl_home_modules().join(name).join("lib.mire");
        if let Ok(path) = owl_candidate.canonicalize() {
            return Ok(path);
        }
        // 4. Global owl modules with code/: ~/.owl/modules/<name>/code/lib.mire
        let owl_code_candidate = self
            .owl_home_modules()
            .join(name)
            .join("code")
            .join("lib.mire");
        if let Ok(path) = owl_code_candidate.canonicalize() {
            return Ok(path);
        }
        // 5. Bundled with compiler (workspace): <CARGO_MANIFEST_DIR>/<name>/modules/lib.mire
        let workspace_project_candidate = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join(name)
            .join("modules")
            .join("lib.mire");
        if let Ok(path) = workspace_project_candidate.canonicalize() {
            return Ok(path);
        }
        // 6. Bundled with compiler (src): <CARGO_MANIFEST_DIR>/src/modules/<name>/lib.mire
        let bundled_candidate = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("src/modules")
            .join(name)
            .join("lib.mire");
        if let Ok(path) = bundled_candidate.canonicalize() {
            return Ok(path);
        }
        // 7. Bundled with compiler (src): <CARGO_MANIFEST_DIR>/src/modules/<name>/mod.mire
        let bundled_mod_candidate = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("src/modules")
            .join(name)
            .join("mod.mire");
        bundled_mod_candidate.canonicalize().map_err(|err| {
            MireError::new(ErrorKind::Runtime {
                message: format!(
                    "Could not resolve module '{}' (tried '{}', '{}', '{}', '{}', '{}', '{}' and '{}'): {}",
                    name,
                    project_candidate.display(),
                    local_modules_candidate.display(),
                    owl_candidate.display(),
                    owl_code_candidate.display(),
                    workspace_project_candidate.display(),
                    bundled_candidate.display(),
                    bundled_mod_candidate.display(),
                    err
                ),
            })
        })
    }

    fn resolve_module_dir(&self, name: &str) -> Result<PathBuf> {
        // 0. Check manifest imports first
        if let Some(path) = self.resolve_import_from_manifest(name)
            && let Some(dir) = path.parent()
        {
            return Ok(dir.to_path_buf());
        }
        // 1. Project local: ./<name>/
        let project_candidate = self.project_root.join(name);
        if let Ok(path) = project_candidate.canonicalize() {
            return Ok(path);
        }
        // 2. Project modules dir: ./modules/<name>/
        let local_modules_candidate = self.project_root.join("modules").join(name);
        if let Ok(path) = local_modules_candidate.canonicalize() {
            return Ok(path);
        }
        // 3. Global owl modules: ~/.owl/modules/<name>/
        let owl_candidate = self.owl_home_modules().join(name);
        if let Ok(path) = owl_candidate.canonicalize() {
            return Ok(path);
        }
        // 4. Global owl modules with code/: ~/.owl/modules/<name>/code/
        let owl_code_candidate = self.owl_home_modules().join(name).join("code");
        if let Ok(path) = owl_code_candidate.canonicalize() {
            return Ok(path);
        }
        // 5. Project local: ./<name>/modules/
        let project_modules_candidate = self.project_root.join(name).join("modules");
        if let Ok(path) = project_modules_candidate.canonicalize() {
            return Ok(path);
        }
        // 6. Bundled with compiler (workspace): <CARGO_MANIFEST_DIR>/<name>/modules/
        let workspace_project_candidate = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join(name)
            .join("modules");
        if let Ok(path) = workspace_project_candidate.canonicalize() {
            return Ok(path);
        }
        // 7. Bundled with compiler (src): <CARGO_MANIFEST_DIR>/src/modules/<name>/
        let bundled_candidate = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("src/modules")
            .join(name);
        bundled_candidate.canonicalize().map_err(|err| {
            MireError::new(ErrorKind::Runtime {
                message: format!(
                    "Could not resolve module directory '{}' (tried '{}', '{}', '{}', '{}', '{}', '{}' and '{}'): {}",
                    name,
                    project_candidate.display(),
                    local_modules_candidate.display(),
                    owl_candidate.display(),
                    owl_code_candidate.display(),
                    project_modules_candidate.display(),
                    workspace_project_candidate.display(),
                    bundled_candidate.display(),
                    err
                ),
            })
        })
    }

    fn load_module(&mut self, module_name: &str, path: &Path) -> Result<Vec<ExpandedStatement>> {
        let loaded = self.load_file(path)?;
        Ok(loaded
            .into_iter()
            .map(|mut stmt| {
                stmt.statement = prefix_statement_name(&stmt.statement, module_name);
                stmt
            })
            .collect())
    }

    fn load_module_selected(
        &mut self,
        module_name: &str,
        path: &Path,
        items: Option<&[String]>,
    ) -> Result<Vec<ExpandedStatement>> {
        let loaded = self.load_selected_imports(path, items)?;
        Ok(loaded
            .into_iter()
            .map(|mut stmt| {
                stmt.statement = prefix_statement_name(&stmt.statement, module_name);
                stmt
            })
            .collect())
    }

    fn load_all_modules(
        &mut self,
        module_name: &str,
        module_dir: &Path,
        items: &[String],
    ) -> Result<Vec<ExpandedStatement>> {
        let mut result = Vec::new();
        for item in items {
            let file_path = module_dir.join(format!("{}.mire", item));
            let file_path = if file_path.exists() {
                file_path
            } else {
                module_dir.join(item).join("mod.mire")
            };
            let canonical = file_path.canonicalize().map_err(|err| {
                MireError::new(ErrorKind::Runtime {
                    message: format!(
                        "Could not resolve module file '{}': {}",
                        file_path.display(),
                        err
                    ),
                })
            })?;
            let loaded = self.load_file(&canonical)?;
            let prefix = if item == "lib" {
                module_name.to_string()
            } else {
                item.to_string()
            };
            result.extend(loaded.into_iter().map(|mut stmt| {
                stmt.statement = prefix_statement_name(&stmt.statement, &prefix);
                stmt
            }));
        }
        Ok(result)
    }
}

fn collect_program_dependency_candidates(program: &Program) -> HashSet<String> {
    let mut candidates = HashSet::new();
    for statement in &program.statements {
        if matches!(statement, Statement::Use { .. }) {
            continue;
        }
        let mut deps = Vec::new();
        collect_statement_dependencies(statement, &mut deps);
        for dep in deps {
            candidates.insert(dep.clone());
            if let Some((_, tail)) = dep.rsplit_once('.') {
                candidates.insert(tail.to_string());
            }
            if let Some((_, tail)) = dep.rsplit_once("::") {
                candidates.insert(tail.to_string());
            }
        }
    }
    candidates
}

fn prefix_statement_name(statement: &Statement, prefix: &str) -> Statement {
    let mut stmt = statement.clone();
    let name = match &mut stmt {
        Statement::Let { name, .. }
        | Statement::Function { name, .. }
        | Statement::Type { name, .. }
        | Statement::Skill { name, .. }
        | Statement::Module { name, .. }
        | Statement::Enum { name, .. } => Some(name),
        _ => None,
    };
    if let Some(name) = name {
        *name = format!("{}.{}", prefix, name);
    }
    stmt
}

fn select_imported_statements(
    statements: &[ExpandedStatement],
    items: Option<&[String]>,
    import_path: &Path,
) -> Result<Vec<ExpandedStatement>> {
    if let Some(items) = items {
        let mut selected_indices = Vec::new();
        let mut selected = HashSet::new();
        for item in items {
            let statement_idx = statements
                .iter()
                .enumerate()
                .find(|statement| {
                    statement_export_name(&statement.1.statement) == Some(item.as_str())
                })
                .map(|(idx, _)| idx)
                .ok_or_else(|| {
                    MireError::new(ErrorKind::Runtime {
                        message: format!(
                            "Local import '{}' does not export '{}'",
                            import_path.display(),
                            item
                        ),
                    })
                })?;
            if selected.insert(statement_idx) {
                selected_indices.push(statement_idx);
            }
        }

        let mut cursor = 0usize;
        while cursor < selected_indices.len() {
            let idx = selected_indices[cursor];
            cursor += 1;

            let mut deps = Vec::new();
            collect_statement_dependencies(&statements[idx].statement, &mut deps);
            for dependency in deps {
                for candidate in [
                    Some(dependency.as_str()),
                    dependency.rsplit_once('.').map(|(_, tail)| tail),
                ] {
                    let Some(candidate_name) = candidate else {
                        continue;
                    };
                    for (dep_idx, statement) in statements.iter().enumerate() {
                        if statement_export_name(&statement.statement) == Some(candidate_name)
                            && selected.insert(dep_idx)
                        {
                            selected_indices.push(dep_idx);
                        }
                    }
                }
            }
        }

        let mut reachable = Vec::new();
        for (idx, statement) in statements.iter().enumerate() {
            if selected.contains(&idx) {
                reachable.push(statement.clone());
            }
        }
        return Ok(reachable);
    }

    Ok(statements
        .iter()
        .filter(|statement| statement_export_name(&statement.statement).is_some())
        .cloned()
        .collect())
}

fn contains_local_import(statements: &[Statement]) -> bool {
    statements.iter().any(|statement| match statement {
        Statement::Use { is_local, .. } => *is_local,
        _ => false,
    })
}

fn read_source_file(path: &Path) -> Result<String> {
    fs::read_to_string(path).map_err(|err| {
        MireError::new(ErrorKind::Runtime {
            message: format!("Could not read '{}': {}", path.display(), err),
        })
    })
}

struct ResolvedFile {
    hash: u64,
    program: Program,
    exports: Vec<String>,
    local_imports: Vec<CachedImport>,
}

#[derive(Clone)]
struct ExpandedStatement {
    statement: Statement,
    origin: PathBuf,
}

impl ResolvedFile {
    fn from_cached(cached: CachedParsedFile, source: String) -> Self {
        drop(source);
        Self {
            hash: cached.hash,
            program: cached.program,
            exports: cached.exports,
            local_imports: cached.local_imports,
        }
    }
}
