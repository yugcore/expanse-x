//! Maintained integrated Core Preview proof; the test runs the same workload.
#[path = "triangle/png.rs"]
mod png;
#[path = "core_preview/workload.rs"]
mod workload;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = std::env::args_os()
        .nth(1)
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| "target/core-preview.png".into());
    let proof = workload::run(&ex::Instance::new().default_adapter())?;
    png::write_rgba(&output, proof.width, proof.height, &proof.pixels)?;
    println!("Core Preview 1: verified two frames, preflight rejection, late compute rollback and repaired reuse.");
    println!("{} commands, {} resource uses; all {} final pixels, offscreen pixels, attachments and padding verified.",
        proof.command_count, proof.resource_use_count, proof.pixels.len());
    println!("Verified output: {}", output.display());
    Ok(())
}
