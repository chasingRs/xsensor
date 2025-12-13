use dioxus::prelude::*;

#[derive(Clone, Copy)]
pub struct ConnectedDevice {
    pub id: Signal<String>,
}

pub fn use_connected_device_provider() {
    let id = use_signal(|| String::new());
    use_context_provider(|| ConnectedDevice { id });
}

pub fn use_connected_device() -> ConnectedDevice {
    use_context::<ConnectedDevice>()
}
