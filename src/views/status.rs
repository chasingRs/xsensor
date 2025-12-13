use crate::api::ble_service::use_ble;
use crate::context::use_connected_device;
use dioxus::prelude::*;
use futures::StreamExt;

const SERVICE_UUID: &str = "0000ffe0-0000-1000-8000-00805f9b34fb";
const PS_DATA_UUID: &str = "0000ffe6-0000-1000-8000-00805f9b34fb";
const ACC_DATA_UUID: &str = "0000ffe7-0000-1000-8000-00805f9b34fb";
const PS_FREQ_UUID: &str = "0000ffe8-0000-1000-8000-00805f9b34fb";
const PS_INT_UUID: &str = "0000ffe4-0000-1000-8000-00805f9b34fb";
const ACC_INT_UUID: &str = "0000ffe5-0000-1000-8000-00805f9b34fb";
const BATTERY_SERVICE_UUID: &str = "0000180f-0000-1000-8000-00805f9b34fb";
const BATTERY_LEVEL_UUID: &str = "00002a19-0000-1000-8000-00805f9b34fb";

#[component]
pub fn Status() -> Element {
    let connected_device = use_connected_device();
    let ble = use_ble();
    let device_id = connected_device.id.read().clone();

    // 真实数据状态
    let mut battery_level = use_signal(|| 0);
    let mut sensor_data_1 = use_signal(|| vec![0.0; 50]); // PS Data
    let mut sensor_data_2 = use_signal(|| vec![0.0; 50]); // ACC Data
    let mut trigger_count_1 = use_signal(|| 0); // PS Interrupt
    let mut trigger_count_2 = use_signal(|| 0); // ACC Interrupt
    let mut trigger_active_1 = use_signal(|| false);
    let mut trigger_active_2 = use_signal(|| false);
    let mut ps_refresh_freq = use_signal(|| 5); // 刷新频率
    let mut acc_refresh_freq = use_signal(|| 5);
    let mut is_loaded = use_signal(|| false);

    let device_id_ps_write = device_id.clone();
    let write_ps_freq_task = use_coroutine(move |mut rx: UnboundedReceiver<i32>| {
        let id = device_id_ps_write.clone();
        async move {
            while let Some(new_val) = rx.next().await {
                let acc_val = acc_refresh_freq();
                let freq_byte = (new_val & 0x0F) | ((acc_val & 0x0F) << 4);
                if let Err(e) = ble
                    .write(&id, SERVICE_UUID, PS_FREQ_UUID, vec![freq_byte as u8])
                    .await
                {
                    error!("Failed to write PS freq: {}", e);
                }
            }
        }
    });
    let on_ps_freq_change = move |new_val: i32| {
        ps_refresh_freq.set(new_val);
        write_ps_freq_task.send(new_val);
    };

    let device_id_acc_write = device_id.clone();
    let write_acc_freq_task = use_coroutine(move |mut rx: UnboundedReceiver<i32>| {
        let id = device_id_acc_write.clone();
        async move {
            while let Some(new_val) = rx.next().await {
                let ps_val = ps_refresh_freq();
                let freq_byte = (ps_val & 0x0F) | ((new_val & 0x0F) << 4);
                if let Err(e) = ble
                    .write(&id, SERVICE_UUID, PS_FREQ_UUID, vec![freq_byte as u8])
                    .await
                {
                    error!("Failed to write ACC freq: {}", e);
                }
            }
        }
    });
    let on_acc_freq_change = move |new_val: i32| {
        acc_refresh_freq.set(new_val);
        write_acc_freq_task.send(new_val);
    };

    // 读取刷新频率和电池电量
    use_resource(move || {
        let id = connected_device.id.read().clone();
        async move {
            if id.is_empty() {
                return;
            }

            // Wait for services to be discovered
            loop {
                if let Ok(services) = ble.list_characteristics(id.clone()).await {
                    if services.iter().any(|s| s.uuid == SERVICE_UUID) {
                        break;
                    }
                }
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            }

            // 读取刷新频率特性 (0xFFE8)
            match ble.read(&id, SERVICE_UUID, PS_FREQ_UUID).await {
                Ok(data) => {
                    if !data.is_empty() {
                        let byte = data[0];
                        let ps_freq = (byte & 0x0F) as i32; // Lower 4 bits
                        let acc_freq = ((byte & 0xF0) >> 4) as i32; // Upper 4 bits
                        ps_refresh_freq.set(ps_freq);
                        acc_refresh_freq.set(acc_freq);
                    }
                }
                Err(e) => {
                    error!("Failed to read freq: {}", e);
                }
            }

            // 读取电池电量 (0x2A19)
            match ble
                .read(&id, BATTERY_SERVICE_UUID, BATTERY_LEVEL_UUID)
                .await
            {
                Ok(data) => {
                    if !data.is_empty() {
                        battery_level.set(data[0] as i32);
                    }
                }
                Err(e) => {
                    error!("Failed to read battery: {}", e);
                }
            }

            is_loaded.set(true);
        }
    });

    // Battery Subscription
    use_resource(move || {
        let id = connected_device.id.read().clone();
        async move {
            if id.is_empty() {
                return;
            }
            // Wait for services to be discovered
            for _ in 0..20 {
                if let Ok(services) = ble.list_characteristics(id.clone()).await {
                    if services.iter().any(|s| s.uuid == BATTERY_SERVICE_UUID) {
                        break;
                    }
                }
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            }
            match ble
                .subscribe(&id, BATTERY_SERVICE_UUID, BATTERY_LEVEL_UUID)
                .await
            {
                Ok(mut stream) => {
                    while let Some(data) = stream.next().await {
                        if !data.value.is_empty() {
                            battery_level.set(data.value[0] as i32);
                        }
                    }
                }
                Err(e) => error!("Failed to subscribe battery: {}", e),
            }
        }
    });

    // PS Polling
    use_resource(move || {
        let id = connected_device.id.read().clone();
        async move {
            if id.is_empty() {
                return;
            }
            // Wait for services to be discovered
            for _ in 0..20 {
                if let Ok(services) = ble.list_characteristics(id.clone()).await {
                    if services.iter().any(|s| s.uuid == SERVICE_UUID) {
                        break;
                    }
                }
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            }
            loop {
                let freq = ps_refresh_freq();
                if freq > 0 {
                    match ble.read(&id, SERVICE_UUID, PS_DATA_UUID).await {
                        Ok(data) => {
                            if data.len() >= 2 {
                                let val = u16::from_le_bytes([data[0], data[1]]) as f32;
                                sensor_data_1.with_mut(|d| {
                                    d.remove(0);
                                    d.push(val);
                                });
                            }
                        }
                        Err(e) => error!("Failed to read PS data: {}", e),
                    }
                    let delay = 1000 / freq as u64;
                    tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                } else {
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                }
            }
        }
    });

    // ACC Polling
    use_resource(move || {
        let id = connected_device.id.read().clone();
        async move {
            if id.is_empty() {
                return;
            }
            // Wait for services to be discovered
            for _ in 0..20 {
                if let Ok(services) = ble.list_characteristics(id.clone()).await {
                    if services.iter().any(|s| s.uuid == SERVICE_UUID) {
                        break;
                    }
                }
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            }
            loop {
                let freq = acc_refresh_freq();
                if freq > 0 {
                    match ble.read(&id, SERVICE_UUID, ACC_DATA_UUID).await {
                        Ok(data) => {
                            if data.len() >= 4 {
                                let val = f32::from_le_bytes([data[0], data[1], data[2], data[3]]);
                                sensor_data_2.with_mut(|d| {
                                    d.remove(0);
                                    d.push(val);
                                });
                            }
                        }
                        Err(e) => error!("Failed to read ACC data: {}", e),
                    }
                    let delay = 1000 / freq as u64;
                    tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                } else {
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                }
            }
        }
    });

    // PS Interrupt Subscription
    use_resource(move || {
        let id = connected_device.id.read().clone();
        async move {
            if id.is_empty() {
                return;
            }
            // Wait for services to be discovered
            for _ in 0..20 {
                if let Ok(services) = ble.list_characteristics(id.clone()).await {
                    if services.iter().any(|s| s.uuid == SERVICE_UUID) {
                        break;
                    }
                }
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            }
            match ble.subscribe(&id, SERVICE_UUID, PS_INT_UUID).await {
                Ok(mut stream) => {
                    while let Some(_) = stream.next().await {
                        trigger_count_1.with_mut(|c| *c += 1);
                        trigger_active_1.set(true);
                        let mut active = trigger_active_1.clone();
                        spawn(async move {
                            tokio::time::sleep(std::time::Duration::from_millis(150)).await;
                            active.set(false);
                        });
                    }
                }
                Err(e) => error!("Failed to subscribe PS int: {}", e),
            }
        }
    });

    // ACC Interrupt Subscription
    use_resource(move || {
        let id = connected_device.id.read().clone();
        async move {
            if id.is_empty() {
                return;
            }
            // Wait for services to be discovered
            loop {
                if let Ok(services) = ble.list_characteristics(id.clone()).await {
                    if services.iter().any(|s| s.uuid == SERVICE_UUID) {
                        break;
                    }
                }
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            }
            match ble.subscribe(&id, SERVICE_UUID, ACC_INT_UUID).await {
                Ok(mut stream) => {
                    while let Some(_) = stream.next().await {
                        trigger_count_2.with_mut(|c| *c += 1);
                        trigger_active_2.set(true);
                        let mut active = trigger_active_2.clone();
                        spawn(async move {
                            tokio::time::sleep(std::time::Duration::from_millis(150)).await;
                            active.set(false);
                        });
                    }
                }
                Err(e) => error!("Failed to subscribe ACC int: {}", e),
            }
        }
    });

    rsx! {
        div { class: "h-full w-full text-gray-100 overflow-auto relative",
            if device_id.is_empty() {
                StatusEmptyState {}
            } else {
                if !is_loaded() {
                    div { class: "absolute inset-0 z-50 flex items-center justify-center bg-[#1b1b1b]/80 backdrop-blur-sm",
                        div { class: "flex flex-col items-center gap-3",
                            span { class: "animate-spin inline-block w-8 h-8 border-4 border-[#60cd18] border-t-transparent rounded-full" }
                            span { class: "text-sm text-gray-300 font-medium", "正在发现服务..." }
                        }
                    }
                }
                StatusContent {
                    battery_level: battery_level(),
                    sensor_data_1: sensor_data_1(),
                    sensor_data_2: sensor_data_2(),
                    trigger_count_1: trigger_count_1(),
                    trigger_count_2: trigger_count_2(),
                    trigger_active_1: trigger_active_1(),
                    trigger_active_2: trigger_active_2(),
                    ps_freq: ps_refresh_freq(),
                    acc_freq: acc_refresh_freq(),
                    on_ps_freq_change,
                    on_acc_freq_change,
                    on_reset_ps: move |_| trigger_count_1.set(0),
                    on_reset_acc: move |_| trigger_count_2.set(0),
                }
            }
        }
    }
}

#[component]
fn StatusEmptyState() -> Element {
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
fn StatusContent(
    battery_level: i32,
    sensor_data_1: Vec<f32>,
    sensor_data_2: Vec<f32>,
    trigger_count_1: i32,
    trigger_count_2: i32,
    trigger_active_1: bool,
    trigger_active_2: bool,
    ps_freq: i32,
    acc_freq: i32,
    on_ps_freq_change: EventHandler<i32>,
    on_acc_freq_change: EventHandler<i32>,
    on_reset_ps: EventHandler<()>,
    on_reset_acc: EventHandler<()>,
) -> Element {
    rsx! {
        div { class: "w-full px-4 py-4 space-y-4",
            StatusHeader { battery_level }
            SensorCharts {
                ps_data: sensor_data_1,
                acc_data: sensor_data_2,
                ps_freq,
                acc_freq,
                on_ps_freq_change,
                on_acc_freq_change,
            }
            TriggerCounters {
                ps_int_count: trigger_count_1,
                acc_int_count: trigger_count_2,
                ps_int_active: trigger_active_1,
                acc_int_active: trigger_active_2,
                on_reset_ps,
                on_reset_acc,
            }
        }
    }
}

#[component]
fn StatusHeader(battery_level: i32) -> Element {
    let (battery_color, battery_bg) = get_battery_style(battery_level);

    rsx! {
        div { class: "flex items-center justify-between",
            h1 { class: "text-xl font-semibold tracking-tight", "设备状态" }
            BatteryIndicator { level: battery_level, color: battery_color, bg: battery_bg }
        }
    }
}

#[component]
fn BatteryIndicator(level: i32, color: &'static str, bg: &'static str) -> Element {
    rsx! {
        div { class: "flex items-center gap-2 px-3 py-1.5 rounded-lg border border-[#2a2a2a] bg-[#1f1f1f]",
            div { class: "flex items-center gap-2",
                div { class: "relative w-8 h-4 border border-gray-400 rounded",
                    div {
                        class: "absolute top-0 left-0 h-full rounded-sm transition-all duration-300 {bg}",
                        style: "width: {level}%",
                    }
                    div { class: "absolute -right-0.5 top-1/2 -translate-y-1/2 w-0.5 h-1.5 bg-gray-400 rounded-r" }
                }
                span { class: "text-sm font-semibold {color}", "{level}%" }
            }
        }
    }
}

#[component]
fn SensorCharts(
    ps_data: Vec<f32>,
    acc_data: Vec<f32>,
    ps_freq: i32,
    acc_freq: i32,
    on_ps_freq_change: EventHandler<i32>,
    on_acc_freq_change: EventHandler<i32>,
) -> Element {
    rsx! {
        div { class: "grid grid-cols-1 lg:grid-cols-2 gap-4",
            SensorCard {
                title: "PS 传感器数值",
                subtitle: "接近传感器实时数据 (UUID: 0xFFE6)",
                current_value: ps_data.last().copied().unwrap_or(0.0),
                data: ps_data,
                color: "#60cd18",
                freq: ps_freq,
                max_value: 4096.0,
                on_freq_change: on_ps_freq_change,
            }
            SensorCard {
                title: "ACC 传感器数值",
                subtitle: "加速度传感器实时数据 (UUID: 0xFFE7)",
                current_value: acc_data.last().copied().unwrap_or(0.0),
                data: acc_data,
                color: "#2d6cdf",
                freq: acc_freq,
                max_value: 20.0, // 假设加速度最大值为 20.0
                on_freq_change: on_acc_freq_change,
            }
        }
    }
}

#[component]
fn SensorCard(
    title: String,
    subtitle: String,
    current_value: f32,
    data: Vec<f32>,
    color: String,
    freq: i32,
    max_value: f64,
    on_freq_change: EventHandler<i32>,
) -> Element {
    let value_color = if color == "#60cd18" {
        "text-[#60cd18]"
    } else {
        "text-[#2d6cdf]"
    };

    rsx! {
        div { class: "rounded-xl border border-[#2a2a2a] bg-[#1f1f1f] p-4 space-y-2",
            div {
                div { class: "flex items-center justify-between mb-1",
                    h2 { class: "text-base font-medium", "{title}" }
                    div { class: "flex items-center gap-2 text-xs text-gray-400",
                        span { "Freq:" }
                        input {
                            class: "w-10 bg-[#2a2a2a] border border-gray-600 rounded px-1 text-center text-white focus:outline-none focus:border-blue-500",
                            r#type: "number",
                            min: "0",
                            max: "7",
                            value: "{freq}",
                            oninput: move |e| on_freq_change.call(e.value().parse().unwrap_or(freq)),
                        }
                        span { "Hz" }
                    }
                }
                p { class: "text-[10px] text-gray-500", "{subtitle}" }
            }
            div { class: "text-xl font-bold {value_color}", "{current_value:.2}" }
            ChartView { data, color, max_value }
        }
    }
}

#[component]
fn TriggerCounters(
    ps_int_count: i32,
    acc_int_count: i32,
    ps_int_active: bool,
    acc_int_active: bool,
    on_reset_ps: EventHandler<()>,
    on_reset_acc: EventHandler<()>,
) -> Element {
    rsx! {
        div { class: "rounded-xl border border-[#2a2a2a] bg-[#1f1f1f] p-4",
            div { class: "mb-3",
                h2 { class: "text-base font-medium", "中断通知计数器" }
                p { class: "text-[10px] text-gray-500 mt-0.5", "接收来自设备的中断通知" }
            }
            div { class: "grid grid-cols-1 md:grid-cols-2 gap-4",
                TriggerCounter {
                    label: "PS 中断通知",
                    subtitle: "UUID: 0xFFE4",
                    count: ps_int_count,
                    is_active: ps_int_active,
                    color: "emerald",
                    on_reset: on_reset_ps,
                }
                TriggerCounter {
                    label: "ACC 中断通知",
                    subtitle: "UUID: 0xFFE5",
                    count: acc_int_count,
                    is_active: acc_int_active,
                    color: "blue",
                    on_reset: on_reset_acc,
                }
            }
        }
    }
}

#[component]
fn TriggerCounter(
    label: String,
    subtitle: String,
    count: i32,
    is_active: bool,
    color: &'static str,
    on_reset: EventHandler<()>,
) -> Element {
    let (text_color, border_color, bg_color, ripple_color) = get_trigger_colors(color);

    let card_class = if is_active {
        format!(
            "rounded-lg border-2 {} {} p-3 space-y-1 transition-all duration-150",
            border_color, bg_color
        )
    } else {
        "rounded-lg border border-[#2a2a2a] bg-[#191919] p-3 space-y-1 transition-all duration-150"
            .to_string()
    };

    rsx! {
        div { class: "relative",
            // 波纹效果层 - 使用 pointer-events-none 避免影响布局
            if is_active {
                div {
                    class: "absolute inset-0 rounded-lg animate-ripple pointer-events-none z-0",
                    style: "--ripple-color: {ripple_color};",
                }
                div {
                    class: "absolute inset-0 rounded-lg pointer-events-none z-0",
                    style: "box-shadow: 0 0 30px 10px {ripple_color}; opacity: 0.6;",
                }
            }

            // 卡片内容 - 固定高度避免布局变化
            div { class: "{card_class} relative h-[120px] flex flex-col justify-between z-10",
                // 标题区域 - 固定高度
                div { class: "flex flex-col gap-0.5 min-h-8",
                    div { class: "flex items-center justify-between min-h-4",
                        div { class: "text-xs text-gray-400 leading-none", "{label}" }
                        div { class: "flex items-center justify-end min-w-16 min-h-4",
                            div {
                                class: format!(
                                    "text-[10px] px-1.5 py-0.5 rounded-full {} {} font-semibold leading-none transition-opacity duration-150 {}",
                                    bg_color,
                                    text_color,
                                    if is_active { "opacity-100 animate-pulse" } else { "opacity-0" },
                                ),
                                "触发!"
                            }
                        }
                    }
                    div { class: "text-[10px] text-gray-500 leading-none min-h-3",
                        "{subtitle}"
                    }
                }

                // 固定高度的数字容器 - 不再缩放字号，只改颜色/阴影
                div { class: "h-12 flex items-center overflow-hidden",
                    div {
                        class: format!(
                            "text-2xl font-bold {} transition-all duration-150 leading-none",
                            text_color,
                        ),
                        style: if is_active { "text-shadow: 0 0 12px rgba(255,255,255,0.35);" } else { "" },
                        "{count}"
                    }
                }

                // 底部操作区 - 固定高度
                div { class: "h-6 flex items-center",
                    button {
                        class: "text-[10px] px-2 py-0.5 rounded bg-[#2a2a2a] hover:bg-[#333] cursor-pointer text-gray-300 transition-colors",
                        onclick: move |_| on_reset.call(()),
                        "重置"
                    }
                }
            }
        }
    }
}

fn get_trigger_colors(color: &str) -> (&'static str, &'static str, &'static str, &'static str) {
    match color {
        "emerald" => (
            "text-emerald-400",
            "border-emerald-500",
            "bg-emerald-500/20",
            "rgba(16, 185, 129, 0.6)", // emerald-500
        ),
        "blue" => (
            "text-blue-400",
            "border-blue-500",
            "bg-blue-500/20",
            "rgba(59, 130, 246, 0.6)", // blue-500
        ),
        "purple" => (
            "text-purple-400",
            "border-purple-500",
            "bg-purple-500/20",
            "rgba(168, 85, 247, 0.6)", // purple-500
        ),
        _ => (
            "text-gray-400",
            "border-gray-500",
            "bg-gray-500/20",
            "rgba(107, 114, 128, 0.6)", // gray-500
        ),
    }
}

#[component]
fn ChartView(data: Vec<f32>, color: String, max_value: f64) -> Element {
    let path_data = generate_chart_path(&data, max_value);

    rsx! {
        div { class: "w-full h-32 bg-[#161616] rounded-lg p-3 border border-[#2a2a2a]",
            svg {
                class: "w-full h-full",
                view_box: "0 0 100 150",
                preserve_aspect_ratio: "none",
                ChartGrid {}
                ChartLine { path: path_data.line_path, color: color.clone() }
                ChartFill { path: path_data.fill_path, color }
            }
        }
    }
}

#[component]
fn ChartGrid() -> Element {
    rsx! {
        for i in 0..5 {
            line {
                x1: "0",
                y1: "{i as f64 * 150.0 / 4.0}",
                x2: "100",
                y2: "{i as f64 * 150.0 / 4.0}",
                stroke: "#2a2a2a",
                stroke_width: "0.5",
            }
        }
    }
}

#[component]
fn ChartLine(path: String, color: String) -> Element {
    rsx! {
        path {
            d: "{path}",
            fill: "none",
            stroke: "{color}",
            stroke_width: "2",
            stroke_linejoin: "round",
            stroke_linecap: "round",
        }
    }
}

#[component]
fn ChartFill(path: String, color: String) -> Element {
    rsx! {
        path { d: "{path}", fill: "{color}", fill_opacity: "0.1" }
    }
}

struct ChartPath {
    line_path: String,
    fill_path: String,
}

fn generate_chart_path(data: &[f32], max_value: f64) -> ChartPath {
    let chart_height = 150.0;
    let chart_width = 100.0;
    let data_points = data.len();

    let mut path_parts: Vec<String> = Vec::new();

    for (i, value) in data.iter().enumerate() {
        let x = (i as f64 / (data_points - 1) as f64) * chart_width;
        let y = chart_height - ((*value as f64 / max_value) * chart_height);

        if i == 0 {
            path_parts.push(format!("M {:.2} {:.2}", x, y));
        } else {
            path_parts.push(format!("L {:.2} {:.2}", x, y));
        }
    }

    let line_path = path_parts.join(" ");
    let fill_path = format!(
        "{} L {:.2} {:.2} L 0 {:.2} Z",
        line_path, chart_width, chart_height, chart_height
    );

    ChartPath {
        line_path,
        fill_path,
    }
}

fn get_battery_style(level: i32) -> (&'static str, &'static str) {
    if level > 50 {
        ("text-emerald-400", "bg-emerald-500")
    } else if level > 20 {
        ("text-yellow-400", "bg-yellow-500")
    } else {
        ("text-red-400", "bg-red-500")
    }
}
