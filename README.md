# shellver

[![coverage](https://img.shields.io/endpoint?url=https://szilvasipeter.github.io/shellver/coverage/badge.json&format=svg)](https://szilvasipeter.github.io/shellver/coverage/index.html)
[![crates](https://img.shields.io/crates/v/shellver)](https://crates.io/crates/shellver)
[![unsafe forbidden](https://img.shields.io/badge/unsafe-forbidden-success.svg)](https://github.com/SzilvasiPeter/shellver/blob/main/Cargo.toml#L19)

Detect the current shell and its version on Linux by traversing `/proc`.

## Features

- Linux-only detection via `/proc`
- Library API and CLI binary
- Best-effort version parsing for common shells

## Installation

Library:

```bash
cargo add shellver
```

Binary:

```bash
cargo install shellver
```

CLI:

```bash
shellver
```

Example output:

```text
bash 5.3.9
```

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

## Supported Shells

The supported shell list is exposed via `Shell::supported_shells()`, see the [SHELLS](https://docs.rs/shellver/1.0.1/src/shellver/lib.rs.html#11-14) in src.

## Errors

`Shell::detect()` returns an `io::Result` and may fail if the process chain
cannot be read or if no supported shell is found within the hop limit.

## Platform

Linux only. This crate relies on `/proc` to traverse parent processes.

## License

MIT
