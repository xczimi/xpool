//! Resolving the local server's bind address from the environment.
//!
//! The dev/e2e stacks run the same `api` binary on different ports so they can
//! coexist (dev on `:3000`, the Playwright e2e stack on `:3001`). The port is a
//! dev-only knob read from `XPOOL_PORT`; it has no effect on the Lambda build,
//! which never binds a TCP listener.

/// The default local bind port when `XPOOL_PORT` is unset or unusable.
pub const DEFAULT_PORT: u16 = 3000;

/// Resolve the `127.0.0.1:<port>` bind address from an optional `XPOOL_PORT`
/// value. Falls back to [`DEFAULT_PORT`] when the value is missing, blank, or
/// not a valid non-zero port number.
pub fn listen_addr(port_env: Option<&str>) -> String {
    let port = port_env
        .and_then(|s| s.trim().parse::<u16>().ok())
        .filter(|&p| p != 0)
        .unwrap_or(DEFAULT_PORT);
    format!("127.0.0.1:{port}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_3000_when_unset() {
        assert_eq!(listen_addr(None), "127.0.0.1:3000");
    }

    #[test]
    fn uses_the_configured_port() {
        assert_eq!(listen_addr(Some("3001")), "127.0.0.1:3001");
    }

    #[test]
    fn blank_or_whitespace_falls_back_to_default() {
        assert_eq!(listen_addr(Some("")), "127.0.0.1:3000");
        assert_eq!(listen_addr(Some("   ")), "127.0.0.1:3000");
    }

    #[test]
    fn surrounding_whitespace_is_trimmed() {
        assert_eq!(listen_addr(Some("  3001 ")), "127.0.0.1:3001");
    }

    #[test]
    fn non_numeric_or_zero_falls_back_to_default() {
        assert_eq!(listen_addr(Some("not-a-port")), "127.0.0.1:3000");
        assert_eq!(listen_addr(Some("0")), "127.0.0.1:3000");
        assert_eq!(listen_addr(Some("99999")), "127.0.0.1:3000"); // > u16::MAX
    }
}
