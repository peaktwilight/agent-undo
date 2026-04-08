use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};

use crate::hook::ActiveSession;
use crate::paths::ProjectPaths;
use crate::session;
use crate::store::Store;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Request {
    Status,
    SessionStart {
        agent: String,
        metadata: Option<String>,
    },
    SessionEnd {
        session_id: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Response {
    Status {
        events: i64,
        active_session: Option<ActiveSession>,
    },
    SessionStarted {
        session_id: String,
    },
    SessionEnded,
    Error {
        message: String,
    },
}

pub fn send(paths: &ProjectPaths, request: &Request) -> Result<Response> {
    #[cfg(unix)]
    {
        use std::os::unix::net::UnixStream;

        let mut stream = UnixStream::connect(&paths.socket_path)
            .with_context(|| format!("connecting to {}", paths.socket_path.display()))?;
        let payload = serde_json::to_vec(request)?;
        stream.write_all(&payload)?;
        stream.shutdown(std::net::Shutdown::Write)?;

        let mut buf = Vec::new();
        stream.read_to_end(&mut buf)?;
        let response: Response = serde_json::from_slice(&buf)?;
        Ok(response)
    }
    #[cfg(not(unix))]
    {
        let _ = paths;
        let _ = request;
        anyhow::bail!("daemon socket control is only supported on unix platforms");
    }
}

#[cfg(unix)]
pub fn spawn_server(paths: ProjectPaths) -> Result<SocketGuard> {
    use std::os::unix::net::UnixListener;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

    if paths.socket_path.exists() {
        let _ = std::fs::remove_file(&paths.socket_path);
    }

    let listener = UnixListener::bind(&paths.socket_path)
        .with_context(|| format!("binding {}", paths.socket_path.display()))?;
    listener
        .set_nonblocking(true)
        .with_context(|| format!("setting nonblocking {}", paths.socket_path.display()))?;

    let stop = Arc::new(AtomicBool::new(false));
    let stop_thread = Arc::clone(&stop);
    let thread_paths = paths.clone();

    let handle = thread::spawn(move || {
        while !stop_thread.load(Ordering::Relaxed) {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    if let Err(err) = handle_client(&thread_paths, &mut stream) {
                        let _ = write_error_response(
                            &mut stream,
                            &format!("daemon control error: {err}"),
                        );
                    }
                }
                Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(50));
                }
                Err(_) => break,
            }
        }
    });

    Ok(SocketGuard {
        path: paths.socket_path,
        stop,
        handle: Some(handle),
    })
}

#[cfg(not(unix))]
pub fn spawn_server(paths: ProjectPaths) -> Result<SocketGuard> {
    let _ = paths;
    Ok(SocketGuard {})
}

#[cfg(unix)]
fn handle_client(paths: &ProjectPaths, stream: &mut std::os::unix::net::UnixStream) -> Result<()> {
    let mut buf = Vec::new();
    stream.read_to_end(&mut buf)?;
    let request: Request = serde_json::from_slice(&buf)?;
    let response = handle_request(paths, request)?;
    let payload = serde_json::to_vec(&response)?;
    stream.write_all(&payload)?;
    Ok(())
}

fn handle_request(paths: &ProjectPaths, request: Request) -> Result<Response> {
    let store = Store::open(paths.clone())?;
    match request {
        Request::Status => Ok(Response::Status {
            events: store.event_count()?,
            active_session: crate::hook::read_active_session(paths)?,
        }),
        Request::SessionStart { agent, metadata } => {
            let parsed = session::parse_metadata(metadata.as_deref())?;
            let session_id = session::start(
                &store,
                session::SessionStart {
                    session_id: parsed.session_id,
                    agent,
                    prompt: parsed.prompt,
                    model: parsed.model,
                    metadata: parsed.raw,
                    tool_name: parsed.tool_name,
                    intended_file: parsed.intended_file,
                    activate: true,
                },
            )?;
            Ok(Response::SessionStarted { session_id })
        }
        Request::SessionEnd { session_id } => {
            session::end(&store, &session_id, true)?;
            Ok(Response::SessionEnded)
        }
    }
}

#[cfg(unix)]
fn write_error_response(stream: &mut std::os::unix::net::UnixStream, message: &str) -> Result<()> {
    let payload = serde_json::to_vec(&Response::Error {
        message: message.into(),
    })?;
    stream.write_all(&payload)?;
    Ok(())
}

#[cfg(unix)]
pub struct SocketGuard {
    path: std::path::PathBuf,
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
    handle: Option<std::thread::JoinHandle<()>>,
}

#[cfg(unix)]
impl Drop for SocketGuard {
    fn drop(&mut self) {
        self.stop.store(true, std::sync::atomic::Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
        let _ = std::fs::remove_file(&self.path);
    }
}

#[cfg(not(unix))]
pub struct SocketGuard {}

#[cfg(test)]
mod tests {
    use super::{send, spawn_server, Request, Response};
    use crate::paths::ProjectPaths;
    use crate::store::Store;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_tmp_dir(label: &str) -> PathBuf {
        let ns = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let pid = std::process::id();
        let dir = std::env::temp_dir().join(format!("agent-undo-ipc-unit-{label}-{pid}-{ns}"));
        fs::create_dir_all(&dir).expect("create tmp dir");
        dir
    }

    #[test]
    fn socket_server_handles_status_and_session_requests() {
        let dir = unique_tmp_dir("socket");
        let paths = ProjectPaths::for_root(dir.clone());
        let store = Store::init(paths.clone()).expect("init store");
        store
            .record_event(&crate::store::NewEvent {
                ts_ns: 1,
                path: "f.rs".into(),
                before_hash: None,
                after_hash: Some("hash".into()),
                size_before: None,
                size_after: Some(1),
                attribution: "initial-scan".into(),
                confidence: "high".into(),
                session_id: None,
                pid: None,
                process_name: None,
                tool_name: None,
            })
            .expect("record event");

        let _guard = spawn_server(paths.clone()).expect("spawn socket server");

        match send(&paths, &Request::Status).expect("status request") {
            Response::Status { events, .. } => assert_eq!(events, 1),
            other => panic!("unexpected status response: {other:?}"),
        }

        let session_id = match send(
            &paths,
            &Request::SessionStart {
                agent: "codex".into(),
                metadata: Some(r#"{"prompt":"refactor auth","tool_name":"Write"}"#.into()),
            },
        )
        .expect("session start")
        {
            Response::SessionStarted { session_id } => session_id,
            other => panic!("unexpected session start response: {other:?}"),
        };

        let active = crate::hook::read_active_session(&paths)
            .expect("read active session")
            .expect("active session exists");
        assert_eq!(active.agent, "codex");

        match send(
            &paths,
            &Request::SessionEnd {
                session_id: session_id.clone(),
            },
        )
        .expect("session end")
        {
            Response::SessionEnded => {}
            other => panic!("unexpected session end response: {other:?}"),
        }

        assert!(
            crate::hook::read_active_session(&paths)
                .expect("read active session after end")
                .is_none(),
            "active session should be cleared"
        );

        fs::remove_dir_all(&dir).ok();
    }
}
