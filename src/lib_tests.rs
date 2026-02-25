#[expect(clippy::unnecessary_wraps, reason = "Needs for mocking I/O operations")]
mod tests {
    use crate::*;

    #[test]
    fn supported_shells_size() {
        let shells = Shell::supported_shells();
        assert_eq!(shells.len(), 13);
        assert!(["bash", "zsh", "fish"].iter().all(|s| shells.contains(s)));
    }

    fn read_mock(text: &str) -> io::Result<String> {
        Ok(text.to_string())
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
        fn read_mock_err(_path: &str) -> io::Result<String> {
            Err(io::Error::new(io::ErrorKind::PermissionDenied, "deny"))
        }

        let err = ppid_from_path_with("/proc/1/status", read_mock_err).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::PermissionDenied);
    }

    fn run_mock(name: &str, _args: &[&str]) -> io::Result<Vec<u8>> {
        if name.is_empty() {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "name empty"));
        }
        if name == "bad_utf" {
            return Ok(vec![0xff, 0xfe]);
        }
        Ok(name.as_bytes().to_vec())
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
    fn shell_version_returns_none_dash() {
        fn run_never(_name: &str, _args: &[&str]) -> io::Result<Vec<u8>> {
            unreachable!("should not be reachable");
        }

        let val = shell_version_with("dash", run_never).unwrap();
        assert_eq!(val, None);
    }

    #[test]
    fn shell_version_returns_some_bash() {
        let val = shell_version_with("GNU bash, version 5.3.9(1)-release", run_mock).unwrap();
        assert_eq!(val, Some("5.3.9".to_string()));
    }

    #[test]
    fn shell_version_returns_some_ksh() {
        let val = shell_version_with("sh (AT&T Research) 2020.0.0", run_mock).unwrap();
        assert_eq!(val, Some("2020.0.0".to_string()));
    }

    #[test]
    fn shell_version_returns_some_elvish() {
        let val = shell_version_with("0.21.0+archlinux1", run_mock).unwrap();
        assert_eq!(val, Some("0.21.0".to_string()));
    }

    #[test]
    fn shell_version_returns_some_mksh() {
        fn run_mksh(_name: &str, _args: &[&str]) -> io::Result<Vec<u8>> {
            Ok(b"@(#)MIRBSD KSH R59 2020/10/31".to_vec())
        }

        let val = shell_version_with("mksh", run_mksh).unwrap();
        assert_eq!(val, Some("R59".to_string()));
    }

    fn read_detect_run_err(path: &str) -> io::Result<String> {
        match path {
            "/proc/self/status" => Ok("PPid:\t100\n".to_string()),
            "/proc/100/comm" => Ok("bash\n".to_string()),
            _ => unreachable!("bad path"),
        }
    }

    fn run_detect_ok(_name: &str, _args: &[&str]) -> io::Result<Vec<u8>> {
        Ok(b"bash 5.2.0".to_vec())
    }

    #[test]
    fn detect_with_ok() {
        fn read_detect_ok(path: &str) -> io::Result<String> {
            match path {
                "/proc/self/status" => Ok("PPid:\t100\n".to_string()),
                "/proc/100/comm" => Ok("bash\n".to_string()),
                _ => unreachable!("bad path"),
            }
        }

        let shell = Shell::detect_with(read_detect_ok, run_detect_ok).unwrap();
        assert_eq!(shell.name(), "bash");
        assert_eq!(shell.version(), Some("5.2.0".to_string()));
    }

    #[test]
    fn detect_with_not_found() {
        fn read_detect_not_found(path: &str) -> io::Result<String> {
            match path {
                "/proc/self/status" => Ok("PPid:\t100\n".to_string()),
                "/proc/100/comm" => Ok("unknown\n".to_string()),
                "/proc/100/status" => Ok("PPid:\t1\n".to_string()),
                _ => unreachable!("bad path"),
            }
        }

        let err = Shell::detect_with(read_detect_not_found, run_detect_ok).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
    }

    #[test]
    fn detect_with_read_error() {
        fn read_detect_err(path: &str) -> io::Result<String> {
            match path {
                "/proc/self/status" => Err(io::Error::new(io::ErrorKind::PermissionDenied, "deny")),
                _ => unreachable!("bad path"),
            }
        }

        let err = Shell::detect_with(read_detect_err, run_detect_ok).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::PermissionDenied);
    }

    #[test]
    fn detect_with_comm_read_error() {
        fn read_detect_err(path: &str) -> io::Result<String> {
            match path {
                "/proc/self/status" => Ok("PPid:\t100\n".to_string()),
                "/proc/100/comm" => Err(io::Error::new(io::ErrorKind::PermissionDenied, "deny")),
                _ => unreachable!("bad path"),
            }
        }

        let err = Shell::detect_with(read_detect_err, run_detect_ok).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::PermissionDenied);
    }

    #[test]
    fn detect_with_status_read_error() {
        fn read_detect_err(path: &str) -> io::Result<String> {
            match path {
                "/proc/self/status" => Ok("PPid:\t100\n".to_string()),
                "/proc/100/comm" => Ok("unknown\n".to_string()),
                "/proc/100/status" => Err(io::Error::new(io::ErrorKind::PermissionDenied, "deny")),
                _ => unreachable!("bad path"),
            }
        }

        let err = Shell::detect_with(read_detect_err, run_detect_ok).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::PermissionDenied);
    }

    #[test]
    fn detect_with_run_error() {
        fn run_detect_err(_name: &str, _args: &[&str]) -> io::Result<Vec<u8>> {
            Err(io::Error::new(io::ErrorKind::InvalidInput, "bad cmd"))
        }

        let err = Shell::detect_with(read_detect_run_err, run_detect_err).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
    }
}
