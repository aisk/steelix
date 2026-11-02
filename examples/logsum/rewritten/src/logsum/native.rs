//! The parts of logsum that are plain Rust now. Nothing in here knows about
//! Python; `logsum.rs` converts between these types and Python objects.

/// One request from the access log, borrowing from the line it was read from.
#[derive(Debug, PartialEq)]
pub struct Request<'a> {
    pub ip: &'a str,
    pub hour: &'a str,
    pub method: &'a str,
    pub path: &'a str,
    pub status: u16,
    pub bytes: u64,
    pub ms: Option<u64>,
}

/// Splits one log line into a request, or returns `None` if it is malformed.
pub fn parse_line(line: &str) -> Option<Request<'_>> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 10 || !parts[3].starts_with('[') {
        return None;
    }
    let status = parts[8];
    if status.len() != 3 || !status.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let bytes = match parts[9] {
        "-" => 0,
        size => size.parse().ok()?,
    };
    let ms = match parts.get(10).and_then(|time| time.strip_suffix("ms")) {
        Some(ms) => Some(ms.parse().ok()?),
        None => None,
    };
    let (path, _query) = parts[6].split_once('?').unwrap_or((parts[6], ""));
    Some(Request {
        ip: parts[0],
        hour: parts[3].get(13..15).unwrap_or_default(),
        method: parts[5].trim_start_matches('"'),
        path,
        status: status.parse().ok()?,
        bytes,
        ms,
    })
}

/// Nearest-rank percentile of an ascending slice.
pub fn percentile(sorted_values: &[u64], pct: usize) -> u64 {
    if sorted_values.is_empty() {
        return 0;
    }
    let rank = (pct * sorted_values.len()).div_ceil(100).max(1);
    sorted_values[rank - 1]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_request() {
        let line = r#"203.0.113.9 - - [12/Mar/2024:10:15:32 +0000] "GET /api/users?page=2 HTTP/1.1" 200 512 41ms"#;
        let request = parse_line(line).unwrap();
        assert_eq!(request.hour, "10");
        assert_eq!(request.method, "GET");
        assert_eq!(request.path, "/api/users");
        assert_eq!(
            (request.status, request.bytes, request.ms),
            (200, 512, Some(41))
        );
    }

    #[test]
    fn rejects_malformed_lines() {
        assert_eq!(parse_line("### log rotated ###"), None);
        let bad_status =
            r#"192.0.2.44 - - [12/Mar/2024:11:02:11 +0000] "GET / HTTP/1.1" 2OO 512 12ms"#;
        assert_eq!(parse_line(bad_status), None);
    }

    #[test]
    fn percentile_uses_the_nearest_rank() {
        let values = [10, 20, 30, 40];
        assert_eq!(percentile(&values, 50), 20);
        assert_eq!(percentile(&values, 99), 40);
        assert_eq!(percentile(&[], 50), 0);
    }
}
