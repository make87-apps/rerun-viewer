use make87::interfaces::rerun::RerunGRpcInterface;
use rerun as rr;
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::TcpListener;

async fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // This is where the main logic would go

    let rerun_grpc_interface = RerunGRpcInterface::from_default_env("rerun-grpc")?;
    let rec = rerun_grpc_interface.get_server_recording_stream("rerun-grpc-server")?;

    // Spawn TCP receiver task for Vector logs
    tokio::spawn(async move {
        let listener = TcpListener::bind("0.0.0.0:9000")
            .await
            .expect("TCP bind failed");
        println!("TCP log receiver listening on 0.0.0.0:9000");

        loop {
            let (stream, addr) = match listener.accept().await {
                Ok(pair) => pair,
                Err(e) => {
                    eprintln!("Failed to accept TCP connection: {}", e);
                    continue;
                }
            };

            let rec = rec.clone();
            tokio::spawn(async move {
                let reader = BufReader::new(stream);
                let mut lines = reader.lines();

                while let Ok(Some(line)) = lines.next_line().await {
                    match serde_json::from_str::<Value>(&line) {
                        Ok(json) => {
                            let container_name = json
                                .get("container_name")
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown_container");
                            let message = json
                                .get("message")
                                .or_else(|| json.get("msg"))
                                .and_then(|v| v.as_str());
                            if let Some(message) = message {
                                // extract loglevel form message
                                let lower_message = message.to_lowercase();
                                let log_level = if lower_message.contains("error") {
                                    rr::TextLogLevel::ERROR
                                } else if lower_message.contains("warn")
                                    || lower_message.contains("warning")
                                {
                                    rr::TextLogLevel::WARN
                                } else if lower_message.contains("info") {
                                    rr::TextLogLevel::INFO
                                } else if lower_message.contains("debug") {
                                    rr::TextLogLevel::DEBUG
                                } else {
                                    rr::TextLogLevel::TRACE
                                };
                                let _ = rec.log(
                                    container_name,
                                    &rerun::TextLog::new(message).with_level(log_level),
                                );
                            }
                        }
                        Err(e) => eprintln!("Invalid JSON from {}: {}", addr, e),
                    }
                }
            });
        }
    });

    // Keep the application alive - use std::future::pending() 
    // This never resolves but allows the runtime to handle signals properly
    std::future::pending::<()>().await;
    Ok(())
}


#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("Error running application: {}", e);
        std::process::exit(1);
    }
}
