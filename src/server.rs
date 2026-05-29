use crate::console::{self, ConsoleConfig};
use crate::event::LogEvent;
use crate::file_logger::FileLogger;
use std::io::Write;
use std::sync::Arc;
use tokio::io::AsyncReadExt;
use tokio::net::{TcpListener, UdpSocket};
use tokio::sync::mpsc;

const MAX_DATAGRAM_SIZE: usize = 65535;
const TCP_BUF_SIZE: usize = 65536;

pub enum ServerCommand {
    Event(Box<LogEvent>),
    Shutdown,
}

pub async fn start_tcp(
    addr: &str,
    tx: mpsc::Sender<ServerCommand>,
    mut shutdown: tokio::sync::watch::Receiver<bool>,
) -> std::io::Result<()> {
    let listener = TcpListener::bind(addr).await?;
    eprintln!("TCP server listening on {addr}");

    loop {
        let accept = tokio::select! {
            result = listener.accept() => result,
            _ = shutdown.wait_for(|v| *v) => return Ok(()),
        };

        let (stream, peer) = accept?;
        let tx = tx.clone();

        tokio::spawn(async move {
            let (mut reader, _) = tokio::io::split(stream);
            let mut buf = vec![0u8; TCP_BUF_SIZE];

            loop {
                let n = match reader.read(&mut buf).await {
                    Ok(0) => break,
                    Ok(n) => n,
                    Err(e) => {
                        eprintln!("TCP read error from {peer}: {e}");
                        break;
                    }
                };

                let data_str = String::from_utf8_lossy(&buf[..n]);
                for line in data_str.lines() {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }
                    match serde_json::from_str::<LogEvent>(trimmed) {
                        Ok(mut event) => {
                            event.source = Some(peer.to_string());
                            let _ = tx.send(ServerCommand::Event(Box::new(event))).await;
                        }
                        Err(_) => {
                            let _ = tx
                                .send(ServerCommand::Event(Box::new(LogEvent {
                                    application_name: "Unknown".into(),
                                    timestamp: None,
                                    level: "INFO".into(),
                                    message: format!("Raw TCP data from {peer}: {trimmed}"),
                                    trace: None,
                                    traces: None,
                                    exception: None,
                                    source: Some(peer.to_string()),
                                })))
                                .await;
                        }
                    }
                }
            }
        });
    }
}

pub async fn start_udp(
    addr: &str,
    tx: mpsc::Sender<ServerCommand>,
    mut shutdown: tokio::sync::watch::Receiver<bool>,
) -> std::io::Result<()> {
    let socket = UdpSocket::bind(addr).await?;
    eprintln!("UDP server listening on {addr}");

    let mut buf = vec![0u8; MAX_DATAGRAM_SIZE];

    loop {
        let result = tokio::select! {
            result = socket.recv_from(&mut buf) => result,
            _ = shutdown.wait_for(|v| *v) => return Ok(()),
        };

        let (n, peer) = result?;
        let data_str = String::from_utf8_lossy(&buf[..n]);
        let trimmed = data_str.trim();

        if trimmed.is_empty() {
            continue;
        }

        match serde_json::from_str::<LogEvent>(trimmed) {
            Ok(mut event) => {
                event.source = Some(peer.to_string());
                let _ = tx.send(ServerCommand::Event(Box::new(event))).await;
            }
            Err(_) => {
                let _ = tx
                    .send(ServerCommand::Event(Box::new(LogEvent {
                        application_name: "Unknown".into(),
                        timestamp: None,
                        level: "INFO".into(),
                        message: format!("Raw UDP data from {peer}: {trimmed}"),
                        trace: None,
                        traces: None,
                        exception: None,
                        source: Some(peer.to_string()),
                    })))
                    .await;
            }
        }
    }
}

pub async fn event_processor(
    mut rx: mpsc::Receiver<ServerCommand>,
    console_config: ConsoleConfig,
    file_logger: Option<Arc<FileLogger>>,
) {
    while let Some(cmd) = rx.recv().await {
        match cmd {
            ServerCommand::Event(boxed_event) => {
                let (output, is_error) = console::format_event(&boxed_event, &console_config);

                if is_error {
                    let _ = writeln!(std::io::stderr(), "{output}");
                } else {
                    let _ = writeln!(std::io::stdout(), "{output}");
                }

                if let Some(ref logger) = file_logger {
                    if let Err(e) = logger.write_event(&boxed_event) {
                        eprintln!("Failed to write log file: {e}");
                    }
                }
            }
            ServerCommand::Shutdown => break,
        }
    }
}
