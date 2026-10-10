# SDK Preview development

Core Preview 1 is complete. SDK Preview remains in progress: the source workflow
and the first bounded shader import/build/reflection workflow are implemented.
The SDK uses the existing explicit software profile.

## Source SDK commands

The standard-library-only ex-sdk package builds the ex executable, separate from
core execution. Expected misuse reports a diagnostic and a failing exit status.

| Command | Implemented behavior |
| --- | --- |
| ex info | Query the linked runtime's adapters, profile and ten numeric limits; identify software execution |
| ex doctor [--sdk PATH] | Check the source checkout, Rust/Cargo/rustfmt/Clippy and native shader tool versions; build the core_preview executable locked/offline, including linking |
| ex new PATH [--name NAME] [--sdk PATH] | Generate a standalone Rust starter with local runtime/build dependencies, GLSL files, build.rs, checked PNG rendering and its own lockfile |
| ex shader build FILE --stage vertex\|fragment --output FILE.rs | Compile GLSL, validate SPIR-V, lower supported EX instructions and exclusively create a Rust module; print JSON reflection |
| ex shader import FILE.spv --output FILE.rs | Validate a private binary snapshot, lower its supported main entry and exclusively create a Rust module; print JSON reflection |
| ex shader reflect FILE --stage vertex\|fragment | Compile/lower GLSL and print JSON reflection without writing an artifact |
| ex help / --help, --version | Show implemented commands or the package version |

Find the checkout in ancestors of the current directory/executable, or use --sdk.
The executable has no compiled-in machine path. --sdk selects the starter/build
checkout; info reports the runtime linked into the tool. Shader commands do not
need checkout discovery. Paths are subprocess arguments, never shell fragments.

Creation refuses every existing destination, even an empty directory. Validate
arguments/checkout before exclusive creation; the parent must already exist.
On failure, remove only this attempt's files/directories. Cleanup is best effort
if the OS denies removal; interruption can leave a partial new project. No Git
initialization, installation, publication or global configuration change occurs.
Cargo generates the starter lockfile offline from metadata, without building or
running the application. A malformed selected checkout fails with diagnostics.
Non-Unicode manifest paths are rejected; TOML quoting preserves spaces/backslashes.

## Shader build boundary

[SHADER-IMPORT.md](SHADER-IMPORT.md) records the contract chosen before implementation.
The installed glslangValidator compiles GLSL to SPIR-V 1.0; spirv-val checks the
same private snapshot that EX imports. EX-GLSL-1 supports vertex/fragment main,
straight-line float/vec4 arithmetic, constants, local values, swizzles, dot,
component extraction/construction, consecutive vec4 locations and uniform vec4
blocks in descriptor set zero. Reflection describes declared interfaces, uniform
slots/minimum bytes and actual EX instruction count. EX owns shader execution.

This is limited language import: other vector sizes, textures, matrices, compute,
branches, loops, calls, includes and extensions are explicitly rejected. Existing
Rust-built EX IR remains available, including its broader core operations.
Compiler errors preserve native diagnostics; lowering errors identify opcode and
SPIR-V word offset. There is no source-level debugger or complete interpreter.

Native tools must be installed separately and available through PATH, or selected
with EX_GLSLANG and EX_SPIRV_VAL executable paths. They run only during import/
build; rendered applications do not need them at runtime. EX does not install
anything. [glslang](https://github.com/KhronosGroup/glslang) and
[SPIRV-Tools](https://github.com/KhronosGroup/SPIRV-Tools) describe those tools.
Full SDK tests also require them; core-only builds/tests remain independent.
Rust subprocesses use RUSTUP_AUTO_INSTALL=0 as documented by
[Rustup](https://rust-lang.github.io/rustup/devel/environment-variables.html).
The stable channel moves; doctor reports the actual versions, not a release pin.

## Quickstart

From the checkout, with its Rust components and shader tools installed:

```text
cargo run -p ex-sdk --locked --offline -- doctor
cargo run -p ex-sdk --locked --offline -- info
cargo run -p ex-sdk --locked --offline -- new target/my-ex-app --name my-ex-app
```

From target/my-ex-app:

```text
cargo run --locked --offline
cargo build --release --locked --offline
cargo fmt -- --check
```

Open target/ex-starter.png inside that project. Edit shaders/triangle.vert or
triangle.frag, then build/run again. build.rs compiles into Cargo's OUT_DIR and
src/shaders.rs includes the generated modules. src/renderer.rs owns resources,
explicit transfer-to-vertex ordering, drawing and readback. The Float32x2 position
attribute supplies Z=0/W=1 to the GLSL vec4 input. Export errors describe file I/O.
The starter checks corner background/centre coverage before PNG export.

Cargo caches successful build results. Source files, shader directory, selected
tool environment variables, PATH and resolved native executable files are rebuild
inputs; SDK dependency changes also invalidate results. Broken shaders stop Cargo
instead of using old generated instructions. Includes are unsupported. There is
no separate persistent artifact cache, portable EX binary format or hot reload.

The generated project is its own workspace with its own lockfile and absolute
SDK paths. Update both runtime and build dependency paths if the checkout moves.
There are no registry crates. Quote shell paths containing spaces. Creation and
shader CLI output refuse existing destinations; choose a new artifact path.

Example from the checkout, after project creation:

```text
cargo run -p ex-sdk --locked --offline -- shader reflect target/my-ex-app/shaders/triangle.frag --stage fragment
cargo run -p ex-sdk --locked --offline -- shader build target/my-ex-app/shaders/triangle.frag --stage fragment --output target/fragment.rs
```

## Evidence and remaining gates

[CLI tests](../crates/ex-sdk/tests/cli.rs) exercise arguments, file preservation,
cleanup, missing tools, build/import/reflection and standalone format/build/run
in a path with spaces. GLSL edits change the rendered image, invalid source stops
the build, tool selection invalidates the build even with unchanged source, and
repair recovers the original image. [Importer tests](../crates/ex-shader/tests/import.rs)
evaluate math/swizzles/locals, validate binary import, reject malformed/unsupported
inputs, enforce instruction limits, and use reflected uniform layouts to change
both geometry and colours across reused submissions. Every 16x16 pixel is checked.
[A varying test](../crates/ex-shader/tests/varying.rs) verifies all gradient and
background samples after actual GLSL vertex/fragment interpolation.

The starter produces a 64x64 image with 1152 shaded samples; independent PNG
CRC/decompression/geometry checks verify all 4096 pixels. Native tools and doctor
ran successfully on this Windows environment. All 272 workspace tests and project
checks pass. The graphics library remains 121 types, 229 methods and 34 error types;
[api.md](../api.md#sdk-shader-tooling) catalogs the separate SDK shader library.

| SDK gate | Status |
| --- | --- |
| Setup, capabilities and standalone starter | Working |
| First established shader import/build, supported lowering and reflection | Working for EX-GLSL-1 |
| Incremental shader builds | Working through Cargo's input tracking/cache |
| Broader source import and independent artifact cache | Future extensions |
| Safe reload and optional asset helpers | Planned; next selected workflow is safe shader reload |
| Window/engine integrations and diagnostic/capture tooling | Planned |
| Preview package distribution | v0.1.0 Windows archive workflow in [RELEASING.md](RELEASING.md); source plus binary, checksums and provenance |
| Contributor onboarding, license and broader releases | Planned; no project license selected |

SDK Preview is not complete. The first v0.1.0 preview package records its tested
Windows target and actual toolchain/dependency versions; its [release notes](releases/v0.1.0.md)
state the bounded capability and unselected project license. Broader platforms,
release pinning and licensing policy remain later work. This adds no hardware,
window presentation or managed API claim.
