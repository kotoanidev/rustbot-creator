//! RustBot Creator, the desktop application.

use gpui_kit::{
    AppContext, Context, IntoElement, Render, Styled, TitlebarOptions, Window, WindowOptions, div,
};

/// The main window. Empty until the slice adds the dock (D-044).
struct Workspace;

impl Render for Workspace {
    fn render(&mut self, _: &mut Window, _: &mut Context<'_, Self>) -> impl IntoElement {
        div().size_full()
    }
}

fn main() {
    gpui_kit::application().run(|cx| {
        gpui_kit::init(cx);

        let options = WindowOptions {
            titlebar: Some(TitlebarOptions {
                title: Some("RustBot Creator".into()),
                ..Default::default()
            }),
            ..Default::default()
        };
        gpui_kit::open_window(options, cx, |_, cx| cx.new(|_| Workspace))
            .expect("failed to open the main window");
    });
}
