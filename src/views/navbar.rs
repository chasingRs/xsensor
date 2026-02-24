use crate::api::ble_service::use_ble;
use crate::context::use_app_state;
use crate::Route;
use dioxus::prelude::*;
use dioxus_free_icons::icons::fa_brands_icons::FaBluetooth;
use dioxus_free_icons::icons::fa_solid_icons::{FaChartLine, FaSliders};
use dioxus_free_icons::Icon;

use super::TitleBar;

#[component]
pub fn Navbar() -> Element {
    let mut app_state = use_app_state();
    let ble = use_ble();

    // 轮询检查连接状态
    use_future(move || {
        let ble = ble.clone();
        async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                let current_id = app_state.connected_device_id.read().clone();
                if !current_id.is_empty() {
                    if ble.is_connected(current_id.clone()).await {
                        // connected
                    } else {
                        app_state.connected_device_id.set(String::new());
                        app_state.scanned_devices.set(ble.get_devices().await);
                    }
                }
            }
        }
    });

    let theme = app_state.theme.read().clone();

    // 根据主题选取颜色
    let root_bg     = theme.pick("bg-[#0f0f0f]", "bg-[#f0f0f0]");
    let sidebar_bg  = theme.pick("bg-[#1f1f1f]", "bg-[#d8d8d8]");
    let text_color  = theme.pick("text-gray-100", "text-gray-800");
    let link_class  = theme.pick(
        "flex items-center px-4 py-3 cursor-pointer no-underline text-[#e0e0e0] transition-colors duration-200 text-sm hover:bg-[#303030]",
        "flex items-center px-4 py-3 cursor-pointer no-underline text-[#333333] transition-colors duration-200 text-sm hover:bg-[#c8c8c8]",
    );
    let active_class = theme.pick(
        "bg-[#303030] relative before:content-[''] before:absolute before:left-0 before:top-2 before:bottom-2 before:w-1 before:bg-[#60cd18]",
        "bg-[#c0c0c0] relative before:content-[''] before:absolute before:left-0 before:top-2 before:bottom-2 before:w-1 before:bg-[#3aa010]",
    );
    let main_bg     = theme.pick("bg-[#1b1b1b]", "bg-[#f7f7f7]");

    rsx! {
        div { class: "flex flex-col h-screen w-full {root_bg} {text_color} font-sans overflow-hidden",
            // 自定义标题栏
            TitleBar {}

            // 主体区域：侧边栏 + 内容
            div { class: "flex flex-1 overflow-hidden",
                nav { class: "w-48 {sidebar_bg} flex flex-col py-4 shrink-0",
                    Link {
                        to: Route::Connection {},
                        class: "{link_class}",
                        active_class: "{active_class}",
                        Icon { width: 18, height: 18, icon: FaBluetooth, class: "mr-3" }
                        span { "连接" }
                    }

                    Link {
                        to: Route::Status {},
                        class: "{link_class}",
                        active_class: "{active_class}",
                        Icon { width: 18, height: 18, icon: FaChartLine, class: "mr-3" }
                        span { "状态" }
                    }

                    Link {
                        to: Route::Parameters {},
                        class: "{link_class}",
                        active_class: "{active_class}",
                        Icon { width: 18, height: 18, icon: FaSliders, class: "mr-3" }
                        span { "调参" }
                    }
                }

                main { class: "flex-1 overflow-y-auto {main_bg} px-4 py-4",
                    Outlet::<Route> {}
                }
            }
        }
    }
}
