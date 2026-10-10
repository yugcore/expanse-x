mod project;
mod shader;
use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
const HELP: &str = "EX source SDK - Core Preview 1 (software)
Usage:
  ex info
  ex doctor [--sdk PATH]
  ex new PATH [--name NAME] [--sdk PATH]
  ex shader build FILE --stage vertex|fragment --output FILE.rs
  ex shader import FILE.spv --output FILE.rs
  ex shader reflect FILE --stage vertex|fragment
  ex help

Development: cargo run -p ex-sdk --locked --offline -- COMMAND
new creates an independent headless Rust project; default package name: ex-app.
doctor checks tools/build without installing anything. info queries the linked runtime.
Use --sdk to select a source checkout when running outside one.
Shader tools: glslangValidator and spirv-val (PATH, EX_GLSLANG, EX_SPIRV_VAL).
Shader import supports EX-GLSL-1; windowing, reload and capture are planned.";

fn main() -> ExitCode {
    match run(std::env::args_os().skip(1).collect()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("EX SDK: {error}");
            ExitCode::FAILURE
        }
    }
}
fn argument_error(message: impl Into<String>) -> Box<dyn std::error::Error> {
    io::Error::new(io::ErrorKind::InvalidInput, message.into()).into()
}
fn run(args: Vec<OsString>) -> Result<()> {
    let Some(command) = args.first().and_then(|a| a.to_str()) else {
        if args.is_empty() {
            println!("{HELP}");
            return Ok(());
        }
        return Err(argument_error("command must be Unicode; use ex help"));
    };
    if matches!(command, "help" | "--help" | "-h") && args.len() == 1 {
        println!("{HELP}");
        return Ok(());
    }
    if command == "--version" && args.len() == 1 {
        println!("ex-sdk {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    if command == "info" && args.len() == 1 {
        info();
        return Ok(());
    }
    if command == "shader" {
        return shader::run(&args[1..]);
    }
    if !matches!(command, "new" | "doctor") {
        return Err(argument_error(
            "unknown command or unexpected argument; use ex help",
        ));
    }
    let mut sdk = None;
    let mut name = None;
    let mut destination = None;
    let mut rest = args[1..].iter();
    while let Some(arg) = rest.next() {
        match arg.to_str() {
            Some("--sdk") if sdk.is_none() => {
                sdk = Some(PathBuf::from(
                    rest.next()
                        .ok_or_else(|| argument_error("--sdk requires a path"))?,
                ));
            }
            Some("--name") if command == "new" && name.is_none() => {
                name = Some(
                    rest.next()
                        .ok_or_else(|| argument_error("--name requires a package name"))?
                        .to_str()
                        .ok_or_else(|| argument_error("package name must be Unicode"))?
                        .to_owned(),
                );
            }
            Some(value) if value.starts_with('-') => {
                return Err(argument_error(format!(
                    "unknown or repeated option {value:?}; use ex help"
                )))
            }
            _ if command == "new" && destination.is_none() => {
                destination = Some(PathBuf::from(arg))
            }
            _ => return Err(argument_error("unexpected argument; use ex help")),
        }
    }
    if command == "new" && destination.is_none() {
        return Err(argument_error(
            "new requires a destination path; use ex help",
        ));
    }
    let root = sdk_root(sdk)?;
    match command {
        "doctor" => doctor(&root),
        "new" => {
            let path = destination.ok_or_else(|| argument_error("new requires a destination"))?;
            project::create(&path, name.as_deref().unwrap_or("ex-app"), &root)?;
            println!("Created EX starter: {}", path.display());
            println!("Build and run from that directory: cargo run --locked --offline");
            println!(
                "Edit shaders/triangle.vert or triangle.frag; cargo compiles them into EX shaders."
            );
            println!("The app writes target/ex-starter.png; native shader build tools must be installed.");
            Ok(())
        }
        _ => Err(argument_error("unknown command")),
    }
}
fn is_sdk(root: &Path) -> bool {
    [
        "Cargo.toml",
        "crates/ex-api/Cargo.toml",
        "crates/expansex-core/Cargo.toml",
        "docs/CORE-PROFILE.md",
    ]
    .iter()
    .all(|p| root.join(p).is_file())
}
fn sdk_root(explicit: Option<PathBuf>) -> Result<PathBuf> {
    if let Some(root) = explicit {
        let root = root.canonicalize()?;
        return if is_sdk(&root) {
            Ok(root)
        } else {
            Err(argument_error("--sdk must name an EX source checkout"))
        };
    }
    for location in [std::env::current_dir()?, std::env::current_exe()?] {
        for candidate in location.ancestors() {
            if is_sdk(candidate) {
                return Ok(candidate.canonicalize()?);
            }
        }
    }
    Err(argument_error(
        "EX source checkout not found; pass --sdk PATH",
    ))
}
fn info() {
    println!(
        "EX SDK {} - linked runtime capabilities",
        env!("CARGO_PKG_VERSION")
    );
    for adapter in ex::Instance::new().adapters() {
        let profile = adapter.core_profile();
        let limits = profile.limits();
        println!(
            "Adapter: {} ({:?}); profile: {profile:?}",
            adapter.info().name,
            adapter.info().backend
        );
        println!("Execution: synchronous software; one queue, whole-resource dependencies");
        println!(
            "Buffer bytes: {}; image axis: {}",
            limits.max_buffer_size, limits.max_image_extent
        );
        println!(
            "Shader inputs/instructions/vertex outputs: {}/{}/{}",
            limits.max_shader_inputs, limits.max_shader_instructions, limits.max_vertex_outputs
        );
        println!(
            "Binding slots: {}; uniform bytes: {}",
            limits.binding_slots, limits.max_uniform_buffer_size
        );
        println!(
            "Local size/product: {}; groups/axis: {}; total invocations: {}",
            limits.max_workgroup_size, limits.max_workgroup_count, limits.max_compute_invocations
        );
    }
}
fn doctor(root: &Path) -> Result<()> {
    println!("Source SDK: {}", root.display());
    let mut failures = Vec::new();
    for (program, args) in [
        ("rustc", vec!["--version"]),
        ("cargo", vec!["--version"]),
        ("rustfmt", vec!["--version"]),
        ("cargo", vec!["clippy", "--version"]),
    ] {
        match Command::new(program)
            .args(&args)
            .env("RUSTUP_AUTO_INSTALL", "0")
            .current_dir(root)
            .output()
        {
            Ok(out) if out.status.success() => {
                println!("OK: {}", String::from_utf8_lossy(&out.stdout).trim())
            }
            Ok(out) => failures.push(format!(
                "{program} {} failed: {}",
                args.join(" "),
                String::from_utf8_lossy(&out.stderr).trim()
            )),
            Err(error) => {
                failures.push(format!("{program} {} unavailable: {error}", args.join(" ")))
            }
        }
    }
    match ex_shader::check_tools() {
        Ok(versions) => {
            for version in versions {
                println!("OK: {version}");
            }
        }
        Err(error) => failures.push(error.to_string()),
    }
    if !failures.is_empty() {
        return Err(io::Error::other(format!("{}\nInstall the repository's Rust toolchain/components and GLSL/SPIR-V build tools, then rerun doctor. See docs/SDK.md.", failures.join("\n"))).into());
    }
    let status = Command::new("cargo")
        .env("RUSTUP_AUTO_INSTALL", "0")
        .args(["build", "--manifest-path"])
        .arg(root.join("crates/ex-api/Cargo.toml"))
        .args(["--example", "core_preview", "--locked", "--offline"])
        .current_dir(root)
        .status()?;
    if !status.success() {
        return Err(io::Error::other(
            "offline API build failed; resolve Cargo diagnostics above and rerun doctor",
        )
        .into());
    }
    println!("OK: locked offline core example build (including linking); source SDK ready for the headless starter.");
    println!("Stable is a moving channel; the versions above describe this environment.");
    Ok(())
}
