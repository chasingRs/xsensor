use crate::api::UiDevice;
use dioxus::prelude::*;

#[derive(Clone, Copy)]
pub struct AppState {
    pub connected_device_id: Signal<String>,
    pub scanned_devices: Signal<Vec<UiDevice>>,
    pub is_scanning: Signal<bool>,
    pub is_connecting: Signal<bool>,
}

pub fn use_app_state_provider() {
    let connected_device_id = use_signal(|| String::new());
    let scanned_devices = use_signal(Vec::new);
    let is_scanning = use_signal(|| false);
    let is_connecting = use_signal(|| false);

    use_context_provider(|| AppState {
        connected_device_id,
        scanned_devices,
        is_scanning,
        is_connecting,
    });
}

pub fn use_app_state() -> AppState {
    use_context::<AppState>()
}
