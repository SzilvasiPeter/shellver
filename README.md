# shellver

![coverage](https://img.shields.io/endpoint?url=https://szilvasipeter.github.io/shellver/coverage/badge.json)

Detect the current shell and its version on Linux by traversing `/proc`.

## Usage

```rust
use shellver::Shell;

fn main() -> std::io::Result<()> {
    let shell = Shell::detect()?;
    let name = shell.name();
    let version = shell.version().unwrap_or_else(|| "unknown".to_string());
    println!("{name} {version}");
    Ok(())
}
```
