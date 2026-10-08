# Expanse-X (EX)

An independent low-level graphics API written in Rust. EX will define its own
public API, runtime, command model, and execution backend. The first backend will
be a software/reference GPU; rendering will not use Vulkan, Direct3D, Metal,
OpenGL, or wgpu.

The [project design](DOCUMENTATION.md) defines one core with managed
and explicit control levels, Rust-based ownership, and validated execution.
Build the explicit core first; managed planning is a future layer over the same
resources and execution rules.

## Current status: Phase 1 complete — synchronous reference copy execution

EX now discovers its built-in software adapter, creates independent logical
devices, and returns default queue handles with shared ownership. The workspace
also allocates owned reference buffers with explicit CPU upload/readback
permissions, bounds checks, and shared storage. Device-bound encoders now record
validated buffer copies and retain their resources without executing them.
Queue submission executes finished copies synchronously in order on the CPU,
with whole-submission serialization against CPU access and defined partial failure.
Applications import the `ex-api` package as `ex`. Shaders and rendering remain planned.

```text
cargo run -p ex-api --example submit_copy --locked --offline
```

The example uploads bytes, records an ordered chain through a host-inaccessible
buffer, verifies unchanged bytes before submission, then submits and reads results.
`record_copy` separately verifies that recording/finishing never execute work.
`buffer_roundtrip` demonstrates CPU access and denied writes; `discover` shows
device/queue ownership; `api_draft` retains the standalone label-only prototype.

## Build and check

Install Rust through rustup. `rust-toolchain.toml` selects stable Rust with
rustfmt, Clippy, and Rust source; no extra platform target is required. Current
packages use only the Rust standard library and local workspace dependencies.
After the toolchain is installed, these commands work without network access:

```text
cargo fmt --all -- --check
cargo build --workspace --all-targets --locked --offline
cargo clippy --workspace --all-targets --locked --offline -- -D warnings
cargo test --workspace --locked --offline
cargo doc --workspace --no-deps --locked --offline
```

`scripts/check.ps1` runs these checks in order and fails on the first error. Run
`pwsh -NoProfile -File scripts/check.ps1` from any directory, or use the direct
commands from the project root in CI. Tests include executable and compile-fail
documentation examples. Keep the workspace lockfile committed. The stable
toolchain follows stable releases; it is not a pinned compiler-version guarantee.

## Repository map

| Path | Responsibility |
| --- | --- |
| `DOCUMENTATION.md` | Project design and commitments |
| `api.md` | Complete catalog of currently exposed public APIs |
| `crates/ex-api/` | Public Rust entry point, imported as `ex`; runnable example |
| `crates/expansex-core/` | Discovery, ownership, reference buffers, copy recording/execution |
| `docs/` | Current EX contracts, architecture, assessment, and conventions |
| `scripts/` | Repeatable production checks |

## Next milestones

The [development roadmap](docs/ROADMAP.md) is the current progress tracker and
defines completion checks for each phase. Phases 0 and 1 are complete. Discovery,
owned buffers, recorded copies, submission, and synchronous readback now work.

Phase 2 targets the first software triangle; Phase 3 adds programmable shaders
and pipelines to complete the original graphics milestone. Modern GPU features
and hardware drivers follow working reference semantics, one capability at a time.

Read the [starting assessment](docs/ASSESSMENT.md),
[architecture](docs/ARCHITECTURE.md), [public API catalog](api.md),
[current API contract](docs/API-DRAFT.md),
[object ownership contract](docs/OBJECT-MODEL.md), and
[buffer contract](docs/BUFFERS.md), [copy-recording contract](docs/COMMANDS.md), and
[submission contract](docs/SUBMISSION.md).
See [coding conventions](docs/CONTRIBUTING.md)
for implementation and commit policy.
