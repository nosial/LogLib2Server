mod console;
mod event;
mod file_logger;
mod server;

use clap::Parser;
use console::{ConsoleConfig, TraceFormatKind};
use file_logger::FileLogger;
use server::{event_processor, start_tcp, start_udp, ServerCommand};
use std::sync::Arc;
use tokio::signal;
use tokio::sync::{mpsc, watch};

#[derive(Parser)]
#[command(name = "LogLib2Server")]
#[command(version = "1.0.5")]
#[command(about = "LogLib2 logging event server (v1.0.5)")]
struct Args {
    #[arg(long, default_value = "both", value_parser = protocol_parser)]
    protocol: Protocol,

    #[arg(short = 'p', long, default_value = "5131")]
    port: u16,

    #[arg(short = 'H', long, default_value = "0.0.0.0")]
    host: String,

    #[arg(short = 'o', long)]
    output_path: Option<String>,

    #[arg(long, default_value = "basic", value_parser = trace_parser)]
    trace: TraceFormatKind,

    #[arg(long = "show-st", default_value_t = false)]
    always_show_stacktraces: bool,

    #[arg(long = "no-color", default_value_t = false)]
    no_color: bool,
}

#[derive(Debug, Clone, PartialEq)]
enum Protocol {
    Tcp,
    Udp,
    Both,
}

fn protocol_parser(s: &str) -> Result<Protocol, String> {
    match s.to_lowercase().as_str() {
        "tcp" => Ok(Protocol::Tcp),
        "udp" => Ok(Protocol::Udp),
        "both" => Ok(Protocol::Both),
        _ => Err(format!(
            "Invalid protocol '{s}'. Use 'tcp', 'udp', or 'both'"
        )),
    }
}

fn trace_parser(s: &str) -> Result<TraceFormatKind, String> {
    match s.to_lowercase().as_str() {
        "none" => Ok(TraceFormatKind::None),
        "basic" => Ok(TraceFormatKind::Basic),
        "full" => Ok(TraceFormatKind::Full),
        _ => Err(format!(
            "Invalid trace format '{s}'. Use 'none', 'basic', or 'full'"
        )),
    }
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let console_config =
        ConsoleConfig::new(!args.no_color, args.trace, args.always_show_stacktraces);

    let file_logger = args.output_path.as_ref().map_or_else(
        || None,
        |path| match FileLogger::new(path) {
            Ok(logger) => {
                eprintln!("File logging enabled, output directory: {path}");
                Some(Arc::new(logger))
            }
            Err(e) => {
                eprintln!("Failed to create file logger at '{path}': {e}");
                None
            }
        },
    );

    let (tx, rx) = mpsc::channel::<ServerCommand>(1024);
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    let tcp_addr = if args.protocol == Protocol::Udp {
        None
    } else {
        Some(format!("{}:{}", args.host, args.port))
    };

    let udp_addr = if args.protocol == Protocol::Tcp {
        None
    } else {
        Some(format!("{}:{}", args.host, args.port))
    };

    let mut handles = Vec::new();

    if let Some(addr) = &tcp_addr {
        let addr = addr.clone();
        let tx_clone = tx.clone();
        let rx_clone = shutdown_rx.clone();
        handles.push(tokio::spawn(async move {
            if let Err(e) = start_tcp(&addr, tx_clone, rx_clone).await {
                eprintln!("TCP server error: {e}");
            }
        }));
    }

    if let Some(addr) = &udp_addr {
        let addr = addr.clone();
        let tx_clone = tx.clone();
        let rx_clone = shutdown_rx.clone();
        handles.push(tokio::spawn(async move {
            if let Err(e) = start_udp(&addr, tx_clone, rx_clone).await {
                eprintln!("UDP server error: {e}");
            }
        }));
    }

    handles.push(tokio::spawn(event_processor(
        rx,
        console_config,
        file_logger,
    )));

    signal::ctrl_c().await.expect("Failed to listen for Ctrl+C");
    eprintln!("\nShutting down...");
    let _ = shutdown_tx.send(true);

    let _ = tx.send(ServerCommand::Shutdown).await;

    for handle in handles {
        let _ = handle.await;
    }

    eprintln!("Server stopped.");
}
