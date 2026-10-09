use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT: AtomicUsize = AtomicUsize::new(0);
struct Fixture {
    root: PathBuf,
    base: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let base = sdk().join("target/sdk-tests");
        fs::create_dir_all(&base).unwrap();
        let root = base.join(format!(
            "{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        Self { root, base }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        // Remove only this exclusively created fixture under the verified target root.
        if let (Ok(root), Ok(base)) = (self.root.canonicalize(), self.base.canonicalize()) {
            if root.parent() == Some(base.as_path()) {
                let _ = fs::remove_dir_all(root);
            }
        }
    }
}
fn sdk() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}
fn cli(args: &[&std::ffi::OsStr]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ex"))
        .args(args)
        .current_dir(sdk())
        .output()
        .unwrap()
}
fn passed(out: &Output) {
    assert!(
        out.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}
#[test]
fn help_info_and_argument_errors() {
    let help = cli(&["--help".as_ref()]);
    passed(&help);
    assert!(String::from_utf8_lossy(&help.stdout).contains("ex doctor"));
    let info = cli(&["info".as_ref()]);
    passed(&info);
    let info = String::from_utf8_lossy(&info.stdout);
    assert!(
        info.contains("Preview1")
            && info.contains("synchronous software")
            && info.contains("Binding slots: 8")
    );
    for args in [
        vec!["unknown"],
        vec!["info", "extra"],
        vec!["new"],
        vec!["doctor", "--sdk"],
        vec!["new", "somewhere", "--name"],
    ] {
        let args: Vec<_> = args.iter().map(std::ffi::OsStr::new).collect();
        let out = cli(&args);
        assert!(!out.status.success());
        assert!(String::from_utf8_lossy(&out.stderr).starts_with("EX SDK:"));
    }
}
#[test]
fn refuses_existing_files_directories_and_invalid_inputs() {
    let f = Fixture::new();
    let occupied = f.root.join("occupied");
    fs::create_dir(&occupied).unwrap();
    fs::write(occupied.join("keep.txt"), b"keep").unwrap();
    let out = cli(&["new".as_ref(), occupied.as_os_str()]);
    assert!(!out.status.success());
    assert_eq!(fs::read(occupied.join("keep.txt")).unwrap(), b"keep");
    let empty = f.root.join("empty");
    fs::create_dir(&empty).unwrap();
    assert!(!cli(&["new".as_ref(), empty.as_os_str()]).status.success());
    assert_eq!(fs::read_dir(&empty).unwrap().count(), 0);
    let file = f.root.join("file");
    fs::write(&file, b"unchanged").unwrap();
    assert!(!cli(&["new".as_ref(), file.as_os_str()]).status.success());
    assert_eq!(fs::read(&file).unwrap(), b"unchanged");
    let invalid = f.root.join("invalid");
    assert!(!cli(&[
        "new".as_ref(),
        invalid.as_os_str(),
        "--name".as_ref(),
        "Bad Name".as_ref()
    ])
    .status
    .success());
    assert!(!invalid.exists());
    assert!(!cli(&[
        "new".as_ref(),
        invalid.as_os_str(),
        "--sdk".as_ref(),
        f.root.as_os_str()
    ])
    .status
    .success());
    assert!(!invalid.exists());
}
#[test]
fn failed_lockfile_generation_removes_only_the_new_project() {
    let f = Fixture::new();
    let broken_sdk = f.root.join("broken sdk");
    for path in ["crates/ex-api", "crates/expansex-core", "docs"] {
        fs::create_dir_all(broken_sdk.join(path)).unwrap();
    }
    for file in [
        "Cargo.toml",
        "crates/ex-api/Cargo.toml",
        "crates/expansex-core/Cargo.toml",
    ] {
        fs::write(broken_sdk.join(file), "invalid TOML {").unwrap();
    }
    fs::write(broken_sdk.join("docs/CORE-PROFILE.md"), "fixture").unwrap();
    let output = f.root.join("new project");
    let result = cli(&[
        "new".as_ref(),
        output.as_os_str(),
        "--sdk".as_ref(),
        broken_sdk.as_os_str(),
    ]);
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("lockfile generation failed"));
    assert!(!output.exists());
    assert_eq!(
        fs::read_to_string(broken_sdk.join("Cargo.toml")).unwrap(),
        "invalid TOML {"
    );
}
#[test]
fn standalone_starter_builds_runs_and_formats_offline_in_a_path_with_spaces() {
    let f = Fixture::new();
    let destination = f.root.join("engine with spaces");
    passed(&cli(&[
        "new".as_ref(),
        destination.as_os_str(),
        "--name".as_ref(),
        "sample-engine".as_ref(),
        "--sdk".as_ref(),
        sdk().as_os_str(),
    ]));
    assert!(destination.join("Cargo.lock").is_file());
    for args in [
        vec!["fmt", "--", "--check"],
        vec!["build", "--locked", "--offline"],
        vec!["run", "--locked", "--offline"],
    ] {
        let out = Command::new("cargo")
            .args(args)
            .current_dir(&destination)
            .env("CARGO_TARGET_DIR", f.root.join("isolated build"))
            .output()
            .unwrap();
        passed(&out);
    }
    let png = fs::read(destination.join("target/ex-starter.png")).unwrap();
    assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
    assert_eq!(u32::from_be_bytes(png[16..20].try_into().unwrap()), 64);
    assert_eq!(u32::from_be_bytes(png[20..24].try_into().unwrap()), 64);
    // The manifest stores a quoted path, and this project has its own workspace.
    let manifest = fs::read_to_string(destination.join("Cargo.toml")).unwrap();
    assert!(manifest.contains("[workspace]") && manifest.contains("sample-engine"));
    let shader = destination.join("src/shaders.rs");
    let original = fs::read_to_string(&shader).unwrap();
    let changed = original.replace("[0.25, 0.75, 1.0, 1.0]", "[1.0, 0.5, 0.25, 1.0]");
    assert_ne!(original, changed);
    fs::write(shader, changed).unwrap();
    passed(
        &Command::new("cargo")
            .args(["run", "--locked", "--offline"])
            .current_dir(&destination)
            .env("CARGO_TARGET_DIR", f.root.join("isolated build"))
            .output()
            .unwrap(),
    );
    assert_ne!(
        fs::read(destination.join("target/ex-starter.png")).unwrap(),
        png
    );
}

#[test]
fn doctor_reports_missing_tools_without_installing_them() {
    let f = Fixture::new();
    let result = Command::new(env!("CARGO_BIN_EXE_ex"))
        .args(["doctor", "--sdk"])
        .arg(sdk())
        .current_dir(&f.root)
        .env("PATH", "")
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("unavailable"));
    assert_eq!(fs::read_dir(&f.root).unwrap().count(), 0);
}
