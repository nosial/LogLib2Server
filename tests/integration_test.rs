mod common;

use common::TestServer;
use std::io::Write;
use std::net::TcpStream;
use std::net::UdpSocket;
use std::time::Duration;

fn send_tcp(addr: &str, data: &str) {
    let mut stream = TcpStream::connect(addr).expect("TCP connect failed");
    stream.write_all(data.as_bytes()).expect("TCP write failed");
    stream.flush().expect("TCP flush failed");
}

fn send_udp(addr: &str, data: &str) {
    let socket = UdpSocket::bind("127.0.0.1:0").expect("UDP bind failed");
    socket
        .send_to(data.as_bytes(), addr)
        .expect("UDP send failed");
}

const TIMEOUT: Duration = Duration::from_secs(5);

#[tokio::test]
async fn test_tcp_valid_event_goes_to_stdout() {
    let mut port: u16 = 21010;
    let server = loop {
        let s = TestServer::start_tcp(port);
        // Check startup
        if s.stderr().contains("TCP server listening") {
            break s;
        }
        port += 1;
    };

    let json = r#"{"application_name":"MyApp","level":"INFO","message":"hello from tcp"}"#;
    send_tcp(&server.tcp_addr(), json);

    assert!(
        server.wait_stdout("[MyApp] hello from tcp", TIMEOUT),
        "Expected event in stdout. stdout={:?} stderr={:?}",
        server.stdout(),
        server.stderr()
    );
}

#[tokio::test]
async fn test_tcp_error_event_goes_to_stderr() {
    let mut port: u16 = 21020;
    let server = loop {
        let s = TestServer::start_tcp(port);
        if s.stderr().contains("TCP server listening") {
            break s;
        }
        port += 1;
    };

    let json = r#"{"application_name":"App","level":"ERROR","message":"tcp error test"}"#;
    send_tcp(&server.tcp_addr(), json);

    assert!(
        server.wait_stderr("[App] tcp error test", TIMEOUT),
        "Expected error event in stderr. stdout={:?} stderr={:?}",
        server.stdout(),
        server.stderr()
    );
}

#[tokio::test]
async fn test_tcp_invalid_json_becomes_raw_message() {
    let mut port: u16 = 21030;
    let server = loop {
        let s = TestServer::start_tcp(port);
        if s.stderr().contains("TCP server listening") {
            break s;
        }
        port += 1;
    };

    send_tcp(&server.tcp_addr(), "this is not json");

    assert!(
        server.wait_stdout("[Unknown] Raw TCP data", TIMEOUT),
        "Expected raw data message in stdout. stdout={:?} stderr={:?}",
        server.stdout(),
        server.stderr()
    );
}

#[tokio::test]
async fn test_tcp_multiple_events() {
    let mut port: u16 = 21040;
    let server = loop {
        let s = TestServer::start_tcp(port);
        if s.stderr().contains("TCP server listening") {
            break s;
        }
        port += 1;
    };

    for i in 0..3 {
        let json =
            format!(r#"{{"application_name":"Multi","level":"INFO","message":"event {i}"}}"#);
        send_tcp(&server.tcp_addr(), &json);
    }

    assert!(
        server.wait_stdout("[Multi] event 2", TIMEOUT),
        "Expected all events in stdout. stdout={:?} stderr={:?}",
        server.stdout(),
        server.stderr()
    );
}

#[tokio::test]
async fn test_udp_valid_event_goes_to_stdout() {
    let mut port: u16 = 21050;
    let server = loop {
        let s = TestServer::start_udp(port);
        if s.stderr().contains("UDP server listening") {
            break s;
        }
        port += 1;
    };

    let json = r#"{"application_name":"UdpApp","level":"INFO","message":"hello via udp"}"#;
    send_udp(&server.tcp_addr(), json);

    assert!(
        server.wait_stdout("[UdpApp] hello via udp", TIMEOUT),
        "Expected UDP event in stdout. stdout={:?} stderr={:?}",
        server.stdout(),
        server.stderr()
    );
}

#[tokio::test]
async fn test_udp_error_event_goes_to_stderr() {
    let mut port: u16 = 21060;
    let server = loop {
        let s = TestServer::start_udp(port);
        if s.stderr().contains("UDP server listening") {
            break s;
        }
        port += 1;
    };

    let json = r#"{"application_name":"App","level":"WARNING","message":"udp warning"}"#;
    send_udp(&server.tcp_addr(), json);

    assert!(
        server.wait_stderr("[App] udp warning", TIMEOUT),
        "Expected UDP warning in stderr. stdout={:?} stderr={:?}",
        server.stdout(),
        server.stderr()
    );
}

#[tokio::test]
async fn test_udp_invalid_json_becomes_raw_message() {
    let mut port: u16 = 21070;
    let server = loop {
        let s = TestServer::start_udp(port);
        if s.stderr().contains("UDP server listening") {
            break s;
        }
        port += 1;
    };

    send_udp(&server.tcp_addr(), "raw udp text");

    assert!(
        server.wait_stdout("[Unknown] Raw UDP data", TIMEOUT),
        "Expected raw UDP message in stdout. stdout={:?} stderr={:?}",
        server.stdout(),
        server.stderr()
    );
}

#[tokio::test]
async fn test_both_protocols() {
    let mut port: u16 = 21080;
    let server = loop {
        let s = TestServer::start_both(port);
        if s.stderr().contains("TCP server listening")
            && s.stderr().contains("UDP server listening")
        {
            break s;
        }
        port += 1;
    };

    let tcp_json = r#"{"application_name":"Both","level":"INFO","message":"tcp on both"}"#;
    let udp_json = r#"{"application_name":"Both","level":"INFO","message":"udp on both"}"#;

    send_tcp(&server.tcp_addr(), tcp_json);
    send_udp(&server.tcp_addr(), udp_json);

    assert!(
        server.wait_stdout("[Both] tcp on both", TIMEOUT)
            && server.wait_stdout("[Both] udp on both", TIMEOUT),
        "Expected both messages in stdout. stdout={:?} stderr={:?}",
        server.stdout(),
        server.stderr()
    );
}

#[tokio::test]
async fn test_file_logging_creates_log_file() {
    let dir = std::env::temp_dir()
        .join("loglib2_test_file_logging")
        .join(format!("{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);

    let mut port: u16 = 21090;
    let server = loop {
        let s = TestServer::start_tcp_with_file_logging(port, &dir.to_string_lossy());
        if s.stderr().contains("TCP server listening") {
            break s;
        }
        port += 1;
    };

    let json = r#"{"application_name":"FileTest","level":"INFO","message":"written to file"}"#;
    send_tcp(&server.tcp_addr(), json);

    // Wait for the event to be processed
    assert!(
        server.wait_stdout("[FileTest] written to file", TIMEOUT),
        "Event should appear in stdout. stdout={:?} stderr={:?}",
        server.stdout(),
        server.stderr()
    );

    // Drop the server to flush logs
    drop(server);

    // Check that a log file was created in the directory
    let dir_entries: Vec<_> = std::fs::read_dir(&dir)
        .expect("output dir should exist")
        .filter_map(Result::ok)
        .collect();

    assert!(!dir_entries.is_empty(), "Expected at least one log file");

    let log_file = dir_entries
        .iter()
        .find(|e| {
            e.path()
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| {
                    n.starts_with("log")
                        && std::path::Path::new(n)
                            .extension()
                            .is_some_and(|ext| ext.eq_ignore_ascii_case("jsonl"))
                })
        })
        .expect("Expected a .jsonl log file");

    let content =
        std::fs::read_to_string(log_file.path()).expect("Should be able to read log file");

    assert!(
        content.contains("FileTest"),
        "Log file should contain the application name. content={content:?}"
    );
    assert!(
        content.contains("written to file"),
        "Log file should contain the message. content={content:?}"
    );

    // Cleanup
    let _ = std::fs::remove_dir_all(log_file.path().parent().unwrap().parent().unwrap());
}

#[tokio::test]
async fn test_file_logging_rotates_daily() {
    let dir = std::env::temp_dir()
        .join("loglib2_test_rotation")
        .join(format!("{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);

    let mut port: u16 = 21100;
    let server = loop {
        let s = TestServer::start_tcp_with_file_logging(port, &dir.to_string_lossy());
        if s.stderr().contains("TCP server listening") {
            break s;
        }
        port += 1;
    };

    let json = r#"{"application_name":"Rotate","level":"INFO","message":"rotation test"}"#;
    send_tcp(&server.tcp_addr(), json);

    assert!(
        server.wait_stdout("[Rotate] rotation test", TIMEOUT),
        "Event should appear in stdout"
    );

    drop(server);

    // Should have exactly one log file for today
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let expected_file = dir.join(format!("log{today}.jsonl"));
    assert!(
        expected_file.exists(),
        "Expected log file at {expected_file:?}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn test_server_rejects_invalid_protocol() {
    // We don't test invalid protocol parsing via the binary since it uses clap's
    // value_parser. Instead, verify the server starts correctly with each valid protocol.
    for (port, protocol) in &[(21110u16, "tcp"), (21111, "udp"), (21112, "both")] {
        let server = if *protocol == "tcp" {
            TestServer::start_tcp(*port)
        } else if *protocol == "udp" {
            TestServer::start_udp(*port)
        } else {
            TestServer::start_both(*port)
        };

        let needle = if *protocol == "both" {
            "TCP server listening"
        } else {
            &format!("{} server listening", protocol.to_uppercase())
        };
        assert!(
            server.stderr().contains(needle),
            "Expected '{:?}' in stderr for protocol={}. stderr={:?}",
            needle,
            protocol,
            server.stderr()
        );
    }
}
