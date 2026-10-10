# expansex-core

The implementation crate for Expanse-X. Applications use the `ex-api` package,
imported as `ex`; this crate contains the resource model, validation, shader
interpreter, CPU renderer and command execution.

The software backend implements [Core Preview 1](../../docs/CORE-PROFILE.md):
owned buffers/images, recorded transfers, programmable and indexed graphics,
depth/stencil, blending, compute, reusable bindings and explicit whole-resource
dependencies. Submission is synchronous, with one queue per device.

```text
cargo run -p ex-api --example core_preview --locked --offline
cargo test -p expansex-core --locked --offline
```

See the [API catalog](../../api.md) for exported types and methods, and
[architecture](../../docs/ARCHITECTURE.md) for module responsibilities.
