use std::{error::Error, time::Duration};

use btleplug::{
    api::{BDAddr, Central, Manager as _, Peripheral, ScanFilter},
    platform::Manager,
};
use log::{error, info};

pub async fn scan() -> Vec<btleplug::platform::Peripheral> {
    let manager = Manager::new().await.unwrap();

    // This works even with the adapter turned off in the OS. At least on Windows it seems to.
    info!("Enumerating adapters. Pick first one found.");
    let adapters = manager.adapters().await.unwrap();
    let adapter = adapters.into_iter().next().unwrap();

    // This does NOT work with the adapter turned off.
    let scan_time = 2;
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

pub async fn identify_device(
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

pub async fn characteristic_subscribe(
    characteristic_uuid: &str,
    device: btleplug::platform::Peripheral,
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
