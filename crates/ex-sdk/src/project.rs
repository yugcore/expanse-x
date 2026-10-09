use crate::{argument_error, Result};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

struct CreatedProject {
    directory: PathBuf,
    files: Vec<PathBuf>,
    directories: Vec<PathBuf>,
    complete: bool,
}
impl Drop for CreatedProject {
    fn drop(&mut self) {
        if !self.complete {
            for file in self.files.iter().rev() {
                let _ = fs::remove_file(file);
            }
            for directory in self.directories.iter().rev() {
                let _ = fs::remove_dir(directory);
            }
            let _ = fs::remove_dir(&self.directory);
        }
    }
}
fn quoted_toml(value: &str) -> String {
    let mut out = String::from("\"");
    for c in value.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if c.is_control() => out.push_str(&format!("\\u{:04X}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
pub(super) fn create(destination: &Path, name: &str, sdk: &Path) -> Result<()> {
    let keywords = [
        "self", "Self", "super", "crate", "fn", "mod", "use", "pub", "type", "struct", "enum",
        "trait", "impl", "const", "static", "let", "mut", "ref", "match", "if", "else", "loop",
        "while", "for", "in", "return", "break", "continue", "async", "await", "move", "unsafe",
        "extern", "where", "as", "true", "false", "dyn",
    ];
    if name.is_empty()
        || name.len() > 64
        || !name.starts_with(|c: char| c.is_ascii_lowercase())
        || !name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '-' | '_'))
        || keywords.contains(&name)
    {
        return Err(argument_error("package name must start with a lowercase ASCII letter and contain up to 64 lowercase letters, digits, hyphens or underscores; Rust keywords are reserved"));
    }
    let dependency = sdk.join("crates/ex-api");
    let dependency = dependency.to_str().ok_or_else(|| {
        argument_error("SDK path cannot be represented in a UTF-8 Cargo manifest")
    })?;
    let shader_dependency = sdk.join("crates/ex-shader");
    let shader_dependency = shader_dependency.to_str().ok_or_else(|| {
        argument_error("shader SDK path cannot be represented in a UTF-8 Cargo manifest")
    })?;
    let manifest = format!("[package]\nname = {}\nversion = \"0.1.0\"\nedition = \"2021\"\npublish = false\n\n[dependencies]\nex = {{ package = \"ex-api\", path = {} }}\n\n[build-dependencies]\nex-shader = {{ path = {} }}\nex = {{ package = \"ex-api\", path = {} }}\n\n# Standalone, including when created inside the SDK checkout.\n[workspace]\n\n[lints.rust]\nunsafe_code = \"deny\"\n", quoted_toml(name), quoted_toml(dependency), quoted_toml(shader_dependency), quoted_toml(dependency));
    // Exclusive creation refuses files, symlinks, and even empty directories.
    fs::create_dir(destination).map_err(|error| {
        argument_error(format!(
            "cannot create {}: {error}; choose a new destination with an existing parent",
            destination.display()
        ))
    })?;
    let mut created = CreatedProject {
        directory: destination.to_path_buf(),
        files: Vec::new(),
        directories: Vec::new(),
        complete: false,
    };
    let source = destination.join("src");
    fs::create_dir(&source)?;
    created.directories.push(source);
    let shaders = destination.join("shaders");
    fs::create_dir(&shaders)?;
    created.directories.push(shaders);
    for (relative, contents) in [
        ("Cargo.toml", manifest.as_str()),
        ("README.md", include_str!("../templates/README.md")),
        (".gitignore", "target/\n"),
        (
            "rust-toolchain.toml",
            include_str!("../../../rust-toolchain.toml"),
        ),
        ("rustfmt.toml", include_str!("../../../rustfmt.toml")),
        ("src/main.rs", include_str!("../templates/main.rs")),
        ("src/shaders.rs", include_str!("../templates/shaders.rs")),
        ("build.rs", include_str!("../templates/build.rs")),
        (
            "shaders/triangle.vert",
            include_str!("../templates/triangle.vert"),
        ),
        (
            "shaders/triangle.frag",
            include_str!("../templates/triangle.frag"),
        ),
        ("src/renderer.rs", include_str!("../templates/renderer.rs")),
        (
            "src/png.rs",
            include_str!("../../ex-api/examples/triangle/png.rs"),
        ),
    ] {
        let path = destination.join(relative);
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)?;
        created.files.push(path);
        file.write_all(contents.as_bytes())?;
    }
    // Cargo resolves only metadata here; it neither builds nor runs the app.
    created.files.push(destination.join("Cargo.lock"));
    let out = Command::new("cargo")
        .env("RUSTUP_AUTO_INSTALL", "0")
        .args(["generate-lockfile", "--offline", "--manifest-path"])
        .arg(destination.join("Cargo.toml"))
        .output()?;
    if !out.status.success() {
        return Err(argument_error(format!(
            "offline starter lockfile generation failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        )));
    }
    created.complete = true;
    Ok(())
}
