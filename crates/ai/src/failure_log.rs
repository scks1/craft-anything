//! Logs of failed runs, one file each, to open and inspect when something goes wrong: what ran,
//! what it printed, and what we made of it. They stay on this computer, readable only by its
//! owner. The person's message is never written, and secret-looking flags are masked
//! ([`craft_anything_platform::process::printable_command`]). Agent apps' runs write them, and so
//! do API backends' turns ([`write_api_failure`], without the request, the key or the service's
//! message) and a tool server that can't start (`craft-anything-cli mcp`).

use craft_anything_evidence::{ErrorCode, ToolError};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

/// Logs kept; the oldest go first.
const KEEP: usize = 50;
/// The most of each output stream kept in a log.
const MAX_STREAM_BYTES: usize = 256 * 1024;

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// The folder for failure logs inside a config folder.
pub fn dir_in(config_dir: &Path) -> PathBuf {
    config_dir.join("logs")
}

/// Everything known about a failed run.
#[derive(Debug, Clone, Default)]
pub struct Report {
    /// What failed, in words.
    pub what: String,
    /// What ran, with the person's message left out.
    pub command: String,
    /// A command that tests the program without a message.
    pub check: String,
    /// What we made of it.
    pub diagnosis: Option<String>,
    /// The error, when the program didn't run to an exit code.
    pub error: Option<String>,
    pub cwd: Option<PathBuf>,
    pub exit_code: Option<i32>,
    pub duration: Option<Duration>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

fn stream(name: &str, bytes: &[u8]) -> String {
    if bytes.is_empty() {
        return format!("--- {name}: empty ---\n");
    }
    let start = bytes.len().saturating_sub(MAX_STREAM_BYTES);
    let kept = String::from_utf8_lossy(bytes.get(start..).unwrap_or_default());
    let note = if start > 0 { format!(" (the last {} of {} bytes)", bytes.len() - start, bytes.len()) } else { String::new() };
    format!("--- {name}{note} ---\n{kept}\n")
}

fn render(r: &Report, date: &str, time: &str) -> String {
    let mut s = format!("Craft Anything {} failure log\nWhen: {date} {time} UTC\nWhat: {}\n", env!("CARGO_PKG_VERSION"), r.what);
    if let Some(d) = &r.diagnosis {
        s.push_str(&format!("Diagnosis: {d}\n"));
    }
    if let Some(e) = &r.error {
        s.push_str(&format!("Error: {e}\n"));
    }
    if let Some(c) = r.exit_code {
        s.push_str(&format!("Exit code: {c}\n"));
    }
    if let Some(d) = r.duration {
        s.push_str(&format!("Ran for: {:.1} s\n", d.as_secs_f64()));
    }
    if let Some(c) = &r.cwd {
        s.push_str(&format!("Working folder: {}\n", c.display()));
    }
    s.push_str(&format!("\nTo test the program:\n{}\n\nWhat ran (the message is left out):\n{}\n\n", r.check, r.command));
    s.push_str(&stream("stderr", &r.stderr));
    s.push('\n');
    s.push_str(&stream("stdout", &r.stdout));
    s
}

/// Write a log into `dir`, creating it, and return its path. Removes the oldest beyond [`KEEP`].
pub fn write(dir: &Path, report: &Report) -> Result<PathBuf, ToolError> {
    std::fs::create_dir_all(dir).map_err(|e| ToolError::new(ErrorCode::FileError, format!("Couldn't create the log folder {}: {e}", dir.display())))?;
    let (_, date, time) = craft_anything_engine::journal::now_utc();
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let name = format!("agent-{date}-{}-{}-{n}.log", time.replace(':', ""), std::process::id());
    let path = dir.join(name);
    craft_anything_platform::write_atomic_private(&path, render(report, &date, &time).as_bytes())?;
    prune(dir, KEEP);
    Ok(path)
}

/// Write the log for an API backend's turn that failed for good: the backend and model, the
/// address the request went to (without a user name, password or query), the error as
/// [`api_error_summary`] gives it, the advice shown with it, and how long the request ran. The
/// request itself, with the person's message and the key, is not written, and neither is the
/// service's message, which can repeat them. Returns the log's path, or what to tell the person
/// when it couldn't be written.
pub fn write_api_failure(
    dir: &Path,
    backend: &str,
    model: &str,
    endpoint: Option<&str>,
    error: &ToolError,
    ran_for: Duration,
) -> (Option<PathBuf>, Option<String>) {
    let report = Report {
        what: format!("{backend} ({model})"),
        command: match endpoint {
            Some(url) => format!("A request to {} for {model}", address(url)),
            None => format!("A request for {model}"),
        },
        check: format!("craft-anything-cli ai test {}", crate::ssh::shell_quote(backend)),
        diagnosis: error.remediation.clone(),
        error: Some(api_error_summary(error)),
        duration: Some(ran_for),
        ..Report::default()
    };
    match write(dir, &report) {
        Ok(path) => {
            log::warn!("{backend} failed; details in {}", path.display());
            (Some(path), None)
        }
        Err(e) => {
            log::warn!("couldn't write a failure log: {e}");
            (None, Some(format!("The details couldn't be saved: {e}")))
        }
    }
}

/// What a log keeps of an API error: Craft Anything's code for it, and the fields from the
/// service that can't carry its text: the HTTP status, its error code when it is written like one,
/// the context window and request size it gave, and how long it asked to wait. Its message and any
/// other text it sent are left out, because a service or a proxy in front of it can repeat the key
/// or part of the request there. The person saw the message with the error.
fn api_error_summary(error: &ToolError) -> String {
    let code = serde_json::to_value(error.code).ok().and_then(|v| v.as_str().map(str::to_string)).unwrap_or_else(|| "unknown".into());
    let mut s = format!("{code} (the service's message is left out)");
    let field = |k: &str| error.details.as_ref().and_then(|d| d.get(k));
    let number = |k: &str| field(k).and_then(serde_json::Value::as_u64);
    if let Some(status) = number("status") {
        s.push_str(&format!("\nHTTP status: {status}"));
    }
    if let Some(p) = field("provider_error").and_then(serde_json::Value::as_str).filter(|p| looks_like_error_code(p)) {
        s.push_str(&format!("\nThe service's error code: {p}"));
    }
    if let Some(w) = number("context_window") {
        s.push_str(&format!("\nContext window: {w} tokens"));
    }
    if let Some(t) = number("prompt_tokens") {
        s.push_str(&format!("\nThe request: {t} tokens"));
    }
    if let Some(r) = number("retry_after_secs") {
        s.push_str(&format!("\nAsked to wait: {r} s"));
    }
    s
}

/// An error code as services write them (`authentication_error`, `invalid_api_key`): lower-case
/// letters and underscores, at least one underscore, at most 64 characters. Other text, digits
/// included, could be a key or words from the request.
fn looks_like_error_code(code: &str) -> bool {
    code.len() <= 64 && code.contains('_') && code.bytes().all(|b| b.is_ascii_lowercase() || b == b'_')
}

/// A URL as a log shows it: without a user name, password, query or fragment.
fn address(url: &str) -> String {
    let url = craft_anything_engine::tools::web_common::sanitize_url(url);
    url.split(['?', '#']).next().unwrap_or_default().to_string()
}

/// Failing to tidy up never fails the run; the next write tries again.
fn prune(dir: &Path, keep: usize) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    let mut logs: Vec<PathBuf> = rd
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "log") && p.file_name().is_some_and(|n| n.to_string_lossy().starts_with("agent-")))
        .collect();
    // Names start with the date and time, so they sort oldest first.
    logs.sort();
    for old in logs.iter().take(logs.len().saturating_sub(keep)) {
        if let Err(e) = std::fs::remove_file(old) {
            log::warn!("couldn't remove the old log {}: {e}", old.display());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_log_holds_what_is_known_and_is_private() {
        let dir = tempfile::tempdir().unwrap();
        let report = Report {
            what: "X".into(),
            command: "x --go".into(),
            check: "x --version".into(),
            exit_code: Some(2),
            stderr: b"boom".to_vec(),
            ..Default::default()
        };
        let path = write(&dir.path().join("logs"), &report).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        for part in ["What: X", "Exit code: 2", "x --go", "x --version", "boom", "--- stdout: empty ---"] {
            assert!(text.contains(part), "{part}: {text}");
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            assert_eq!(std::fs::metadata(&path).unwrap().permissions().mode() & 0o777, 0o600);
        }
    }

    #[test]
    fn an_api_failure_log_names_the_backend_and_address_but_not_the_request() {
        let dir = tempfile::tempdir().unwrap();
        let error = ToolError::new(ErrorCode::NoPermission, "invalid x-api-key")
            .with_remediation("The API key was rejected.")
            .with_details(serde_json::json!({"status": 401, "provider_error": "authentication_error"}));
        let url = "https://user:pw@api.example/v1/messages?key=abc";
        let (path, unsaved) = write_api_failure(&dir.path().join("logs"), "My Claude", "claude-x", Some(url), &error, Duration::from_millis(1500));
        assert!(unsaved.is_none());
        let text = std::fs::read_to_string(path.unwrap()).unwrap();
        for part in [
            "What: My Claude (claude-x)",
            "Diagnosis: The API key was rejected.",
            "Error: no_permission (the service's message is left out)",
            "HTTP status: 401",
            "The service's error code: authentication_error",
            "Ran for: 1.5 s",
            "A request to https://api.example/v1/messages for claude-x",
            "craft-anything-cli ai test 'My Claude'",
        ] {
            assert!(text.contains(part), "{part}: {text}");
        }
        for hidden in ["user:pw", "key=abc", "invalid x-api-key"] {
            assert!(!text.contains(hidden), "{hidden}: {text}");
        }
        // A log folder that can't be made: no log, and a note for the person instead.
        let file = dir.path().join("a file");
        std::fs::write(&file, "").unwrap();
        let (path, unsaved) = write_api_failure(&file, "B", "m", None, &error, Duration::ZERO);
        assert!(path.is_none());
        assert!(unsaved.is_some_and(|u| u.contains("The details couldn't be saved")));
    }

    #[test]
    fn long_output_keeps_its_end_and_says_so() {
        let mut out = vec![b'a'; MAX_STREAM_BYTES + 10];
        out.extend_from_slice(b"the end");
        let s = stream("stderr", &out);
        assert!(s.ends_with("the end\n") && s.contains("the last") && s.len() < MAX_STREAM_BYTES + 100);
    }

    #[test]
    fn old_logs_are_removed_and_other_files_left_alone() {
        let dir = tempfile::tempdir().unwrap();
        for i in 0..5 {
            std::fs::write(dir.path().join(format!("agent-2026-01-0{i}.log")), "").unwrap();
        }
        std::fs::write(dir.path().join("notes.log"), "").unwrap();
        prune(dir.path(), 2);
        let mut left: Vec<String> = std::fs::read_dir(dir.path()).unwrap().flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect();
        left.sort();
        assert_eq!(left, ["agent-2026-01-03.log", "agent-2026-01-04.log", "notes.log"]);
    }
}
