# Expanse-X: project philosophy

Design adopted on 2026-10-08.

This document describes EX's purpose and guiding design commitments. The [roadmap](docs/ROADMAP.md) tracks implementation progress;
[architecture](docs/ARCHITECTURE.md) and topic contracts describe how the current
implementation works. Accepting a philosophy does not make its planned features
implemented.

## Purpose

EX aims to provide explicit graphics control with Rust-based ownership and
validated execution. Applications can let EX manage execution or supply their
own plan, using the same resources and core semantics.

The long-term ambition is Vulkan-level control over meaningful application
decisions: resource allocation and reuse, memory placement, command recording,
queue selection, dependencies, and submission. This is a design target, not a
claim of current feature parity, performance, or control over every internal GPU
scheduling decision. EX defines its own contracts rather than copying Vulkan's
API.

EX should make common state, lifetime, and synchronization mistakes harder to
express, detect invalid work where static proof is impractical, and explain what
valid execution requires. It does not promise to eliminate all programming
errors or infer every shader's data-dependent memory access.

## One core, two control levels

```text
                     Application / game engine
                                |
                               EX
                                |
                    +-----------+-----------+
                    |                       |
              Managed level           Explicit level
                    |                       |
                    +-----------+-----------+
                                |
                             EX Core
                                |
                           EX Runtime
                                |
                         EX Driver Layer
                                |
                   EX reference GPU / future GPU
```

This is the intended logical architecture. It does not mandate a separate crate
for every box or imply that execution and hardware drivers exist today.

**Managed: "Describe what I want rendered."** The application supplies work and
resource-use information. EX derives necessary dependencies and chooses a valid
execution plan within the application's requirements. Managed defaults should
make ordinary work straightforward; their execution decisions and costs should
be inspectable.

**Explicit: "Let me decide how that work executes."** The application supplies
the plan and dependencies for the controls EX exposes. EX validates that plan
against the execution contract. Invalid plans produce understandable errors;
EX must not silently repair missing dependencies by substituting a different
schedule. Correctness-required behavior documented in the contract is distinct
from a hidden repair.

Both levels must share resource identity, device ownership, command meaning,
shader and pipeline contracts, and completion semantics. Applications should be
able to move individual workloads between levels without replacing their
resources. Mixing levels requires explicit, documented boundary information:
resource uses, dependencies, and completion. Interoperability is a commitment;
the exact public methods and boundary representation remain future design work.

The managed level is built on the same explicit core and execution rules. It
must not become a separate renderer with different resource or safety semantics.
Explicit control means choosing a valid plan, not disabling safety checks.

## Safety responsibilities

| Mechanism | Intended responsibility | Boundary |
| --- | --- | --- |
| Rust types and ownership | Express legal states and prevent misuse such as using a consumed encoder | The compiler does not automatically model GPU execution or prove all resource dependencies |
| Runtime validation | Check dynamic device ownership, ranges, usages, state compatibility, and declared dependencies | Validation needs sufficient resource-use information; unavailable information must have a defined contract |
| Execution lifetime management | Retain resources used by recorded/submitted work and coordinate CPU access with pending execution | Keeping storage alive does not establish ordering, visibility, or completion |

The safe public interface must uphold its documented safety guarantees in both
levels and in all build profiles. Optional expensive diagnostics may have a
separate policy, but turning them off must not remove mechanisms required for
safe execution. Any future unsafe platform or interoperability boundary must be
isolated, justified, and document the obligations it transfers to the caller.

Failures should identify the operation, relevant resource, violated requirement,
and useful next action, with machine-readable errors where appropriate. EX must
define failure effects, including whether earlier work may have executed. Safety
does not imply correct application logic, an intended image, deadlock freedom,
or identical pixels on every future hardware target.

## Example: upload followed by drawing

This example describes the intended design, not available API methods.

1. An upload writes a vertex buffer.
2. A draw reads that buffer.
3. The draw requires the upload's writes to be visible before it reads them.

At the managed level, EX uses declared resource accesses to establish this
dependency and exposes the resulting plan for inspection. At the explicit level,
the application supplies the dependency and EX checks it. Both must retain the
buffer for the work's lifetime and follow the same visibility rules.

The distinction is who chooses the plan. Neither level makes an invalid
read/write conflict acceptable. Shader-internal synchronization and arbitrary
memory access require their own contracts as programmability grows.

## Independent execution and useful differentiation

EX is an independent Rust graphics API with its own runtime, commands, resource
semantics, shader direction, and execution backend. Vulkan, Direct3D, Metal,
OpenGL, and wgpu may be studied as references; they do not perform EX rendering.
The first execution target is EX's own CPU software/reference GPU. Direct
hardware-driver work remains deferred until reference semantics are useful and
testable, with a viable target and operating-system interfaces.

EX's proposed value is a coherent combination of explicit control, managed
convenience, understandable diagnostics, and inspectable reference execution.
Rust, automatic dependency handling, and two control levels alone are not claims
of a feature unique to EX. Performance and usability benefits require examples
and measurements rather than assertions.

## EX Witness: experimental differentiation

Status: accepted for exploration on 2026-10-08; not implemented. EX Witness is a
working name for turning a selected graphics failure into an explanation, a
replay, and a smaller runnable test. Its usefulness must be demonstrated before
it becomes a promised capability.

The intended workflow is:

1. Select a validation error, unexpected buffer value, or pixel with an explicit
   condition describing the failure to preserve.
2. Explain the relevant commands, resource accesses, and available intermediate
   results. Report observed facts; application intent remains the developer's
   responsibility.
3. Capture the work, resource descriptions, starting contents, dependencies, and
   other inputs needed to replay the supported case through EX's reference backend.
4. Attempt to remove unrelated commands and data while keeping the selected
   failure reproducible. Revalidate each candidate and distinguish the target
   failure from unrelated errors introduced by reduction.
5. Export a runnable reproduction with its required EX version and environment
   information. After a fix, the developer can turn it into a regression test.

For example, a future pixel explanation could show that a character fragment
reached a pixel but failed its depth test against a value written by a background
draw. A reproduction could retain those draws and their relevant resources.
This example requires rendering and depth support that EX does not yet have;
it does not establish that the background draw was logically wrong.

Witness must use the same EX command and execution contracts for managed and
explicit work. It is optional debugging machinery, not a third control level or
a second renderer. Capturing a rejected operation must reproduce the validation
failure without executing unsafe or invalid work. Detailed tracing and repeated
replay have memory and execution costs and should be explicitly enabled.

The smallest useful experiment comes after recorded copies and submission work:
capture a supported buffer-copy failure and the resource state/operation inputs
needed to reproduce it, then export a standalone reference test. A failure may
be a rejected copy request or an application-provided readback assertion. First
prove capture and replay; then attempt reduction. Pixel explanations, shader
tracing, and frame reduction follow the corresponding execution features.

Limits and success criteria:

- Reduction requires an observable failure condition. A wrong-looking image
  alone does not tell EX what the application intended.
- Aim for a smaller reproduction, not a guarantee of the absolute smallest case.
- Replay covers only supported operations and captured inputs. Hardware-specific
  faults and timing behavior may not reproduce in the reference backend.
- A standalone case should need EX and its stated build/runtime requirements,
  rather than the original engine or asset pipeline. Portability must be tested
  for the supported scope; arbitrary-machine or bit-identical GPU replay is not
  promised.
- The first experiment succeeds when an exported copy case reproduces the
  selected failure independently of its originating application. A reduced case
  must preserve that condition, and the fixed case must support a passing test.

Pixel history, shader debugging, and capture exports already exist in tools such
as [RenderDoc](https://github.com/baldurk/renderdoc/blob/v1.x/docs/how/how_inspect_pixel.rst)
and [Nsight](https://docs.nvidia.com/nsight-graphics/UserGuide/frame-debugger-overview.html).
The proposed distinction is integrating explanation, reduction, and executable
tests into EX's own execution model, not claiming those individual ideas are new.

## EX Proof: experimental execution certificates

Status: accepted for exploration on 2026-10-08; not implemented. EX Proof is a
working name for proof-carrying execution: a plan is accompanied by a
machine-checkable certificate explaining why specified resource-access
obligations hold under a defined EX model. It does not certify an entire frame
as correct without stating what was checked and assumed.

The intended workflow is:

1. Define the supported command/access model and the properties being checked.
   Begin with buffer copies and their ranges and dependencies; expand only when
   the corresponding execution rules exist.
2. Describe the plan, relevant resource relationships, access ranges, and
   execution/visibility dependencies.
3. Generate a certificate with the evidence for supported obligations. Managed
   work produces a plan and certificate; explicit work supplies a plan that EX
   attempts to certify without silently changing it.
4. Verify the certificate using a small, separate checker that does not render
   the work or simply trust the planner's verdict. Record the model version,
   coverage, assumptions, and binding to the exact plan and resource parameters.
5. Return structured results explaining satisfied obligations, missing
   requirements, invalid certificates, or unsupported scope. Unsupported work
   must not be labelled proven or automatically treated as an invalid plan.

For example, if copy A writes a temporary buffer and copy B reads those bytes,
the report can identify the dependency that orders those accesses and makes A's
writes visible to B. A separate copy with disjoint modeled resource accesses
may need no additional dependency. This is an illustrative report, not an
implemented API or a claim about today's queue handles.

The proposed benefit is making synchronization reasoning inspectable and
exchangeable. Automated checks could verify supported plan properties without
the target GPU, and developers could recheck changes to dependencies, queue
assignments, or memory reuse. A dependency unnecessary for the modeled resource
accesses is not automatically a performance problem; actual speed needs measurement.

### Results and presentation

The result is structured data, not mandatory console printing. Optional views
can include a readable console report, editor/debugger links to commands and
resources, automated-test results, and an exported certificate for an independent
checker. Developers explicitly request reports or enable diagnostics; normal
rendering remains quiet. Reporting policy does not disable required safety checks.
No console format, editor integration, certificate format, or public method is
implemented or frozen by this decision.

### Scope, trust, and relation to Witness

A certificate proves only the stated properties within its model and assumptions.
Possible later obligations include resource retention, valid ranges, access
compatibility, and execution/memory dependencies. Rust ownership alone does not
prove those GPU properties. Shader-dependent accesses, aliasing, CPU concurrency,
and external inputs must be modeled or explicitly excluded from the certified
scope; the current implementation provides no such certificates.

The checker and its rules remain part of the trusted implementation. Execution
must obey the checked plan, and actual resource parameters must satisfy the
certificate's assumptions. Changing relevant commands, parameters, resources,
or assumptions requires rechecking. A certificate does not replace runtime
enforcement, prove driver/hardware correctness, guarantee the intended image,
or establish identical output or optimal performance across GPUs.

EX Proof explains why a supported plan satisfies specified obligations. EX
Witness captures and reduces selected failures. A future failed proof check
could supply a condition for Witness, but neither experimental feature depends
on the other being fully implemented. Both use the same managed/explicit core
contracts and introduce no third control level.

The smallest experiment follows the copy/submission foundation: define a narrow
buffer-copy model, generate a certificate for a valid plan, and independently
check it without executing copies. Verify that the checker rejects a missing
required dependency, a corrupted certificate, and a certificate reused against
an altered plan or resource parameters. Report unsupported inputs distinctly.
Full shader proofs and hardware synchronization are later research, not pilot scope.

Formal GPU synchronization verification and synthesis already exist, including
[NVIDIA's WEFT research](https://research.nvidia.com/publication/2015-06_verification-producer-consumer-synchronization-gpu-programs)
and [AutoSync](https://cseweb.ucsd.edu/~npolikarpova/publications/fmcad18.pdf).
EX's proposed distinction is independently checkable execution certificates
integrated with readable explanations and Witness reproductions. No claim of
market uniqueness or a sound, complete verifier is established by this document.

## Implementation order and current reality

Build the explicit core first and prove its rules with small working operations.
Then add managed conveniences over those proven contracts when a real example
justifies them. Do not create two API trees, empty backend crates, or a complete
theoretical scheduler before work can execute.

Today EX implements software discovery, logical device/queue ownership, owned
buffers, checked synchronous CPU access, and device-bound buffer-copy recording
with transfer/device/range/overlap validation and resource retention. Standalone
prototype labels remain separate. Synchronous reference submission now executes
copies in order, samples live bytes, serializes CPU access, and reports partial
failure. This completes the minimum runtime phase. Rendering, shaders, managed
planning, general execution hazard validation, EX Witness, EX Proof, and hardware
drivers remain planned. The next milestone is the first software triangle,
as defined by the roadmap and [submission contract](docs/SUBMISSION.md).

## Design commitments

Future designs and roadmap changes must conform to these commitments. Replacing
the independent execution model, splitting the levels into incompatible systems,
or weakening the safe-interface guarantees requires an explicit design
revision of this document, with the reason recorded here and related documents
updated together.

Public names, data structures, module boundaries, and individual algorithms can
evolve through normal implementation work within these commitments. "Locked"
means a stable design direction, not a frozen API or a claim of completeness.

Decision record: 2026-10-08 — accepted one core with managed and explicit control
levels, Rust-based ownership, validated execution, explicit core first, and EX's
own reference execution before hardware drivers.

Decision record: 2026-10-08 — addition of EX Witness as an
experimental direction. Start with supported buffer-copy failures after the
execution foundation; retain the locked core philosophy and existing phase gates.

Decision record: 2026-10-08 — addition of EX Proof as an experimental
direction, with structured results and optional console/tool presentation. Start
with independently checked buffer-copy certificates after the execution
foundation; retain the locked core philosophy and existing phase gates.
