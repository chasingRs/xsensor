use dioxus::prelude::*;

/// 通用空状态提示组件（未连接设备等场景）
#[component]
pub fn EmptyState(message: &'static str, hint: &'static str) -> Element {
    rsx! {
        div { class: "flex items-center justify-center h-full",
            div { class: "text-center space-y-4",
                div { class: "text-gray-500 text-lg", "{message}" }
                div { class: "text-gray-600 text-sm", "{hint}" }
            }
        }
    }
}
