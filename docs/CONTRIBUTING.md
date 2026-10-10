# Contributing to EX

The API is still changing. For a substantial feature, describe the intended
behavior and a workload that needs it before writing the implementation. Changes
to the shared managed/explicit model or safety guarantees need a design discussion;
the agreed direction is in [DOCUMENTATION.md](../DOCUMENTATION.md).

## Code

- Prefer safe Rust and the standard library. Add dependencies when they solve a
  specific problem. EX rendering uses its own execution backend.
- Use `snake_case` for functions and modules, and `PascalCase` for types. Make
  units, ownership and completion behavior clear in public descriptors and methods.
- Return `Result` for expected misuse. Reserve assertions for tests and internal
  invariants; avoid `unwrap` and `expect` in production code.
- The workspace denies unsafe code. Any future platform boundary that needs it
  must document its safety requirements and expose a safe interface.
- Keep features small enough to review. Add working behavior and tests together,
  rather than placeholders for objects that have no use yet.

## Tests and documentation

Exercise new behavior through the public API, including relevant failure cases.
Examples should run against the implementation; distinguish proposed designs
from available features. Update [api.md](../api.md) when exports or signatures
change, and keep the relevant feature contract and roadmap current.

Run the checks from the repository root:

```powershell
pwsh -NoProfile -File scripts/check.ps1
```

The script checks formatting, builds all targets, runs Clippy with warnings denied,
runs tests and builds Rustdoc. Rust and the shader tools must already be installed;
see the [README](../README.md) for prerequisites. Keep `Cargo.lock` tracked.

Review the diff before committing. Keep commits focused on one change and use a
short description of what changed. Commit source, tests, examples, build settings
and project documentation; keep temporary output and local experiments out of Git.

The project has not selected a license yet. License and contribution terms are
listed as pending work in the roadmap.
