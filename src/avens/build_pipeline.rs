use super::*;

pub fn compile_file_with_avenys(source_path: &Path, options: &BuildOptions) -> Result<BuildResult> {
    let source = fs::read_to_string(source_path)?;
    let source_filename = source_path.display().to_string();
    let output_dir = default_output_dir(source_path, options.mode);
    fs::create_dir_all(&output_dir).map_err(|err| {
        MireError::new(ErrorKind::Runtime {
            message: format!(
                "Could not create build directory '{}': {}",
                output_dir.display(),
                err
            ),
        })
    })?;

    let stem = source_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("main");
    let binary_path = options
        .output
        .clone()
        .unwrap_or_else(|| output_dir.join(stem));
    let ir_path = options
        .persist_ir
        .then(|| output_dir.join(format!("{stem}.ll")));
    let optimized_ir_path = options
        .persist_ir
        .then(|| output_dir.join(format!("{stem}.opt.ll")));
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c_source_files: Vec<String> = {
        let mut files = Vec::new();
        for entry in std::fs::read_dir(manifest_dir.join("src/runtime")).map_err(|err| {
            MireError::new(ErrorKind::Runtime {
                message: format!("Could not read src/runtime: {}", err),
            })
        })? {
            let entry = entry.map_err(|err| {
                MireError::new(ErrorKind::Runtime {
                    message: format!("Could not read entry: {}", err),
                })
            })?;
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "c") {
                files.push(path.to_string_lossy().to_string());
            }
        }
        for entry in std::fs::read_dir(manifest_dir.join("src/pal/linux")).map_err(|err| {
            MireError::new(ErrorKind::Runtime {
                message: format!("Could not read src/pal/linux: {}", err),
            })
        })? {
            let entry = entry.map_err(|err| {
                MireError::new(ErrorKind::Runtime {
                    message: format!("Could not read entry: {}", err),
                })
            })?;
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "c") {
                files.push(path.to_string_lossy().to_string());
            }
        }
        files.sort();
        files
    };
    let c_sources_combined: String = {
        let mut combined = String::new();
        for src in &c_source_files {
            if let Ok(content) = fs::read_to_string(src) {
                combined.push_str(&content);
            }
        }
        combined
    };
    let cache_settings = CacheSettings::resolve_for(source_path, options.cache)?;
    let mut cache = IncrementalCache::load_with_settings(source_path, cache_settings)?;
    let loaded = load_program_with_metadata_with_settings(
        source_path,
        cache_settings,
        options.import_mode,
    )?;
    if options.debug_dump
        && let Some(report) = cache.analysis_invalidation_report(source_path, &loaded.program)
    {
        eprintln!(
            "[AVENYS][incremental] changed_units={} invalidated_units={} added_units={} removed_units={}",
            report.changed_units.len(),
            report.invalidated_units.len(),
            report.added_units.len(),
            report.removed_units.len(),
        );
    }
    let fingerprint = build_fingerprint(
        source_path,
        &loaded.files,
        options.mode,
        options.import_mode,
        options.opt_level,
        options.emit_binary,
        &c_sources_combined,
    );

    if let Some(entry) = cache.build_entry(
        source_path,
        options.mode,
        options.import_mode,
        options.emit_binary,
        options.persist_ir,
    ) && entry.fingerprint == fingerprint
        && (!options.emit_binary || entry.binary_path.exists())
        && entry.binary_path == binary_path
        && entry.ir_path == ir_path
        && entry.optimized_ir_path == optimized_ir_path
        && entry.ir_path.as_ref().is_none_or(|path| path.exists())
        && entry
            .optimized_ir_path
            .as_ref()
            .is_none_or(|path| path.exists())
    {
        cache.record_build_hit();
        if options.debug_dump {
            let metrics = cache.metrics();
            eprintln!(
                "[AVENYS][incremental] cache_metrics file_hit={} file_miss={} analysis_hit={} analysis_miss={} build_hit={} build_miss={} evictions={}",
                metrics.file_hits,
                metrics.file_misses,
                metrics.analysis_hits,
                metrics.analysis_misses,
                metrics.build_hits,
                metrics.build_misses,
                metrics.evictions,
            );
        }
        return Ok(BuildResult {
            binary_path,
            ir_path,
            optimized_ir_path,
            used_optimizations: !matches!(options.opt_level, OptLevel::O0),
        });
    }
    cache.record_build_miss();

    let program = if let Some(cached) = cache.cached_analysis(source_path) {
        match cached {
            CachedAnalysis::Success(program) => program,
            CachedAnalysis::Error(error) => return Err(error),
        }
    } else {
        let mut program = loaded.program;
        let analysis_result = if let Some(cached) = cache.latest_successful_analysis(source_path) {
            let (selection, _) = prepare_program_with_partial_analysis_reuse(&mut program, cached);
            if selection
                .statement_mask
                .iter()
                .all(|should_check| !should_check)
            {
                Ok(())
            } else {
                analyze_program_with_origins_partial(
                    &mut program,
                    &source,
                    &loaded.statement_origins,
                    &loaded.sources,
                    &selection,
                )
                .map(|_| ())
            }
        } else {
            analyze_program_with_origins(
                &mut program,
                &source,
                &loaded.statement_origins,
                &loaded.sources,
            )
            .map(|_| ())
        };

        if let Err(err) = analysis_result {
            let err = if err.source().is_none() {
                err.with_source(source.clone())
            } else {
                err
            };
            let err = if err.filename().is_none() {
                err.with_filename(source_filename.clone())
            } else {
                err
            };
            cache.store_analysis_error(source_path, &program, &err)?;
            cache.save()?;
            return Err(err);
        }
        cache.store_analysis(source_path, &program)?;
        program
    };

    let mut warning_program = program.clone();
    let warning_report = analyze_program_with_warnings(
        &mut warning_program,
        &source,
        Some(&source_filename),
        WarningConfig {
            filter: options.warning_filter.clone(),
            deny: options.deny_warnings.clone(),
        },
    )?;
    for diagnostic in &warning_report.diagnostics {
        eprintln!("{}", format_diagnostic(diagnostic, true));
    }
    if warning_report
        .diagnostics
        .iter()
        .any(|diag| matches!(diag.severity, Severity::Error))
    {
        return Err(MireError::runtime(
            "Compilation aborted due to denied warnings".to_string(),
        ));
    }

    let (ir, extern_libs) = LlvmIrGen::new().compile_program(&program).map_err(|err| {
        let err = if err.source().is_none() {
            err.with_source(source.clone())
        } else {
            err
        };
        if err.filename().is_none() {
            err.with_filename(source_filename.clone())
        } else {
            err
        }
    })?;
    if let Some(path) = &ir_path {
        fs::write(path, &ir).map_err(|err| {
            MireError::new(ErrorKind::Runtime {
                message: format!("Could not write '{}': {}", path.display(), err),
            })
        })?;
    }

    let final_ir = if matches!(options.opt_level, OptLevel::O0) {
        ir
    } else {
        optimize_ir(&ir, options.opt_level)?
    };

    if let Some(path) = &optimized_ir_path {
        fs::write(path, &final_ir).map_err(|err| {
            MireError::new(ErrorKind::Runtime {
                message: format!("Could not write '{}': {}", path.display(), err),
            })
        })?;
    }

    if options.emit_binary {
        compile_binary_from_ir(&final_ir, &c_source_files, &binary_path, options.opt_level, &extern_libs, &manifest_dir)?;
    }

    cache.store_build(
        source_path,
        BuildCacheEntry {
            fingerprint,
            mode: options.mode,
            import_mode: options.import_mode,
            opt_level: options.opt_level,
            emit_binary: options.emit_binary,
            persist_ir: options.persist_ir,
            binary_path: binary_path.clone(),
            ir_path: ir_path.clone(),
            optimized_ir_path: optimized_ir_path.clone(),
        },
    );
    if options.debug_dump {
        let metrics = cache.metrics();
        eprintln!(
            "[AVENYS][incremental] cache_metrics file_hit={} file_miss={} analysis_hit={} analysis_miss={} build_hit={} build_miss={} evictions={}",
            metrics.file_hits,
            metrics.file_misses,
            metrics.analysis_hits,
            metrics.analysis_misses,
            metrics.build_hits,
            metrics.build_misses,
            metrics.evictions,
        );
    }
    cache.save()?;

    Ok(BuildResult {
        binary_path,
        ir_path,
        optimized_ir_path,
        used_optimizations: !matches!(options.opt_level, OptLevel::O0),
    })
}

pub fn default_output_dir(source_path: &Path, mode: BuildMode) -> PathBuf {
    if let Some(project_root) =
        find_project_root(source_path.parent().unwrap_or_else(|| Path::new(".")))
    {
        return project_root.join("bin").join(match mode {
            BuildMode::Debug => "debug",
            BuildMode::Release => "release",
        });
    }

    source_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(match mode {
            BuildMode::Debug => "debug",
            BuildMode::Release => "release",
        })
}
