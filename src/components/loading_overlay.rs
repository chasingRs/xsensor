use dioxus::prelude::*;

/// 全屏加载遮罩（绝对定位，覆盖父容器）
#[component]
pub fn LoadingOverlay(message: &'static str) -> Element {
    rsx! {
        div { class: "absolute inset-0 z-50 flex items-center justify-center bg-[#1b1b1b]/80 backdrop-blur-sm",
            div { class: "flex flex-col items-center gap-3",
                span { class: "animate-spin inline-block w-8 h-8 border-4 border-[#60cd18] border-t-transparent rounded-full" }
                span { class: "text-sm text-gray-300 font-medium", "{message}" }
            }
        }
    }
}
