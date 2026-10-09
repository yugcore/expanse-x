# Core completion milestone

Accepted sequence: finish the EX core first, then build the SDK around it.
This milestone defines a Core Preview, not a complete or stable v1 graphics API.
Status: Core Preview 1 complete on 2026-10-09, under the bounded software profile
and [integrated proof](CORE-PROOF.md). SDK Preview is the next development milestone.
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

## Completion gates

| Gate | Required evidence | Current status |
| --- | --- | --- |
| Resource and pipeline execution | Owned storage, programmable graphics, compute, and validation execute through `ex` with documented limits and failure rules | Working under current contracts; richer resources below remain pending |
| Resource uses and dependencies | Explicit read/write access and transition/dependency rules for supported copies, compute, and rendering; invalid use is rejected with meaningful context | Working under [DEPENDENCIES.md](DEPENDENCIES.md): whole-resource roles, full explicit dependencies, preflight errors, and copy/compute/render evidence; range/stage/async precision deferred |
| Shader/resource interfaces | A defined supported shader profile plus reusable binding/layout and parameter-buffer contracts for supported graphics/compute workloads | Working under [BINDINGS.md](BINDINGS.md): supported bounded float4 IR, reusable stage/type/size-checked layouts/groups, uniform loads in all stages, paired textures and compute storage; external shader import and broader profiles remain future work |
| Resource flow across passes | Supported upload/transfer and render-to-texture paths sufficient for a small compute/render/readback workload, with documented formats and ownership | Working under [TEXTURE-FLOW.md](TEXTURE-FLOW.md): padded buffer/image transfers, rectangular image copies, shared full-image attachments, alias-aware dependencies and a two-pass compute/render/readback workload; broader formats/mips remain future work |
| Execution and backend boundary | Defined ownership/completion/error obligations, an implementation boundary, and reusable tests for a target implementing the supported profile | Working under [CORE-PROFILE.md](CORE-PROFILE.md): named profile and shared validation limits, device-owned selected executor, runtime-owned sequencing/completion/error indices, and nine reusable adapter-parameterized contract cases plus internal boundary verification |
| Completion proof | One maintained public workload exercises the supported profile; required checks pass; API catalog, contracts and capability limits agree | Met under [CORE-PROOF.md](CORE-PROOF.md): the example/test share a 15-command, 27-use workload covering upload, compute, indexed multi-pass rendering, depth/stencil/blend, readback, preflight rejection, late failure and repaired reuse; independently verified PNG, 257 tests and all project checks pass |

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
The [source SDK foundation](SDK.md) now supplies setup/build diagnostics,
capability queries and a standalone Rust starter. The wider SDK Preview remains
in progress; shader import/build and reflection are the next selected work area.
SDK infrastructure builds on the verified core and its existing development
aids. The [roadmap](ROADMAP.md) tracks implementation evidence.
