# SDK Preview release distribution

The v0.1.0 SDK is distributed through GitHub Releases as a Windows x64 preview.
The package includes an executable, source dependencies, setup instructions,
build provenance and a checksum. The SDK and public API are still evolving.

## Distribution boundary

GitHub Releases associates notes/downloads with a Git tag and supplies source
archives ([documentation](https://docs.github.com/en/repositories/releasing-projects-on-github/about-releases)).
GitHub Packages' supported registries do not include Cargo
([supported formats](https://docs.github.com/en/packages/learn-github-packages/introduction-to-github-packages#support-for-package-registries)).
The workspace keeps `publish = false`; this workflow does not publish to crates.io.
EX uses the [MIT license](../LICENSE); include it in SDK archives.

Distribute a Windows x86_64 MSVC archive containing the release-built ex.exe,
the exact tracked source snapshot and documentation needed by the source SDK,
an install/readiness guide and machine-readable build provenance. Include a
SHA-256 checksum asset for the ZIP. Do not bundle Rust, Visual Studio components,
glslangValidator or spirv-val; document their existing build-time requirements.
Rendering remains EX's CPU software execution. Native shader tools run during
shader import and generated application builds, not inside the renderer.

Use a single root directory expanse-x-sdk-v0.1.0/ with bin/ex.exe. Keeping the
source beside the executable lets normal ancestor discovery find the SDK after
extraction, without a developer-specific path baked into the binary. Generated
apps use their existing absolute source dependencies; keep the extracted SDK
directory in place or update those manifest paths after moving it.

## Packaging and evidence

scripts/package_sdk.py uses Python 3.11+ and standard libraries. It must refuse
an existing output artifact, require a clean main/source snapshot, read the SDK
version from Cargo.toml, build locked/offline for x86_64-pc-windows-msvc, and use
git archive HEAD for source contents. It must include no ignored guides/research,
build directories, Git metadata or credentials. The executable is the only
generated payload, apart from package guide/provenance/checksum. Version and
target mismatches fail before distribution. The archive is explicitly Windows;
source ZIP/tarballs alone do not certify other platforms.

Record the exact commit, actual Rust/Cargo/native shader tool versions, target,
executable SHA-256, actual UTC build time and package format. Failures remove
only output files created by the attempt; never overwrite another artifact.
Native tool executables are selected through existing PATH/EX_GLSLANG/
EX_SPIRV_VAL conventions. Full check results belong in the release notes.

Before tagging or publishing, run normal project checks, package the committed
snapshot, independently inspect ZIP contents and checksums, and extract into a
new path containing spaces. Exercise packaged version/info/doctor/new, then
format/build/run the generated application locked/offline with separate build
output. Verify the generated PNG against the established geometry/pixel oracle.
Prove the source checkout works independently of the developer's workspace.

Create an annotated v0.1.0 tag at that verified commit. Publish the release with
the ZIP and checksum, using a draft while uploading and verifying assets; publish
the completed draft only after evidence is ready. Existing tags/releases must
be inspected rather than silently replaced. No history rewrite is needed.

## Consumer quickstart

Extract the ZIP into a permanent directory. From its root in PowerShell:

```powershell
.\bin\ex.exe --version
.\bin\ex.exe info
.\bin\ex.exe doctor
.\bin\ex.exe new my-ex-app --name my-ex-app
Set-Location my-ex-app
cargo run --locked --offline
```

Install stable Rust with rustfmt/Clippy and the Windows MSVC linking prerequisites
first. Install glslangValidator and spirv-val separately or set EX_GLSLANG/
EX_SPIRV_VAL to executable paths. doctor diagnoses availability/build failures
without installing software. Open my-ex-app/target/ex-starter.png and edit the
GLSL files under my-ex-app/shaders/. The source SDK's [SDK.md](SDK.md) and
[SHADER-IMPORT.md](SHADER-IMPORT.md) define the bounded supported operations.

The channel stable is moving; provenance records the tested compiler/tool
versions without promising every later version. EX is licensed under MIT.
Safe reload, window/engine integration, broader source import, independent asset
cache and other platform packaging remain later SDK gates.
