use futures::StreamExt;
use std::{error::Error, time::Duration};

use btleplug::{
    api::{BDAddr, Central, Manager as _, Peripheral, ScanFilter},
    platform::Manager,
};
use log::{error, info};
use rusqlite::Connection;

use crate::heartrate::{self};

pub async fn record_hrp_device(mac: &str, conn: &Connection) {
    let device = match identify_device(mac, scan().await).await {
        Some(device) => {
            info!("Identified device {}", mac);
            device
        }
        None => {
            error!("Failed to find device {}", mac);
            return;
        }
    };

    match device.connect().await {
        Ok(_) => info!("Connected to {}", mac),
        Err(err) => {
            error!("Failed to connect to device {}\n{}", mac, err);
            return;
        }
    };

    match device.discover_services().await {
        Ok(_) => info!("Discovered services for {}", mac),
        Err(err) => {
            error!("Failed to discover services for {}\n{}", mac, err);
            return;
        }
    };

    let characteristic_uuid = "00002a37-0000-1000-8000-00805f9b34fb";
    match characteristic_subscribe(characteristic_uuid, &device).await {
        Ok(_) => info!(
            "Subscribed to characteristic {} on {}",
            characteristic_uuid, mac
        ),
        Err(err) => {
            error!(
                "Failed to subscribe to characteristic {} on {}\n{}",
                characteristic_uuid, mac, err
            )
        }
    };

    let mut notification_stream = device.notifications().await.unwrap();
    while let Some(data) = notification_stream.as_mut().next().await {
        info!("Receiving data {:?}", data.value);
        match heartrate::write_heartrate(data.value[1], conn) {
            Ok(_) => info!("Recorded heartrate: {}bpm", data.value[1]),
            Err(err) => error!("Failed to write heartrate data\n{}", err),
        }
    }

    match device.disconnect().await {
        Ok(_) => info!("Disconnected from {}", mac),
        Err(err) => error!("Failed to disconnected from device {}\n{}", mac, err),
    };
}

pub async fn scan() -> Vec<btleplug::platform::Peripheral> {
    let manager = Manager::new().await.unwrap();

    // This works even with the adapter turned off in the OS. At least on Windows it seems to.
    info!("Enumerating adapters. Pick first one found.");
    let adapters = manager.adapters().await.unwrap();
    let adapter = adapters.into_iter().next().unwrap();

    // This does NOT work with the adapter turned off.
    let scan_time = 3; // Heart Rate Profile v10, p.13, Table 5.1 recommends up to 2.5s
    match adapter.start_scan(ScanFilter { services: vec![] }).await {
        Ok(_) => info!("Scanning devices for {}s", scan_time),
        Err(err) => {
            error!("Adapter is not ready: {}", err);
            return vec![];
        }
    }
    tokio::time::sleep(Duration::from_secs(scan_time)).await;

    info!("Returning devices");
    adapter.peripherals().await.unwrap()
}

async fn identify_device(
    mac: &str,
    devices: Vec<btleplug::platform::Peripheral>,
) -> Option<btleplug::platform::Peripheral> {
    let bdaddr = match BDAddr::from_str_delim(mac) {
        Ok(bdaddr) => bdaddr,
        Err(err) => {
            error!("Invalid device MAC: {}", err);
            return None;
        }
    };

    for dev in devices {
        let properties = match dev.properties().await {
            Ok(p_opt) => match p_opt {
                Some(p) => p,
                None => continue,
            },
            Err(_) => continue,
        };

        if bdaddr == properties.address {
            return Some(dev);
        }
    }

    None
}

async fn characteristic_subscribe(
    characteristic_uuid: &str,
    device: &btleplug::platform::Peripheral,
) -> Result<(), Box<dyn Error>> {
    let characteristics = device.characteristics();
    let characteristic = characteristics
        .iter()
        .find(|&c| c.uuid.to_string() == characteristic_uuid);

    let characteristic = match characteristic {
        Some(c) => c,
        None => return Err("Characteristic not found".into()),
    };

    match device.subscribe(characteristic).await {
        Ok(_) => Ok(()),
        Err(err) => Err(Box::new(err)),
    }
}
