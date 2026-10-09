# Core completion milestone

Accepted sequence: finish the EX core first, then build the SDK around it.
The immediate target is a Core Preview, not a complete or stable v1 graphics API.
This is a development milestone under the [project design](../DOCUMENTATION.md),
not a change to independent execution or the shared managed/explicit core.

## Purpose and boundary

The core defines and executes resources, shaders, pipelines, commands,
dependencies, completion, validation, and lifetime. The SDK helps applications
use those contracts through project templates, compiler/build integration,
asset utilities, reload tools, inspection, and packaging. SDK convenience must
use actual core behavior; it cannot substitute for an unimplemented capability.

The first completion target is a bounded explicit core running on EX's software
reference target. Declare its supported profile and prove that profile with an
integrated workload and contract tests. A Core Preview is useful working behavior
with explicit limits; it does not establish hardware support, performance parity,
all engine features, or permanent API stability.

## Planned completion gates

| Gate | Required evidence | Current status |
| --- | --- | --- |
| Resource and pipeline execution | Owned storage, programmable graphics, compute, and validation execute through `ex` with documented limits and failure rules | Working under current contracts; richer resources below remain pending |
| Resource uses and dependencies | Explicit read/write access and transition/dependency rules for supported copies, compute, and rendering; invalid use is rejected with meaningful context | Working under [DEPENDENCIES.md](DEPENDENCIES.md): whole-resource roles, full explicit dependencies, preflight errors, and copy/compute/render evidence; range/stage/async precision deferred |
| Shader/resource interfaces | A defined supported shader profile plus reusable binding/layout and parameter-buffer contracts for supported graphics/compute workloads | Partial: small EX vector IR, texture slots, and per-element compute storage; general layouts/parameters pending |
| Resource flow across passes | Supported upload/transfer and render-to-texture paths sufficient for a small compute/render/readback workload, with documented formats and ownership | Partial: byte copies, CPU texture upload, standalone attachments, and compute-to-draw work; texture transfers and render-to-texture pending |
| Execution and backend boundary | Defined ownership/completion/error obligations, an implementation boundary, and reusable tests for a target implementing the supported profile | Partial: synchronous queue/device gate and failure contracts work; backend boundary/profile tests pending |
| Completion proof | One maintained public workload exercises the supported profile; required checks pass; API catalog, contracts and capability limits agree | Partial: individual examples and 209 passing tests exist; final integrated gate remains pending |

Choose each gate's concrete subset through its workload and topic contract before
implementation. Unsupported shader operations and capabilities must be reported
explicitly. A new name, interface, or documentation checkbox is not completion.
The milestone must remain bounded rather than absorbing every possible v1 feature.

## Work beyond the first core milestone

The existing Phase 4 checklist remains the broader feature roadmap. Asynchronous
execution, multiple queues, indirect operations, advanced memory/residency,
presentation, and managed planning are separate capabilities whose priority
depends on demonstrated workloads. They are not automatically all prerequisites
for the first supported core profile. Where a gate requires one, implement and
test the actual contract before including it in that profile.

The two-level philosophy remains locked: managed planning will use the explicit
core's resources, execution, and completion rules. The initial gate proves the
explicit foundation. Hardware drivers, EX Witness, and EX Proof retain their
separate deferred/experimental status.

## SDK follows core completion

After the core gate is met, develop the SDK workflow: setup diagnostics and
templates, established shader toolchain integration for the implemented shader
profile, reflection/build caching, safe reload, asset/window examples, diagnostics
and capture tooling, contributor onboarding, and releases. Add SDK utilities
only when they exercise the real core; expand core features when an SDK workload
demonstrates the need, with their own contracts and tests.

Existing examples, checks, and documentation continue to support core development.
This ordering defers new SDK infrastructure; it does not remove those working
development aids. The [roadmap](ROADMAP.md) tracks implementation evidence.
