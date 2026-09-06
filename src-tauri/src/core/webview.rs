use tauri::WebviewUrl;
use tauri::utils::config::{Config, WindowConfig};

pub(crate) fn secondary_window_config(
    config: &Config,
    label: &str,
    url: WebviewUrl,
) -> Result<WindowConfig, String> {
    let main = config
        .app
        .windows
        .iter()
        .find(|window| window.label == "main")
        .ok_or_else(|| "Missing main window configuration".to_string())?;

    // WebView2 rejects different environment options in the same data directory.
    // Inherit those options, not main's size, visibility or other window settings.
    Ok(WindowConfig {
        label: label.to_string(),
        url,
        additional_browser_args: main.additional_browser_args.clone(),
        browser_extensions_enabled: main.browser_extensions_enabled,
        scroll_bar_style: main.scroll_bar_style.clone(),
        ..Default::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tauri::utils::config::ScrollBarStyle;

    fn app_config() -> Config {
        serde_json::from_str(include_str!("../../tauri.conf.json")).unwrap()
    }

    #[test]
    fn secondary_windows_inherit_the_configured_webview_environment() {
        let mut config = app_config();
        let main = config.app.windows[0].clone();
        config.app.windows.insert(
            0,
            WindowConfig {
                label: "other".into(),
                ..Default::default()
            },
        );

        for (label, url) in [
            (
                "microsoft_auth_test",
                WebviewUrl::External("https://login.microsoftonline.com/".parse().unwrap()),
            ),
            ("log-test", WebviewUrl::App("/?log=test".into())),
        ] {
            let window = secondary_window_config(&config, label, url.clone()).unwrap();
            assert_eq!(window.additional_browser_args, main.additional_browser_args);
            assert_eq!(
                window.browser_extensions_enabled,
                main.browser_extensions_enabled
            );
            assert_eq!(window.scroll_bar_style, main.scroll_bar_style);
            assert_eq!(window.label, label);
            assert_eq!(window.url, url);
            assert!(window.create);
            assert_eq!(window.min_width, None);
            assert_eq!(window.min_height, None);
            assert_eq!(window.devtools, None);
        }
    }

    #[test]
    fn environment_options_follow_main_without_replacing_default_arguments() {
        let mut config = app_config();
        let main = &mut config.app.windows[0];
        main.additional_browser_args = None;
        main.browser_extensions_enabled = true;
        main.scroll_bar_style = ScrollBarStyle::FluentOverlay;

        let window =
            secondary_window_config(&config, "log-test", WebviewUrl::App("/?log=test".into()))
                .unwrap();
        assert_eq!(window.additional_browser_args, None);
        assert!(window.browser_extensions_enabled);
        assert_eq!(window.scroll_bar_style, ScrollBarStyle::FluentOverlay);
    }

    #[test]
    fn missing_main_configuration_is_an_error() {
        let mut config = app_config();
        config.app.windows.clear();
        assert!(
            secondary_window_config(&config, "log-test", WebviewUrl::App("index.html".into()))
                .is_err()
        );
    }
}
