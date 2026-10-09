mod png;
mod renderer;
mod shaders;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = std::env::args_os()
        .nth(1)
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| "target/ex-starter.png".into());
    let pixels = renderer::render()?;
    png::write_rgba(&output, 64, 64, &pixels)?;
    println!(
        "EX Preview1 software: triangle rendered and checked. Output: {}",
        output.display()
    );
    Ok(())
}
