# Your EX application

This standalone starter uses the EX source checkout selected during creation.
Its Cargo.toml has an absolute path dependency; update that path if the SDK moves.
Rendering executes on the CPU through EX's software reference target.

```text
cargo run --locked --offline
cargo build --release --locked --offline
cargo fmt -- --check
```

The app writes target/ex-starter.png. Pass another output path after `--`.
Use src/shaders.rs to change the triangle colour or supported EX instructions,
and src/renderer.rs to change vertices, resources and explicit dependencies.
Shader source is currently Rust-built bounded EX IR; no external shader compiler
or window system is needed. SDK shader-language import is future work.

Rendering verifies the corner background and triangle centre before export.
Changing the triangle geometry or matching its colour to the background may
require updating those checks. Result errors describe EX misuse; PNG export
errors describe output I/O after rendering. Read the selected SDK checkout's
api.md, docs/CORE-PROFILE.md and docs/SDK.md for current contracts.

The generated Cargo.lock is for this app; keep it with your project. The stable
toolchain channel follows releases rather than pinning one compiler version.
The release build is a local executable, not a published SDK distribution.
