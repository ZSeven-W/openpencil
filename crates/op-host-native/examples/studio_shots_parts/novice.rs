//! Normal-mode onboarding, contextual edit actions and readable recent names.
use super::{home_host, shoot, workspace_host, HomeDevice, HomeFamily, Locale};
use op_editor_core::{size_class::EditorSizeClass, HomeHit, RecentFile, ThemeMode};

pub(super) fn run(out: &str) {
    for (theme, theme_name) in [(ThemeMode::Dark, "dark"), (ThemeMode::Light, "light")] {
        for (compact, width, height, platform) in [
            (false, 1440.0, 1000.0, "desktop"),
            (true, 390.0, 844.0, "mobile"),
        ] {
            let mut home = home_host(Locale::ZhCn, HomeFamily::AppUi, HomeDevice::Mobile);
            {
                let state = home.editor_state_mut();
                state.chat.available_models.clear();
                state.editor_ui.agent_settings.connected.fill(false);
                state.editor_ui.agent_settings.web_served_models = false;
                state.editor_ui.account_ui_available = false;
                state.editor_ui.external_cli_available = !compact;
                state.editor_ui.theme_mode = theme;
                state.editor_ui.home.connect_card_open = true;
                if compact {
                    state.editor_ui.touch = true;
                    state.editor_ui.size_class = EditorSizeClass::Compact;
                }
            }
            shoot(
                &mut home,
                width,
                height,
                out,
                &format!("connect-{platform}-{theme_name}"),
            );
            op_editor_ui::widgets::agent_settings_press_focus::open_builtin_setup(
                home.editor_state_mut(),
                10_000,
            );
            shoot(
                &mut home,
                width,
                height,
                out,
                &format!("setup-{platform}-{theme_name}"),
            );
        }
        let mut work = workspace_host(Locale::ZhCn, "sample.op", HomeFamily::AppUi, 1180.0, 820.0);
        work.editor_state_mut().chat.messages.clear();
        work.editor_state_mut().editor_ui.theme_mode = theme;
        shoot(
            &mut work,
            1180.0,
            820.0,
            out,
            &format!("work-actions-{theme_name}"),
        );
        work.editor_state_mut().chat.messages.push(op_editor_core::ChatMessage::assistant(
            "看了整个页面，下面是最值得先做的三条改进，不会修改设计。\n\n1. **搜索框的提示文字预留空间太窄**：把较长的提示完整显示出来，避免重要内容被右边界裁掉。\n\n2. **底部标签栏的文字挤在一起**：适当拉开每一项之间的距离，让用户更容易看清和点选。\n\n3. **让主要按钮更醒目**：减少旁边不必要的装饰，让第一次打开页面的人知道下一步该做什么。",
        ));
        shoot(
            &mut work,
            1180.0,
            820.0,
            out,
            &format!("chat-wrap-{theme_name}"),
        );
        let mut home = home_host(Locale::ZhCn, HomeFamily::AppUi, HomeDevice::Mobile);
        home.editor_state_mut().editor_ui.theme_mode = theme;
        home.editor_state_mut().editor_ui.recent_files = [
            "咖啡门店季度营销活动发布会完整设计稿-版本一.op",
            "咖啡门店季度营销活动发布会完整设计稿-版本二.op",
            "用户调研与需求分析汇报-终稿.op",
        ]
        .iter()
        .map(|name| RecentFile {
            path: format!("/tmp/{name}"),
            modified_at: 0,
        })
        .collect();
        home.editor_state_mut().editor_ui.home.hover = Some(HomeHit::Recent(0));
        shoot(
            &mut home,
            1180.0,
            1000.0,
            out,
            &format!("recent-names-{theme_name}"),
        );
    }
}
