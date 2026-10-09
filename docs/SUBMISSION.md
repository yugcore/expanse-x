# EX synchronous reference submission

Status: implemented Phase 1 contract, extended with fixed and programmable render scopes;
public names remain provisional. This is CPU execution, not hardware support.

## Design

Recording describes work; submission makes that work happen. The reference target
needs a small, observable upload → record → submit → readback path before drawing.
`Queue::submit(&self, commands: &CommandBuffer) -> Result<(), QueueSubmitError>`
borrows finished work and executes copies/render scopes synchronously in recording order.
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
2. Acquire the device's shared execution gate.
3. Execute commands in order, reading source contents at each operation.
4. Return `Ok(())` after every operation has finished. Subsequent permitted CPU reads
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
- `StoragePoisoned { command_index: usize, role: CopyBufferRole, label: String }`:
  a nonempty copy could not trust its source/destination storage. The index is
  zero-based. Earlier commands completed; the failing copy changes no destination
  bytes, and later commands do not execute. There is no transaction or rollback.
  For an aliased copy the single lock is checked as `Source`.
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
check attachment poison. See [DEPTH.md](DEPTH.md). No GPU driver,
asynchronous job, multiple queues, fence, semaphore,
barrier, timeline, cancellation, or execution-backend ABI is implemented here.
Completion covers the submitted command buffer, not future calls from other threads.
