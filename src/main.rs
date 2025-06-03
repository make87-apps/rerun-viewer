use make87_messages::core::Header;
use make87_messages::google::protobuf::Timestamp;
use make87_messages::text::PlainText;

fn main() {
    make87::initialize();

    let endpoint_name = "PROVIDER_ENDPOINT";
    match make87::resolve_endpoint_name(endpoint_name) {
        Some(endpoint_name) => {
            if let Some(endpoint) = make87::get_provider::<PlainText, PlainText>(endpoint_name) {
                endpoint
                    .provide(|request| {
                        println!("Received message '{:?}'", request);

                        // Get defaulted values out of the incoming header
                        let reference_id =
                            request.header.as_ref().map(|h| h.reference_id).unwrap_or(0);

                        let entity_path = request
                            .header
                            .as_ref()
                            .map(|h| h.entity_path.clone())
                            .unwrap_or_else(|| "/".to_string());

                        // Build the new header
                        let header = Header {
                            timestamp: Timestamp::get_current_time().into(),
                            reference_id,
                            entity_path,
                        };

                        let response = PlainText {
                            header: Some(header),
                            body: request.body.chars().rev().collect(),
                        };

                        response
                    })
                    .unwrap();
            }
        }
        None => {
            panic!(
                "{}",
                format!("Failed to resolve topic name '{}'", endpoint_name)
            );
        }
    }

    make87::keep_running();
}
