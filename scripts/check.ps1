# Production checks, usable locally and from a PowerShell CI runner.
$ErrorActionPreference = 'Stop'
Push-Location (Split-Path -Parent $PSScriptRoot)
try {
    & cargo fmt --all -- --check
    if ($LASTEXITCODE -ne 0) { throw 'Rust formatting check failed.' }

    & cargo build --workspace --all-targets --locked --offline
    if ($LASTEXITCODE -ne 0) { throw 'Production build failed.' }

    & cargo clippy --workspace --all-targets --locked --offline -- -D warnings
    if ($LASTEXITCODE -ne 0) { throw 'Rust lint check failed.' }

    & cargo test --workspace --locked --offline
    if ($LASTEXITCODE -ne 0) { throw 'Production tests failed.' }

    & cargo doc --workspace --no-deps --locked --offline
    if ($LASTEXITCODE -ne 0) { throw 'API documentation build failed.' }
} finally {
    Pop-Location
}
