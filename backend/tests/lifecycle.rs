#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod common;
#[path = "common/providers.rs"]
mod stubs;
use branch_backend::{
    access::issue_cookie,
    app::{AppState, api_router},
    config::Limits,
    jobs::Coordinator,
    providers::{HttpProviders, ProviderEndpoints},
    server::Server,
    telemetry::Telemetry,
};
use serde_json::json;
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;
struct Running {
    addr: SocketAddr,
    shutdown: CancellationToken,
    task: Option<JoinHandle<Result<(), branch_backend::error::AppError>>>,
    state: Arc<AppState>,
    _upstream: stubs::StubProviders,
}
impl Drop for Running {
    fn drop(&mut self) {
        self.shutdown.cancel();
        if let Some(task) = &self.task {
            task.abort();
        }
    }
}
async fn start(limits: Limits, gate: Option<Arc<tokio::sync::Notify>>) -> Running {
    let upstream = stubs::StubProviders::start(vec![stubs::Script {
        gate,
        ..stubs::Script::json(json!({"results":[]}))
    }])
    .await;
    let mut config = (*common::config("http://127.0.0.1:3000")).clone();
    config.limits = limits.clone();
    let config = Arc::new(config);
    let telemetry = Arc::new(Telemetry::new());
    let provider = Arc::new(
        HttpProviders::with_endpoints(
            config.clone(),
            ProviderEndpoints {
                openai: upstream.url(),
                tavily: upstream.url(),
            },
        )
        .unwrap()
        .with_telemetry(telemetry.clone()),
    );
    let state = Arc::new(AppState {
        jobs: Coordinator::instrumented(provider, limits.clone(), telemetry.clone()),
        config,
        telemetry,
        draining: CancellationToken::new(),
    });
    let server = Server::bind(
        api_router(state.clone()),
        "127.0.0.1:0".parse().unwrap(),
        state.jobs.clone(),
        state.draining.clone(),
        limits,
    )
    .await
    .unwrap();
    let addr = server.local_addr();
    let shutdown = CancellationToken::new();
    let signal = shutdown.clone();
    let task = tokio::spawn(server.run(signal.cancelled_owned()));
    Running {
        addr,
        shutdown,
        task: Some(task),
        state,
        _upstream: upstream,
    }
}
async fn exchange(addr: SocketAddr, request: &str) -> String {
    let mut socket = TcpStream::connect(addr).await.unwrap();
    let _ = socket.write_all(request.as_bytes()).await;
    let mut reply = String::new();
    let _ = tokio::time::timeout(Duration::from_secs(2), socket.read_to_string(&mut reply))
        .await
        .unwrap();
    reply
}
async fn wait_for(mut ready: impl FnMut() -> bool) {
    for _ in 0..200 {
        if ready() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    assert!(ready(), "condition did not complete");
}
fn research_request() -> String {
    let body=json!({"explorationId":"e","nodeId":"n","question":"q","clarification":null,"ancestors":[]}).to_string();
    format!(
        "POST /api/research HTTP/1.1\r\nHost: 127.0.0.1:3000\r\nOrigin: http://127.0.0.1:3000\r\nCookie: branch_access={}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        issue_cookie(&"a".repeat(64), 4102444800000).unwrap(),
        body.len()
    )
}

#[tokio::test]
async fn connection_and_header_limits_are_bounded() {
    let server = start(Limits::default(), None).await;
    let mut sockets = vec![];
    for _ in 0..128 {
        sockets.push(TcpStream::connect(server.addr).await.unwrap());
    }
    let mut extra = TcpStream::connect(server.addr).await.unwrap();
    let mut byte = [0];
    let result = tokio::time::timeout(Duration::from_secs(1), extra.read(&mut byte))
        .await
        .unwrap();
    assert!(result.is_err() || result.unwrap() == 0);
    drop(sockets);
    drop(server);
    let limits = Limits {
        headers: Duration::from_millis(30),
        ..Limits::default()
    };
    let server = start(limits, None).await;
    let slow = exchange(
        server.addr,
        "GET /health/live HTTP/1.1\r\nHost: localhost\r\n",
    )
    .await;
    assert!(slow.is_empty() || slow.starts_with("HTTP/1.1 408"));
    let large=exchange(server.addr,&format!("GET /health/live HTTP/1.1\r\nHost: localhost\r\nX-Large: {}\r\nConnection: close\r\n\r\n","x".repeat(32768))).await;
    assert!(large.starts_with("HTTP/1.1 431"), "{large}");
    let many = exchange(
        server.addr,
        &format!(
            "GET /health/live HTTP/1.1\r\nHost: localhost\r\n{}Connection: close\r\n\r\n",
            (0..65).map(|n| format!("X-{n}: a\r\n")).collect::<String>()
        ),
    )
    .await;
    assert!(many.starts_with("HTTP/1.1 431"), "{many}");
}
#[tokio::test]
async fn disconnect_releases_only_its_waiter() {
    let gate = Arc::new(tokio::sync::Notify::new());
    let server = start(Limits::default(), Some(gate.clone())).await;
    let mut a = TcpStream::connect(server.addr).await.unwrap();
    a.write_all(research_request().as_bytes()).await.unwrap();
    let mut b = TcpStream::connect(server.addr).await.unwrap();
    b.write_all(research_request().as_bytes()).await.unwrap();
    wait_for(|| server.state.jobs.snapshot().waiters == 2).await;
    drop(a);
    wait_for(|| server.state.jobs.snapshot().waiters == 1).await;
    gate.notify_one();
    let mut reply = String::new();
    tokio::time::timeout(Duration::from_secs(2), b.read_to_string(&mut reply))
        .await
        .unwrap()
        .unwrap();
    assert!(reply.starts_with("HTTP/1.1 200"), "{reply}");
    wait_for(|| server.state.jobs.snapshot().waiters == 0).await;
}
#[tokio::test]
async fn shutdown_drains_and_exits() {
    let gate = Arc::new(tokio::sync::Notify::new());
    let mut server = start(Limits::default(), Some(gate.clone())).await;
    let mut socket = TcpStream::connect(server.addr).await.unwrap();
    socket
        .write_all(research_request().as_bytes())
        .await
        .unwrap();
    wait_for(|| server.state.jobs.snapshot().active == 1).await;
    server.shutdown.cancel();
    wait_for(|| server.state.draining.is_cancelled()).await;
    gate.notify_one();
    let mut reply = String::new();
    socket.read_to_string(&mut reply).await.unwrap();
    assert!(reply.starts_with("HTTP/1.1 200"));
    tokio::time::timeout(Duration::from_secs(2), server.task.take().unwrap())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(server.state.jobs.snapshot().active, 0);
}
#[tokio::test]
async fn shutdown_cancels_stalled_jobs_after_grace() {
    let limits = Limits {
        shutdown_grace: Duration::from_millis(30),
        shutdown_total: Duration::from_secs(1),
        ..Limits::default()
    };
    let mut server = start(limits, Some(Arc::new(tokio::sync::Notify::new()))).await;
    let mut socket = TcpStream::connect(server.addr).await.unwrap();
    socket
        .write_all(research_request().as_bytes())
        .await
        .unwrap();
    wait_for(|| server.state.jobs.snapshot().active == 1).await;
    server.shutdown.cancel();
    tokio::time::timeout(Duration::from_secs(2), server.task.take().unwrap())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(server.state.jobs.snapshot().active, 0);
    assert_eq!(server.state.jobs.snapshot().waiters, 0);
}
#[test]
fn fatal_startup_is_nonzero_and_does_not_echo_configuration() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_branch-backend"))
        .env_clear()
        .env("APP_ENV_FILE", "")
        .env("APP_ORIGIN", "private-secret-invalid-origin")
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(!String::from_utf8_lossy(&output.stderr).contains("private-secret-invalid-origin"));
}

struct ProcessGuard {
    child: std::process::Child,
    root: tempfile::TempDir,
    origin: String,
    log: std::path::PathBuf,
}
impl Drop for ProcessGuard {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}
fn spawn_process(with_env_file: bool) -> ProcessGuard {
    use sha2::{Digest, Sha256};
    use std::{
        fs,
        process::{Command, Stdio},
    };
    let root = tempfile::tempdir().unwrap();
    let public = root.path().join("public");
    fs::create_dir(&public).unwrap();
    let html = "<!doctype html><html><body>Fixture</body></html>";
    fs::write(public.join("index.html"), html).unwrap();
    fs::write(root.path().join("frontend-manifest.json"),json!({"version":1,"files":[{"path":"index.html","sha256":format!("{:x}",Sha256::digest(html)),"immutable":false}],"inlineScriptHashes":[]}).to_string()).unwrap();
    let executable = root.path().join("branch-backend");
    fs::copy(env!("CARGO_BIN_EXE_branch-backend"), &executable).unwrap();
    let unrelated = root.path().join("unrelated");
    fs::create_dir(&unrelated).unwrap();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let origin = format!("http://127.0.0.1:{port}");
    let env_file = root.path().join("fixture.env");
    if with_env_file {
        fs::write(
            &env_file,
            "APP_ORIGIN=private-file-origin\nAPP_ACCESS_KEY=private-short-file-key\n",
        )
        .unwrap();
    }
    let log = root.path().join("stdout.log");
    let child = Command::new(executable)
        .current_dir(unrelated)
        .env_clear()
        .env(
            "APP_ENV_FILE",
            if with_env_file {
                env_file.as_os_str()
            } else {
                std::ffi::OsStr::new("")
            },
        )
        .env("APP_ORIGIN", &origin)
        .env("APP_ACCESS_KEY", "a".repeat(64))
        .env("PORT", port.to_string())
        .stdout(Stdio::from(fs::File::create(&log).unwrap()))
        .stderr(Stdio::from(
            fs::File::create(root.path().join("stderr.log")).unwrap(),
        ))
        .spawn()
        .unwrap();
    ProcessGuard {
        child,
        root,
        origin,
        log,
    }
}
async fn process_ready(process: &mut ProcessGuard) {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(100))
        .build()
        .unwrap();
    for _ in 0..200 {
        assert!(
            process.child.try_wait().unwrap().is_none(),
            "server exited before ready"
        );
        if let Ok(response) = client
            .get(format!("{}/health/live", process.origin))
            .send()
            .await
            && response.status() == 200
        {
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("server did not become ready");
}
fn signal(process: &ProcessGuard, signal: rustix::process::Signal) {
    rustix::process::kill_process(
        rustix::process::Pid::from_raw(process.child.id() as i32).unwrap(),
        signal,
    )
    .unwrap();
}
async fn process_exit(process: &mut ProcessGuard) -> std::process::ExitStatus {
    for _ in 0..200 {
        if let Some(status) = process.child.try_wait().unwrap() {
            return status;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("server did not exit");
}
#[tokio::test]
async fn executable_handles_sigint_sigterm_env_precedence_and_unrelated_cwd() {
    for termination in [rustix::process::Signal::INT, rustix::process::Signal::TERM] {
        let mut process = spawn_process(true);
        process_ready(&mut process).await;
        assert!(
            reqwest::get(&process.origin)
                .await
                .unwrap()
                .text()
                .await
                .unwrap()
                .contains("Fixture")
        );
        signal(&process, termination);
        assert!(process_exit(&mut process).await.success());
        let logs = std::fs::read_to_string(&process.log).unwrap()
            + &std::fs::read_to_string(process.root.path().join("stderr.log")).unwrap();
        assert!(!logs.contains("private-file-origin"));
        assert!(!logs.contains("private-short-file-key"));
        assert!(logs.contains("shutdown_completed"));
    }
}
#[tokio::test]
async fn second_signal_forces_exit_while_an_upload_is_pending() {
    let mut process = spawn_process(false);
    process_ready(&mut process).await;
    let mut socket = TcpStream::connect(process.origin.strip_prefix("http://").unwrap())
        .await
        .unwrap();
    let token = issue_cookie(&"a".repeat(64), 4102444800000).unwrap();
    let authority = process.origin.strip_prefix("http://").unwrap();
    socket.write_all(format!("POST /api/research HTTP/1.1\r\nHost: {authority}\r\nOrigin: {}\r\nCookie: branch_access={token}\r\nContent-Type: application/json\r\nTransfer-Encoding: chunked\r\n\r\n1\r\n{{\r\n",process.origin).as_bytes()).await.unwrap();
    wait_for(|| {
        std::fs::read_to_string(&process.log)
            .unwrap()
            .lines()
            .any(|line| line.contains("http_started") && line.contains("/api/research"))
    })
    .await;
    signal(&process, rustix::process::Signal::TERM);
    wait_for(|| {
        std::fs::read_to_string(&process.log)
            .unwrap()
            .contains("shutdown_started")
    })
    .await;
    signal(&process, rustix::process::Signal::TERM);
    assert_eq!(process_exit(&mut process).await.code(), Some(130));
}
