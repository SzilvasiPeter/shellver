#![forbid(unsafe_code)]
use regex::Regex;
use std::fs;
use std::io;
use std::process::Command;

const SHELLS: [&str; 9] = [
    "bash", "sh", "dash", "zsh", "fish", "ksh", "mksh", "tcsh", "csh",
];

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
        let run_cmd = |name: &str| -> io::Result<Vec<u8>> {
            Ok(Command::new(name).arg("--version").output()?.stdout)
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
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn version(&self) -> Option<String> {
        self.version.clone()
    }
}

type ReadFn = fn(&str) -> io::Result<String>;
type RunFn = fn(&str) -> io::Result<Vec<u8>>;

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
    let out = run(name)?;
    let text = String::from_utf8(out)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "non utf8 bytes"))?;
    let re = Regex::new(r"[0-9]+\.[0-9]+(?:\.[0-9]+)?").unwrap();
    Ok(re.find(&text).map(|m| m.as_str().to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run_mock(name: &str) -> io::Result<Vec<u8>> {
        if name.is_empty() {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "name empty"));
        }
        if name == "bad_utf" {
            return Ok(vec![0xff, 0xfe]);
        }
        Ok(name.as_bytes().to_vec())
    }

    #[expect(clippy::unnecessary_wraps, reason = "Needs for mocking")]
    fn read_mock(text: &str) -> io::Result<String> {
        Ok(text.to_string())
    }

    fn read_mock_err(_path: &str) -> io::Result<String> {
        Err(io::Error::new(io::ErrorKind::PermissionDenied, "deny"))
    }

    #[test]
    fn shell_from_pid_returns_some() {
        let val = shell_from_pid_with("bash\n", read_mock).unwrap();
        assert_eq!(val, Some("bash"));
    }

    #[test]
    fn shell_from_pid_returns_none() {
        let val = shell_from_pid_with("unknown\n", read_mock).unwrap();
        assert_eq!(val, None);
    }

    #[test]
    fn ppid_from_path_parse_ok() {
        let val = ppid_from_path_with("Name:\tbash\nPPid:\t123\n", read_mock).unwrap();
        assert_eq!(val, 123);
    }

    #[test]
    fn ppid_from_path_missing() {
        let err = ppid_from_path_with("Name:\tbash\n", read_mock).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
    }

    #[test]
    fn ppid_from_path_parse_error() {
        let err = ppid_from_path_with("Name:\tbash\nPPid:\tbad\n", read_mock).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn ppid_from_path_read_error() {
        let err = ppid_from_path_with("/proc/1/status", read_mock_err).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::PermissionDenied);
    }

    #[test]
    fn shell_version_on_invalid_command() {
        let err = shell_version_with("", run_mock).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
    }

    #[test]
    fn shell_version_on_invalid_input() {
        let err = shell_version_with("bad_utf", run_mock).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn shell_version_returns_none() {
        let val = shell_version_with("no version here", run_mock).unwrap();
        assert_eq!(val, None);
    }

    #[test]
    fn shell_version_returns_some() {
        let val = shell_version_with("bash 5.2.0", run_mock).unwrap();
        assert_eq!(val, Some("5.2.0".to_string()));
    }

    fn read_detect_ok(path: &str) -> io::Result<String> {
        match path {
            "/proc/self/status" => Ok("PPid:\t100\n".to_string()),
            "/proc/100/comm" => Ok("bash\n".to_string()),
            _ => Err(io::Error::new(io::ErrorKind::InvalidInput, "bad path")),
        }
    }

    fn read_detect_not_found(path: &str) -> io::Result<String> {
        match path {
            "/proc/self/status" => Ok("PPid:\t100\n".to_string()),
            "/proc/100/comm" => Ok("unknown\n".to_string()),
            "/proc/100/status" => Ok("PPid:\t1\n".to_string()),
            _ => Err(io::Error::new(io::ErrorKind::InvalidInput, "bad path")),
        }
    }

    fn read_detect_err(path: &str) -> io::Result<String> {
        match path {
            "/proc/self/status" => Err(io::Error::new(io::ErrorKind::PermissionDenied, "deny")),
            _ => Err(io::Error::new(io::ErrorKind::InvalidInput, "bad path")),
        }
    }

    fn read_detect_run_err(path: &str) -> io::Result<String> {
        match path {
            "/proc/self/status" => Ok("PPid:\t100\n".to_string()),
            "/proc/100/comm" => Ok("bash\n".to_string()),
            _ => Err(io::Error::new(io::ErrorKind::InvalidInput, "bad path")),
        }
    }

    #[expect(clippy::unnecessary_wraps, reason = "Needs for mocking")]
    fn run_detect_ok(_name: &str) -> io::Result<Vec<u8>> {
        Ok(b"bash 5.2.0".to_vec())
    }

    fn run_detect_err(_name: &str) -> io::Result<Vec<u8>> {
        Err(io::Error::new(io::ErrorKind::InvalidInput, "bad cmd"))
    }

    #[test]
    fn detect_with_ok() {
        let shell = Shell::detect_with(read_detect_ok, run_detect_ok).unwrap();
        assert_eq!(shell.name(), "bash");
        assert_eq!(shell.version(), Some("5.2.0".to_string()));
    }

    #[test]
    fn detect_with_not_found() {
        let err = Shell::detect_with(read_detect_not_found, run_detect_ok).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
    }

    #[test]
    fn detect_with_read_error() {
        let err = Shell::detect_with(read_detect_err, run_detect_ok).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::PermissionDenied);
    }

    #[test]
    fn detect_with_run_error() {
        let err = Shell::detect_with(read_detect_run_err, run_detect_err).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
    }
}
