use super::*;

pub(super) fn optimize_ir(ir: &str, opt_level: OptLevel) -> Result<String> {
    let mut command = Command::new("opt");
    command
        .arg("-S")
        .arg(opt_level.as_opt_flag())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped());
    let mut child = command.spawn().map_err(|err| {
        MireError::new(ErrorKind::Runtime {
            message: format!("Failed to run opt: {}", err),
        })
    })?;
    if let Some(stdin) = child.stdin.as_mut() {
        stdin.write_all(ir.as_bytes()).map_err(|err| {
            MireError::new(ErrorKind::Runtime {
                message: format!("Failed to stream IR into opt: {}", err),
            })
        })?;
    }
    let output = child.wait_with_output().map_err(|err| {
        MireError::new(ErrorKind::Runtime {
            message: format!("Failed to wait for opt: {}", err),
        })
    })?;
    if !output.status.success() {
        return Err(MireError::new(ErrorKind::Runtime {
            message: format!(
                "opt failed with status {}.\nstderr:\n{}",
                output.status,
                String::from_utf8_lossy(&output.stderr).trim()
            ),
        }));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

pub(super) fn compile_binary_from_ir(
    ir: &str,
    c_sources: &[String],
    binary_path: &Path,
    opt_level: OptLevel,
    extern_libs: &[(String, String)],
    manifest_dir: &Path,
) -> Result<()> {
    let mut clang = Command::new("clang");
    clang
        .arg("-x")
        .arg("ir")
        .arg("-")
        .arg("-x")
        .arg("c")
        .arg("-I")
        .arg(manifest_dir.join("src/runtime"))
        .arg("-I")
        .arg(manifest_dir.join("src/pal"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for src in c_sources {
        clang.arg(src);
    }
    clang.arg("-o").arg(binary_path);
    clang.arg(opt_level.as_opt_flag());
    clang.arg("-lm");
    for (lib_name, lib_path) in extern_libs {
        if lib_path.ends_with(".so") || lib_path.ends_with(".a") || lib_path.ends_with(".dylib") {
            clang.arg(lib_path);
        } else if !lib_path.is_empty() {
            clang.arg(format!("-l:{}", lib_path));
        }
        clang.arg("-l");
        clang.arg(lib_name);
    }

    let mut child = clang.spawn().map_err(|err| {
        MireError::new(ErrorKind::Runtime {
            message: format!("Failed to run clang: {}", err),
        })
    })?;
    if let Some(stdin) = child.stdin.as_mut() {
        stdin.write_all(ir.as_bytes()).map_err(|err| {
            MireError::new(ErrorKind::Runtime {
                message: format!("Failed to stream IR into clang: {}", err),
            })
        })?;
    }
    let output = child.wait_with_output().map_err(|err| {
        MireError::new(ErrorKind::Runtime {
            message: format!("Failed to wait for clang: {}", err),
        })
    })?;
    if output.status.success() {
        return Ok(());
    }

    Err(MireError::new(ErrorKind::Runtime {
        message: format!(
            "clang failed with status {}.\nstdout:\n{}\nstderr:\n{}",
            output.status,
            String::from_utf8_lossy(&output.stdout).trim(),
            String::from_utf8_lossy(&output.stderr).trim()
        ),
    }))
}

pub(super) fn llvm_version() -> Result<String> {
    let output = Command::new("llvm-config")
        .arg("--version")
        .output()
        .map_err(|err| {
            MireError::new(ErrorKind::Runtime {
                message: format!("Failed to run llvm-config: {}", err),
            })
        })?;
    if !output.status.success() {
        return Err(MireError::new(ErrorKind::Runtime {
            message: "llvm-config --version failed".to_string(),
        }));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}
