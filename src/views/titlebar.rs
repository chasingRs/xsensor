use crate::context::use_app_state;
use dioxus::desktop::window;
use dioxus::prelude::*;
use dioxus_free_icons::icons::fa_solid_icons::{
    FaCompress, FaExpand, FaMinus, FaMoon, FaSun, FaXmark,
};
use dioxus_free_icons::Icon;

#[component]
pub fn TitleBar() -> Element {
    let mut app_state = use_app_state();
    let theme = app_state.theme.read().clone();
    // 跟踪最大化状态（初始值读取窗口实际状态）
    let mut is_maximized = use_signal(|| window().window.is_maximized());

    let title_bar_bg     = theme.pick("bg-[#1a1a1a]", "bg-[#e8e8e8]");
    let title_text_color = theme.pick("text-gray-100", "text-gray-800");
    let title_bar_border = theme.pick("border-[#2a2a2a]", "border-[#d0d0d0]");
    let btn_hover        = theme.pick("hover:bg-[#3a3a3a]", "hover:bg-[#cccccc]");
    let close_hover      = "hover:bg-red-500 hover:text-white";

    rsx! {
        div { class: "flex items-center justify-between h-9 px-3 select-none {title_bar_bg} {title_text_color} border-b {title_bar_border}",
            // 拖拽区域：app 图标 + 标题
            div {
                class: "flex items-center gap-2 flex-1 min-w-0 cursor-default",
                onmousedown: move |_| {
                    window().drag();
                },
                div { class: "w-3 h-3 rounded-full bg-[#60cd18] shrink-0" }
                span { class: "text-xs font-semibold tracking-wide truncate", "XSensor" }
            }

            // 右侧按钮组
            div { class: "flex items-center gap-1",
                // 主题切换按钮
                button {
                    class: "flex items-center justify-center w-7 h-7 rounded transition-colors duration-150 {btn_hover}",
                    title: if theme.is_dark() { "切换到浅色主题" } else { "切换到深色主题" },
                    onclick: move |_| {
                        let next = app_state.theme.read().toggle();
                        app_state.theme.set(next);
                    },
                    if theme.is_dark() {
                        Icon { width: 14, height: 14, icon: FaSun }
                    } else {
                        Icon { width: 14, height: 14, icon: FaMoon }
                    }
                }

                // 最小化按钮
                button {
                    class: "flex items-center justify-center w-7 h-7 rounded transition-colors duration-150 {btn_hover}",
                    title: "最小化",
                    onclick: move |_| {
                        window().window.set_minimized(true);
                    },
                    Icon { width: 12, height: 12, icon: FaMinus }
                }

                // 最大化 / 还原按钮
                button {
                    class: "flex items-center justify-center w-7 h-7 rounded transition-colors duration-150 {btn_hover}",
                    title: if is_maximized() { "向下还原" } else { "最大化" },
                    onclick: move |_| {
                        window().toggle_maximized();
                        is_maximized.set(!is_maximized());
                    },
                    if is_maximized() {
                        Icon { width: 12, height: 12, icon: FaCompress }
                    } else {
                        Icon { width: 12, height: 12, icon: FaExpand }
                    }
                }

                // 关闭按钮
                button {
                    class: "flex items-center justify-center w-7 h-7 rounded transition-colors duration-150 {close_hover}",
                    title: "关闭",
                    onclick: move |_| {
                        window().close();
                    },
                    Icon { width: 12, height: 12, icon: FaXmark }
                }
            }
        }
    }
}
