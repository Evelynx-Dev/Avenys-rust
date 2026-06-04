use mire::error::diagnostic::{DiagnosticCode, Severity, WarningFilter};
use mire::error::format::format_diagnostic;
use mire::lexer::tokenize;
use mire::parser::parse;
use mire::{
    BuildMode, BuildOptions, CacheOverrides, ImportMode, MireError, MireImportEntry, MireManifest,
    MireProject, OptLevel, WarningConfig, analyze_program, analyze_program_with_warnings,
    compile_file_with_avenys, default_output_dir, load_program_from_file, load_project_manifest,
    project_manifest_path, write_manifest,
};
use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

#[derive(Debug, Clone)]
struct CommonOptions {
    mode: BuildMode,
    opt_level: OptLevel,
    import_mode: ImportMode,
    output: Option<PathBuf>,
    cache: CacheOverrides,
    warn: WarningCliOptions,
}

#[derive(Debug, Clone)]
struct WarningCliOptions {
    filter: WarningFilter,
    deny: HashSet<DiagnosticCode>,
}

#[derive(Debug, Clone)]
struct DebugOptions {
    common: CommonOptions,
    file: Option<String>,
    show_tokens: bool,
    show_ast: bool,
    run_binary: bool,
    emit_ir_only: bool,
}

fn main() -> ExitCode {
    match run_cli() {
        Ok(code) => ExitCode::from(code as u8),
        Err(err) => {
            eprintln!("{}", err.format_color());
            ExitCode::from(1)
        }
    }
}

fn run_cli() -> Result<i32, MireError> {
    let args: Vec<String> = env::args().collect();
    let cwd = env::current_dir().map_err(runtime_err)?;

    if args.len() <= 1 {
        print_help();
        return Ok(1);
    }

    match args[1].as_str() {
        "run" => run_command(&cwd, &args[2..]),
        "build" => build_command(&cwd, &args[2..]),
        "check" => check_command(&cwd, &args[2..]),
        "debug" => debug_command(&cwd, &args[2..]),
        "test" => test_command(&cwd, &args[2..]),
        "import" => import_command(&cwd, &args[2..]),
        "help" | "--help" | "-h" => {
            print_help();
            Ok(0)
        }
        _ => {
            print_help();
            Ok(1)
        }
    }
}

fn run_command(cwd: &Path, args: &[String]) -> Result<i32, MireError> {
    let (common, file, pass_through) = parse_run_options(cwd, args)?;
    let path = resolve_source_path(cwd, file)?;
    let options = BuildOptions {
        mode: common.mode,
        opt_level: common.opt_level,
        debug_dump: matches!(common.mode, BuildMode::Debug),
        output: common
            .output
            .clone()
            .or_else(|| Some(default_binary_path(&path, common.mode))),
        emit_binary: true,
        persist_ir: false,
        import_mode: common.import_mode,
        cache: common.cache,
        warning_filter: common.warn.filter,
        deny_warnings: common.warn.deny,
        module_paths: Vec::new(),
    };
    let build = compile_file_with_avenys(&path, &options)?;
    let mut cmd = Command::new(&build.binary_path);
    for arg in pass_through {
        cmd.arg(arg);
    }
    let status = cmd.status().map_err(runtime_err)?;
    Ok(status.code().unwrap_or(1))
}

fn build_command(cwd: &Path, args: &[String]) -> Result<i32, MireError> {
    let (common, file) = parse_common_with_file(cwd, args)?;
    let path = resolve_source_path(cwd, file)?;
    let options = BuildOptions {
        mode: common.mode,
        opt_level: common.opt_level,
        debug_dump: matches!(common.mode, BuildMode::Debug),
        output: common
            .output
            .or_else(|| Some(default_binary_path(&path, common.mode))),
        emit_binary: true,
        persist_ir: false,
        import_mode: common.import_mode,
        cache: common.cache,
        warning_filter: common.warn.filter,
        deny_warnings: common.warn.deny,
        module_paths: Vec::new(),
    };
    let build = compile_file_with_avenys(&path, &options)?;
    println!("{}", build.binary_path.display());
    Ok(0)
}

fn check_command(cwd: &Path, args: &[String]) -> Result<i32, MireError> {
    let (common, file) = parse_common_with_file(cwd, args)?;
    let path = resolve_source_path(cwd, file)?;
    let source = fs::read_to_string(&path).map_err(runtime_err)?;
    let mut program = load_program_from_file(&path)?;
    let _ = analyze_program(&mut program, &source)?;
    let report = analyze_program_with_warnings(
        &mut program,
        &source,
        Some(&path.display().to_string()),
        WarningConfig {
            filter: common.warn.filter,
            deny: common.warn.deny,
        },
    )?;

    let mut has_error = false;
    for diagnostic in &report.diagnostics {
        eprintln!("{}", format_diagnostic(diagnostic, true));
        if matches!(diagnostic.severity, Severity::Error) {
            has_error = true;
        }
    }
    Ok(if has_error { 1 } else { 0 })
}

fn debug_command(cwd: &Path, args: &[String]) -> Result<i32, MireError> {
    let options = parse_debug_options(cwd, args)?;
    let path = resolve_source_path(cwd, options.file.clone())?;
    let source = fs::read_to_string(&path).map_err(runtime_err)?;

    if options.show_tokens {
        let tokens = tokenize(&source).map_err(|err| {
            err.with_source(source.clone())
                .with_filename(path.display().to_string())
        })?;
        for token in &tokens {
            println!("{:?}", token);
        }
    }

    if options.show_ast {
        let program = parse(&source).map_err(|err| {
            err.with_source(source.clone())
                .with_filename(path.display().to_string())
        })?;
        println!("{:#?}", program);
    }

    let build = compile_file_with_avenys(
        &path,
        &BuildOptions {
            mode: options.common.mode,
            opt_level: options.common.opt_level,
            debug_dump: true,
            output: options
                .common
                .output
                .clone()
                .or_else(|| Some(default_binary_path(&path, options.common.mode))),
            emit_binary: !options.emit_ir_only,
            persist_ir: true,
            import_mode: options.common.import_mode,
            cache: options.common.cache,
            warning_filter: options.common.warn.filter,
            deny_warnings: options.common.warn.deny,
            module_paths: Vec::new(),
        },
    )?;

    if let Some(ir) = &build.ir_path {
        println!("IR: {}", ir.display());
    }
    if let Some(ir) = &build.optimized_ir_path {
        println!("OPT IR: {}", ir.display());
    }
    if options.run_binary && !options.emit_ir_only {
        let status = Command::new(&build.binary_path)
            .status()
            .map_err(runtime_err)?;
        return Ok(status.code().unwrap_or(1));
    }
    Ok(0)
}

fn test_command(cwd: &Path, args: &[String]) -> Result<i32, MireError> {
    let mut run = true;
    let mut verbose = false;
    let mut paths: Vec<String> = Vec::new();

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--no-run" => run = false,
            "--verbose" | "-v" => verbose = true,
            _ => paths.push(args[i].clone()),
        }
        i += 1;
    }

    let mut test_files: Vec<PathBuf> = Vec::new();

    if !paths.is_empty() {
        for p in &paths {
            let path = cwd.join(p);
            if path.is_dir() {
                let mut entries: Vec<_> = walkdir(&path, "*.mire")?;
                entries.sort();
                test_files.extend(entries);
            } else if path.is_file() {
                test_files.push(path);
            } else {
                eprintln!("warning: test path not found: {}", path.display());
            }
        }
    } else {
        let tests_dir = cwd.join("tests");
        if tests_dir.is_dir() {
            let mut entries: Vec<_> = walkdir(&tests_dir, "*.mire")?;
            entries.sort();
            test_files = entries;
        }
    }

    if test_files.is_empty() {
        println!("no extra tests found");
        return Ok(0);
    }

    let _total = test_files.len();
    let mut passed = 0u32;
    let mut failed = 0u32;

    for file in &test_files {
        let display = file.strip_prefix(cwd).unwrap_or(file).display().to_string();

        if verbose {
            print!("test {} ... ", display);
        }

        if !file.exists() {
            if verbose {
                println!("FAILED");
            } else {
                println!("FAILED: {} - file not found", display);
            }
            failed += 1;
            continue;
        }

        let options = BuildOptions {
            mode: BuildMode::Debug,
            opt_level: OptLevel::O0,
            debug_dump: false,
            output: Some(default_output_dir(file, BuildMode::Debug).join("test")),
            emit_binary: run,
            persist_ir: false,
            import_mode: ImportMode::default(),
            cache: Default::default(),
            warning_filter: WarningFilter::Default,
            deny_warnings: HashSet::new(),
            module_paths: Vec::new(),
        };

        match compile_file_with_avenys(file, &options) {
            Ok(build) => {
                if run {
                    match Command::new(&build.binary_path).status() {
                        Ok(status) if status.success() => {
                            if verbose {
                                println!("ok");
                            }
                            passed += 1;
                        }
                        Ok(status) => {
                            if verbose {
                                println!("FAILED");
                            } else {
                                println!("FAILED: {} (exit code: {:?})", display, status.code());
                            }
                            failed += 1;
                        }
                        Err(e) => {
                            if verbose {
                                println!("FAILED");
                            } else {
                                println!("FAILED: {} - run error: {}", display, e);
                            }
                            failed += 1;
                        }
                    }
                } else {
                    if verbose {
                        println!("ok");
                    }
                    passed += 1;
                }
            }
            Err(e) => {
                if verbose {
                    println!("FAILED");
                } else {
                    println!("FAILED: {} - {}", display, e);
                }
                failed += 1;
            }
        }
    }

    let status = if failed == 0 { "ok" } else { "FAILED" };
    println!(
        "\ntest result: {}. {} passed; {} failed; finished",
        status, passed, failed
    );

    Ok(if failed == 0 { 0 } else { 1 })
}

fn walkdir(dir: &Path, _pattern: &str) -> Result<Vec<PathBuf>, MireError> {
    let mut results = Vec::new();
    if !dir.is_dir() {
        return Ok(results);
    }
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        let Ok(entries) = fs::read_dir(&current) else {
            continue;
        };
        for entry in entries {
            let Ok(entry) = entry else { continue };
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if let Some(ext) = path.extension()
                && ext == "mire"
            {
                results.push(path);
            }
        }
    }
    Ok(results)
}

fn parse_run_options(
    cwd: &Path,
    args: &[String],
) -> Result<(CommonOptions, Option<String>, Vec<String>), MireError> {
    let mut split = 0usize;
    while split < args.len() {
        if args[split] == "--" {
            break;
        }
        split += 1;
    }
    let (left, right) = if split < args.len() {
        (&args[..split], args[split + 1..].to_vec())
    } else {
        (args, Vec::new())
    };

    let (common, file) = parse_common_with_file(cwd, left)?;
    Ok((common, file, right))
}

fn parse_common_with_file(
    cwd: &Path,
    args: &[String],
) -> Result<(CommonOptions, Option<String>), MireError> {
    let mut mode = BuildMode::Debug;
    let mut opt_level = OptLevel::O0;
    let mut import_mode = ImportMode::Legacy;
    let mut output = None;
    let mut file = None;
    let mut cache = CacheOverrides::default();
    let mut warn_all = false;
    let mut warn_codes = HashSet::new();
    let mut deny_codes = HashSet::new();

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--debug" => {
                mode = BuildMode::Debug;
                if matches!(opt_level, OptLevel::O0) {
                    opt_level = OptLevel::O0;
                }
            }
            "--release" => {
                mode = BuildMode::Release;
                if matches!(opt_level, OptLevel::O0) {
                    opt_level = OptLevel::O3;
                }
            }
            "-O" | "--opt-level" => {
                i += 1;
                let level = args.get(i).ok_or_else(|| {
                    runtime_msg("Missing optimization level after -O/--opt-level")
                })?;
                opt_level = OptLevel::parse(level)
                    .ok_or_else(|| runtime_msg("Invalid optimization level, use 0/1/2/3/s/z"))?;
            }
            flag if flag.starts_with("-O") && flag.len() > 2 => {
                opt_level = OptLevel::parse(&flag[2..])
                    .ok_or_else(|| runtime_msg("Invalid optimization level, use 0/1/2/3/s/z"))?;
            }
            "-o" | "--output" => {
                i += 1;
                let value = args
                    .get(i)
                    .ok_or_else(|| runtime_msg("Missing output path after -o/--output"))?;
                output = Some(PathBuf::from(value));
            }
            "--import-mode" => {
                i += 1;
                let value = args
                    .get(i)
                    .ok_or_else(|| runtime_msg("Missing value for --import-mode"))?;
                import_mode = ImportMode::parse(value).ok_or_else(|| {
                    runtime_msg("Invalid --import-mode value, use legacy|reachable")
                })?;
            }
            "--cache-max-units" => {
                i += 1;
                let value = args
                    .get(i)
                    .ok_or_else(|| runtime_msg("Missing value for --cache-max-units"))?;
                let parsed = value
                    .parse::<usize>()
                    .map_err(|_| runtime_msg("Invalid --cache-max-units value"))?;
                cache.max_units = Some(parsed);
            }
            "--no-analysis-cache" => cache.analysis_cache = Some(false),
            "--analysis-cache" => cache.analysis_cache = Some(true),
            "--warn-all" => warn_all = true,
            "-W" => {
                i += 1;
                let code = args
                    .get(i)
                    .ok_or_else(|| runtime_msg("Missing warning code after -W"))?;
                warn_codes.insert(parse_warning_code(code)?);
            }
            "--deny" => {
                i += 1;
                let code = args
                    .get(i)
                    .ok_or_else(|| runtime_msg("Missing warning code after --deny"))?;
                deny_codes.insert(parse_warning_code(code)?);
            }
            value if value.starts_with('-') => {
                return Err(runtime_msg(&format!("Unknown option: {value}")));
            }
            value => {
                if file.is_some() {
                    return Err(runtime_msg("Only one input file is supported"));
                }
                file = Some(value.to_string());
            }
        }
        i += 1;
    }

    if !matches!(mode, BuildMode::Release) && !matches!(opt_level, OptLevel::O0) {
        mode = BuildMode::Release;
    }

    let warning_filter = if warn_all {
        WarningFilter::All
    } else if warn_codes.is_empty() {
        WarningFilter::Default
    } else {
        WarningFilter::Codes(warn_codes)
    };

    if file.is_none() {
        file = default_entry_from_manifest(cwd)?;
    }

    Ok((
        CommonOptions {
            mode,
            opt_level,
            import_mode,
            output,
            cache,
            warn: WarningCliOptions {
                filter: warning_filter,
                deny: deny_codes,
            },
        },
        file,
    ))
}

fn parse_debug_options(cwd: &Path, args: &[String]) -> Result<DebugOptions, MireError> {
    let mut show_tokens = false;
    let mut show_ast = false;
    let mut run_binary = false;
    let mut emit_ir_only = false;
    let mut filtered = Vec::new();

    for arg in args {
        match arg.as_str() {
            "--tokens" | "-t" => show_tokens = true,
            "--ast" | "-p" => show_ast = true,
            "--run" | "-r" => run_binary = true,
            "--ir" => emit_ir_only = true,
            _ => filtered.push(arg.clone()),
        }
    }

    let (mut common, file) = parse_common_with_file(cwd, &filtered)?;
    common.mode = BuildMode::Debug;
    if matches!(common.opt_level, OptLevel::O0) {
        common.opt_level = OptLevel::O1;
    }

    Ok(DebugOptions {
        common,
        file,
        show_tokens,
        show_ast,
        run_binary,
        emit_ir_only,
    })
}

fn import_command(cwd: &Path, args: &[String]) -> Result<i32, MireError> {
    if args.is_empty() {
        eprintln!("Usage: mire import <module> [--version <ver>] [--path <path>]");
        eprintln!("  mire import kioto              add kioto module (version-based)");
        eprintln!("  mire import kioto --version 0.2 add kioto with version");
        eprintln!("  mire import ./local-mod         add local module");
        return Ok(1);
    }

    let module = &args[0];
    let mut version = String::new();
    let mut path = String::new();
    let mut json_output = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--version" | "-v" => {
                i += 1;
                version = args
                    .get(i)
                    .ok_or_else(|| {
                        MireError::runtime("Missing version after --version".to_string())
                    })?
                    .clone();
            }
            "--path" | "-p" => {
                i += 1;
                path = args
                    .get(i)
                    .ok_or_else(|| MireError::runtime("Missing path after --path".to_string()))?
                    .clone();
            }
            "--json" => {
                json_output = true;
            }
            _ => {
                return Err(MireError::runtime(format!("Unknown option: {}", args[i])));
            }
        }
        i += 1;
    }

    let manifest_path = project_manifest_path(cwd);
    let output_version = if version.is_empty() {
        "latest".to_string()
    } else {
        version.clone()
    };
    let output_path = if path.is_empty() {
        None
    } else {
        Some(path.clone())
    };

    // Load or create manifest
    let mut manifest = match load_project_manifest(cwd) {
        Ok(Some(m)) => m,
        _ => MireManifest {
            project: MireProject {
                name: cwd
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| "project".to_string()),
                version: "0.1.0".to_string(),
                entry: "code/main.mire".to_string(),
            },
            cache: None,
            imports: Default::default(),
        },
    };

    let entry = if !path.is_empty() {
        if !version.is_empty() {
            MireImportEntry::WithPath {
                version,
                path: path.clone(),
            }
        } else {
            MireImportEntry::PathOnly { path: path.clone() }
        }
    } else if !version.is_empty() {
        MireImportEntry::Simple { version }
    } else {
        MireImportEntry::Simple {
            version: "latest".to_string(),
        }
    };

    manifest.imports.entries.insert(module.clone(), entry);
    write_manifest(&manifest, &manifest_path)?;
    if json_output {
        println!(
            "{}",
            serde_json::json!({
                "status": "ok",
                "module": module,
                "manifest": manifest_path,
                "version": output_version,
                "path": output_path,
            })
        );
    } else {
        println!("Added import '{}' to {}", module, manifest_path.display());
    }
    Ok(0)
}

fn default_entry_from_manifest(cwd: &Path) -> Result<Option<String>, MireError> {
    let owl_toml = cwd.join("owl.toml");
    let path = if owl_toml.exists() {
        Some(owl_toml)
    } else {
        None
    };

    let Some(path) = path else { return Ok(None) };
    let raw = fs::read_to_string(&path).map_err(runtime_err)?;
    let value = raw
        .lines()
        .find_map(|line| line.trim().strip_prefix("entry"))
        .and_then(|tail| tail.split('=').nth(1))
        .map(|v| v.trim().trim_matches('"').to_string());
    Ok(value)
}

fn resolve_source_path(cwd: &Path, file: Option<String>) -> Result<PathBuf, MireError> {
    let file = file.ok_or_else(|| {
        runtime_msg("No input file provided and no `entry` was found in owl.toml")
    })?;
    let path = PathBuf::from(&file);
    let resolved = if path.is_absolute() {
        path
    } else {
        cwd.join(path)
    };
    if !resolved.exists() {
        return Err(runtime_msg(&format!(
            "Input file not found: {}",
            resolved.display()
        )));
    }
    Ok(resolved)
}

fn default_binary_path(source_path: &Path, mode: BuildMode) -> PathBuf {
    let stem = source_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("main");
    default_output_dir(source_path, mode).join(stem)
}

fn parse_warning_code(value: &str) -> Result<DiagnosticCode, MireError> {
    match value.trim().to_ascii_uppercase().as_str() {
        "W0001" => Ok(DiagnosticCode::W0001),
        "W0002" => Ok(DiagnosticCode::W0002),
        "W0003" => Ok(DiagnosticCode::W0003),
        "W0004" => Ok(DiagnosticCode::W0004),
        "W0005" => Ok(DiagnosticCode::W0005),
        "W0006" => Ok(DiagnosticCode::W0006),
        "W0007" => Ok(DiagnosticCode::W0007),
        "W0008" => Ok(DiagnosticCode::W0008),
        "W0009" => Ok(DiagnosticCode::W0009),
        "W0010" => Ok(DiagnosticCode::W0010),
        "W0011" => Ok(DiagnosticCode::W0011),
        "W0012" => Ok(DiagnosticCode::W0012),
        "W0013" => Ok(DiagnosticCode::W0013),
        "W0014" => Ok(DiagnosticCode::W0014),
        "W0015" => Ok(DiagnosticCode::W0015),
        "W0016" => Ok(DiagnosticCode::W0016),
        "W0017" => Ok(DiagnosticCode::W0017),
        "W0018" => Ok(DiagnosticCode::W0018),
        "W0019" => Ok(DiagnosticCode::W0019),
        "W0020" => Ok(DiagnosticCode::W0020),
        "W0021" => Ok(DiagnosticCode::W0021),
        "W0022" => Ok(DiagnosticCode::W0022),
        "W0023" => Ok(DiagnosticCode::W0023),
        "W0024" => Ok(DiagnosticCode::W0024),
        "W0025" => Ok(DiagnosticCode::W0025),
        "W0026" => Ok(DiagnosticCode::W0026),
        "W0027" => Ok(DiagnosticCode::W0027),
        "W0028" => Ok(DiagnosticCode::W0028),
        "W0029" => Ok(DiagnosticCode::W0029),
        "W0030" => Ok(DiagnosticCode::W0030),
        "W0031" => Ok(DiagnosticCode::W0031),
        "W0032" => Ok(DiagnosticCode::W0032),
        "W0033" => Ok(DiagnosticCode::W0033),
        "W0034" => Ok(DiagnosticCode::W0034),
        "W0035" => Ok(DiagnosticCode::W0035),
        "W0036" => Ok(DiagnosticCode::W0036),
        "W0037" => Ok(DiagnosticCode::W0037),
        "W0038" => Ok(DiagnosticCode::W0038),
        "W0039" => Ok(DiagnosticCode::W0039),
        "W0040" => Ok(DiagnosticCode::W0040),
        _ => Err(runtime_msg("Warning code must look like W0001")),
    }
}

fn runtime_msg(message: &str) -> MireError {
    MireError::runtime(message.to_string())
}

fn runtime_err(err: std::io::Error) -> MireError {
    MireError::runtime(err.to_string())
}

fn print_help() {
    println!("Mire / Avenys v{}", env!("CARGO_PKG_VERSION"));
    println!("Usage: mire <run|build|check|debug> [file] [options]\n");
    println!("Profiles:");
    println!("  --debug               Build profile debug (default)");
    println!("  --release             Build profile release");
    println!("  -O, --opt-level <n>   0|1|2|3|s|z");
    println!("  --import-mode <m>     legacy|reachable (default: legacy)");
    println!("\nCommands:");
    println!("  run [file] [-- args]  Compile + execute");
    println!("  build [file]          Compile only");
    println!("  check [file]          Analyze only");
    println!("  debug [file]          Debug build, emits IR");
    println!("  import <module>       Add import to owl.toml");
    println!("    --json              Emit machine-readable import result");
    println!("  test [paths...]       Run integration tests from tests/");
    println!("    --no-run            Compile only, skip execution");
    println!("    --verbose, -v       Show per-test results");
}
