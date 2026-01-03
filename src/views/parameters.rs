use crate::api::ble_service::use_ble;
use crate::context::use_app_state;
use dioxus::prelude::*;

const SERVICE_UUID: &str = "0000ffe0-0000-1000-8000-00805f9b34fb";
const PS_LOW_UUID: &str = "0000ffe1-0000-1000-8000-00805f9b34fb";
const PS_HIGH_UUID: &str = "0000ffe2-0000-1000-8000-00805f9b34fb";
const ACC_THRESHOLD_UUID: &str = "0000ffe3-0000-1000-8000-00805f9b34fb";

#[component]
pub fn Parameters() -> Element {
    let app_state = use_app_state();
    let ble = use_ble();
    let device_id = app_state.connected_device_id.read().clone();
    let device_id_for_effect = device_id.clone();
    let device_id_for_save = device_id.clone();
    let device_id_for_reset = device_id.clone();

    // 参数状态
    let mut threshold_low = use_signal(|| 500);
    let mut threshold_high = use_signal(|| 3000);
    let mut accel_threshold = use_signal(|| 2000);
    let mut has_changes = use_signal(|| false);
    let mut is_loaded = use_signal(|| false);

    // 读取初始参数
    use_effect(move || {
        let id = device_id_for_effect.clone();
        if id.is_empty() {
            return;
        }

        if is_loaded() {
            return;
        }

        spawn(async move {
            let id_ps_low = id.clone();
            let id_ps_high = id.clone();
            let id_acc = id.clone();

            // Wait for services to be discovered
            loop {
                if let Ok(services) = ble.list_characteristics(id.clone()).await {
                    if services.iter().any(|s| s.uuid == SERVICE_UUID) {
                        break;
                    }
                }
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            }

            // 读取 PS Low (0xFFE1)
            if let Ok(data) = ble.read(&id_ps_low, SERVICE_UUID, PS_LOW_UUID).await {
                if data.len() >= 2 {
                    let val = u16::from_le_bytes([data[0], data[1]]) as i32;
                    threshold_low.set(val);
                }
            }

            // 读取 PS High (0xFFE2)
            if let Ok(data) = ble.read(&id_ps_high, SERVICE_UUID, PS_HIGH_UUID).await {
                if data.len() >= 2 {
                    let val = u16::from_le_bytes([data[0], data[1]]) as i32;
                    threshold_high.set(val);
                }
            }

            // 读取 ACC Threshold (0xFFE3)
            if let Ok(data) = ble.read(&id_acc, SERVICE_UUID, ACC_THRESHOLD_UUID).await {
                if data.len() >= 2 {
                    let val = u16::from_le_bytes([data[0], data[1]]) as i32;
                    accel_threshold.set(val);
                }
            }

            is_loaded.set(true);
        });
    });

    rsx! {
        div { class: "h-full w-full text-gray-100 overflow-auto relative",
            if device_id.is_empty() {
                ParametersEmptyState {}
            } else {
                if !is_loaded() {
                    div { class: "absolute inset-0 z-50 flex items-center justify-center bg-[#1b1b1b]/80 backdrop-blur-sm",
                        div { class: "flex flex-col items-center gap-3",
                            span { class: "animate-spin inline-block w-8 h-8 border-4 border-[#60cd18] border-t-transparent rounded-full" }
                            span { class: "text-sm text-gray-300 font-medium", "正在读取参数..." }
                        }
                    }
                }
                ParametersContent {
                    threshold_low: threshold_low(),
                    threshold_high: threshold_high(),
                    accel_threshold: accel_threshold(),
                    has_changes: has_changes(),
                    on_save: move |_| {
                        let id = device_id_for_save.clone();
                        let low = threshold_low();
                        let high = threshold_high();
                        let acc = accel_threshold();
                        spawn(async move {
                            // 写入 PS Low (0xFFE1)
                            let _ = ble
                                .write(
                                    &id,
                                    SERVICE_UUID,
                                    PS_LOW_UUID,
                                    (low as u16).to_le_bytes().to_vec(),
                                )
                                // 写入 PS High (0xFFE2)
                                .await;
                            // 写入 ACC Threshold (0xFFE3)
                            let _ = ble
                                .write(
                                    &id,
                                    SERVICE_UUID,
                                    PS_HIGH_UUID,
                                    (high as u16).to_le_bytes().to_vec(),
                                )
                                .await;
                            let _ = ble
                                .write(
                                    &id,
                                    SERVICE_UUID,
                                    ACC_THRESHOLD_UUID,
                                    (acc as u16).to_le_bytes().to_vec(),
                                )
                                .await;
                        });
                        has_changes.set(false);
                    },
                    on_reset: move |_| {
                        // 重新读取参数
                        let id = device_id_for_reset.clone();
                        spawn(async move {
                            // 读取 PS Low (0xFFE1)
                            if let Ok(data) = ble.read(&id, SERVICE_UUID, PS_LOW_UUID).await {
                                if data.len() >= 2 {
                                    let val = u16::from_le_bytes([data[0], data[1]]) as i32;
                                    threshold_low.set(val);
                                }
                            }
                            // 读取 PS High (0xFFE2)
                            if let Ok(data) = ble.read(&id, SERVICE_UUID, PS_HIGH_UUID).await {
                                if data.len() >= 2 {
                                    let val = u16::from_le_bytes([data[0], data[1]]) as i32;
                                    threshold_high.set(val);
                                }
                            }
                            // 读取 ACC Threshold (0xFFE3)
                            if let Ok(data) = ble.read(&id, SERVICE_UUID, ACC_THRESHOLD_UUID).await {
                                if data.len() >= 2 {
                                    let val = u16::from_le_bytes([data[0], data[1]]) as i32;
                                    accel_threshold.set(val);
                                }
                            }
                        });
                        has_changes.set(false);
                    },
                    on_low_change: move |value| {
                        threshold_low.set(value);
                        has_changes.set(true);
                    },
                    on_high_change: move |value| {
                        threshold_high.set(value);
                        has_changes.set(true);
                    },
                    on_accel_change: move |value| {
                        accel_threshold.set(value);
                        has_changes.set(true);
                    },
                }
            }
        }
    }
}

#[component]
fn ParametersEmptyState() -> Element {
    rsx! {
        div { class: "flex items-center justify-center h-full",
            div { class: "text-center space-y-4",
                div { class: "text-gray-500 text-lg", "未连接设备" }
                div { class: "text-gray-600 text-sm", "请前往连接页面连接设备" }
            }
        }
    }
}

#[component]
fn ParametersContent(
    threshold_low: i32,
    threshold_high: i32,
    accel_threshold: i32,
    has_changes: bool,
    on_save: EventHandler<()>,
    on_reset: EventHandler<()>,
    on_low_change: EventHandler<i32>,
    on_high_change: EventHandler<i32>,
    on_accel_change: EventHandler<i32>,
) -> Element {
    rsx! {
        div { class: "w-full px-4 py-4 space-y-4",
            ParametersHeader { has_changes, on_save, on_reset }

            ThresholdPairCard {
                low_value: threshold_low,
                high_value: threshold_high,
                on_low_change,
                on_high_change,
            }

            AccelThresholdCard { value: accel_threshold, on_change: on_accel_change }
        }
    }
}

#[component]
fn ParametersHeader(
    has_changes: bool,
    on_save: EventHandler<()>,
    on_reset: EventHandler<()>,
) -> Element {
    rsx! {
        div { class: "flex flex-col md:flex-row md:items-center md:justify-between gap-2",
            div {
                h1 { class: "text-xl font-semibold tracking-tight", "参数设置" }
                if has_changes {
                    p { class: "text-xs text-yellow-400 mt-0.5", "● 有未保存的更改" }
                }
            }
            div { class: "flex gap-2",
                button {
                    class: "inline-flex items-center gap-2 rounded-lg bg-[#2a2a2a] hover:bg-[#333] cursor-pointer px-3 py-1.5 text-xs font-medium text-gray-300 transition-colors",
                    onclick: move |_| on_reset.call(()),
                    "重置"
                }
                button {
                    class: if has_changes { "inline-flex items-center gap-2 rounded-lg bg-[#60cd18] hover:bg-[#6fe12a] cursor-pointer px-3 py-1.5 text-xs font-medium text-gray-900 transition-colors" } else { "inline-flex items-center gap-2 rounded-lg bg-[#2a2a2a] px-3 py-1.5 text-xs font-medium text-gray-500 cursor-not-allowed" },
                    disabled: !has_changes,
                    onclick: move |_| on_save.call(()),
                    "保存"
                }
            }
        }
    }
}

#[component]
fn ThresholdPairCard(
    low_value: i32,
    high_value: i32,
    on_low_change: EventHandler<i32>,
    on_high_change: EventHandler<i32>,
) -> Element {
    let percentage_low = (low_value as f64 / 2047.0 * 100.0) as i32;
    let percentage_high = (high_value as f64 / 2047.0 * 100.0) as i32;

    rsx! {
        div { class: "rounded-xl border border-[#2a2a2a] bg-[#1f1f1f] p-4 space-y-4",
            div {
                h2 { class: "text-base font-medium mb-1", "PS 传感器阈值对" }
                p { class: "text-xs text-gray-400",
                    "设置接近传感器的高低阈值范围 (0-2047)"
                }
                p { class: "text-[10px] text-gray-500 mt-0.5",
                    "UUID: 0xFFE1 (高阈值), 0xFFE2 (低阈值)"
                }
            }

            div { class: "space-y-4",
                // 低阈值
                ThresholdSlider {
                    label: "低阈值",
                    value: low_value,
                    max_value: 2047,
                    percentage: percentage_low,
                    color: "blue",
                    on_change: on_low_change,
                }

                // 高阈值
                ThresholdSlider {
                    label: "高阈值",
                    value: high_value,
                    max_value: 2047,
                    percentage: percentage_high,
                    color: "red",
                    on_change: on_high_change,
                }
            }
        }
    }
}

#[component]
fn AccelThresholdCard(value: i32, on_change: EventHandler<i32>) -> Element {
    let percentage = (value as f64 / 40.0 * 100.0) as i32;

    rsx! {
        div { class: "rounded-xl border border-[#2a2a2a] bg-[#1f1f1f] p-4 space-y-4",
            div {
                h2 { class: "text-base font-medium mb-1", "ACC 传感器阈值" }
                p { class: "text-xs text-gray-400",
                    "设置加速度传感器的中断触发阈值 (0-40)"
                }
                p { class: "text-[10px] text-gray-500 mt-0.5", "UUID: 0xFFE3" }
            }

            ThresholdSlider {
                label: "加速度计中断阈值",
                value,
                max_value: 40,
                percentage,
                color: "purple",
                on_change,
            }
        }
    }
}

#[component]
fn ThresholdSlider(
    label: String,
    value: i32,
    max_value: i32,
    percentage: i32,
    color: &'static str,
    on_change: EventHandler<i32>,
) -> Element {
    let (slider_color, text_color) = get_slider_colors(color);

    rsx! {
        div { class: "space-y-3",
            div { class: "flex items-center justify-between",
                label { class: "text-sm font-medium text-gray-300", "{label}" }
                div { class: "flex items-center gap-2",
                    input {
                        r#type: "number",
                        min: "0",
                        max: "{max_value}",
                        value: "{value}",
                        class: "w-20 px-2 py-1 text-sm font-semibold {text_color} bg-[#2a2a2a] border border-[#3a3a3a] rounded text-right focus:outline-none focus:border-[#60cd18]",
                        oninput: move |evt| {
                            if let Ok(val) = evt.value().parse::<i32>() {
                                if val >= 0 && val <= max_value {
                                    on_change.call(val);
                                }
                            }
                        },
                    }
                    span { class: "text-sm text-gray-500", "/ {max_value}" }
                }
            }

            div { class: "flex items-center gap-4",
                input {
                    r#type: "range",
                    min: "0",
                    max: "{max_value}",
                    value: "{value}",
                    class: "flex-1 h-2 rounded-lg appearance-none cursor-pointer {slider_color}",
                    oninput: move |evt| {
                        if let Ok(val) = evt.value().parse::<i32>() {
                            on_change.call(val);
                        }
                    },
                }
                span { class: "text-sm text-gray-400 min-w-12 text-right", "{percentage}%" }
            }
        }
    }
}

fn get_slider_colors(color: &str) -> (&'static str, &'static str) {
    match color {
        "blue" => (
            "bg-blue-500 [&::-webkit-slider-thumb]:bg-blue-400 [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:w-4 [&::-webkit-slider-thumb]:h-4 [&::-webkit-slider-thumb]:rounded-full [&::-webkit-slider-thumb]:cursor-pointer",
            "text-blue-400"
        ),
        "red" => (
            "bg-red-500 [&::-webkit-slider-thumb]:bg-red-400 [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:w-4 [&::-webkit-slider-thumb]:h-4 [&::-webkit-slider-thumb]:rounded-full [&::-webkit-slider-thumb]:cursor-pointer",
            "text-red-400"
        ),
        "purple" => (
            "bg-purple-500 [&::-webkit-slider-thumb]:bg-purple-400 [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:w-4 [&::-webkit-slider-thumb]:h-4 [&::-webkit-slider-thumb]:rounded-full [&::-webkit-slider-thumb]:cursor-pointer",
            "text-purple-400"
        ),
        _ => (
            "bg-gray-500 [&::-webkit-slider-thumb]:bg-gray-400 [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:w-4 [&::-webkit-slider-thumb]:h-4 [&::-webkit-slider-thumb]:rounded-full [&::-webkit-slider-thumb]:cursor-pointer",
            "text-gray-400"
        ),
    }
}
