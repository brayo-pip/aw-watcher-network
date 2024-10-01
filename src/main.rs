use aw_client_rust::AwClient;
use aw_models::{Bucket, Event};
use chrono::{TimeDelta, Utc};
use dirs::config_dir;
use env_logger::Env;
use log::{info, warn};
use serde_json::{Map, Value};
use std::env;
use std::fs::{DirBuilder, File};
use std::io::prelude::*;
use std::time::Duration;
use tokio::time::interval;
use toml::Value as TomlValue;
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
    match res {
        Ok(_) => Ok(()),
        Err(e) => Err(Box::new(e)),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config_dir = get_config_path().expect("Unable to get config path");
    let config_path = config_dir.join("config.toml");

    let args: Vec<String> = env::args().collect();
    let mut port: u16 = 5600;
    if args.len() > 1 {
        for idx in 1..args.len() {
            if args[idx] == "--port" {
                port = args[idx + 1].parse().expect("Invalid port number");
                break;
            }
            if args[idx] == "--testing" {
                port = 5699;
                break;
            }
            if args[idx] == "--help" {
                println!("Usage: aw-watcher-network [--port PORT] [--testing]");
                return Ok(());
            }
        }
    }

    let env = Env::default()
        .filter_or("RUST_LOG", "info")
        .write_style_or("RUST_LOG_STYLE", "always");

    env_logger::init_from_env(env);

    if !config_path.exists() {
        DirBuilder::new()
            .recursive(true)
            .create(config_dir)
            .expect("Unable to create directory");
        let mut file = File::create(&config_path).expect("Unable to create file");
        file.write_all(b"polling_interval=10")
            .expect("Unable to write to file");
        info!("Config file created at {:?}", config_path);
    }

    let mut config_file = File::open(config_path.clone()).expect("Unable to open file");
    let mut contents = String::new();
    config_file
        .read_to_string(&mut contents)
        .expect("Unable to read file");

    let toml: TomlValue = toml::from_str(&contents).expect("Unable to parse TOML");

    let mut polling_interval = toml["polling_interval"]
        .as_integer()
        .expect("polling_interval must be an integer") as u64;

    if polling_interval < 10 {
        warn!("Polling interval is too low, setting to 10 seconds");
        polling_interval = 10;
    }

    let aw_client = AwClient::new("localhost", port, "aw-watcher-network").unwrap();
    create_bucket(&aw_client).await.unwrap();

    let mut interval = interval(Duration::from_secs(polling_interval));

    loop {
        interval.tick().await;
        let gps_addresses = wifilocate::get_addresses().await;

        match gps_addresses {
            Ok(gps_addresses) => {
                let mut data = Map::new();
                let first_gps_address = gps_addresses.first().expect("Locations is empty");

                // format the string to be in the format "longitude,latitude"
                let location = format!(
                    "{},{}",
                    first_gps_address.gps_location.location.lng,
                    first_gps_address.gps_location.location.lat
                );

                let address = first_gps_address.address.clone();

                data.insert("location".to_string(), Value::String(location));
                data.insert("address".to_string(), Value::String(address));
                let event = Event {
                    id: None,
                    timestamp: Utc::now(),
                    duration: TimeDelta::seconds(polling_interval as i64),
                    data,
                };
                aw_client
                    .heartbeat("aw-watcher-network", &event, polling_interval as f64)
                    .await
                    .unwrap_or_else(|e| {
                        warn!("Error sending heartbeat: {:?}", e);
                    });
            }
            Err(e) => {
                warn!("Error getting locations/addresses: {:?}", e);
            }
        }
    }
}
