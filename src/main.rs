use make87::interfaces::zenoh::ZenohInterface;
use make87_messages::core::Header;
use make87_messages::google::protobuf::Timestamp;
use make87_messages::text::PlainText;
use re_web_viewer_server::WebViewerServerPort;
use rerun as rr;
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::TcpListener;

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    // This is where the main logic would go

    let config = make87::config::load_config_from_default_env();
    if let Err(e) = config {
        eprintln!("Failed to load configuration: {}", e);
        return Err(e);
    }
    let config = config.unwrap();
    let server_memory_limit = config
        .config
        .get("server_memory_limit")
        .map(|value| value.to_string())
        .unwrap_or("2GB".to_string());
    let server_memory_limit =
        // rr::MemoryLimit::parse("2GB").expect("Failed to parse server memory limit");
        rr::MemoryLimit::parse(&server_memory_limit).expect("Failed to parse server memory limit");


    // let zenoh_interface = ZenohInterface::from_default_env();
    let mut builder = rr::RecordingStreamBuilder::new("");
    let rec = builder.serve_grpc_opts(
        "0.0.0.0",
        9876,
        server_memory_limit,
    )?;

    // Spawn TCP receiver task for Vector logs
    tokio::spawn(async move {
        let listener = TcpListener::bind("0.0.0.0:9000").await.expect("TCP bind failed");
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
                            let message = json.get("message")
                                .or_else(|| json.get("msg"))
                                .and_then(|v| v.as_str())
                                .unwrap_or("<no message>");
                            let container_name = json.get("container_name")
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown_container");
                            let message = json.get("message")
                                .or_else(|| json.get("msg"))
                                .and_then(|v| v.as_str());
                            if let Some(message) = message {
                                // extract loglevel form message
                                let lower_message = message.to_lowercase();
                                let log_level = if lower_message.contains("error") {
                                    rr::TextLogLevel::ERROR
                                } else if lower_message.contains("warn") || lower_message.contains("warning") {
                                    rr::TextLogLevel::WARN
                                } else if lower_message.contains("info") {
                                    rr::TextLogLevel::INFO
                                } else if lower_message.contains("debug") {
                                    rr::TextLogLevel::DEBUG
                                } else {
                                    rr::TextLogLevel::INFO
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
