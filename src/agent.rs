//! Background bridge to the installed Omarchy default agent.
use serde_json::Value;
use std::os::unix::process::CommandExt;
use std::{
    io::{Read, Write},
    process::{Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};

const LIMIT: u64 = 2 * 1024 * 1024;

fn stop(child: &mut std::process::Child) {
    // Only the process group created for this particular invocation.
    let _ = Command::new("/usr/bin/kill")
        .args(["-KILL", "--", &format!("-{}", child.id())])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    let _ = child.kill();
    let _ = child.wait();
}

fn run(
    mut command: Command,
    input: Vec<u8>,
    cancel: &AtomicBool,
    timeout: Duration,
) -> Result<Vec<u8>, String> {
    if cancel.load(Ordering::Relaxed) {
        return Err("Agent request canceled.".into());
    }
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .process_group(0)
        .spawn()
        .map_err(|_| "Could not start the installed agent. Check installation and login.")?;
    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = stdin.write_all(&input);
    });
    std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let result = stdout
            .take(LIMIT + 1)
            .read_to_end(&mut bytes)
            .map(|_| bytes);
        let _ = tx.send(result);
    });
    let start = Instant::now();
    let mut output = None;
    loop {
        if cancel.load(Ordering::Relaxed) || start.elapsed() > timeout {
            stop(&mut child);
            return Err("Agent request canceled or timed out.".into());
        }
        if let Ok(result) = rx.try_recv() {
            match result {
                Ok(bytes) if bytes.len() <= LIMIT as usize => output = Some(bytes),
                _ => {
                    stop(&mut child);
                    return Err("Agent output exceeded its limit or could not be read.".into());
                }
            }
        }
        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    stop(&mut child);
                    return Err("Agent failed. Check your Codex login, model access and CLI version in a terminal.".into());
                }
                if let Some(bytes) = output {
                    return Ok(bytes);
                }
            }
            Ok(None) => {}
            Err(_) => {
                stop(&mut child);
                return Err("Could not monitor the agent process.".into());
            }
        }
        std::thread::sleep(Duration::from_millis(25));
    }
}

fn codex_command() -> Command {
    let mut command = Command::new("codex");
    command.args([
        "exec",
        "--ignore-user-config",
        "--ignore-rules",
        "--ephemeral",
        "--skip-git-repo-check",
        "--sandbox",
        "read-only",
        "--json",
        "--color",
        "never",
    ]);
    for setting in [
        "approval_policy=\"never\"",
        "features.shell_tool=false",
        "features.unified_exec=false",
        "features.multi_agent=false",
        "web_search=\"disabled\"",
        "tools.view_image=false",
        "apps._default.enabled=false",
        "project_doc_max_bytes=0",
        "model_reasoning_effort=\"low\"",
    ] {
        command.args(["-c", setting]);
    }
    command
}

fn parse(bytes: &[u8]) -> Result<String, String> {
    let text = std::str::from_utf8(bytes).map_err(|_| "Agent output was not valid UTF-8.")?;
    let mut answer = None;
    let mut complete = false;
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        let event: Value =
            serde_json::from_str(line).map_err(|_| "Agent returned malformed event data.")?;
        match event["type"].as_str() {
            Some("error" | "turn.failed") => {
                return Err("The agent reported a failed generation.".into());
            }
            Some("turn.completed") => complete = true,
            Some("item.completed") if event["item"]["type"] == "agent_message" => {
                answer = event["item"]["text"].as_str().map(str::to_owned);
            }
            _ => {}
        }
    }
    if !complete {
        return Err("Agent generation did not complete.".into());
    }
    answer.ok_or("Agent returned no final artwork.".into())
}

pub fn generate(prompt: String, model: String, cancel: Arc<AtomicBool>) -> Result<String, String> {
    let preference = run(
        Command::new("omarchy-default-agent"),
        vec![],
        &cancel,
        Duration::from_secs(5),
    )?;
    if preference != b"codex\n" && preference != b"codex" {
        return Err("Omarchy agent mode currently supports Codex. Your default is different; use API/Ollama or choose Codex in Omarchy.".into());
    }
    let directory = tempfile::Builder::new()
        .prefix("mote-agent-")
        .tempdir()
        .map_err(|_| "Could not create a private agent workspace.")?;
    let mut command = codex_command();
    command.current_dir(directory.path());
    command.args([
        "--model",
        if model.is_empty() {
            "gpt-5.6-sol"
        } else {
            &model
        },
    ]);
    command.arg("-");
    parse(&run(
        command,
        prompt.into_bytes(),
        &cancel,
        Duration::from_secs(120),
    )?)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn requires_successful_final_event() {
        let final_message =
            "{\"type\":\"item.completed\",\"item\":{\"type\":\"agent_message\",\"text\":\"{}\"}}\n";
        assert!(parse(final_message.as_bytes()).is_err());
        assert_eq!(
            parse(format!("{final_message}{{\"type\":\"turn.completed\"}}\n").as_bytes()).unwrap(),
            "{}"
        );
        assert!(parse(b"{\"type\":\"turn.failed\"}").is_err());
        assert!(parse(b"not json").is_err());
    }
    #[test]
    fn restrictive_flags_and_stdin_output() {
        let command = codex_command();
        let args: Vec<_> = command.get_args().map(|s| s.to_str().unwrap()).collect();
        assert!(args.contains(&"--ignore-user-config"));
        assert!(args.contains(&"read-only"));
        assert!(args.contains(&"features.shell_tool=false"));
        assert!(args.contains(&"model_reasoning_effort=\"low\""));
        let mut echo = Command::new("/usr/bin/cat");
        echo.arg("-");
        assert_eq!(
            run(
                echo,
                b"private prompt".to_vec(),
                &AtomicBool::new(false),
                Duration::from_secs(2)
            )
            .unwrap(),
            b"private prompt"
        );
    }
    #[test]
    fn timeout_and_cancel_reap_child() {
        let mut sleep = Command::new("/usr/bin/sleep");
        sleep.arg("10");
        assert!(
            run(
                sleep,
                vec![],
                &AtomicBool::new(false),
                Duration::from_millis(50)
            )
            .is_err()
        );
        assert!(
            run(
                Command::new("/usr/bin/false"),
                vec![],
                &AtomicBool::new(true),
                Duration::from_secs(1)
            )
            .is_err()
        );
    }

    #[test]
    fn running_process_cancels_and_failures_hide_output() {
        let cancel = Arc::new(AtomicBool::new(false));
        let signal = cancel.clone();
        let worker = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(50));
            signal.store(true, Ordering::Relaxed);
        });
        let mut command = Command::new("/usr/bin/sleep");
        command.arg("10");
        let start = Instant::now();
        assert!(run(command, vec![], &cancel, Duration::from_secs(2)).is_err());
        assert!(start.elapsed() < Duration::from_secs(2));
        worker.join().unwrap();
        assert!(
            run(
                Command::new("/usr/bin/false"),
                vec![],
                &AtomicBool::new(false),
                Duration::from_secs(2)
            )
            .is_err()
        );
        assert!(
            run(
                Command::new("/usr/bin/yes"),
                vec![],
                &AtomicBool::new(false),
                Duration::from_secs(2)
            )
            .unwrap_err()
            .contains("limit")
        );
    }
}
