# Core Preview 1 and the execution boundary

Selected contract (2026-10-09). This gate defines the supported core profile,
routes real submissions through a device-owned executor, and provides a reusable
public contract corpus. Implemented evidence is recorded below.
The final integrated Core Preview completion gate remains separate.

## Purpose and scope

BackendKind identifies an implementation; a profile specifies what applications
can rely on. A target's name alone cannot establish shader, resource, or execution
capabilities. EX now selects `CoreProfile::Preview1`: the existing bounded
software behavior, with queryable `CoreLimits`. Adapter and device expose
`core_profile()`, and the profile exposes `limits()`. Queries perform no execution
or allocation. Returned limit values are copies; modifying a local copy does not
configure the device. Existing descriptor literals and submission APIs remain.

Other APIs commonly distinguish implementation discovery from supported features.
EX chooses one coherent fixed profile for this first core rather than a matrix
of optional capabilities. A future profile or backend requires implemented
behavior and tests before discovery can advertise it. Preview1 is provisional,
not a stable v1 specification or a hardware/performance claim.

## Supported behavior and limits

| Area | Preview1 requirement |
| --- | --- |
| Storage | Owned nonzero buffers with explicit usages/HostAccess; maximum contiguous buffer bytes are this process's isize::MAX, subject to allocation failure |
| Images | Positive 2D axes at most 16384; linear RGBA8 colour, separate f32 depth and u8 stencil; one mip; whole-image texture/framebuffer aliases preserve identity/permissions |
| Transfers | Byte buffer copies, four-byte-aligned padded RGBA8 buffer/image rows, equal-extent image rectangles, disjoint self-copies; live sources and preserved outside bytes/pixels |
| Shaders | Finite float4 EX IR; at most 16 inputs and 256 instructions; vertex outputs 1..=17 (position then varyings), fragment exactly one, compute 1..=8 with exactly four ID inputs |
| Instructions | Input, Constant, Add, Multiply, Dot4, Swizzle; uniform loads in all stages; paired texture sampling in Fragment; per-invocation storage loads in Compute |
| Interfaces | One retained group, eight sparse slots, stage/type/minimum-size checked layouts; 16-byte-aligned uniform/storage views; uniforms at most 4096 bytes (256 float4 elements) |
| Graphics | Fixed and programmable triangle lists, one vertex buffer, Float32/Float32x2/3/4 attributes, u16/u32 indices and signed base vertex, positive clip w, x/y/z clipping in [-w,+w], z/w mapped to [0,1], and perspective interpolation |
| Fragment operations | Nearest/linear sampling, clamp/repeat, depth comparisons/write control, RGB/alpha blending/channel masks, front/back stencil comparison/update; one colour attachment |
| Compute | Positive local axes and product at most 1024; group counts at most 65535 per axis, at most 1048576 invocations; zero group axes execute no invocations; bounded float4 storage outputs |
| Synchronization | Ordered or Explicit; whole-allocation roles and full transition dependencies; alias-aware preflight before execution; synchronous completion and one device execution gate |
| Failure | Recording/preflight errors execute nothing; runtime failure preserves the failed command's destination or all outputs/attachments, retains completed prefix, and skips later commands |
| Lifetime | Commands retain original devices and resources; immutable command buffers are reusable and read live contents on each submission |

Queryable maxima are defined once in profile.rs and used by the relevant
validators. Array capacities and representation widths remain fixed implementation
invariants consistent with Preview1, not user-configurable settings. Maximum
resource sizes describe valid requests, not guaranteed available memory. This
gate does not attempt giant allocations to prove every maximum can fit in RAM.
Topic contracts specify arithmetic, addressing, clipping and error precedence.

## Runtime and backend responsibilities

Adapter selection creates one private executor in each logical device. Device
and queue clones share it and the execution gate. The private ExecutionBackend
trait executes one validated command and returns a structured, unindexed
ExecutionError. The ReferenceBackend implementation interprets actual copies,
dispatches, render scopes and transitions using EX's existing algorithms.

Queue policy rejects foreign work and preflights the whole explicit plan before
calling the device's execution path. The device acquires its gate, executes
commands in order through its selected executor, assigns command indices to
errors, and stops on the first failure. Backend code must not select the next
command, submit nested work, re-acquire the public gate, retain submitted work
after synchronous return, or repair a missing dependency. Each command's commit
boundary and structured error must match the profile contract.

Creation, host access and storage ownership remain in the working resource
modules. This is a private Rust execution interface, not a public driver/plugin
ABI or a complete abstraction for hardware memory allocation. A hardware target
needs real resource/storage and completion integration before it is selectable.
No external graphics API renderer, alternate backend, async queue or SDK is added.

## Reusable profile checks

The public integration harness discovers adapters advertising Preview1 and runs
the same adapter-parameterized cases for each. Cases must assert observable
bytes, pixels and errors, without inspecting private resource locks or relying
on a software adapter name. At least one matching adapter must be discovered;
an empty test run is not success.

The corpus checks profile limits/rejection, shader arithmetic/stage rules, live
ordered copies, explicit preflight and dependencies, padded image flow and
sampling, indexed graphics with parameters, depth/blend/stencil and render
rollback, compute parameters/output rollback, eight-output compute IDs and late numeric rollback, and retained-work lifetime/reuse.
Existing detailed tests continue to verify numeric/alias/limit edge cases. Private
poison tests remain implementation-specific; they are not portable target tests.
An internal traced executor additionally verifies that queue policy calls the
selected executor only after preflight, under the gate, and stops at its first
failure with a runtime-assigned command index. It is a test fixture, not another
implemented graphics backend.

Run the corpus independently with:

```text
cargo test -p ex-api --test core_profile --locked --offline
```

Passing the corpus is required evidence for a target advertising Preview1; the
corpus is not external certification or proof of all possible programs. Detailed
topic suites, full project checks and final integrated workload remain required.
Unsupported operations absent from EX's enum/API cannot be requested; supported
syntax in the wrong stage or outside the profile bounds returns existing errors.

## Tests and examples

Implemented in [profile.rs](../crates/expansex-core/src/profile.rs),
[backend.rs](../crates/expansex-core/src/backend.rs),
[device state and dispatch](../crates/expansex-core/src/object.rs),
[queue policy](../crates/expansex-core/src/submission.rs), and
[ReferenceBackend](../crates/expansex-core/src/reference.rs).
Shared profile bounds are used by buffer/image, shader, binding and compute
validators; existing requests and structured errors are preserved.

Nine [public harness tests](../crates/ex-api/tests/core_profile.rs) invoke the
[adapter-parameterized corpus](../crates/ex-api/tests/core_profile/cases.rs).
Independent expected bytes/pixels check live copies, padded multi-pass images,
indexed sampling with vertex/fragment uniforms, exact bilinear means,
RGB/alpha blending with depth and stencil, render rollback, parameterized
compute, all four built-in ID vectors and eight outputs, late eight-output
rollback, explicit preflight, and retained reusable work. Boundary checks reject
oversized requests without giant allocations and accept zero-invocation work.
The internal traced-executor test verifies selected dispatch, gate ownership,
no execution on foreign/dependency rejection, runtime error indexing, prefix
preservation, early stop, and absence of an ownership cycle. The fixture injects
an error; it does not implement an alternate target.

All 256 workspace tests and required project checks pass. The existing
texture_flow workload runs through the selected backend and its 640x320 image
matches the previously independently verified bytes exactly. This gate is met;
the final maintained integrated workload/profile completion review is still
pending before Core Preview completion and subsequent SDK work.
