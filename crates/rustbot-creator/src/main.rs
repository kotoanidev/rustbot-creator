//! RustBot Creator, the desktop application. `rustbot-creator <folder>` opens a project.

mod canvas;
mod i18n;
mod measure;
mod panels;
mod run;
mod settings;
mod state;
mod templates;
mod trust;
mod workspace;

use gpui_kit::{AppContext as _, Bounds, TitlebarOptions, WindowBounds, WindowOptions, px, size};

fn main() {
    measure::start();
    let path = std::env::args_os().nth(1).map(std::path::PathBuf::from);
    gpui_kit::application().run(move |cx| {
        gpui_kit::init(cx);
        // Without a home folder the defaults are used, and changes are not kept.
        let config = settings::config_dir().unwrap_or_default();
        settings::init(config.join("settings.toml"), cx);
        // The user's own themes (D-127), loaded again whenever one changes.
        let themes = config.join("themes");
        if std::fs::create_dir_all(&themes).is_ok() {
            let _ = gpui_kit::component::ThemeRegistry::watch_dir(themes, cx, |cx| {
                settings::apply(None, cx);
            });
        }

        let options = WindowOptions {
            titlebar: Some(TitlebarOptions {
                title: Some("RustBot Creator".into()),
                ..Default::default()
            }),
            window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                None,
                size(px(1280.), px(800.)),
                cx,
            ))),
            ..Default::default()
        };
        gpui_kit::open_window(options, cx, |window, cx| {
            cx.new(|cx| workspace::Workspace::new(path, window, cx))
        })
        .expect("failed to open the main window");
    });
}
