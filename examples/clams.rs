use blocknative::{
    models::Blockchain,
    ws::{models::WatchConfig, ws::Ws},
};
use futures_util::StreamExt;
use std::collections::HashMap;
use tokio::fs::read_to_string;

#[tokio::main]
pub async fn main() {
    tracing_subscriber::fmt::init();
    tracing::info!("Connecting to blocknative..");
    let ws = Ws::connect(
        "wss://api.blocknative.com/v0",
        "7d507b2c-48f2-48bb-bd79-fc16ced6f8cf",
        Blockchain::polygon(),
    )
    .await
    .unwrap();

    let s = read_to_string("examples/quickswap.json").await.unwrap();
    let abi = serde_json::from_str(&s).unwrap();

    let mut filters = HashMap::new();
    filters.insert(
        "contractCall.params.path".to_string(),
        "0x4d6A30EFBE2e9D7A9C143Fce1C5Bb30d9312A465".to_string(),
    );

    let config = WatchConfig {
        scope: "0xa5E0829CaCEd8fFDD4De3c43696c57F7D7A678ff".to_string(),
        filters: vec![filters],
        abi,
        watch_address: true,
    };
    tracing::info!(
        "Subscribing to filter on: {:?}",
        "0xa5E0829CaCEd8fFDD4De3c43696c57F7D7A678ff"
    );

    let mut stream = ws.subscribe(config).await.unwrap();
    tracing::info!("Waiting for events..");

    while let Some(response) = stream.next().await {
        if let Some(event) = response.event {
            tracing::info!(
                "I sense a disturbance in the force! {}, {}",
                event.event_code,
                event.category_code
            );
            if let Some(cc) = event.contract_call {
                tracing::info!(
                    "CLAMS on the move! method: {} status: {}",
                    cc.method_name,
                    response.status
                );

                // if let Some(tx) = event.transaction {
                //     tracing::info!()
                // }
            }
        }
        // break;
    }
}
