#![forbid(unsafe_code)]
//! Detect the current shell and its version on Linux by traversing `/proc`.
//!
//! The primary entry point is [`Shell::detect`], which walks the parent process
//! chain to find a known shell and optionally extracts its version.
use regex::Regex;
use std::fs;
use std::io;
use std::process::Command;

const SHELLS: [&str; 13] = [
    "bash", "zsh", "sh", "tcsh", "csh", "ksh", "mksh", "fish", "dash", "nu", "elvish", "xonsh",
    "pwsh",
];
const SEMVER_PATTERN: &str = r"[0-9]+\.[0-9]+(?:\.[0-9]+)?";
const MKSH_PATTERN: &str = r"R[0-9]+";
const ARGS_VERSION: &[&str] = &["--version"];
const ARGS_MKSH: &[&str] = &["-c", "printf %s \"$KSH_VERSION\""];

/// Information about the detected shell.
#[derive(Debug)]
pub struct Shell {
    name: String,
    version: Option<String>,
}

impl Shell {
    /// # Errors
    ///
    /// Returns an error if the parent process chain cannot be read or if no
    /// known shell is found within the hop limit.
    pub fn detect() -> io::Result<Self> {
        let read_file = |path: &str| -> io::Result<String> { fs::read_to_string(path) };
        let run_cmd = |name: &str, args: &[&str]| -> io::Result<Vec<u8>> {
            Ok(Command::new(name).args(args).output()?.stdout)
        };
        Self::detect_with(read_file, run_cmd)
    }

    fn detect_with(read: ReadFn, run: RunFn) -> io::Result<Self> {
        let mut pid = ppid_from_path_with("/proc/self/status", read)?;
        let mut hops: u32 = 0;
        while pid > 1 && hops < 32 {
            let path = format!("/proc/{pid}/comm");
            if let Some(name) = shell_from_pid_with(&path, read)? {
                let version = shell_version_with(name, run)?;
                let name = name.to_string();
                return Ok(Self { name, version });
            }

            let path = format!("/proc/{pid}/status");
            pid = ppid_from_path_with(&path, read)?;
            hops += 1;
        }
        Err(io::Error::new(io::ErrorKind::NotFound, "shell not found"))
    }

    #[must_use]
    /// Returns the detected shell name.
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    /// Returns the detected shell version, if available.
    pub fn version(&self) -> Option<String> {
        self.version.clone()
    }

    /// Returns the list of supported shell names.
    #[must_use]
    pub const fn supported_shells() -> &'static [&'static str] {
        &SHELLS
    }
}

type ReadFn = fn(&str) -> io::Result<String>;
type RunFn = fn(&str, &[&str]) -> io::Result<Vec<u8>>;

fn ppid_from_path_with(path: &str, read: ReadFn) -> io::Result<u32> {
    let text = read(path)?;
    ppid_from_text(&text)
}

fn ppid_from_text(text: &str) -> io::Result<u32> {
    for line in text.lines() {
        if let Some(ppid) = line.strip_prefix("PPid:") {
            let val = ppid
                .trim()
                .parse::<u32>()
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "PPid parse failed"))?;
            return Ok(val);
        }
    }
    Err(io::Error::new(io::ErrorKind::NotFound, "PPid not found"))
}

fn shell_from_pid_with(path: &str, read: ReadFn) -> io::Result<Option<&'static str>> {
    let text = read(path)?;
    let shell = SHELLS.iter().copied().find(|sh| text.trim_end() == *sh);
    Ok(shell)
}

fn shell_version_with(name: &str, run: RunFn) -> io::Result<Option<String>> {
    let Some(args) = shell_args(name) else {
        return Ok(None);
    };
    let out = run(name, args)?;
    let text = String::from_utf8(out)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "non utf8 bytes"))?;
    let re = Regex::new(version_pattern(name)).unwrap();
    Ok(re.find(&text).map(|m| m.as_str().to_string()))
}

fn shell_args(name: &str) -> Option<&'static [&'static str]> {
    match name {
        // Dash doesn't have version option or any other argument to get its version.
        // One way to retrieve the version is using the system package manager.
        "dash" => None,
        "mksh" => Some(ARGS_MKSH),
        _ => Some(ARGS_VERSION),
    }
}

fn version_pattern(name: &str) -> &'static str {
    if name == "mksh" {
        MKSH_PATTERN
    } else {
        SEMVER_PATTERN
    }
}

#[cfg(test)]
mod lib_tests;
