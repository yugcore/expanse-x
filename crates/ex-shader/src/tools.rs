use crate::{fail, path_error, spirv, Result, ShaderModule};
use ex::ShaderStage;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT: AtomicUsize = AtomicUsize::new(0);
const MAX_SOURCE_BYTES: u64 = 4 * 1024 * 1024;
const MAX_SPIRV_BYTES: u64 = 1024 * 1024;

struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Result<Self> {
        for _ in 0..100 {
            let path = std::env::temp_dir().join(format!(
                "ex-shader-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            match fs::create_dir(&path) {
                Ok(()) => return Ok(Self(path)),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(path_error(&path, e)),
            }
        }
        Err(fail(
            "cannot create an exclusive shader temporary directory",
        ))
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        // Only known files created inside this exclusively owned directory.
        let _ = fs::remove_file(self.0.join("shader.spv"));
        let _ = fs::remove_file(self.0.join("source.glsl"));
        let _ = fs::remove_dir(&self.0);
    }
}
fn tool(variable: &str, default: &str) -> OsString {
    std::env::var_os(variable).unwrap_or_else(|| default.into())
}
fn invoke(variable: &str, default: &str, args: &[&std::ffi::OsStr]) -> Result<Output> {
    let executable = tool(variable, default);
    let out = Command::new(&executable).args(args).output().map_err(|e| {
        fail(format!(
            "{} unavailable: {e}; install the shader tools or set {variable} to an executable path",
            executable.to_string_lossy()
        ))
    })?;
    if !out.status.success() {
        return Err(fail(format!(
            "{} failed ({}):\n{}{}",
            executable.to_string_lossy(),
            out.status,
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        )));
    }
    Ok(out)
}
fn read_bounded(path: &Path, maximum: u64) -> Result<Vec<u8>> {
    // Read at most maximum+1 even if the file changes after metadata inspection.
    use std::io::Read;
    let file = fs::File::open(path).map_err(|e| path_error(path, e))?;
    let mut bytes = Vec::new();
    file.take(maximum + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| path_error(path, e))?;
    if bytes.len() as u64 > maximum {
        return Err(path_error(path, format!("input exceeds {maximum} bytes")));
    }
    Ok(bytes)
}
fn validate(path: &Path) -> Result<()> {
    invoke(
        "EX_SPIRV_VAL",
        "spirv-val",
        &[
            "--target-env".as_ref(),
            "vulkan1.0".as_ref(),
            path.as_os_str(),
        ],
    )?;
    Ok(())
}

/// Compile GLSL main, validate its SPIR-V and lower the supported graphics subset.
/// Tools use PATH or EX_GLSLANG/EX_SPIRV_VAL; no shell or installation is invoked.
pub fn build_glsl(source: &Path, stage: ShaderStage) -> Result<ShaderModule> {
    let selector = match stage {
        ShaderStage::Vertex => "vert",
        ShaderStage::Fragment => "frag",
        ShaderStage::Compute => {
            return Err(fail("compute shader import is not supported by EX-GLSL-1"))
        }
    };
    let source_bytes = read_bounded(source, MAX_SOURCE_BYTES)?;
    let scratch = Scratch::new()?;
    let input = scratch.0.join("source.glsl");
    let binary = scratch.0.join("shader.spv");
    fs::write(&input, source_bytes)?;
    invoke(
        "EX_GLSLANG",
        "glslangValidator",
        &[
            "-V".as_ref(),
            "--target-env".as_ref(),
            "vulkan1.0".as_ref(),
            "-Od".as_ref(),
            "-S".as_ref(),
            selector.as_ref(),
            "-o".as_ref(),
            binary.as_os_str(),
            input.as_os_str(),
        ],
    )
    .map_err(|e| path_error(source, e))?;
    // Snapshot the emitted bytes before validating that same snapshot.
    let bytes = read_bounded(&binary, MAX_SPIRV_BYTES)?;
    fs::write(&binary, &bytes)?;
    validate(&binary).map_err(|e| path_error(source, e))?;
    let module =
        spirv::lower(&bytes, &source.to_string_lossy()).map_err(|e| path_error(source, e))?;
    if module.reflection().stage != stage {
        return Err(path_error(
            source,
            "compiler output stage differs from the requested stage",
        ));
    }
    Ok(module)
}

/// Validate a private snapshot of SPIR-V 1.0 and import its supported main entry.
pub fn import_spirv(source: &Path) -> Result<ShaderModule> {
    let bytes = read_bounded(source, MAX_SPIRV_BYTES)?;
    let scratch = Scratch::new()?;
    let binary = scratch.0.join("shader.spv");
    fs::write(&binary, &bytes)?;
    validate(&binary).map_err(|e| path_error(source, e))?;
    spirv::lower(&bytes, &source.to_string_lossy()).map_err(|e| path_error(source, e))
}

/// Report the actual installed compiler and validator versions, or an error.
pub fn check_tools() -> Result<Vec<String>> {
    [
        ("EX_GLSLANG", "glslangValidator"),
        ("EX_SPIRV_VAL", "spirv-val"),
    ]
    .iter()
    .map(|(var, program)| {
        let out = invoke(var, program, &["--version".as_ref()])?;
        Ok(format!(
            "{program}: {}",
            String::from_utf8_lossy(&out.stdout).trim()
        ))
    })
    .collect()
}

fn resolve_tool(variable: &str, default: &str) -> Result<PathBuf> {
    let selected = PathBuf::from(tool(variable, default));
    if selected.components().count() > 1 || selected.is_absolute() {
        return selected
            .canonicalize()
            .map_err(|e| path_error(&selected, e));
    }
    let candidates = std::env::var_os("PATH").unwrap_or_default();
    for directory in std::env::split_paths(&candidates) {
        for path in [
            directory.join(&selected),
            directory.join(format!("{}.exe", selected.to_string_lossy())),
        ] {
            if path.is_file() {
                return path.canonicalize().map_err(|e| path_error(&path, e));
            }
        }
    }
    Err(fail(format!(
        "{default} unavailable; install shader tools or set {variable}"
    )))
}

/// Cargo build.rs helper. Rebuilds track source/directory, tool selection and binaries.
/// Writes only a simple .rs filename inside Cargo's OUT_DIR, after successful import.
pub fn cargo_build(source: &Path, stage: ShaderStage, output_name: &str) -> Result<()> {
    if !output_name.ends_with(".rs")
        || Path::new(output_name).components().count() != 1
        || output_name.contains(['/', '\\', ':'])
    {
        return Err(fail(
            "generated shader output must be a simple .rs filename",
        ));
    }
    println!("cargo:rerun-if-changed={}", source.display());
    if let Some(parent) = source.parent() {
        println!("cargo:rerun-if-changed={}", parent.display());
    }
    for variable in ["EX_GLSLANG", "EX_SPIRV_VAL", "PATH"] {
        println!("cargo:rerun-if-env-changed={variable}");
    }
    for (variable, program) in [
        ("EX_GLSLANG", "glslangValidator"),
        ("EX_SPIRV_VAL", "spirv-val"),
    ] {
        println!(
            "cargo:rerun-if-changed={}",
            resolve_tool(variable, program)?.display()
        );
    }
    let directory = std::env::var_os("OUT_DIR").ok_or_else(|| {
        fail("cargo_build must run inside a Cargo build script (OUT_DIR missing)")
    })?;
    let module = build_glsl(source, stage)?;
    fs::write(
        PathBuf::from(directory).join(output_name),
        module.rust_source(),
    )?;
    Ok(())
}
