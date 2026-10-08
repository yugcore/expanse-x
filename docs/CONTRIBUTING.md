# EX coding conventions

- Follow the [project design](../DOCUMENTATION.md). Routine API and
  implementation choices can evolve within it. Changes to guiding commitments
  require an explicit design revision recorded in that document.

- Keep production code, examples, tests and EX contracts tracked.
  Keep temporary output and local experiments out of Git.

- Use Rust and the standard library where practical. Introduce dependencies for
  a demonstrated requirement. No rendering dependency on another graphics API.

- Keep API changes small and functional. Do not add empty modules for future
  objects. Preserve existing working contracts until a justified change replaces
  them; clearly distinguish implemented behavior from plans in documentation.

- Use snake_case for functions/modules, PascalCase for types, and explicit units
  in fields and documentation. Public descriptors should describe what they
  borrow; real handles must document storage ownership and completion rules.

- Prefer safe code, enums, ownership, RAII, and `Result`. Expected misuse belongs
  in descriptive errors rather than panics. Production source must avoid
  convenience `unwrap`/`expect`; assertions are appropriate in tests.

- The workspace denies unsafe code by default. If a genuine platform boundary
  needs it, isolate the code, locally allow the lint only at that boundary,
  document safety invariants, and expose a safe interface. Existing code is safe.

- Document a major abstraction's capability, need, comparison, chosen contract,
  and limitations before implementing it. Unit-test meaningful behavior and
  misuse; use public API examples/integration checks across subsystem boundaries.
  Documentation examples must agree with executable behavior.

- Review the staged diff, stage explicit paths, and make a local commit for each
  coherent change. Use short messages such as `get command recording working`.
  Preserve history; publishing or hardware support is not implied by a commit.
