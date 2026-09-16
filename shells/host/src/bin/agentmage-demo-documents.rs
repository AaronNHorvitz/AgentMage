//! JSON-lines local document adapter; no model, network, or source-write authority.

#[cfg(target_os = "linux")]
fn main() {
    use std::io::{BufRead, Read, Write};
    let mut runtime = agentmage_host::demo_documents::DemoDocuments::default();
    let input = std::io::stdin();
    let mut output = std::io::stdout().lock();
    let mut reader = input.lock();
    loop {
        // Bounded reads protect the privileged document adapter before JSON parsing.
        let mut line = Vec::new();
        let result = (&mut reader).take(65_537).read_until(b'\n', &mut line);
        match result {
            Ok(0) | Err(_) => break,
            Ok(_) if line.len() > 65_536 => break,
            Ok(_) => {}
        }
        let response = match serde_json::from_slice(&line) {
            Ok(request) => runtime.execute(&request),
            Err(_) => serde_json::json!({"ok":false,"error":"invalid JSON request"}),
        };
        if writeln!(output, "{response}")
            .and_then(|()| output.flush())
            .is_err()
        {
            break;
        }
    }
}

#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("The document demo adapter requires Linux openat2 boundary protection.");
    std::process::exit(1);
}
