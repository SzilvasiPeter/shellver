use shellver::Shell;

fn main() -> std::io::Result<()> {
    let shell = Shell::detect()?;
    let name = shell.name();
    let ver = shell.version().unwrap_or_else(|| "unknown".to_string());
    println!("{name} {ver}");
    Ok(())
}
