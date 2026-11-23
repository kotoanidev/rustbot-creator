//! The dock's panels (D-044, D-066). Each holds the shared [`AppState`] and redraws when
//! it changes.

mod editor;
mod logs;
mod problems;
mod project_tree;
mod properties;

pub(crate) use editor::Editor;
pub(crate) use logs::Logs;
pub(crate) use problems::Problems;
pub(crate) use project_tree::ProjectTree;
pub(crate) use properties::Properties;

/// The dock traits of a panel with a `focus` field, named `$name` in saved layouts and
/// `$tab` (a Fluent id) on its tab. Panels cannot be closed yet: nothing reopens them.
macro_rules! dock_panel {
    // Extra items, such as `toolbar_buttons`, go into the `Panel` impl.
    ($panel:ty, $name:literal, $tab:literal $(, $($panel_item:item)*)?) => {
        impl gpui_kit::component::dock::BasePanel for $panel {
            fn panel_name(&self) -> &'static str {
                $name
            }

            fn closable(&self, _: &gpui_kit::App) -> bool {
                false
            }
        }

        impl gpui_kit::component::dock::Panel for $panel {
            fn tab_name(&self, _: &gpui_kit::App) -> Option<gpui_kit::SharedString> {
                Some(crate::i18n::t($tab).into())
            }

            // A group with one panel shows its title instead of a tab.
            fn title(
                &mut self,
                _: &mut gpui_kit::Window,
                _: &mut gpui_kit::Context<'_, Self>,
            ) -> impl gpui_kit::IntoElement {
                gpui_kit::SharedString::from(crate::i18n::t($tab))
            }

            $($($panel_item)*)?
        }

        impl gpui_kit::EventEmitter<gpui_kit::component::dock::PanelEvent> for $panel {}

        impl gpui_kit::Focusable for $panel {
            fn focus_handle(&self, _: &gpui_kit::App) -> gpui_kit::FocusHandle {
                self.focus.clone()
            }
        }
    };
}
pub(crate) use dock_panel;

/// A muted line of help text for an empty panel.
fn placeholder(text: String, cx: &gpui_kit::App) -> gpui_kit::Div {
    use gpui_kit::component::ActiveTheme as _;
    use gpui_kit::{ParentElement as _, Styled as _, px};
    gpui_kit::div()
        .p(px(12.))
        .text_sm()
        .text_color(cx.theme().muted_foreground)
        .child(text)
}
