use crate::api::ble::BleManager;
use crate::api::{NotificationData, UiDevice, UiService};
use dioxus::prelude::*;
use futures::channel::oneshot;
use futures::Stream;
use futures::StreamExt;
use std::pin::Pin;
pub enum BleRequest {
    Scan(oneshot::Sender<Result<(), String>>),
    GetDevices(oneshot::Sender<Vec<UiDevice>>),
    Connect(String, oneshot::Sender<Result<(), String>>),
    Disconnect(String, oneshot::Sender<Result<(), String>>),
    IsConnected(String, oneshot::Sender<bool>),
    Read(
        String, // device_id
        String, // service_uuid
        String, // char_uuid
        oneshot::Sender<Result<Vec<u8>, String>>,
    ),
    Write(
        String, // device_id
        String, // service_uuid
        String, // char_uuid
        Vec<u8>,
        oneshot::Sender<Result<(), String>>,
    ),
    Subscribe(
        String, // device_id
        String, // service_uuid
        String, // char_uuid
        oneshot::Sender<Result<Pin<Box<dyn Stream<Item = NotificationData> + Send>>, String>>,
    ),
    ListCharacteristics(String, oneshot::Sender<Result<Vec<UiService>, String>>),
}

#[derive(Clone, Copy)]
pub struct BleService {
    tx: Coroutine<BleRequest>,
}

impl BleService {
    pub async fn scan(&self) -> Result<(), String> {
        let (tx, rx) = oneshot::channel();
        self.tx.send(BleRequest::Scan(tx));
        rx.await.map_err(|_| "Channel closed".to_string())?
    }

    pub async fn get_devices(&self) -> Vec<UiDevice> {
        let (tx, rx) = oneshot::channel();
        self.tx.send(BleRequest::GetDevices(tx));
        rx.await.unwrap_or_default()
    }

    pub async fn connect(&self, id: String) -> Result<(), String> {
        let (tx, rx) = oneshot::channel();
        self.tx.send(BleRequest::Connect(id, tx));
        rx.await.map_err(|_| "Channel closed".to_string())?
    }

    pub async fn disconnect(&self, id: String) -> Result<(), String> {
        let (tx, rx) = oneshot::channel();
        self.tx.send(BleRequest::Disconnect(id, tx));
        rx.await.map_err(|_| "Channel closed".to_string())?
    }

    pub async fn is_connected(&self, id: String) -> bool {
        let (tx, rx) = oneshot::channel();
        self.tx.send(BleRequest::IsConnected(id, tx));
        rx.await.unwrap_or(false)
    }

    pub async fn read(
        &self,
        id: &str,
        service: &str,
        characteristic: &str,
    ) -> Result<Vec<u8>, String> {
        let (tx, rx) = oneshot::channel();
        self.tx.send(BleRequest::Read(
            id.to_string(),
            service.to_string(),
            characteristic.to_string(),
            tx,
        ));
        rx.await.map_err(|_| "Channel closed".to_string())?
    }

    pub async fn write(
        &self,
        id: &str,
        service: &str,
        characteristic: &str,
        data: Vec<u8>,
    ) -> Result<(), String> {
        let (tx, rx) = oneshot::channel();
        self.tx.send(BleRequest::Write(
            id.to_string(),
            service.to_string(),
            characteristic.to_string(),
            data,
            tx,
        ));
        rx.await.map_err(|_| "Channel closed".to_string())?
    }

    pub async fn subscribe(
        &self,
        id: &str,
        service: &str,
        characteristic: &str,
    ) -> Result<Pin<Box<dyn Stream<Item = NotificationData> + Send>>, String> {
        let (tx, rx) = oneshot::channel();
        self.tx.send(BleRequest::Subscribe(
            id.to_string(),
            service.to_string(),
            characteristic.to_string(),
            tx,
        ));
        rx.await.map_err(|_| "Channel closed".to_string())?
    }

    pub async fn list_characteristics(&self, id: String) -> Result<Vec<UiService>, String> {
        let (tx, rx) = oneshot::channel();
        self.tx.send(BleRequest::ListCharacteristics(id, tx));
        rx.await.map_err(|_| "Channel closed".to_string())?
    }
}

pub fn use_ble_provider() {
    let tx = use_coroutine(|mut rx: UnboundedReceiver<BleRequest>| async move {
        let mut manager = BleManager::new();
        while let Some(msg) = rx.next().await {
            match msg {
                BleRequest::Scan(reply) => {
                    let _ = reply.send(manager.start_scan().await.map_err(|e| e.to_string()));
                }
                BleRequest::GetDevices(reply) => {
                    let _ = reply.send(manager.get_devices().await.unwrap_or_default());
                }
                BleRequest::Connect(id, reply) => {
                    let _ = reply.send(manager.connect(id).await.map_err(|e| e.to_string()));
                }
                BleRequest::Disconnect(id, reply) => {
                    let _ = reply.send(manager.disconnect(id).await.map_err(|e| e.to_string()));
                }
                BleRequest::IsConnected(id, reply) => {
                    let _ = reply.send(manager.is_connected(id).await.unwrap_or(false));
                }
                BleRequest::Read(id, service, char, reply) => {
                    let _ = reply.send(
                        manager
                            .read(&id, &service, &char)
                            .await
                            .map_err(|e| e.to_string()),
                    );
                }
                BleRequest::Write(id, service, char, data, reply) => {
                    let _ = reply.send(
                        manager
                            .write(&id, &service, &char, data)
                            .await
                            .map_err(|e| e.to_string()),
                    );
                }
                BleRequest::Subscribe(id, service, char, reply) => {
                    let _ = reply.send(
                        manager
                            .subscribe(&id, &service, &char)
                            .await
                            .map_err(|e| e.to_string()),
                    );
                }
                BleRequest::ListCharacteristics(id, reply) => {
                    let _ = reply.send(
                        manager
                            .list_characteristics(&id)
                            .await
                            .map_err(|e| e.to_string()),
                    );
                }
            }
        }
    });

    use_context_provider(|| BleService { tx });
}

pub fn use_ble() -> BleService {
    use_context::<BleService>()
}
