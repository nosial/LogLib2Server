use std::io::Read;
use std::net::TcpStream;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// A running `LogLib2Server` instance for integration tests.
pub struct TestServer {
    child: Child,
    port: u16,
    stopped: Arc<AtomicBool>,
    stdout_thread: Option<JoinHandle<()>>,
    stderr_thread: Option<JoinHandle<()>>,
    stdout_buf: Arc<Mutex<String>>,
    stderr_buf: Arc<Mutex<String>>,
}

impl TestServer {
    /// Start a TCP-only server.
    pub fn start_tcp(port: u16) -> Self {
        Self::start(port, "tcp", None)
    }

    /// Start a UDP-only server.
    pub fn start_udp(port: u16) -> Self {
        Self::start(port, "udp", None)
    }

    /// Start a server listening on both TCP and UDP.
    pub fn start_both(port: u16) -> Self {
        Self::start(port, "both", None)
    }

    /// Start a TCP server with file logging enabled.
    pub fn start_tcp_with_file_logging(port: u16, output_dir: &str) -> Self {
        Self::start(port, "tcp", Some(output_dir))
    }

    fn start(port: u16, protocol: &str, output_dir: Option<&str>) -> Self {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

        let binary = manifest_dir
            .join("target")
            .join("release")
            .join("LogLib2Server");
        let binary = if binary.exists() {
            binary
        } else {
            manifest_dir
                .join("target")
                .join("debug")
                .join("LogLib2Server")
        };

        let mut cmd = Command::new(&binary);
        cmd.arg("-H")
            .arg("127.0.0.1")
            .arg("-p")
            .arg(port.to_string())
            .arg("--protocol")
            .arg(protocol)
            .arg("--trace")
            .arg("none")
            .arg("--no-color");

        if let Some(dir) = output_dir {
            cmd.arg("-o").arg(dir);
        }

        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        let mut child = cmd.spawn().expect("Failed to start LogLib2Server");

        let stdout = child.stdout.take().expect("stdout not captured");
        let stderr = child.stderr.take().expect("stderr not captured");

        let stopped = Arc::new(AtomicBool::new(false));
        let stdout_buf = Arc::new(Mutex::new(String::new()));
        let stderr_buf = Arc::new(Mutex::new(String::new()));

        // Drain stdout in a background thread
        let s_stopped = Arc::clone(&stopped);
        let s_buf = Arc::clone(&stdout_buf);
        let stdout_thread = std::thread::spawn(move || {
            let mut reader = stdout;
            let mut tmp = [0u8; 4096];
            while !s_stopped.load(Ordering::Relaxed) {
                match reader.read(&mut tmp) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        if let Ok(mut buf) = s_buf.lock() {
                            buf.push_str(&String::from_utf8_lossy(&tmp[..n]));
                        }
                    }
                }
            }
        });

        // Drain stderr in a background thread
        let e_stopped = Arc::clone(&stopped);
        let e_buf = Arc::clone(&stderr_buf);
        let stderr_thread = std::thread::spawn(move || {
            let mut reader = stderr;
            let mut tmp = [0u8; 4096];
            while !e_stopped.load(Ordering::Relaxed) {
                match reader.read(&mut tmp) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        if let Ok(mut buf) = e_buf.lock() {
                            buf.push_str(&String::from_utf8_lossy(&tmp[..n]));
                        }
                    }
                }
            }
        });

        // Wait for the server to be ready
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if Instant::now() > deadline {
                let _ = child.kill();
                let _ = child.wait();
                panic!("Server failed to start on port {port} within 10s (protocol={protocol})");
            }

            if protocol == "udp" {
                // UDP: check stderr for the startup message
                if let Ok(buf) = stderr_buf.lock() {
                    if buf.contains("UDP server listening") {
                        break;
                    }
                }
            } else {
                // TCP / both: wait for TCP port to be ready
                if TcpStream::connect(format!("127.0.0.1:{port}")).is_ok() {
                    break;
                }
            }

            std::thread::sleep(Duration::from_millis(100));
        }

        // Give the server a moment to print its startup message
        std::thread::sleep(Duration::from_millis(200));

        Self {
            child,
            port,
            stopped,
            stdout_thread: Some(stdout_thread),
            stderr_thread: Some(stderr_thread),
            stdout_buf,
            stderr_buf,
        }
    }

    #[allow(dead_code)]
    pub const fn port(&self) -> u16 {
        self.port
    }

    pub fn tcp_addr(&self) -> String {
        format!("127.0.0.1:{}", self.port)
    }

    /// Return all stdout output captured so far.
    pub fn stdout(&self) -> String {
        self.stdout_buf.lock().unwrap().clone()
    }

    /// Return all stderr output captured so far.
    pub fn stderr(&self) -> String {
        self.stderr_buf.lock().unwrap().clone()
    }

    /// Wait for a substring to appear in stdout (with timeout).
    pub fn wait_stdout(&self, needle: &str, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        loop {
            if Instant::now() > deadline {
                return false;
            }
            if self.stdout().contains(needle) {
                return true;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    /// Wait for a substring to appear in stderr (with timeout).
    pub fn wait_stderr(&self, needle: &str, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        loop {
            if Instant::now() > deadline {
                return false;
            }
            if self.stderr().contains(needle) {
                return true;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.stopped.store(true, Ordering::Relaxed);
        let _ = self.child.kill();
        let _ = self.child.wait();
        // Give threads time to notice the stop and exit
        if let Some(h) = self.stdout_thread.take() {
            let _ = h.join();
        }
        if let Some(h) = self.stderr_thread.take() {
            let _ = h.join();
        }
    }
}
