use crate::api::ble_service::use_ble;
use crate::api::{UiCharacteristic, UiDevice, UiService};
use crate::context::use_connected_device;
use dioxus::prelude::*;
use futures::StreamExt;
use std::collections::{HashMap, HashSet};

const SERVICE_UUID: &str = "0000ffe0-0000-1000-8000-00805f9b34fb";

#[component]
pub fn Connection() -> Element {
    let connected_device = use_connected_device();
    let ble = use_ble();
    let mut poll_trigger = use_signal(|| 0);
    let expanded_devices = use_signal(HashSet::<String>::new);
    let loading_services = use_signal(HashSet::<String>::new);
    let device_services = use_signal(HashMap::<String, Vec<UiService>>::new);
    let mut is_scanning = use_signal(|| false);
    let mut adapter_available = use_signal(|| true);
    let mut manual_disconnect = use_signal(|| false);

    use_resource(move || async move {
        if let Ok(available) = crate::api::ble::is_adapter_available().await {
            adapter_available.set(available);
        }
    });

    let devices_resource = use_resource(move || async move {
        let _ = poll_trigger();

        let devices_list = ble.get_devices().await;
        info!("{}", devices_list.len());
        devices_list
    });

    let connect_task = use_coroutine(move |mut rx: UnboundedReceiver<String>| {
        let ble = ble.clone();
        async move {
            while let Some(dev_id) = rx.next().await {
                if let Err(e) = ble.connect(dev_id.clone()).await {
                    error!("Failed to connect to device: {}", e.to_string());
                } else {
                    let mut id = connected_device.id;
                    id.set(dev_id);
                    poll_trigger += 1;
                }
            }
        }
    });

    let disconnect_task = use_coroutine(move |mut rx: UnboundedReceiver<String>| {
        let ble = ble.clone();
        async move {
            while let Some(dev_id) = rx.next().await {
                if let Err(e) = ble.disconnect(dev_id.clone()).await {
                    error!("Failed to disconnect device: {}", e.to_string());
                } else {
                    let mut id = connected_device.id;
                    id.set(String::new());
                    poll_trigger += 1;
                }
            }
        }
    });

    // 自动连接逻辑
    use_effect(move || {
        if manual_disconnect() {
            return;
        }
        if let Some(devices) = devices_resource.read().as_ref() {
            if connected_device.id.read().is_empty() {
                for dev in devices {
                    if dev.name.to_lowercase().contains("proximity sensor") {
                        let dev_id = dev.id.clone();
                        info!("Auto connecting to device: {}", dev_id);
                        connect_task.send(dev_id);
                        break; // 只连接第一个匹配的设备
                    }
                }
            }
        }
    });

    let scan_task = use_coroutine(move |mut rx: UnboundedReceiver<()>| {
        let ble = ble.clone();
        async move {
            while let Some(_) = rx.next().await {
                is_scanning.set(true);
                let _ = ble.scan().await;
                poll_trigger.set(poll_trigger() + 1);
                is_scanning.set(false);
            }
        }
    });

    use_future(move || async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            poll_trigger += 1;
        }
    });
    rsx! {
        div { class: "h-full w-full text-gray-100",
            div { class: "w-full px-4 py-4 space-y-4",
                if !adapter_available() {
                    div { class: "rounded-xl border border-red-500/50 bg-red-500/10 p-3 flex items-center gap-3 text-red-400 text-sm",
                        svg {
                            class: "w-5 h-5",
                            fill: "none",
                            view_box: "0 0 24 24",
                            stroke: "currentColor",
                            stroke_width: "2",
                            path {
                                stroke_linecap: "round",
                                stroke_linejoin: "round",
                                d: "M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z",
                            }
                        }
                        span { "未检测到蓝牙适配器，请确保蓝牙已开启。" }
                    }
                }
                ConnectionHeader {
                    is_scanning: is_scanning(),
                    on_scan: move |_| {
                        scan_task.send(());
                    },
                }

                match &*devices_resource.read() {
                    Some(devices) => rsx! {
                        if devices.is_empty() {
                            div { class: "rounded-xl border border-[#2a2a2a] bg-[#1f1f1f] p-6 text-gray-400 text-sm",
                                "未发现设备，尝试重新扫描。"
                            }
                        } else {
                            DeviceList {
                                devices: devices.clone(),
                                expanded_devices,
                                loading_services,
                                device_services,
                                on_connect: move |dev_id: String| {
                                    manual_disconnect.set(false);
                                    connect_task.send(dev_id);
                                },
                                on_disconnect: {
                                    let mut expanded_devices = expanded_devices.clone();
                                    let mut loading_services = loading_services.clone();
                                    let mut device_services = device_services.clone();
                                    move |dev_id: String| {
                                        // Set manual disconnect flag  Set manual disconnect flag
                                        manual_disconnect.set(true);
                                        expanded_devices.write().remove(&dev_id);
                                        loading_services.write().remove(&dev_id);
                                        device_services.write().remove(&dev_id);
                                        disconnect_task.send(dev_id);
                                    }
                                },
                                on_toggle: move |dev_id: String| {
                                    let mut expanded_devices = expanded_devices.clone();
                                    let mut loading_services = loading_services.clone();
                                    let mut device_services = device_services.clone();
                                    let ble = ble.clone();
                                    spawn(async move {
                                        if expanded_devices.read().contains(&dev_id) {
                                            expanded_devices.write().remove(&dev_id);
                                            return;
                                        }
                                        let already_loaded = device_services.read().contains_key(&dev_id);
                                        if !already_loaded {
                                            loading_services.write().insert(dev_id.clone());
                                            let mut services = Vec::new();
                                            for _ in 0..20 {
                                                match ble.list_characteristics(dev_id.clone()).await {
                                                    Ok(list) => {
                                                        services = list;
                                                        if services.iter().any(|s| s.uuid == SERVICE_UUID) {
                                                            break;
                                                        }
                                                    }
                                                    Err(e) => {
                                                        error!("Failed to list characteristics: {}", e);
                                                    }
                                                }
                                                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                                            }
                                            device_services.write().insert(dev_id.clone(), services);
                                            loading_services.write().remove(&dev_id);
                                        }
                                        expanded_devices.write().insert(dev_id);
                                    });
                                },
                            }
                        }
                    },
                    None => rsx! {
                        div { class: "rounded-xl border border-[#2a2a2a] bg-[#1f1f1f] p-6 text-sm text-gray-300",
                            "正在扫描..."
                        }
                    },
                }
            }
        }
    }
}

#[component]
fn ConnectionHeader(is_scanning: bool, on_scan: EventHandler<()>) -> Element {
    rsx! {
        div { class: "flex flex-col md:flex-row md:items-center md:justify-between gap-2",
            div {
                h1 { class: "text-xl font-semibold tracking-tight", "设备连接" }
            }
            button {
                class: "inline-flex items-center gap-2 rounded-lg bg-[#60cd18] hover:bg-[#6fe12a] disabled:opacity-50 disabled:cursor-not-allowed cursor-pointer px-3 py-1.5 text-xs font-medium text-gray-900 transition-colors",
                disabled: is_scanning,
                onclick: move |_| on_scan.call(()),
                if is_scanning {
                    span { class: "animate-spin inline-block w-3 h-3 border-2 border-gray-900 border-t-transparent rounded-full" }
                    span { "扫描中..." }
                } else {
                    span { "重新扫描" }
                }
            }
        }
    }
}

#[component]
fn DeviceList(
    devices: Vec<UiDevice>,
    expanded_devices: Signal<HashSet<String>>,
    loading_services: Signal<HashSet<String>>,
    device_services: Signal<HashMap<String, Vec<UiService>>>,
    on_connect: EventHandler<String>,
    on_disconnect: EventHandler<String>,
    on_toggle: EventHandler<String>,
) -> Element {
    rsx! {
        div { class: "rounded-xl border border-[#2a2a2a] bg-[#1f1f1f] divide-y divide-[#2a2a2a] shadow-lg shadow-black/20",
            for dev in devices {
                DeviceEntry {
                    device: dev.clone(),
                    is_expanded: expanded_devices.read().contains(&dev.id),
                    loading: loading_services.read().contains(&dev.id),
                    services: device_services.read().get(&dev.id).cloned(),
                    on_connect: move |id| on_connect.call(id),
                    on_disconnect: move |id| on_disconnect.call(id),
                    on_toggle: move |id| on_toggle.call(id),
                }
            }
        }
    }
}

#[component]
fn DeviceEntry(
    device: UiDevice,
    is_expanded: bool,
    loading: bool,
    services: Option<Vec<UiService>>,
    on_connect: EventHandler<String>,
    on_disconnect: EventHandler<String>,
    on_toggle: EventHandler<String>,
) -> Element {
    let dev_id = device.id.clone();
    let dev_id_for_toggle = dev_id.clone();
    let dev_id_for_disconnect = dev_id.clone();
    let rssi_text = device
        .rssi
        .map(|v| format!("{v} dBm"))
        .unwrap_or_else(|| "未知".to_string());
    let (status_label, status_class) = if device.is_connected {
        (
            "已连接",
            "bg-emerald-500/15 text-emerald-200 border border-emerald-500/30",
        )
    } else {
        (
            "未连接",
            "bg-slate-500/15 text-slate-200 border border-slate-500/30",
        )
    };

    rsx! {
        div {
            key: "{device.id}",
            class: "flex flex-col gap-2 md:flex-row md:items-center md:justify-between px-3 py-3",
            div { class: "flex items-center gap-3",
                div { class: "h-8 w-8 rounded-lg bg-[#242424] flex items-center justify-center text-gray-200 text-xs font-semibold",
                    "BT"
                }
                div {
                    div { class: "text-sm font-medium", "{device.name}" }
                    div { class: "text-[10px] text-gray-500", "ID: {device.id}" }
                }
            }
            div { class: "flex flex-col sm:flex-row sm:items-center gap-2 md:gap-3",
                span { class: "text-xs text-gray-200", "信号: {rssi_text}" }
                span { class: format!("text-[10px] px-2 py-0.5 rounded-full {status_class}"),
                    "{status_label}"
                }
                if device.is_connected {
                    div { class: "flex items-center gap-2",
                        button {
                            class: "inline-flex items-center justify-center rounded-lg bg-[#2d6cdf] hover:bg-[#3c7eff] cursor-pointer px-2.5 py-1 text-xs font-medium text-gray-100 transition-colors",
                            onclick: {
                                let dev_id = dev_id_for_toggle.clone();
                                move |_| on_toggle.call(dev_id.clone())
                            },
                            if loading {
                                span { class: "animate-spin inline-block w-3 h-3 border-2 border-gray-100 border-t-transparent rounded-full mr-2" }
                                span { "正在发现服务..." }
                            } else if is_expanded {
                                span { "收起服务" }
                            } else {
                                span { "查看服务" }
                            }
                        }
                        button {
                            class: "inline-flex items-center justify-center rounded-lg bg-[#c94b4b] hover:bg-[#df5d5d] cursor-pointer px-2.5 py-1 text-xs font-medium text-gray-100 transition-colors",
                            onclick: {
                                let dev_id = dev_id_for_disconnect.clone();
                                move |_| on_disconnect.call(dev_id.clone())
                            },
                            "断开"
                        }
                    }
                } else {
                    button {
                        class: "inline-flex items-center justify-center rounded-lg bg-[#60cd18] hover:bg-[#6fe12a] cursor-pointer px-2.5 py-1 text-xs font-medium text-gray-900 transition-colors",
                        onclick: move |_| on_connect.call(dev_id.clone()),
                        "连接"
                    }
                }
            }
        }

        if is_expanded {
            div { class: "mt-1 mb-2 rounded-lg border border-[#2a2a2a] bg-[#191919] px-3 py-2",
                match services {
                    Some(svcs) if !svcs.is_empty() => rsx! {
                        ServiceList { services: svcs }
                    },
                    Some(_) => rsx! {
                        div { class: "text-xs text-gray-400", "未发现服务" }
                    },
                    None => rsx! {
                        div { class: "text-xs text-gray-400", "正在加载服务..." }
                    },
                }
            }
        }
    }
}

#[component]
fn ServiceList(services: Vec<UiService>) -> Element {
    let expanded = use_signal(HashSet::<String>::new);

    rsx! {
        div { class: "space-y-3",
            for svc in services {
                div { class: "rounded-md border border-[#2f2f2f] bg-[#161616]",
                    div {
                        class: "flex items-center justify-between px-3 py-2 cursor-pointer select-none",
                        onclick: {
                            let mut expanded = expanded.clone();
                            let id = svc.uuid.clone();
                            move |_| {
                                let mut set = expanded.write();
                                if set.contains(&id) {
                                    set.remove(&id);
                                } else {
                                    set.insert(id.clone());
                                }
                            }
                        },
                        div {
                            div { class: "text-sm font-medium", "{svc.name}" }
                            div { class: "text-xs text-gray-500", "{svc.uuid}" }
                        }
                        div { class: "flex items-center gap-2 text-xs text-gray-400",
                            span { "特征: {svc.characteristic.len()}" }
                            span { class: "text-[11px] px-2 py-1 rounded bg-[#222] border border-[#2f2f2f]",
                                if expanded.read().contains(&svc.uuid) {
                                    "收起"
                                } else {
                                    "展开"
                                }
                            }
                        }
                    }
                    if expanded.read().contains(&svc.uuid) {
                        if !svc.characteristic.is_empty() {
                            div { class: "border-t border-[#2f2f2f] bg-[#121212]",
                                CharacteristicList { chars: svc.characteristic.clone() }
                            }
                        } else {
                            div { class: "border-t border-[#2f2f2f] bg-[#121212] px-3 py-2 text-xs text-gray-500",
                                "无特征"
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn CharacteristicList(chars: Vec<UiCharacteristic>) -> Element {
    rsx! {
        table { class: "w-full text-sm text-gray-200",
            thead { class: "text-xs uppercase text-gray-500",
                tr {
                    th { class: "px-3 py-2 text-left", "特征 UUID" }
                    th { class: "px-3 py-2 text-left", "属性" }
                }
            }
            tbody {
                for ch in chars {
                    tr { class: "border-t border-[#1f1f1f] hover:bg-[#181818]",
                        td { class: "px-3 py-2 text-xs text-gray-300 break-all", "{ch.uuid}" }
                        td { class: "px-3 py-2 text-xs text-gray-400",
                            if ch.property.is_empty() {
                                span { "—" }
                            } else {
                                span { {ch.property.join(", ")} }
                            }
                        }
                    }
                }
            }
        }
    }
}
