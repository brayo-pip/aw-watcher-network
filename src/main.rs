use aw_client_rust::AwClient;
use aw_models::{Bucket, Event};
use dirs::config_dir;
use log::{info, warn};
use serde_json::{Map, Value};
use std::time::Duration;
use tokio::time::interval;
use wifilocate;

fn get_config_path() -> Option<std::path::PathBuf> {
    config_dir().map(|mut path| {
        path.push("activitywatch");
        path.push("aw-watcher-network");
        path
    })
}

async fn create_bucket(aw_client: &AwClient) -> Result<(), Box<dyn std::error::Error>> {
    let res = aw_client
        .create_bucket(&Bucket {
            id: "aw-watcher-network".to_string(),
            bid: None,
            _type: "currently-playing".to_string(),
            data: Map::new(),
            metadata: Default::default(),
            last_updated: None,
            hostname: "".to_string(),
            client: "aw-watcher-network".to_string(),
            created: None,
            events: None,
        })
        .await;
    Ok(())
}

#[tokio::main]
async fn main() {
    println!(
        "{:?}",
        wifilocate::get_location(wifilocate::get_networks())
            .await
            .ok()
    );
}
