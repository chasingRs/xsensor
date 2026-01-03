use crate::api::ble_service::use_ble;
use crate::context::use_app_state;
use crate::Route;
use dioxus::prelude::*;
use dioxus_free_icons::icons::fa_brands_icons::FaBluetooth;
use dioxus_free_icons::icons::fa_solid_icons::{FaChartLine, FaSliders};
use dioxus_free_icons::Icon;

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
                        // 如果检查失败，也认为断开连接
                        app_state.connected_device_id.set(String::new());
                        app_state.scanned_devices.set(ble.get_devices().await);
                    }
                }
            }
        }
    });

    rsx! {
        div { class: "flex h-screen w-full bg-[#0f0f0f] text-gray-100 font-sans overflow-hidden",
            nav { class: "w-48 bg-[#1f1f1f] flex flex-col py-4",
                div { class: "px-4 mb-4 text-lg font-semibold", "XSensor" }

                Link {
                    to: Route::Connection {},
                    class: "flex items-center px-4 py-3 cursor-pointer no-underline text-[#e0e0e0] transition-colors duration-200 text-sm hover:bg-[#303030]",
                    active_class: "bg-[#303030] relative before:content-[''] before:absolute before:left-0 before:top-2 before:bottom-2 before:w-1 before:bg-[#60cd18]",
                    Icon {
                        width: 18,
                        height: 18,
                        icon: FaBluetooth,
                        class: "mr-3",
                    }
                    span { "连接" }
                }

                Link {
                    to: Route::Status {},
                    class: "flex items-center px-4 py-3 cursor-pointer no-underline text-[#e0e0e0] transition-colors duration-200 text-sm hover:bg-[#303030]",
                    active_class: "bg-[#303030] relative before:content-[''] before:absolute before:left-0 before:top-2 before:bottom-2 before:w-1 before:bg-[#60cd18]",
                    Icon {
                        width: 18,
                        height: 18,
                        icon: FaChartLine,
                        class: "mr-3",
                    }
                    span { "状态" }
                }

                Link {
                    to: Route::Parameters {},
                    class: "flex items-center px-4 py-3 cursor-pointer no-underline text-[#e0e0e0] transition-colors duration-200 text-sm hover:bg-[#303030]",
                    active_class: "bg-[#303030] relative before:content-[''] before:absolute before:left-0 before:top-2 before:bottom-2 before:w-1 before:bg-[#60cd18]",
                    Icon {
                        width: 18,
                        height: 18,
                        icon: FaSliders,
                        class: "mr-3",
                    }
                    span { "调参" }
                }
            }

            main { class: "flex-1 overflow-y-auto bg-[#1b1b1b] px-4 py-4", Outlet::<Route> {} }
        }
    }
}
