use std::os::fd::AsRawFd;
use std::path::PathBuf;
use std::process::{Child, Command};
use std::sync::atomic::{AtomicU16, Ordering};
use std::time::{Duration, Instant};

static NEXT_PORT: AtomicU16 = AtomicU16::new(15000);

/// Spawn the pelisearch-server binary on a dynamic port.
/// Returns the child process handle and the assigned port.
pub fn start_server(data_dir: &str) -> (Child, u16) {
    let bin_path = find_binary();
    let port = pick_port();

    // Kill any leftover process holding the port (orphaned from a prior panic).
    let _ = Command::new("fuser")
        .args(["-k", &format!("{port}/tcp")])
        .output();

    let mut child = Command::new(bin_path)
        .arg("--port")
        .arg(port.to_string())
        .arg("--data-path")
        .arg(data_dir)
        .spawn()
        .expect("failed to start server binary");

    // Wait for the server to become ready and verify it's OUR child.
    let start = Instant::now();
    let timeout = Duration::from_secs(10);
    let client = reqwest::blocking::Client::new();

    loop {
        if start.elapsed() > timeout {
            panic!("server did not become ready within {timeout:?}");
        }
        // If the child exited, our server failed to bind.
        if let Some(status) = child.try_wait().unwrap() {
            panic!("server exited unexpectedly with {status}");
        }
        match client
            .get(format!("http://127.0.0.1:{port}/health"))
            .timeout(Duration::from_secs(1))
            .send()
        {
            Ok(resp) if resp.status().is_success() => break,
            _ => std::thread::sleep(Duration::from_millis(100)),
        }
    }

    (child, port)
}

/// Kill the server process and wait for it to exit.
pub fn stop_server(mut child: Child) {
    let _ = child.kill();
    for _ in 0..30 {
        if child.try_wait().unwrap().is_some() {
            return;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    let _ = child.wait();
}

/// Build a full URL for the given path.
pub fn url(port: u16, path: &str) -> String {
    format!("http://127.0.0.1:{port}{path}")
}

fn find_binary() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("../../target/debug/pelisearch-server");
    if path.exists() {
        return path;
    }
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("../../target/release/pelisearch-server");
    if path.exists() {
        return path;
    }
    panic!(
        "pelisearch-server binary not found at {:?} or {:?}",
        path.display(),
        path.display()
    );
}

/// Pick a port using an atomic counter (intra-process) and a cross-process
/// `flock` lock file to prevent collisions between separate test binaries.
/// Leaks the flock fd so the lock lives until the process exits.
fn pick_port() -> u16 {
    loop {
        let port = NEXT_PORT.fetch_add(1, Ordering::Relaxed);
        if port >= 16000 {
            panic!("exhausted port range 15000..16000");
        }
        let lock_path = format!("/tmp/pelisearch-port-{port}.lock");
        match std::fs::File::create(&lock_path) {
            Ok(f) => {
                let fd = f.as_raw_fd();
                let ret = unsafe { libc::flock(fd, libc::LOCK_EX | libc::LOCK_NB) };
                if ret == 0 {
                    std::mem::forget(f);
                    return port;
                }
            }
            Err(_) => continue,
        }
    }
}
