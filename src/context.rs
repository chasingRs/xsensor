use crate::api::UiDevice;
use dioxus::prelude::*;

#[derive(Clone, PartialEq, Debug, Copy)]
pub enum Theme {
    Dark,
    Light,
}

impl Theme {
    pub fn is_dark(&self) -> bool {
        matches!(self, Theme::Dark)
    }

    pub fn toggle(&self) -> Theme {
        match self {
            Theme::Dark => Theme::Light,
            Theme::Light => Theme::Dark,
        }
    }

    /// 根据主题选择对应值
    pub fn pick<T: Clone>(&self, dark: T, light: T) -> T {
        if self.is_dark() { dark } else { light }
    }
}

#[derive(Clone, Copy)]
pub struct AppState {
    pub connected_device_id: Signal<String>,
    pub scanned_devices: Signal<Vec<UiDevice>>,
    pub is_scanning: Signal<bool>,
    pub is_connecting: Signal<bool>,
    pub theme: Signal<Theme>,
}

pub fn use_app_state_provider() {
    let connected_device_id = use_signal(|| String::new());
    let scanned_devices = use_signal(Vec::new);
    let is_scanning = use_signal(|| false);
    let is_connecting = use_signal(|| false);
    let theme = use_signal(|| Theme::Dark);

    use_context_provider(|| AppState {
        connected_device_id,
        scanned_devices,
        is_scanning,
        is_connecting,
        theme,
    });
}

pub fn use_app_state() -> AppState {
    use_context::<AppState>()
}
