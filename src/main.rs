use make87::interfaces::rerun::{RerunGRpcInterface, RerunGRpcInterfaceError, RerunGRpcServerConfig};
use rerun as rr;
use rerun::{MemoryLimit, RecordingStreamBuilder};
use rerun::external::uuid::Uuid;
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::TcpListener;
use sha2::{Digest, Sha256};

fn deterministic_uuid_v4_from_string(s: &str) -> Uuid {
    let hash = Sha256::digest(s.as_bytes());
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&hash[..16]);
    bytes[6] = (bytes[6] & 0x0F) | 0x40; // Version 4
    bytes[8] = (bytes[8] & 0x3F) | 0x80; // Variant RFC 4122
    Uuid::from_bytes(bytes)
}

async fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // This is where the main logic would go

    let config = make87::config::load_config_from_default_env()?;
    // let rerun_grpc_interface = RerunGRpcInterface::from_default_env("rerun-grpc")?;
    // let rec = rerun_grpc_interface.get_server_recording_stream("rerun-grpc-server")?;

    // let server_cfg = rerun_grpc_interface
    //     .get_server_service_config("rerun-grpc-server")
    //     .ok_or_else(|| RerunGRpcInterfaceError::ServerServiceNotFound("rerun-grpc-server".to_string()))?;

    let rec = RecordingStreamBuilder::new(config.application_info.system_id.as_str())
        .recording_id(deterministic_uuid_v4_from_string(
            &config.application_info.system_id,
        ))
        .serve_grpc_opts("0.0.0.0", 9876, MemoryLimit::parse("2GB")?)?;

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

    make87::run_forever();
    Ok(())
}


#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("Error running application: {}", e);
        std::process::exit(1);
    }
}
