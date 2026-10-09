use crate::{argument_error, Result};
use ex::ShaderStage;
use std::ffi::OsString;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

pub(super) fn run(args: &[OsString]) -> Result<()> {
    let command = args
        .first()
        .and_then(|a| a.to_str())
        .ok_or_else(|| argument_error("shader requires build, import or reflect; use ex help"))?;
    if !matches!(command, "build" | "import" | "reflect") {
        return Err(argument_error("unknown shader command; use ex help"));
    }
    let mut source = None;
    let mut output = None;
    let mut stage = None;
    let mut rest = args[1..].iter();
    while let Some(arg) = rest.next() {
        match arg.to_str() {
            Some("--output") if command != "reflect" && output.is_none() => {
                output = Some(PathBuf::from(
                    rest.next()
                        .ok_or_else(|| argument_error("--output requires a path"))?,
                ));
            }
            Some("--stage") if command != "import" && stage.is_none() => {
                stage = Some(match rest.next().and_then(|v| v.to_str()) {
                    Some("vertex") => ShaderStage::Vertex,
                    Some("fragment") => ShaderStage::Fragment,
                    _ => return Err(argument_error("--stage requires vertex or fragment")),
                });
            }
            Some(value) if value.starts_with('-') => {
                return Err(argument_error(format!(
                    "unknown or repeated shader option {value}"
                )))
            }
            _ if source.is_none() => source = Some(PathBuf::from(arg)),
            _ => return Err(argument_error("unexpected shader argument")),
        }
    }
    let source = source.ok_or_else(|| argument_error("shader command requires an input file"))?;
    if command != "reflect" && output.is_none() {
        return Err(argument_error("build/import requires --output FILE.rs"));
    }
    if command != "import" && stage.is_none() {
        return Err(argument_error(
            "build/reflect requires --stage vertex|fragment",
        ));
    }
    if let Some(output) = &output {
        if output.try_exists()? {
            return Err(argument_error(
                "shader output already exists; choose a new path",
            ));
        }
    }
    let module = if command == "import" {
        ex_shader::import_spirv(&source)?
    } else {
        ex_shader::build_glsl(
            &source,
            stage.ok_or_else(|| argument_error("missing shader stage"))?,
        )?
    };
    if let Some(output) = output {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&output)?;
        if let Err(error) = file.write_all(module.rust_source().as_bytes()) {
            drop(file);
            let _ = std::fs::remove_file(&output);
            return Err(error.into());
        }
        println!("Built EX shader: {}", output.display());
    }
    println!("{}", module.reflection().to_json());
    Ok(())
}
