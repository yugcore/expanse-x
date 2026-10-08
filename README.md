# Expanse-X (EX)

An independent low-level graphics API written in Rust. EX will define its own
public API, runtime, command model, and execution backend. The first backend will
be a software/reference GPU; rendering will not use Vulkan, Direct3D, Metal,
OpenGL, or wgpu.

The [project design](DOCUMENTATION.md) defines one core with managed
and explicit control levels, Rust-based ownership, and validated execution.
Build the explicit core first; managed planning is a future layer over the same
resources and execution rules.

## Current status: Phase 1 — validated buffer-copy recording

EX now discovers its built-in software adapter, creates independent logical
devices, and returns default queue handles with shared ownership. The workspace
also allocates owned reference buffers with explicit CPU upload/readback
permissions, bounds checks, and shared storage. Device-bound encoders now record
validated buffer copies and retain their resources without executing them.
Applications import the `ex-api` package as `ex`. Queue submission, shaders, and
rendering are not implemented yet.

```text
cargo run -p ex-api --example record_copy --locked --offline
```

The example uploads bytes, records a partial copy, and verifies that the
destination remains unchanged after finishing and dropping original handles.
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
| `crates/ex-api/` | Public Rust entry point, imported as `ex`; runnable example |
| `crates/expansex-core/` | Discovery, ownership, reference buffers, and copy recording |
| `docs/` | Current EX contracts, architecture, assessment, and conventions |
| `scripts/` | Repeatable production checks |

## Next milestones

The [development roadmap](docs/ROADMAP.md) is the current progress tracker and
defines completion checks for each phase. Phase 0 is complete. Phase 1 has
discovery, owned buffers, and recorded copies; queue submission is next.

Phase 2 targets the first software triangle; Phase 3 adds programmable shaders
and pipelines to complete the original graphics milestone. Modern GPU features
and hardware drivers follow working reference semantics, one capability at a time.

Read the [starting assessment](docs/ASSESSMENT.md),
[architecture](docs/ARCHITECTURE.md), [current API contract](docs/API-DRAFT.md),
[object ownership contract](docs/OBJECT-MODEL.md), and
[buffer contract](docs/BUFFERS.md) and [copy-recording contract](docs/COMMANDS.md).
See [coding conventions](docs/CONTRIBUTING.md)
for implementation and commit policy.
