# EX allocated reference buffers

Status: Phase 1 buffer storage. Names and contracts remain provisional. Buffers
allocate and copy actual bytes on the reference target; queues now execute copies
under [SUBMISSION.md](SUBMISSION.md).

## Design

A descriptor is a borrowed request, not storage. EX needs an owned resource that
retains its creating device, has a fixed byte size and permitted roles, and can
survive the metadata passed to creation. Future command buffers must be able to
retain that storage independently of the application's original handle.

`Device::create_buffer(BufferDescriptor)` validates the request, allocates
zero-initialized reference storage, copies label/usage metadata, and returns an
owned `Buffer`. Buffer clones share one allocation; a new creation always yields
a separate allocation. Private storage prevents callers from assuming that every
future backend exposes an ordinary mutable CPU slice.

CPU access is an explicit descriptor permission named `HostAccess`, independent
of device-side `BufferUsage`. Existing APIs distinguish memory placement/access
through Vulkan memory properties, Direct3D heap types, and Metal storage modes.
EX expresses only the required CPU-copy permission in this step, leaving heap
placement, mapping, residency, and device-local uploads for later contracts.


## Descriptor and creation

`BufferDescriptor<'a>` retains its borrowed label/usage fields and `u64` byte size.
It now also requires `host_access`. Existing examples explicitly use
`HostAccess::None` when they only describe future device-side operations. This is
an intentional change to the provisional struct; older struct literals must add
the field. The descriptor is still `Copy` and owns no resource memory.

Creation checks zero size, then missing usages, then whether the size fits this
process's contiguous `Vec<u8>` capacity (`usize` and `isize::MAX`). Duplicate
usages are preserved and have no extra meaning. An empty label remains valid.
Transfer usages are now checked when [recording copies](COMMANDS.md); draws and
compute remain future operations. Recording does not execute a transfer.

Byte and metadata reservations use `try_reserve_exact`. Reservation failures
become `BufferCreateError::AllocationFailed` with their standard Rust source;
invalid descriptions retain their `BufferDescriptorError` as the error source.
Successful creation returns a fixed logical size; storage never resizes. All
initial bytes are zero in the current reference contract. This is documented
initialization during creation, not a recorded queue command or GPU operation.

Small shared-handle allocations and allocating access-error labels still use
ordinary Rust allocation behavior. EX does not recover every possible OOM or
operating-system memory-pressure failure. There is no heap budget, residency
manager, or deterministic recovery from host memory exhaustion.

## Explicit CPU copies

| `HostAccess` | `buffer.write(offset, &[u8])` | `buffer.read(offset, &mut [u8])` |
| --- | --- | --- |
| `None` | Denied | Denied |
| `Write` | Allowed | Denied |
| `Read` | Denied | Allowed |
| `ReadWrite` | Allowed | Allowed |

These methods copy bytes synchronously. Readback writes into caller-provided
storage, so the caller chooses its size and EX does not allocate a readback vector
per call. They expose no mapping or persistent slice. A `TransferDestination`
usage is not required for a CPU write, and `TransferSource` does not grant a CPU
read. Recorded-copy checks independently require the appropriate transfer usages.

Checks run in this order: host permission, full byte range, storage lock. Bounds
use checked addition; overflow returns `OutOfBounds`. Empty copies are allowed
at any in-range offset including exactly the end, but still require permission
and valid storage. An offset beyond the end is invalid even for an empty copy.

Successful writes modify exactly the chosen range; successful reads fill exactly
the destination slice. Rejected accesses do not partially modify the buffer or
the caller's destination. Access errors own the resource label and can outlive
the resource. Messages identify the buffer, failed permission or range, and the
expected access mode.

All reference storage physically resides in CPU memory even with `HostAccess::None`.
The permission is an EX contract, not a claim of inaccessible RAM, a security
boundary, or proof of device-local memory placement. A future hardware backend
must implement or reject requested host-copy support without exposing its private
allocation representation.

## Lifetime, identity, and host serialization

`Buffer` holds an `Arc<BufferState>`; state owns its device handle, immutable
metadata, and a `Mutex<Vec<u8>>`. `Clone` retains storage; `is_same()` compares
allocation identity; `belongs_to()` compares the logical creating device.
`device()` returns another handle to that original device. Public metadata getters
borrow label/usage data without allowing mutation.

A buffer can outlive its original device, adapter, instance, and descriptor.
The final buffer handle drops storage and its device reference. Devices do not
own lists of buffers, so this retention chain creates no ownership cycle.

Each CPU read/write acquires the device gate before locking storage for its whole
copy, preventing races through cloned handles and interleaving inside a submission.
Whole submissions and valid CPU accesses serialize on that device. A sequence of
separate calls is not transactional; concurrent arrival order is unspecified.
This does not define future hardware barriers or asynchronous access. If an internal panic
poisons the lock, later accesses report `StoragePoisoned` rather than trusting
possibly interrupted contents. Public copies provide no user callback under the
lock and do not deliberately panic.

## Verification and remaining work

The `buffer_roundtrip` example allocates 12 bytes, uploads four at offset 4, drops
parent handles, reads back the allocation, and demonstrates a forbidden write.
Public tests cover owned metadata, zero initialization, invalid requests, every
host-access mode, usage/access independence, partial ranges, overflow, empty end
copies, shared identity/device retention, and concurrent CPU copies. Internal
tests verify final allocation/device cleanup and handling of a poisoned lock.

The impossible-capacity test rejects `u64::MAX` before attempting storage. Tests
do not exhaust system memory to induce an actual allocator OOM; reservation-error
mapping exists, but host-memory exhaustion recovery is not claimed as verified.

Recorded copies now validate TransferSource/TransferDestination, common device
ownership, ranges, and overlap, and retain referenced allocations. They do not
read storage or snapshot contents. Queue submission now executes those descriptions,
with ordering, visibility, host concurrency, and failure defined in the submission
contract. Rendering and hardware storage are still future work.
