"""Package the committed EX SDK Preview for Windows; Python 3.11+, standard library only."""

from __future__ import annotations

import argparse
from datetime import datetime, timezone
import hashlib
import io
import json
import os
from pathlib import Path
import subprocess
import sys
import tomllib
import zipfile


def command(root: Path, *args: str, env: dict[str, str] | None = None) -> bytes:
    result = subprocess.run(args, cwd=root, env=env, capture_output=True, check=False)
    if result.returncode:
        raise RuntimeError(
            f"{' '.join(args)} failed ({result.returncode}):\n"
            + result.stdout.decode(errors="replace")
            + result.stderr.decode(errors="replace")
        )
    return result.stdout


def package(output: Path, version: str) -> Path:
    root = Path(__file__).resolve().parent.parent
    target = "x86_64-pc-windows-msvc"
    if sys.platform != "win32":
        raise RuntimeError("this first SDK binary package is tested only on Windows")
    with (root / "crates/ex-sdk/Cargo.toml").open("rb") as file:
        actual_version = tomllib.load(file)["package"]["version"]
    if version != actual_version:
        raise RuntimeError(f"requested version {version!r} differs from SDK {actual_version!r}")
    if command(root, "git", "status", "--porcelain"):
        raise RuntimeError("commit changes before packaging; the source snapshot must be clean")
    commit = command(root, "git", "rev-parse", "HEAD").decode().strip()
    rustc = command(root, "rustc", "-vV").decode().strip()
    if f"host: {target}" not in rustc:
        raise RuntimeError(f"packaging requires the tested native {target} Rust toolchain")
    cargo = command(root, "cargo", "--version").decode().strip()
    native = {}
    for variable, default in [("EX_GLSLANG", "glslangValidator"), ("EX_SPIRV_VAL", "spirv-val")]:
        native[default] = command(root, os.environ.get(variable, default), "--version").decode().strip()
    output = output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    archive = output / f"expanse-x-sdk-v{version}-windows-x86_64.zip"
    checksums = output / "SHA256SUMS.txt"
    if archive.exists() or checksums.exists():
        raise RuntimeError("release output already exists; choose a new output directory")
    env = os.environ.copy()
    env["RUSTUP_AUTO_INSTALL"] = "0"
    env["CARGO_TARGET_DIR"] = str(root / "target/sdk-release-build")
    command(root, "cargo", "build", "-p", "ex-sdk", "--release", "--target", target, "--locked", "--offline", env=env)
    binary_path = Path(env["CARGO_TARGET_DIR"]) / target / "release/ex.exe"
    binary = binary_path.read_bytes()
    reported_version = command(root, str(binary_path), "--version").decode().strip()
    if reported_version != f"ex-sdk {version}":
        raise RuntimeError(f"built binary reports unexpected version: {reported_version}")
    if command(root, "git", "rev-parse", "HEAD").decode().strip() != commit or command(root, "git", "status", "--porcelain"):
        raise RuntimeError("source changed during packaging; retry from a clean committed snapshot")
    prefix = f"expanse-x-sdk-v{version}/"
    source = command(root, "git", "archive", "--format=zip", f"--prefix={prefix}", commit)
    provenance = {
        "format": "EX-SDK-ARCHIVE-1",
        "version": version,
        "preview": True,
        "commit": commit,
        "target": target,
        "built_at_utc": datetime.now(timezone.utc).isoformat(),
        "rustc": rustc,
        "cargo": cargo,
        "shader_tools": native,
        "binary": "bin/ex.exe",
        "binary_sha256": hashlib.sha256(binary).hexdigest(),
        "project_license_selected": True,
        "license": "MIT",
    }
    guide = f"""# Expanse-X SDK v{version} (Preview)

This package contains bin/ex.exe and its source at commit {commit}.
Extract it into a directory you can keep in place, then run bin/ex.exe directly.

Install stable Rust with rustfmt/Clippy and Windows MSVC build/link prerequisites.
Install glslangValidator and spirv-val separately, or select executable paths with
EX_GLSLANG and EX_SPIRV_VAL. No native shader tools or Rust toolchain are bundled.

From this directory in PowerShell:

```powershell
.\\bin\\ex.exe --version
.\\bin\\ex.exe info
.\\bin\\ex.exe doctor
.\\bin\\ex.exe new my-ex-app --name my-ex-app
Set-Location my-ex-app
cargo run --locked --offline
```

Open my-ex-app/target/ex-starter.png. Edit GLSL under my-ex-app/shaders/ and rebuild.
Generated manifests reference this extracted SDK by absolute path; keep it in place
or update both runtime/build dependencies if it moves. All Cargo dependencies are
local; the installed Rust/native tools are required before offline builds.

docs/SDK.md and docs/SHADER-IMPORT.md define current commands and the bounded
EX-GLSL-1 shader subset. docs/releases/v0.1.0.md describes this release. Rendering
is CPU software, without hardware drivers or window presentation. The API remains
provisional. EX uses the MIT license in LICENSE; Cargo registry publishing is disabled.
BUILD-INFO.json records the tested build environment and binary checksum.
"""
    created: list[Path] = []
    try:
        with archive.open("xb") as file:
            created.append(archive)
            with zipfile.ZipFile(file, "w", compression=zipfile.ZIP_DEFLATED) as out:
                with zipfile.ZipFile(io.BytesIO(source)) as source_zip:
                    for entry in source_zip.infolist():
                        relative = entry.filename.removeprefix(prefix)
                        if entry.filename == prefix:
                            continue
                        parts = Path(relative).parts
                        if any(part in {".git", "userRef", "createrChoir", "target"} for part in parts) or Path(relative).name.lower() == "agents.md":
                            raise RuntimeError(f"unexpected excluded path in tracked source: {relative}")
                        out.writestr(entry, source_zip.read(entry))
                out.writestr(prefix + "bin/ex.exe", binary)
                out.writestr(prefix + "SDK-README.md", guide)
                out.writestr(prefix + "BUILD-INFO.json", json.dumps(provenance, indent=2) + "\n")
        with checksums.open("x", encoding="utf-8", newline="\n") as file:
            created.append(checksums)
            file.write(f"{hashlib.sha256(archive.read_bytes()).hexdigest()}  {archive.name}\n")
    except Exception:
        for path in reversed(created):
            path.unlink(missing_ok=True)
        raise
    print(f"Packaged {archive} ({archive.stat().st_size} bytes)")
    print(f"Provenance commit: {commit}; checksums: {checksums}")
    return archive


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--version", default="0.1.0")
    args = parser.parse_args()
    try:
        package(args.output, args.version)
    except (OSError, RuntimeError, ValueError) as error:
        print(f"SDK packaging failed: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
