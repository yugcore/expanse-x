# EX discovery and device ownership

Status: first Phase 1 step. Public names remain provisional. This contract covers
software-target discovery and logical object ownership, not execution.

## Design

Applications must select an execution target and establish which logical device
owns future resources and submissions. EX needs that distinction before buffer
allocation or command execution can validate cross-device use.

`Instance` is the discovery entry point. Today it exposes exactly one built-in
reference adapter, explicitly named as software. `Adapter` describes that target;
it is not a logical device or a claim that a hardware GPU was detected. `Device`
establishes a fresh ownership domain. `Queue` refers to that device's one default
submission endpoint; synchronous copy execution is defined in
[the submission contract](SUBMISSION.md).

The working implementation lives in `expansex-core::object`; the `ex` public crate
re-exports it. A reference driver crate will be justified when there is actual
execution code to separate. Discovery/queue objects manage CPU metadata and
identity; devices now also allocate reference buffers under
[the buffer contract](BUFFERS.md).

## Alternatives and rationale

Vulkan distinguishes physical and logical devices, describes queues during
device creation, and retrieves the created queues afterward. Its physical-device
lifetime follows the instance. EX retains the capability distinction but uses
owned Rust handles and one automatic default queue, without queue-family or
priority configuration in this step.
[Vulkan device and queue contract](https://docs.vulkan.org/spec/latest/chapters/devsandqueues.html).

Direct3D 12 creates a device for an adapter and exposes explicit command-queue
creation. Repeated Direct3D device creation for an adapter returns the existing
device; EX instead creates an independent logical ownership domain each time.
This makes cross-device checks unambiguous even when two devices select the same
software target.
[Device creation](https://learn.microsoft.com/en-us/windows/win32/api/d3d12/nf-d3d12-d3d12createdevice),
[queue creation](https://learn.microsoft.com/en-us/windows/win32/api/d3d12/nf-d3d12-id3d12device-createcommandqueue).

Metal exposes command-queue creation on `MTLDevice`. EX's current single queue
requires no configurable allocation call. Additional queue creation belongs with
multi-queue ordering and synchronization contracts, not an unused constructor.
[Metal command queues](https://developer.apple.com/documentation/Metal/MTLCommandQueue).

EX chooses shared Rust ownership to keep children usable after parent handles are
dropped. This is a small reference-runtime contract; future driver integration
must retain any parent context that its platform requires.

## Current public operations

| Operation | Contract |
| --- | --- |
| `Instance::new()` / `default()` | Construct discovery with the built-in reference adapter |
| `instance.adapters()` | Borrow the discovered adapter slice; exactly one entry today |
| `instance.default_adapter()` | Return an owned handle to that reference adapter |
| `adapter.info()` | Borrow immutable name/backend metadata |
| `adapter.core_profile()` | Inspect the selected target's supported core contract |
| `adapter.create_device()` | Construct a new logical device, distinct from every prior creation |
| `device.adapter()` | Return an owned handle to its selected adapter |
| `device.core_profile()` | Inspect the selected executor's supported core contract |
| `device.default_queue()` | Return a handle to the same default endpoint on every call |
| `device.is_same(&other)` | Compare logical device identity, not adapter metadata |
| `queue.device()` | Return a handle to the original owning device, without creating a new device |
| `queue.belongs_to(&device)` | Test the queue's ownership domain |

`AdapterInfo` reports a name and `BackendKind::Reference`, whose documentation
explicitly identifies CPU software execution as the target. It advertises no
rendering capability, device limit, hardware vendor, or external API dependency.
Capabilities and numeric limits are exposed separately through CoreProfile and
CoreLimits under [CORE-PROFILE.md](CORE-PROFILE.md).
`BackendKind` is non-exhaustive to permit future supported targets; none are
implemented or listed as pretend hardware variants.

## Ownership, identity, and cleanup

Adapters share immutable metadata through `Arc`. A private device-state
allocation owns an adapter handle, a private selected executor, and the shared
execution gate. Device and queue handles each hold an `Arc`
to that same state. `Clone` adds shared ownership; it does not create a new
logical device, default queue, context, or execution thread.

Identity uses private `Arc::ptr_eq` checks. Pointers are never exposed as public
identifiers. There is no global ID counter, overflow path, or structural equality
that confuses two devices on the same adapter. Comparison is valid while both
handles are live; it is not a serializable or persistent resource ID.

The instance can be dropped before an owned adapter; the adapter handle can be
dropped before a device; the device handle can be dropped before a queue. The
remaining handles retain needed metadata/state. The final device/queue handle
releases device state; metadata is released after its last owner disappears.
No ownership cycle or manual destruction API exists. The borrowed adapter slice
and metadata references still obey ordinary Rust borrowing rules.

All queue handles for one device represent its single endpoint and therefore
share the device state. Future multiple queues will need distinct queue state
without ownership cycles. `Arc` alone provides ownership; the shared device
execution mutex establishes current submission and CPU-access serialization.
No hardware fence or general resource-hazard guarantee follows from ownership alone.

## Creation failures and scope

Discovery/device constructors are infallible: they only construct fixed reference
metadata and shared logical state. Ordinary Rust allocation failure is not
converted into a recoverable EX error. There are no unsupported feature requests,
driver calls, discovery failures, or device-loss paths to fabricate errors for.
Hardware discovery and device configuration may introduce fallible operations
later; this provisional API is not frozen.

Buffers and explicit CPU copies now exist under the separate buffer contract;
buffer creation is fallible and retains this device ownership domain. Synchronous
reference copy and fixed-function triangle execution now exist. `CommandRecorder`
remains the independent label-only prototype, returning `DebugMarkerBuffer`.
Device-bound copy recording now exists under the [command contract](COMMANDS.md),
with usage/range/device validation and retained resources. The queue executes
finished copies under the separate submission/completion contract.

## Verification

Public integration tests check reference discovery, unique device creation on
one adapter and across instances, shared identity after cloning/repeated queue
retrieval, cross-device ownership distinction, and children surviving parent-handle
drops. An internal lifecycle test observes that the final queue/device owner
releases device state and then its adapter metadata. A runnable `discover` example
shows the creation chain and ownership check without claiming submitted work.
