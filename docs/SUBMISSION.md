# EX synchronous reference submission

Queue submission executes copies, render scopes and compute dispatches in order
on the CPU. A successful return means that work has completed.

## Design

Recording describes work; submission makes that work happen. The reference target
needs a small, observable upload → record → submit → readback path before drawing.
`Queue::submit(&self, commands: &CommandBuffer) -> Result<(), QueueSubmitError>`
borrows finished work and executes copies/compute dispatches/render scopes synchronously in recording order.
The command buffer remains inspectable and can be submitted again. Each submission
reads live source bytes, including changes since recording and earlier copies.

For comparison, [Vulkan submission](https://docs.vulkan.org/refpages/latest/refpages/source/vkQueueSubmit.html)
accepts command-buffer batches and an optional completion fence, while
[queue-idle waiting](https://docs.vulkan.org/refpages/latest/refpages/source/vkQueueWaitIdle.html)
waits on the host for outstanding work. EX's first reference path combines execution
and completion in one blocking call. It needs no worker, fence, or staging copy.
Future asynchronous targets need explicit completion; this implementation does not
establish their final scheduling API or force future hardware to execute on the CPU.

## Validation, execution, and failure

1. Reject a command buffer from another logical device, even if empty. This changes
   no bytes. Devices on the same adapter are still distinct.
2. Validate the entire Explicit dependency plan; a failure executes nothing.
3. Acquire the device's shared execution gate.
4. Execute commands in order, reading source contents at each operation.
5. Return `Ok(())` after every operation has finished. Subsequent permitted CPU reads
   observe the results unless another ordered operation changes them first.

Only the encoder can construct copies; devices, usages, ranges, and overlap were
validated there. Their metadata and storage lengths cannot change, so submission
does not revalidate immutable facts. CPU host permissions apply to public CPU
access, not to transfer execution. Device-side copies can use `HostAccess::None`.
Same-allocation disjoint copies lock that allocation once and use `copy_within`.
Different allocations lock source and destination before changing any bytes.
Zero-byte copies execute as no-ops without locking storage, even if it is poisoned.

`QueueSubmitError` implements `Debug`, `Clone`, `PartialEq`, `Eq`, `Display`, and
`std::error::Error`:

- `ForeignCommandBuffer`: finished work belongs to another device; nothing executed.
- `DependencyFailed { source: DependencyValidationError }`: full preflight failed;
  no command executed. Exposes its underlying Error::source.
- `StoragePoisoned { command_index: usize, role: CopyBufferRole, label: String }`:
  a nonempty copy could not trust its source/destination storage. The index is
  zero-based. Earlier commands completed; the failing copy changes no destination
  bytes, and later commands do not execute. There is no transaction or rollback.
  For an aliased copy the single lock is checked as `Source`.
- `ComputeFailed { command_index: usize, source: ComputeExecutionError }`: a
  dispatch could not lock storage, reserve outputs, or evaluate an invocation.
  All dispatch outputs remain unchanged; completed earlier commands remain and
  later work is skipped. The source includes poisoned slot/label or global
  invocation coordinates and shader explanation. See [COMPUTE.md](COMPUTE.md).
- `RenderFailed { command_index: usize, source: RenderExecutionError }`: that scope
  changed no target pixels; earlier commands completed and later commands are
  skipped. Vertex decoding/preflight precedes shading into a temporary image;
  the scope commits only after all draws succeed, including fragment invocations.
  This variant exposes its underlying Error::source. See
  [RENDERING.md](RENDERING.md) and [PIPELINES.md](PIPELINES.md).

Retrying a failed buffer replays the earlier commands too. The borrowed command
buffer still owns its resources after failure. Poisoned storage is not repaired.
Error-label allocation follows ordinary Rust OOM behavior.

## Concurrency and locks

Every submission, valid public CPU buffer access, and framebuffer readback uses the same
gate for its entire operation. Queue clones and repeated `default_queue()` calls
share it. Submissions cannot interleave; CPU accesses cannot enter between their
commands. A CPU access may happen before or after a concurrently started submission;
thread start order and mutex fairness are not promised. Applications needing a
particular order must order their calls. Permission/range checks still precede locks.
Unrelated devices use independent gates.

The invariant is **device gate first, then storage locks**. Storage locks never
acquire the gate. All buffers in submitted work have the gate's device identity,
so opposite-direction copies cannot form a lock cycle. Same-buffer aliases take
only one storage lock. Buffer metadata getters and Buffer Debug formatting acquire
neither lock.
This deliberately serializes even independent buffers on a device; finer-grained
parallelism requires a new demonstrated workload and contract.

Compute locks each unique used allocation once, before evaluation, retaining
all guards through atomic output commit. Disjoint views can share storage;
read-only overlaps are allowed, while distinct overlapping views involving a
writer are rejected at recording. Loads see pre-dispatch bytes; following
commands see committed outputs. Zero dispatches and unused slots skip locks.
Public host permissions do not restrict device-side storage execution.

The gate contains only `()`, not resource state. If an internal panic poisons it,
EX recovers its guard and still serializes operations. Byte-storage poison remains
an error: its contents might be untrustworthy. See Rust's
[mutex poisoning documentation](https://doc.rust-lang.org/std/sync/struct.Mutex.html).
This is a deliberate local recovery rule, not general device-loss recovery.

## Ownership, verification, and limits

The borrowed command buffer and retained handles keep all resources alive throughout
the blocking call. No queue-owned pending work remains afterward. Dropping finished
work releases its handles; independently retained buffers remain usable.

Public tests cover actual results, partial ranges, ordered chains, late writes,
repeated and empty submissions, host-inaccessible intermediates, aliases, foreign
devices, retained resources, and concurrent submissions/CPU access. Internal tests
inject poisoned locks and exercise partial failure and gate recovery without adding
a public fault-injection API. The `submit_copy` example demonstrates completion.

Fixed, direct programmable, and indexed rendering use the same queue. Indexed
draws sample live index bytes after earlier copies, validate effective references,
and fetch vertices under sequential storage locks while the device gate holds.
Invalid references/poison preserve the failed scope target; retained commands
remain reusable. See [INDEXED-DRAWING.md](INDEXED-DRAWING.md). Texture upload,
readback, and standalone sampling also acquire the device gate. Nonempty draws
snapshot each used view once before shading, releasing source storage guards
sequentially; aliases cannot deadlock. Poison/allocation/shading failures retain
the same failed-scope boundary. See [TEXTURES.md](TEXTURES.md).
Depth scopes retain their attachment, acquire distinct colour/depth storage locks
under the gate, and stage both images. Successful commit publishes both together
before readers can enter; any expected failure preserves both. Empty scopes still
check attachment poison. See [DEPTH.md](DEPTH.md).
Blending reads/writes staged colour in draw order after shading, without additional
storage locks or per-fragment allocations. Late shader errors discard blended
writes and depth together. See [BLENDING.md](BLENDING.md).
Stencil scopes retain an optional S8 attachment. Colour, depth, and stencil locks
are distinct and acquired under the shared gate; staging commits all attached
images together, or preserves all on failure. Stencil fail/depth-fail updates are
part of that same scope boundary. Clears/empty scopes still check stencil poison.
See [STENCIL.md](STENCIL.md). No GPU driver,
asynchronous job, multiple queues, fence, semaphore,
timeline, cancellation, or public execution-backend ABI is implemented here. Explicit
whole-resource barriers are supported under [DEPENDENCIES.md](DEPENDENCIES.md).
Completion covers the submitted command buffer, not future calls from other threads.

## Selected executor

The queue rejects foreign command buffers and preflights explicit dependencies
before calling the owning device's execution path. The device holds its execution
gate across the complete command loop and calls its private selected executor
once per command. Backend errors carry operation details; the runtime supplies
the command index and stops immediately. Completed commands remain visible, the
failed command preserves its destinations, and later commands are skipped.

The built-in ReferenceBackend implements this interface with existing software
execution. Device/queue clones share the executor and gate. Creation and host
access remain in the resource modules. [CORE-PROFILE.md](CORE-PROFILE.md) defines
the profile, interface obligations and reusable checks; this private Rust boundary
does not provide public backend registration or hardware integration.


## Explicit preflight

Before the device gate, submit rejects foreign work and validates any Explicit plan
in full. DependencyFailed retains its validation source and changes no submitted
resources, including operations before the invalid use. Metadata analysis does not
read resource bytes, check poison, or execute shaders. Ordered mode skips this
analysis. Accepted plans execute under the same gate and atomic operation contracts.
Transitions preserve data and establish full execution/write-visibility dependencies;
the synchronous reference executor already supplies those effects through ordering.
Local declared roles are rebuilt on every submission; there is no persistent layout.


Used uniform parameter views are snapshotted once per nonempty operation under
the execution gate before storage/vertex locks. Locks are released before other
sources are acquired, including same-allocation aliases. Snapshot poison or loaded
nonfinite values retain the same atomic failed-operation/completed-prefix rules.
Unused slots and zero source operations skip parameter reads. [BINDINGS.md](BINDINGS.md)
defines the retained, live group contract.

## Texture flow integration

Texture transfers follow the same completed-prefix rules: TextureTransferFailed
retains command index and TextureTransferExecutionError; failed destinations stay
unchanged and later commands skip. Buffer/image copies lock both operands before
writing rows, preserving padding. Image/image copies reserve and snapshot the
source before destination commit, including disjoint aliases. These executor calls
never acquire the device gate again. Shared texture attachments retain whole-scope
colour/depth/stencil rollback. CPU framebuffer reads enforce the allocation's
HostAccess before metadata/storage checks; standalone framebuffers still allow
reads. [TEXTURE-FLOW.md](TEXTURE-FLOW.md) defines the bounded formats and layouts.
