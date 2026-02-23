#![forbid(unsafe_code)]
use shellver::Shell;

fn main() -> std::io::Result<()> {
    let shell = Shell::detect()?;
    let name = shell.name();
    let version = shell.version().unwrap_or_default();
    println!("{name} {version}");
    Ok(())
}
