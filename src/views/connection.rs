use crate::api::ble_service::use_ble;
use crate::api::{UiCharacteristic, UiDevice, UiService};
use crate::context::use_app_state;
use dioxus::prelude::*;
use futures::StreamExt;
use std::collections::{HashMap, HashSet};

const SERVICE_UUID: &str = "0000ffe0-0000-1000-8000-00805f9b34fb";

#[component]
pub fn Connection() -> Element {
    let mut app_state = use_app_state();
    let ble = use_ble();
    
    let expanded_devices = use_signal(HashSet::<String>::new);
    let loading_services = use_signal(HashSet::<String>::new);
    let device_services = use_signal(HashMap::<String, Vec<UiService>>::new);
    let mut adapter_available = use_signal(|| true);
    let mut manual_disconnect = use_signal(|| false);

    use_resource(move || async move {
        if let Ok(available) = crate::api::ble::is_adapter_available().await {
            adapter_available.set(available);
        }
    });



    let connect_task = use_coroutine(move |mut rx: UnboundedReceiver<String>| {
        let ble = ble.clone();
        async move {
            while let Some(dev_id) = rx.next().await {
                if (app_state.is_scanning)() {
                    let _ = ble.stop_scan().await;
                    app_state.is_scanning.set(false);
                }

                app_state.is_connecting.set(true);
                if let Err(e) = ble.connect(dev_id.clone()).await {
                    error!("Failed to connect to device: {}", e.to_string());
                } else {
                    app_state.connected_device_id.set(dev_id);
                    app_state.scanned_devices.set(ble.get_devices().await);
                }
                app_state.is_connecting.set(false);
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
                    app_state.connected_device_id.set(String::new());
                    app_state.scanned_devices.set(ble.get_devices().await);
                }
            }
        }
    });

    let connect_task_clone = connect_task.clone();
    let scan_task = use_coroutine(move |mut rx: UnboundedReceiver<()>| {
        let ble = ble.clone();
        let connect_task = connect_task_clone.clone();
        async move {
            while let Some(_) = rx.next().await {
                app_state.is_scanning.set(true);
                match ble.scan().await {
                    Ok(mut stream) => {
                        let timeout = tokio::time::sleep(std::time::Duration::from_secs(8));
                        tokio::pin!(timeout);
                        loop {
                            tokio::select! {
                                _ = &mut timeout => {
                                    break;
                                }
                                Some(new_devices) = stream.next() => {
                                    if app_state.connected_device_id.read().is_empty() && !*app_state.is_connecting.read() {
                                        if let Some(target) = new_devices.iter().find(|d| d.services.contains(&SERVICE_UUID.to_string())) {
                                            info!("Found target device {}, auto-connecting...", target.name);
                                            connect_task.send(target.id.clone());
                                            break;
                                        }
                                    }
                                    app_state.scanned_devices.set(new_devices);
                                }
                            }
                        }
                    }
                    Err(e) => error!("Scan failed: {}", e),
                }
                let _ = ble.stop_scan().await;
                app_state.is_scanning.set(false);
            }
        }
    });

    let ble_init = ble.clone();
    let scan_task_init = scan_task.clone();
    use_effect(move || {
        // Auto-refresh device list on entry
        scan_task_init.send(());
        
        spawn(async move {
            if app_state.scanned_devices.read().is_empty() {
                app_state.scanned_devices.set(ble_init.get_devices().await);
            }
        });
    });

    use_future(move || async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            if !(app_state.is_scanning)() && !(app_state.is_connecting)() {
                app_state.scanned_devices.set(ble.get_devices().await);
            }
        }
    });
    let theme = app_state.theme.read().clone();
    let root_text = theme.pick("text-gray-100", "text-gray-800");
    let empty_devices_class = theme.pick(
        "rounded-xl border border-[#2a2a2a] bg-[#1f1f1f] p-6 text-gray-400 text-sm",
        "rounded-xl border border-[#e0e0e0] bg-white p-6 text-gray-500 text-sm",
    );

    rsx! {
        div { class: "h-full w-full {root_text}",
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
                    is_scanning: (app_state.is_scanning)(),
                    on_scan: move |_| {
                        scan_task.send(());
                    },
                }

                {
                    let devices = app_state.scanned_devices.read();
                    rsx! {
                        if devices.is_empty() {
                            div { class: "{empty_devices_class}", "未发现设备，尝试重新扫描。" }
                        } else {
                            DeviceList {
                                devices: devices.clone(),
                                connected_device_id: app_state.connected_device_id.read().clone(),
                                expanded_devices,
                                loading_services,
                                device_services,
                                on_connect: move |dev_id: String| {
                                    if !(app_state.is_connecting)() {
                                        manual_disconnect.set(false);
                                        connect_task.send(dev_id);
                                    }
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
                    }
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
    connected_device_id: String,
    expanded_devices: Signal<HashSet<String>>,
    loading_services: Signal<HashSet<String>>,
    device_services: Signal<HashMap<String, Vec<UiService>>>,
    on_connect: EventHandler<String>,
    on_disconnect: EventHandler<String>,
    on_toggle: EventHandler<String>,
) -> Element {
    let theme = crate::context::use_app_state().theme.read().clone();
    let container_class = theme.pick(
        "rounded-xl border border-[#2a2a2a] bg-[#1f1f1f] divide-y divide-[#2a2a2a] shadow-lg shadow-black/20",
        "rounded-xl border border-[#e0e0e0] bg-white divide-y divide-[#e0e0e0] shadow-sm",
    );
    rsx! {
        div { class: "{container_class}",
            for dev in devices {
                DeviceEntry {
                    device: dev.clone(),
                    is_connected: dev.id == connected_device_id,
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
    is_connected: bool,
    is_expanded: bool,
    loading: bool,
    services: Option<Vec<UiService>>,
    on_connect: EventHandler<String>,
    on_disconnect: EventHandler<String>,
    on_toggle: EventHandler<String>,
) -> Element {
    let theme = crate::context::use_app_state().theme.read().clone();
    let avatar_class = theme.pick(
        "h-8 w-8 rounded-lg bg-[#242424] flex items-center justify-center text-gray-200 text-xs font-semibold",
        "h-8 w-8 rounded-lg bg-[#e8e8e8] flex items-center justify-center text-gray-700 text-xs font-semibold",
    );
    let device_name_class = theme.pick("text-sm font-medium text-gray-100", "text-sm font-medium text-gray-800");
    let device_id_class   = theme.pick("text-[10px] text-gray-500",         "text-[10px] text-gray-400");
    let signal_class      = theme.pick("text-xs text-gray-300",             "text-xs text-gray-600");
    let expanded_panel_class = theme.pick(
        "mt-1 mb-2 rounded-lg border border-[#2a2a2a] bg-[#191919] px-3 py-2",
        "mt-1 mb-2 rounded-lg border border-[#e0e0e0] bg-[#f5f5f5] px-3 py-2",
    );
    let service_empty_class = theme.pick("text-xs text-gray-400", "text-xs text-gray-500");

    let dev_id = device.id.clone();
    let dev_id_for_toggle = dev_id.clone();
    let dev_id_for_disconnect = dev_id.clone();
    let rssi_text = device
        .rssi
        .map(|v| format!("{v} dBm"))
        .unwrap_or_else(|| "未知".to_string());
    let (status_label, status_class) = if is_connected {
        if theme.is_dark() {
            ("已连接", "bg-emerald-500/15 text-emerald-300 border border-emerald-500/30")
        } else {
            ("已连接", "bg-emerald-100 text-emerald-700 border border-emerald-300")
        }
    } else if theme.is_dark() {
        ("未连接", "bg-slate-500/15 text-slate-300 border border-slate-500/30")
    } else {
        ("未连接", "bg-slate-100 text-slate-600 border border-slate-300")
    };

    rsx! {
        div {
            key: "{device.id}",
            class: "flex flex-col gap-2 md:flex-row md:items-center md:justify-between px-3 py-3",
            div { class: "flex items-center gap-3",
                div { class: "{avatar_class}", "BT" }
                div {
                    div { class: "{device_name_class}", "{device.name}" }
                    div { class: "{device_id_class}", "ID: {device.id}" }
                }
            }
            div { class: "flex flex-col sm:flex-row sm:items-center gap-2 md:gap-3",
                span { class: "{signal_class}", "信号: {rssi_text}" }
                span { class: format!("text-[10px] px-2 py-0.5 rounded-full {status_class}"),
                    "{status_label}"
                }
                if is_connected {
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
            div { class: "{expanded_panel_class}",
                match services {
                    Some(svcs) if !svcs.is_empty() => rsx! {
                        ServiceList { services: svcs }
                    },
                    Some(_) => rsx! {
                        div { class: "{service_empty_class}", "未发现服务" }
                    },
                    None => rsx! {
                        div { class: "{service_empty_class}", "正在加载服务..." }
                    },
                }
            }
        }
    }
}

#[component]
fn ServiceList(services: Vec<UiService>) -> Element {
    let theme = crate::context::use_app_state().theme.read().clone();
    let svc_card_class      = theme.pick("rounded-md border border-[#2f2f2f] bg-[#161616]",   "rounded-md border border-[#e0e0e0] bg-[#f8f8f8]");
    let svc_name_class      = theme.pick("text-sm font-medium",                                "text-sm font-medium text-gray-800");
    let svc_uuid_class      = theme.pick("text-xs text-gray-500",                              "text-xs text-gray-400");
    let svc_meta_class      = theme.pick("flex items-center gap-2 text-xs text-gray-400",      "flex items-center gap-2 text-xs text-gray-500");
    let expand_btn_class    = theme.pick("text-[11px] px-2 py-1 rounded bg-[#222] border border-[#2f2f2f]", "text-[11px] px-2 py-1 rounded bg-[#ebebeb] border border-[#ddd] text-gray-600");
    let char_section_class  = theme.pick("border-t border-[#2f2f2f] bg-[#121212]",            "border-t border-[#e0e0e0] bg-[#f0f0f0]");
    let no_char_class       = theme.pick("border-t border-[#2f2f2f] bg-[#121212] px-3 py-2 text-xs text-gray-500", "border-t border-[#e0e0e0] bg-[#f0f0f0] px-3 py-2 text-xs text-gray-400");

    let expanded = use_signal(HashSet::<String>::new);

    rsx! {
        div { class: "space-y-3",
            for svc in services {
                div { class: "{svc_card_class}",
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
                            div { class: "{svc_name_class}", "{svc.name}" }
                            div { class: "{svc_uuid_class}", "{svc.uuid}" }
                        }
                        div { class: "{svc_meta_class}",
                            span { "特征: {svc.characteristic.len()}" }
                            span { class: "{expand_btn_class}",
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
                            div { class: "{char_section_class}",
                                CharacteristicList { chars: svc.characteristic.clone() }
                            }
                        } else {
                            div { class: "{no_char_class}", "无特征" }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn CharacteristicList(chars: Vec<UiCharacteristic>) -> Element {
    let theme = crate::context::use_app_state().theme.read().clone();
    let table_class    = theme.pick("w-full text-sm text-gray-200",              "w-full text-sm text-gray-700");
    let row_class      = theme.pick("border-t border-[#1f1f1f] hover:bg-[#181818]", "border-t border-[#eeeeee] hover:bg-[#f0f0f0]");
    let uuid_td_class  = theme.pick("px-3 py-2 text-xs text-gray-300 break-all", "px-3 py-2 text-xs text-gray-600 break-all");
    let prop_td_class  = theme.pick("px-3 py-2 text-xs text-gray-400",           "px-3 py-2 text-xs text-gray-500");

    rsx! {
        table { class: "{table_class}",
            thead { class: "text-xs uppercase text-gray-500",
                tr {
                    th { class: "px-3 py-2 text-left", "特征 UUID" }
                    th { class: "px-3 py-2 text-left", "属性" }
                }
            }
            tbody {
                for ch in chars {
                    tr { class: "{row_class}",
                        td { class: "{uuid_td_class}", "{ch.uuid}" }
                        td { class: "{prop_td_class}",
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
