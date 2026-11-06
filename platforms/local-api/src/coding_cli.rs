// SPDX-License-Identifier: Apache-2.0
//! Thin terminal client of the versioned local coding API. No command runner.

use std::collections::BTreeSet;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, IsTerminal, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::Path;
use std::time::{Duration, Instant};

use crate::{ApiError, ApiTokens};
use agentmage_platform_linux::LinuxCodingReadScope;
use serde_json::{Value, json};
use zeroize::Zeroizing;

const MAX_REPLY: usize = 4 * 1024 * 1024;
const MAX_REQUEST: usize = 256 * 1024;
const DEADLINE: Duration = Duration::from_secs(180);

type Result<T> = std::result::Result<T, ()>;

/// Runs the terminal coding client over fixed version-1 API routes. Consequential
/// approvals require the controlling terminal; no piped or automatic approval.
/// Returned errors are content-free. Bounded server refusals are escaped on stderr.
pub fn run(args: &[String]) -> std::result::Result<(), ApiError> {
    execute(args).map_err(|()| ApiError::Configuration)
}

struct Client {
    address: SocketAddr,
    auth: std::path::PathBuf,
}

impl Client {
    fn new(address: &str, auth: &str) -> Result<Self> {
        let address = address.parse::<SocketAddr>().map_err(|_| ())?;
        if !address.ip().is_loopback() || address.port() == 0 {
            return Err(());
        }
        Ok(Self {
            address,
            auth: auth.into(),
        })
    }

    fn token(&self, control: bool) -> Result<Zeroizing<String>> {
        let tokens = ApiTokens::load(&self.auth).map_err(|_| ())?;
        Ok(if control {
            tokens.control
        } else {
            tokens.client
        })
    }

    fn call(
        &self,
        method: &str,
        path: &str,
        control: bool,
        value: Option<Value>,
        mut observe: impl FnMut(&Value) -> Result<()>,
    ) -> Result<Vec<Value>> {
        // All callers use fixed literal methods and routes. Never follow a URL,
        // redirect, proxy setting, repository instruction or model-selected route.
        let token = self.token(control)?;
        let body = Zeroizing::new(
            value
                .map(|v| serde_json::to_vec(&v))
                .transpose()
                .map_err(|_| ())?
                .unwrap_or_default(),
        );
        if body.len() > MAX_REQUEST {
            return Err(());
        }
        let mut socket =
            TcpStream::connect_timeout(&self.address, Duration::from_secs(2)).map_err(|_| ())?;
        socket
            .set_read_timeout(Some(Duration::from_millis(100)))
            .map_err(|_| ())?;
        socket
            .set_write_timeout(Some(Duration::from_secs(2)))
            .map_err(|_| ())?;
        let header = Zeroizing::new(format!(
            "{method} {path} HTTP/1.1\r\nHost: {}\r\nAuthorization: Bearer {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            self.address,
            &**token,
            body.len()
        ));
        socket
            .write_all(header.as_bytes())
            .and_then(|()| socket.write_all(&body))
            .map_err(|_| ())?;
        receive(socket, &mut observe)
    }

    fn json(&self, method: &str, path: &str, control: bool, body: Option<Value>) -> Result<Value> {
        let mut values = self.call(method, path, control, body, |_| Ok(()))?;
        if values.len() != 1 {
            return Err(());
        }
        Ok(values.remove(0))
    }

    fn idle(&self) -> Result<()> {
        let start = Instant::now();
        loop {
            let value = self.json("GET", "/v1/status", false, None)?;
            match value["busy"].as_bool() {
                Some(false) => return Ok(()),
                Some(true) if start.elapsed() < Duration::from_secs(2) => {
                    std::thread::sleep(Duration::from_millis(10))
                }
                _ => return Err(()),
            }
        }
    }
}

fn receive(
    mut socket: TcpStream,
    observe: &mut impl FnMut(&Value) -> Result<()>,
) -> Result<Vec<Value>> {
    let deadline = Instant::now() + DEADLINE;
    let mut header = Zeroizing::new(Vec::new());
    while !header.ends_with(b"\r\n\r\n") {
        if header.len() == 8192 {
            return Err(());
        }
        let mut byte = [0];
        if read_until(&mut socket, &mut byte, deadline)? != 1 {
            return Err(());
        }
        header.push(byte[0]);
    }
    let header = std::str::from_utf8(&header).map_err(|_| ())?;
    let mut lines = header[..header.len() - 4].split("\r\n");
    let mut status = lines.next().ok_or(())?.splitn(3, ' ');
    if status.next() != Some("HTTP/1.1") {
        return Err(());
    }
    let code = status.next().ok_or(())?;
    if code.len() != 3 || !code.bytes().all(|b| b.is_ascii_digit()) {
        return Err(());
    }
    let code = code.parse::<u16>().map_err(|_| ())?;
    if code != 200 && !(400..=599).contains(&code) {
        return Err(());
    }
    let mut seen = BTreeSet::new();
    let mut media = None;
    let mut length = None;
    for line in lines {
        let (name, value) = line.split_once(':').ok_or(())?;
        if name.is_empty()
            || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
            || !value.bytes().all(|b| (32..=126).contains(&b))
        {
            return Err(());
        }
        let name = name.to_ascii_lowercase();
        if !seen.insert(name.clone()) {
            return Err(());
        }
        let value = value.trim();
        match name.as_str() {
            "content-type" => media = Some(value),
            "content-length" => {
                if value.is_empty() || !value.bytes().all(|b| b.is_ascii_digit()) {
                    return Err(());
                }
                length = Some(value.parse::<usize>().map_err(|_| ())?);
            }
            "connection" if value.eq_ignore_ascii_case("close") => (),
            "connection" | "transfer-encoding" | "content-encoding" => return Err(()),
            _ => (),
        }
    }
    let streaming = match media {
        Some("application/json") if length.is_some() => false,
        Some("application/x-ndjson") => true,
        _ => return Err(()),
    };
    let limit = if code == 200 { MAX_REPLY } else { 4096 };
    if length.is_some_and(|n| n > limit) || (code != 200 && (streaming || length.is_none())) {
        return Err(());
    }
    let mut bytes = Zeroizing::new(Vec::new());
    let mut total = 0;
    let mut values = Vec::new();
    loop {
        let mut chunk = [0; 4096];
        let n = read_until(&mut socket, &mut chunk, deadline)?;
        if n == 0 {
            break;
        }
        total += n;
        if total > limit || length.is_some_and(|len| total > len) {
            return Err(());
        }
        bytes.extend_from_slice(&chunk[..n]);
        if streaming {
            while let Some(end) = bytes.iter().position(|b| *b == b'\n') {
                if values.len() == 8192 {
                    return Err(());
                }
                let value: Value = serde_json::from_slice(&bytes[..end]).map_err(|_| ())?;
                observe(&value)?;
                values.push(value);
                bytes.drain(..=end);
            }
        }
    }
    if length.is_some_and(|len| total != len) {
        return Err(());
    }
    if streaming {
        if !bytes.is_empty() || values.is_empty() {
            return Err(());
        }
    } else {
        values.push(serde_json::from_slice(&bytes).map_err(|_| ())?);
    }
    if code != 200 {
        // Never turn a refused or uncertain operation into success or retry it.
        // Validate framing first, and render only bounded escaped JSON.
        eprintln!("API refusal ({code}): {}", safe(&values[0])?);
        return Err(());
    }
    Ok(values)
}

fn read_until(socket: &mut TcpStream, bytes: &mut [u8], deadline: Instant) -> Result<usize> {
    loop {
        if Instant::now() >= deadline {
            return Err(());
        }
        match socket.read(bytes) {
            Ok(n) => return Ok(n),
            Err(e)
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::WouldBlock
                        | std::io::ErrorKind::TimedOut
                        | std::io::ErrorKind::Interrupted
                ) => {}
            Err(_) => return Err(()),
        }
    }
}

fn digest(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

// JSON provides structure, then escaping every non-ASCII/control character
// prevents terminal escape, bidi and visually hidden approval instructions.
fn safe(value: &Value) -> Result<String> {
    use std::fmt::Write as _;
    let mut output = String::new();
    for character in serde_json::to_string_pretty(value).map_err(|_| ())?.chars() {
        if character.is_ascii() && character != '\u{7f}' {
            output.push(character);
        } else {
            let mut units = [0; 2];
            for unit in character.encode_utf16(&mut units) {
                write!(&mut output, "\\u{unit:04x}").map_err(|_| ())?;
            }
        }
    }
    Ok(output)
}

fn show(value: &Value) -> Result<()> {
    writeln!(std::io::stdout().lock(), "{}", safe(value)?).map_err(|_| ())
}

fn confirm(review: &Value, challenge: &str) -> Result<()> {
    if !std::io::stdin().is_terminal() {
        return Err(());
    }
    // Never read a piped model answer or accept an auto-approve option. The
    // complete review and the fresh challenge go to the controlling terminal.
    let mut tty = OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/tty")
        .map_err(|_| ())?;
    writeln!(
        tty,
        "{}\nType exactly {challenge} to approve; anything else declines:",
        safe(review)?
    )
    .and_then(|()| tty.flush())
    .map_err(|_| ())?;
    let mut line = String::new();
    BufReader::new((&tty).take(256))
        .read_line(&mut line)
        .map_err(|_| ())?;
    if line
        .strip_suffix('\n')
        .map(|s| s.strip_suffix('\r').unwrap_or(s))
        != Some(challenge)
    {
        return Err(());
    }
    Ok(())
}

fn selection(client: &Client) -> Result<Value> {
    let selected = client.json("GET", "/v1/code/workspace", false, None)?;
    if selected["selected"] != true
        || selected["workspace_revision"].as_u64().is_none()
        || selected["paths"].as_array().is_none()
    {
        return Err(());
    }
    Ok(selected)
}

fn propose(
    client: &Client,
    model: &str,
    message_bytes: &str,
    tokens: &str,
    task: &str,
) -> Result<()> {
    let selected = selection(client)?;
    if selected["model_context_approved"] != true {
        return Err(());
    }
    let message_bytes = message_bytes.parse::<usize>().map_err(|_| ())?;
    let tokens = tokens.parse::<u32>().map_err(|_| ())?;
    let events = client.call(
        "POST",
        "/v1/code/workspace/proposals",
        false,
        Some(json!({
            "workspace_revision":selected["workspace_revision"],"paths":selected["paths"],
            "model":model,"task":task,"max_message_bytes":message_bytes,"max_tokens":tokens
        })),
        show,
    )?;
    let context = events.first().ok_or(())?;
    let terminal = events.last().ok_or(())?;
    if context["type"] != "context"
        || context["workspace_revision"] != selected["workspace_revision"]
        || terminal["type"] != "proposal"
        || terminal["workspace_revision"] != selected["workspace_revision"]
        || terminal["applied"] != false
        || terminal["tested"] != false
    {
        return Err(());
    }
    let mut answer = Zeroizing::new(String::new());
    for event in events.iter().skip(1).take(events.len().saturating_sub(2)) {
        if event["type"] != "text" {
            return Err(());
        }
        let text = event["text"].as_str().ok_or(())?;
        if text.len() > (64 * 1024_usize).saturating_sub(answer.len()) {
            return Err(());
        }
        answer.push_str(text);
    }
    let expected = context["files"]
        .as_array()
        .ok_or(())?
        .iter()
        .map(|file| json!({"path":file["path"],"sha256":file["sha256"]}))
        .collect::<Vec<_>>();
    // Status polling handles the server's documented proposal-terminal/lease
    // release race. It never resends generation, preparation or any effect.
    client.idle()?;
    show(&client.json("POST", "/v1/code/workspace/changes", false, Some(json!({
        "workspace_revision":selected["workspace_revision"],"paths":selected["paths"],
        "expected_sources":expected,"task":task,"max_message_bytes":message_bytes,"answer":&**answer
    })))?)
}

fn apply(client: &Client, root: Option<&str>) -> Result<()> {
    let changes = client.json("GET", "/v1/code/workspace/changes", false, None)?;
    let pending = &changes["pending"];
    let preview = &pending["preview"];
    let revision = pending["workspace_revision"].as_u64().ok_or(())?;
    let change = preview["change_set_sha256"]
        .as_str()
        .filter(|s| digest(s))
        .ok_or(())?;
    let hash = preview["preview_sha256"]
        .as_str()
        .filter(|s| digest(s))
        .ok_or(())?;
    if let Some(root) = root {
        let identity = pending["workspace_root_sha256"].as_str().ok_or(())?;
        LinuxCodingReadScope::verify_root(Path::new(root), identity).map_err(|_| ())?;
        confirm(
            &json!({"selected_root":root,"pending":pending,"retention":"process-only",
            "warning":"Writes selected workspace files. Not an isolated worktree; no tests run. After a lost reply inspect files and records before any new approval."}),
            &format!("apply {hash}"),
        )?;
        LinuxCodingReadScope::verify_root(Path::new(root), identity).map_err(|_| ())?;
    }
    // One exact send. An I/O error is potentially effectful; never auto-retry.
    let result = client.json("POST", "/v1/code/workspace/apply", true, Some(json!({
        "workspace_revision":revision,"change_set_sha256":change,"preview_sha256":hash,"approve":root.is_some()
    })))?;
    show(&result)?;
    if result["outcome"]
        != if root.is_some() {
            "committed"
        } else {
            "denied"
        }
    {
        return Err(());
    }
    Ok(())
}

fn execute(args: &[String]) -> Result<()> {
    let [address, auth, command, tail @ ..] = args else {
        usage();
        return Err(());
    };
    let client = Client::new(address, auth)?;
    match (command.as_str(), tail) {
        ("worktree", []) => show(&client.json("GET", "/v1/code/repository", true, None)?),
        ("worktree-prepare", [root, owned, task, base]) => {
            // The challenge itself is written to the terminal. Validate the
            // immutable object ID before interpolating caller bytes.
            if ![40, 64].contains(&base.len())
                || !base
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            {
                return Err(());
            }
            confirm(
                &json!({"repository":root,"management_directory":owned,"task_id":task,"base_commit":base,
                "warning":"Runs hardened local Git inspection and reads the entire repository to prepare a worktree. No network, model, branch creation or test execution. The management directory must be private, empty and outside the repository and state."}),
                &format!("inspect {base}"),
            )?;
            show(&client.json("POST","/v1/code/repository/prepare",true,Some(json!({
                "root":root,"owned_root":owned,"task_id":task,"base_commit":base,"approve_inspection":true
            })))?)
        }
        ("worktree-create", [root, owned]) => worktree_decide(&client, Some((root, owned))),
        ("worktree-deny", []) => worktree_decide(&client, None),
        ("map" | "map-select", [root, query]) => {
            let map = client.json(
                "POST",
                "/v1/code/map",
                true,
                Some(json!({
                    "root":root,"query":query,"approve_discovery":true
                })),
            )?;
            if command == "map" {
                return show(&map);
            }
            let identity = map["root_sha256"]
                .as_str()
                .filter(|s| digest(s))
                .ok_or(())?;
            let paths = map["entries"]
                .as_array()
                .ok_or(())?
                .iter()
                .filter(|entry| entry["score"].as_u64().is_some_and(|score| score > 0))
                .take(16)
                .map(|entry| entry["path"].as_str().map(str::to_owned).ok_or(()))
                .collect::<Result<Vec<_>>>()?;
            if paths.is_empty() {
                show(&map)?;
                return Err(());
            }
            let settings = client.json("GET", "/v1/settings", true, None)?;
            let revision = settings["settings_revision"].as_u64().ok_or(())?;
            if settings["endpoint"].as_str().is_none() {
                return Err(());
            }
            LinuxCodingReadScope::verify_root(Path::new(root), identity).map_err(|_| ())?;
            let body = json!({"root":root,"paths":paths,"approve_read":true,
                "approve_model_context":true,"settings_revision":revision,
                "settings_endpoint":settings["endpoint"],"expected_root_sha256":identity});
            confirm(
                &json!({"selection":body,"settings":settings,
                "ranking":"lexical path match only; review every selected path",
                "warning":"Allows complete selected file text to be read and sent to this configured endpoint. Mapping is not a source or test result."}),
                &format!("share {revision}"),
            )?;
            LinuxCodingReadScope::verify_root(Path::new(root), identity).map_err(|_| ())?;
            show(&client.json("PUT", "/v1/code/workspace", true, Some(body))?)
        }
        ("status", []) => show(&client.json("GET", "/v1/status", false, None)?),
        ("workspace", []) => show(&client.json("GET", "/v1/code/workspace", false, None)?),
        ("review", []) => show(&client.json("GET", "/v1/code/workspace/changes", false, None)?),
        ("cancel", []) => show(&client.json("POST", "/v1/cancel", false, None)?),
        ("revoke", []) => show(&client.json("DELETE", "/v1/code/workspace", true, None)?),
        ("select" | "select-model", [root, paths @ ..]) if !paths.is_empty() => {
            let mut body = json!({"root":root,"paths":paths,"approve_read":true,"approve_model_context":command == "select-model"});
            if command == "select-model" {
                let settings = client.json("GET", "/v1/settings", true, None)?;
                let revision = settings["settings_revision"].as_u64().ok_or(())?;
                if settings["endpoint"].as_str().is_none() {
                    return Err(());
                }
                confirm(
                    &json!({"selection":body,"settings":settings,"warning":"Allows selected source text to be sent to this configured endpoint."}),
                    &format!("share {revision}"),
                )?;
                body["settings_revision"] = json!(revision);
                body["settings_endpoint"] = settings["endpoint"].clone();
            }
            show(&client.json("PUT", "/v1/code/workspace", true, Some(body))?)
        }
        ("read", []) | ("search", [_]) => {
            let selected = selection(&client)?;
            let mut body = json!({"workspace_revision":selected["workspace_revision"],"paths":selected["paths"]});
            let route = if let [query] = tail {
                body["query"] = json!(query);
                "/v1/code/workspace/search"
            } else {
                "/v1/code/workspace/read"
            };
            show(&client.json("POST", route, false, Some(body))?)
        }
        ("propose", [model, bytes, tokens, task]) => propose(&client, model, bytes, tokens, task),
        ("apply", [root]) => apply(&client, Some(root)),
        ("deny", []) => apply(&client, None),
        _ => {
            usage();
            Err(())
        }
    }
}

fn usage() {
    eprintln!(
        "Usage: agentmage-code <loopback-ip:port> <private-auth-directory> worktree|worktree-prepare <repository> <private-management-directory> <task-id> <base-commit>|worktree-create <repository> <private-management-directory>|worktree-deny|map <absolute-root> <query>|map-select <absolute-root> <query>|status|workspace|select <absolute-root> <paths...>|select-model <absolute-root> <paths...>|read|search <literal>|propose <model> <message-bytes> <output-tokens> <task>|review|apply <absolute-root>|deny|revoke|cancel"
    );
}

fn worktree_decide(client: &Client, roots: Option<(&String, &String)>) -> Result<()> {
    let status = client.json("GET", "/v1/code/repository", true, None)?;
    let preview = &status["pending"];
    let hash = preview["preview_sha256"]
        .as_str()
        .filter(|s| digest(s))
        .ok_or(())?;
    if let Some((root, owned)) = roots {
        let repository_identity = preview["repository_root_sha256"].as_str().ok_or(())?;
        let owned_identity = preview["owned_root_sha256"].as_str().ok_or(())?;
        let verify = || {
            LinuxCodingReadScope::verify_root(Path::new(root), repository_identity)
                .map_err(|_| ())?;
            LinuxCodingReadScope::verify_root(Path::new(owned), owned_identity).map_err(|_| ())
        };
        verify()?;
        confirm(
            &json!({"repository":root,"management_directory":owned,"preview":preview,
            "warning":"Creates the displayed task branch and management_directory/worktree. Does not select source, run tests, commit, push or delete. After a lost reply inspect the canonical receipts and disk; never resend an approval."}),
            &format!("create {hash}"),
        )?;
        verify()?;
    }
    let result = client.json(
        "POST",
        "/v1/code/repository/decide",
        true,
        Some(json!({
            "preview_sha256":hash,"approve":roots.is_some()
        })),
    )?;
    show(&result)?;
    if result["outcome"]
        != if roots.is_some() {
            "succeeded"
        } else {
            "denied"
        }
    {
        return Err(());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    fn wire(header: &str, body: &[u8]) -> Result<Vec<Value>> {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let header = header.to_string();
        let body = body.to_vec();
        let worker = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let _ = socket
                .write_all(header.as_bytes())
                .and_then(|()| socket.write_all(&body));
        });
        let result = receive(TcpStream::connect(address).unwrap(), &mut |_| Ok(()));
        worker.join().unwrap();
        result
    }

    #[test]
    fn refuses_nonlocal_addresses_and_unsafe_terminal_text() {
        for address in [
            "example.invalid:80",
            "192.0.2.1:80",
            "127.0.0.1:0",
            "http://127.0.0.1:80",
            "[2001:db8::1]:80",
        ] {
            assert!(Client::new(address, "unused").is_err());
        }
        assert!(Client::new("127.0.0.1:80", "unused").is_ok());
        assert!(Client::new("[::1]:80", "unused").is_ok());
        let text = safe(&json!({"source":"\u{1b}[2J\u{202e}\u{7f}\t\n"})).unwrap();
        assert!(!text.contains(['\u{1b}', '\u{202e}', '\u{7f}', '\t']));
        assert!(text.contains("\\u007f"));
        assert!(text.contains("\\u202e"));
        assert!(text.is_ascii());
        let unicode = json!({"source":"λ\u{202e}😀"});
        let escaped = safe(&unicode).unwrap();
        assert!(escaped.is_ascii());
        assert_eq!(serde_json::from_str::<Value>(&escaped).unwrap(), unicode);
    }

    #[test]
    fn bounded_wire_parser_refuses_redirects_truncation_and_ambiguous_framing() {
        let valid = "HTTP/1.1 200 Response\r\nContent-Type: application/json\r\nContent-Length: 2\r\nConnection: close\r\n\r\n";
        assert_eq!(wire(valid, b"{}").unwrap(), [json!({})]);
        assert!(wire(valid, b"{").is_err());
        assert!(wire(valid, b"{}x").is_err());
        assert!(wire(&valid.replace("200", "302"), b"{}").is_err());
        assert!(
            wire(
                &valid.replace(
                    "Content-Length: 2",
                    "Content-Length: 2\r\ncontent-length: 2"
                ),
                b"{}"
            )
            .is_err()
        );
        assert!(
            wire(
                &valid.replace("Content-Length: 2", "Transfer-Encoding: chunked"),
                b"{}"
            )
            .is_err()
        );
        assert!(
            wire(
                &valid.replace(
                    "Content-Length: 2",
                    "Content-Encoding: gzip\r\nContent-Length: 2"
                ),
                b"{}"
            )
            .is_err()
        );
        assert!(
            wire(
                &valid.replace("Content-Length: 2", "Content-Length: 4194305"),
                b"{}"
            )
            .is_err()
        );
        let stream = "HTTP/1.1 200 Response\r\nContent-Type: application/x-ndjson\r\nConnection: close\r\n\r\n";
        assert_eq!(wire(stream, b"{}\n{}\n").unwrap().len(), 2);
        assert!(wire(stream, b"{}\n{}").is_err());
        assert!(wire(stream, b"garbage\n").is_err());
        assert!(wire(stream, b"").is_err());
        let error = valid.replace("200", "409");
        assert!(wire(&error, b"{}").is_err());
        assert!(wire(&error, b"{").is_err());
        assert!(
            wire(
                &error.replace("Content-Length: 2", "Content-Length: 4097"),
                b"{}"
            )
            .is_err()
        );
        assert!(wire(&stream.replace("200", "409"), b"{}\n").is_err());
    }
}
