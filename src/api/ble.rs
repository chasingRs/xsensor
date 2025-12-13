use crate::api::{NotificationData, UiCharacteristic, UiDevice, UiService};
use btleplug::api::{Central, Manager as _, Peripheral as _, ScanFilter, WriteType};
use btleplug::platform::{Adapter, Manager, Peripheral};
use dioxus::prelude::info;
use futures::StreamExt;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use uuid::Uuid;

// Simple error type
#[derive(Debug)]
pub struct BleError(String);
impl std::fmt::Display for BleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl std::error::Error for BleError {}
impl BleError {
    pub fn new(msg: impl Into<String>) -> Self {
        Self(msg.into())
    }
}

pub struct BleManager {
    adapter: Option<Adapter>,
    known_peripherals: HashMap<String, Arc<Peripheral>>,
    connected_peripherals: HashMap<String, Arc<Peripheral>>,
}

impl BleManager {
    pub fn new() -> Self {
        Self {
            adapter: None,
            known_peripherals: HashMap::new(),
            connected_peripherals: HashMap::new(),
        }
    }

    pub async fn ensure_adapter(&mut self) -> Result<Adapter, BleError> {
        if let Some(s) = &self.adapter {
            return Ok(s.clone());
        }
        let manager = Manager::new()
            .await
            .map_err(|e| BleError::new(e.to_string()))?;
        let adpaters = manager
            .adapters()
            .await
            .map_err(|e| BleError::new(e.to_string()))?;
        let adapter = adpaters
            .into_iter()
            .nth(0)
            .ok_or(BleError::new("No Bluetooth adapter found"))?;
        self.adapter = Some(adapter.clone());
        Ok(adapter)
    }

    pub async fn start_scan(&mut self) -> Result<(), BleError> {
        let adapter = self.ensure_adapter().await?;
        adapter
            .start_scan(ScanFilter::default())
            .await
            .map_err(|e| BleError::new(e.to_string()))?;
        // Stop after 15s
        tokio::time::sleep(std::time::Duration::from_secs(15)).await;
        adapter
            .stop_scan()
            .await
            .map_err(|e| BleError::new(e.to_string()))?;
        Ok(())
    }

    pub async fn get_devices(&mut self) -> Result<Vec<UiDevice>, BleError> {
        let adapter = self.ensure_adapter().await?;
        let peripherals = adapter
            .peripherals()
            .await
            .map_err(|e| BleError::new(e.to_string()))?;

        for p in peripherals {
            self.known_peripherals
                .insert(p.id().to_string(), std::sync::Arc::new(p));
        }

        let mut p_list = Vec::new();
        for (id, p) in &self.known_peripherals {
            let props = p.properties().await.unwrap_or(None);
            let name = props
                .as_ref()
                .and_then(|x| x.local_name.clone())
                .unwrap_or_else(|| "Unknow".to_string());
            let rssi = props.and_then(|x| x.rssi);

            p_list.push(UiDevice {
                id: id.clone(),
                name,
                is_connected: p.is_connected().await.unwrap_or_default(),
                rssi,
            });
        }

        // Sort: connected devices first, then by RSSI (strongest first, unknown last).
        p_list.sort_by(|a, b| {
            b.is_connected.cmp(&a.is_connected).then_with(|| {
                let a_rssi = a.rssi.unwrap_or(i16::MIN);
                let b_rssi = b.rssi.unwrap_or(i16::MIN);
                b_rssi.cmp(&a_rssi)
            })
        });

        Ok(p_list)
    }

    pub async fn connect(&mut self, id: String) -> Result<(), BleError> {
        let p = self.known_peripherals.get(&id).cloned();
        if let Some(p) = p {
            p.connect()
                .await
                .map_err(|e| BleError::new(e.to_string()))?;

            // Update connected peripheral list
            self.connected_peripherals.insert(id.to_string(), p.clone());

            // Discover services in background
            let p_clone = p.clone();
            tokio::spawn(async move {
                let _ = p_clone.discover_services().await;
                info!("Service discovered");
            });
        } else {
            return Err(BleError::new("Peripheral not found"));
        }
        Ok(())
    }

    pub async fn disconnect(&mut self, id: String) -> Result<(), BleError> {
        let p = self.known_peripherals.get(&id).cloned();
        if let Some(p) = p {
            p.disconnect()
                .await
                .map_err(|e| BleError::new(e.to_string()))?;
            self.connected_peripherals.remove(&id.to_string());
        } else {
            return Err(BleError::new("Peripheral not found"));
        }
        Ok(())
    }

    async fn get_characteristic(
        &self,
        peri_id: &str,
        service_uuid: &str,
        char_uuid: &str,
    ) -> Result<(Arc<Peripheral>, btleplug::api::Characteristic), BleError> {
        let p = self.connected_peripherals.get(peri_id).cloned();

        let service_uuid_obj = Uuid::from_str(service_uuid)
            .map_err(|e| BleError::new(format!("Invalid service UUID: {}", e)))?;
        let char_uuid_obj = Uuid::from_str(char_uuid)
            .map_err(|e| BleError::new(format!("Invalid characteristic UUID: {}", e)))?;

        if let Some(p) = p {
            let chars = p.characteristics();
            if let Some(c) = chars
                .into_iter()
                .find(|c| c.service_uuid == service_uuid_obj && c.uuid == char_uuid_obj)
            {
                return Ok((p, c));
            }
            Err(BleError::new(format!(
                "Characteristic not found: {}",
                char_uuid,
            )))
        } else {
            Err(BleError::new("Device not connected"))
        }
    }

    pub async fn read(
        &self,
        peri_id: &str,
        service_uuid: &str,
        char_uuid: &str,
    ) -> Result<Vec<u8>, BleError> {
        let (p, char) = self
            .get_characteristic(peri_id, service_uuid, char_uuid)
            .await?;
        p.read(&char)
            .await
            .map_err(|e| BleError::new(e.to_string()))
    }

    pub async fn write(
        &self,
        peri_id: &str,
        service_uuid: &str,
        char_uuid: &str,
        value: Vec<u8>,
    ) -> Result<(), BleError> {
        let (p, char) = self
            .get_characteristic(peri_id, service_uuid, char_uuid)
            .await?;
        p.write(&char, &value, WriteType::WithResponse)
            .await
            .map_err(|e| BleError::new(e.to_string()))
    }

    pub async fn subscribe(
        &self,
        id: &str,
        service_uuid: &str,
        char_uuid: &str,
    ) -> Result<std::pin::Pin<Box<dyn futures::Stream<Item = NotificationData> + Send>>, BleError>
    {
        let (p, char) = self.get_characteristic(id, service_uuid, char_uuid).await?;
        p.subscribe(&char)
            .await
            .map_err(|e| BleError::new(e.to_string()))?;
        let s = p
            .notifications()
            .await
            .map_err(|e| BleError::new(e.to_string()))?;

        Ok(Box::pin(s.map(|v| NotificationData {
            uuid: v.uuid.to_string(),
            value: v.value,
        })))
    }
    pub async fn is_connected(&self, id: String) -> Result<bool, BleError> {
        if let Some(p) = self.connected_peripherals.get(&id) {
            return Ok(p.is_connected().await.unwrap_or(false));
        }
        Ok(false)
    }

    pub async fn list_characteristics(&self, id: &str) -> Result<Vec<UiService>, BleError> {
        let p = self
            .connected_peripherals
            .get(id)
            .ok_or(BleError::new("Device not connected"))?;

        let mut ui_services = Vec::new();
        for service in p.services() {
            let mut ui_chars = Vec::new();
            for char in service.characteristics {
                ui_chars.push(UiCharacteristic {
                    uuid: char.uuid.to_string(),
                    service_uuid: service.uuid.to_string(),
                    property: crate::api::prop_flags_to_vec(char.properties),
                });
            }
            ui_services.push(UiService {
                uuid: service.uuid.to_string(),
                name: get_service_name(&service.uuid.to_string()).to_string(),
                characteristic: ui_chars,
            });
        }
        Ok(ui_services)
    }
}

pub fn get_service_name(uuid: &str) -> &str {
    match uuid {
        "00001800-0000-1000-8000-00805f9b34fb" => "Generic Access",
        "00001801-0000-1000-8000-00805f9b34fb" => "Generic Attribute",
        "0000180a-0000-1000-8000-00805f9b34fb" => "Device Information",
        "0000180f-0000-1000-8000-00805f9b34fb" => "Battery Service",
        "00001802-0000-1000-8000-00805f9b34fb" => "Immediate Alert",
        _ => "Unknown Service",
    }
}

pub async fn is_adapter_available() -> Result<bool, BleError> {
    let manager = Manager::new()
        .await
        .map_err(|e| BleError::new(e.to_string()))?;
    let adpaters = manager
        .adapters()
        .await
        .map_err(|e| BleError::new(e.to_string()))?;
    Ok(!adpaters.is_empty())
}
