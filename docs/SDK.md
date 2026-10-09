# SDK Preview development

Selected foundation contract (2026-10-09), before implementation. Core Preview 1
is complete; this begins the SDK developer workflow. SDK Preview as a whole
remains in progress until its additional shader/tool/integration gates are met.
The initial source SDK supports the existing explicit software profile only.

## Foundation scope

Add a standard-library-only `ex-sdk` workspace package with an `ex` command-line
tool using the public `ex` library. Keep the SDK separate from core execution.
Commands must return clear errors and a failing exit status on expected misuse.

| Command | Contract |
| --- | --- |
| `ex info` | Report the linked runtime's adapters, supported profile and numeric limits; identify software execution |
| `ex doctor [--sdk PATH]` | Check the source checkout, Rust/Cargo/rustfmt/Clippy availability and an offline locked core_preview example build including linking; report failures without installing or changing system configuration |
| `ex new PATH [--name NAME] [--sdk PATH]` | Create a standalone Rust starter with an explicit path dependency on the selected checkout; refuse an existing destination, including an empty directory; default package name ex-app |
| `ex help` / `--help`, `--version` | Show executable commands or the tool's package version |

Find a source checkout by inspecting ancestors of the current directory and
tool executable, or accept --sdk explicitly. Do not hard-code the developer's
machine path in the executable. --sdk selects the starter dependency/build
checkout; info reports the runtime compiled into the tool. Paths containing
spaces, backslashes and TOML quoting characters must be handled as path values,
not interpolated shell commands. Non-Unicode manifest paths return a clear error.

The starter must build/run independently of the SDK workspace, with its own
manifest and lockfile and no registry dependencies. Separate shaders.rs makes
current EX IR editable without core changes. Render an offscreen programmable
triangle through explicit upload/vertex dependencies, verify representative
pixels, and export a PNG. Include project-specific instructions and ignore rules.
This is a headless Rust starting point; it does not add shader-language import,
presentation, reload, an asset database or a managed API.

Creation must use exclusive directory/file creation and never overwrite user
files. If writing the new project fails, remove only files created by this
attempt and its own newly created directory; preserve unrelated entries. Cleanup
is best effort if the OS denies removal; interrupted/killed processes can leave
an incomplete new directory. Validate
arguments and the SDK before creating the destination. Do not run project code
or build automatically during creation. Cargo generates the app's lockfile offline
from metadata; the developer explicitly builds/runs the application. The
destination's parent must already exist. No Git initialization is performed.
Tool subprocesses disable Rustup automatic toolchain installation with
RUSTUP_AUTO_INSTALL=0, as defined by the [Rustup documentation](https://rust-lang.github.io/rustup/devel/environment-variables.html).
An absent toolchain is reported rather than installed.

## Verification and remaining gates

Exercise the CLI through real subprocesses. Check argument errors, info metadata,
existing-destination refusal, encoded manifest paths and project creation under a
path containing spaces. Build and run the generated project locked/offline with
an isolated build directory, inspect its output, and verify doctor against the
checkout. Include the tool and its tests in normal workspace checks.

| SDK gate | Initial status |
| --- | --- |
| Setup, capabilities and standalone starter | Working; CLI and generated-project evidence below |
| Established shader import/build, supported lowering and reflection/cache | Planned; current shaders use bounded EX IR |
| Safe reload and optional asset helpers | Planned |
| Window/engine integrations and diagnostic/capture tooling | Planned |
| Contributor onboarding and packaged release policy | Planned; no publication or license selection in this step |

Stable Rust is selected by the repository; its moving channel is not an exact
release compiler pin. Report actual tool versions. A distributable SDK release
still needs tested platform/toolchain/dependency versions and licensing policy.
There is no network installation, global PATH change, published package or
hardware claim in this foundation.

## Tests and examples

Implemented in [ex-sdk](../crates/ex-sdk/Cargo.toml), with
[CLI dispatch/diagnostics](../crates/ex-sdk/src/main.rs),
[exclusive creation and cleanup](../crates/ex-sdk/src/project.rs), and
[templates](../crates/ex-sdk/templates/README.md). The binary is named ex;
its manual is this document, so binary Rustdoc is disabled to preserve the
public library's ex/ documentation output. No library APIs changed.

Five [subprocess integration tests](../crates/ex-sdk/tests/cli.rs) cover real help/
info and argument errors, existing directory/file refusal, invalid names/SDKs,
failed-lockfile cleanup, missing-tool diagnostics and an independent project in
a path with spaces. That project formats, builds and runs locked/offline using
an isolated build directory. Editing its fragment instructions changes the
rendered PNG on the next build/run. Cargo metadata creates its own lockfile;
only the two existing local core packages are dependencies.

The starter renders a 64x64 programmable triangle using explicit transfer-to-
vertex ordering. The retained command buffer submits synchronously, and CPU
readback feeds the PNG writer. Representative coverage checks run in the app.
Independent PNG CRC/decompression/geometry calculations verify all 4096 pixels,
including 1152 shaded samples; the image was visually inspected. doctor and info
also ran successfully against the source checkout on this Windows environment.
All 262 workspace tests and required project checks pass. The public library
inventory remains 121 types, 229 methods and 34 error types.

## Source SDK quickstart

From the EX checkout, after installing its Rust toolchain:

```text
cargo run -p ex-sdk --locked --offline -- doctor
cargo run -p ex-sdk --locked --offline -- info
cargo run -p ex-sdk --locked --offline -- new target/my-ex-app --name my-ex-app
```

The doctor build creates target/ if needed. Move into target/my-ex-app and run:

```text
cargo run --locked --offline
cargo build --release --locked --offline
```

Open target/ex-starter.png inside that project. Edit src/shaders.rs to change
supported instructions/colour; edit src/renderer.rs for resources and explicit
rendering. Expected API errors use Result; export errors describe file I/O after
rendering. Generated projects remain independent workspaces. Their absolute
SDK path must be updated if the source checkout moves. Quote shell path arguments
when they contain spaces. Use --sdk PATH if discovery cannot find the checkout.
The generator does not overwrite an existing destination or initialize Git.

This source foundation is complete; the wider SDK Preview remains in progress.
Next: a chosen established shader import/build and reflection contract for the
supported EX IR, with explicit errors for unsupported language features.
