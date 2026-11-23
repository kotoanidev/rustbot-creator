//! Settings (D-130): an editor tab with the language, the theme (D-127), the font size and
//! the private toolchain's state.

use std::path::Path;

use gpui_kit::component::select::{Select, SelectEvent, SelectState};
use gpui_kit::component::{ActiveTheme as _, IndexPath, ThemeRegistry};
use gpui_kit::component::{h_flex, v_flex};
use gpui_kit::{
    AppContext as _, Context, Entity, FocusHandle, FontWeight, IntoElement, ParentElement as _,
    Render, SharedString, Styled as _, Subscription, Window, div, px,
};

use crate::i18n::{t, t_with};
use crate::settings::{self, FONT_SIZES};

type Choice = Entity<SelectState<Vec<SharedString>>>;

pub(crate) struct SettingsPanel {
    focus: FocusHandle,
    language: Choice,
    theme: Choice,
    font_size: Choice,
    /// The themes the theme list shows after "Follow the system", to notice new ones.
    themes: Vec<SharedString>,
    /// The toolchain's state, once it is measured.
    toolchain: Option<String>,
    _changes: Vec<Subscription>,
}

/// The languages, as the list shows them: the system's, then each in its own name.
const LANGUAGES: [&str; 3] = ["", "en", "pl"];

fn language_labels() -> Vec<SharedString> {
    LANGUAGES
        .iter()
        .map(|&code| match code {
            "en" => "English".into(),
            "pl" => "Polski".into(),
            _ => t("settings-language-system").into(),
        })
        .collect()
}

/// The themes the registry knows, by name, built-in and the user's.
fn theme_names(cx: &gpui_kit::App) -> Vec<SharedString> {
    ThemeRegistry::global(cx)
        .sorted_themes()
        .iter()
        .map(|theme| theme.name.clone())
        .collect()
}

fn theme_labels(themes: &[SharedString]) -> Vec<SharedString> {
    let mut labels = vec![t("settings-theme-system").into()];
    labels.extend(themes.iter().cloned());
    labels
}

impl SettingsPanel {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<'_, Self>) -> Self {
        let chosen = settings::get(cx).clone();
        let themes = theme_names(cx);
        let at = |ix: Option<usize>| Some(IndexPath::new(ix.unwrap_or(0)));
        let language_at = at(LANGUAGES.iter().position(|&l| l == chosen.language));
        let theme_at = at(themes
            .iter()
            .position(|name| name.as_ref() == chosen.theme)
            .map(|ix| ix + 1));
        let size_at = at(FONT_SIZES.iter().position(|&s| s == chosen.font_size));
        let sizes: Vec<SharedString> = FONT_SIZES.iter().map(|s| s.to_string().into()).collect();
        let language = cx.new(|cx| SelectState::new(language_labels(), language_at, window, cx));
        let theme = cx.new(|cx| SelectState::new(theme_labels(&themes), theme_at, window, cx));
        let font_size = cx.new(|cx| SelectState::new(sizes, size_at, window, cx));
        let changes = vec![
            cx.subscribe_in(&language, window, |this, select, event, window, cx| {
                let SelectEvent::Confirm(Some(_)) = event else {
                    return;
                };
                let ix = select.read(cx).selected_index(cx).map_or(0, |ix| ix.row);
                settings::change(cx, |settings| {
                    LANGUAGES[ix].clone_into(&mut settings.language);
                });
                // The lists' own words follow the new language.
                let themes = this.themes.clone();
                this.language
                    .update(cx, |list, cx| list.set_items(language_labels(), window, cx));
                this.theme.update(cx, |list, cx| {
                    list.set_items(theme_labels(&themes), window, cx);
                });
            }),
            cx.subscribe_in(&theme, window, |this, select, event, _, cx| {
                let SelectEvent::Confirm(Some(_)) = event else {
                    return;
                };
                let ix = select.read(cx).selected_index(cx).map_or(0, |ix| ix.row);
                let name = ix
                    .checked_sub(1)
                    .and_then(|ix| this.themes.get(ix))
                    .map_or_else(String::new, ToString::to_string);
                settings::change(cx, |settings| settings.theme = name);
            }),
            cx.subscribe_in(&font_size, window, |_, select, event, _, cx| {
                let SelectEvent::Confirm(Some(_)) = event else {
                    return;
                };
                if let Some(ix) = select.read(cx).selected_index(cx) {
                    settings::change(cx, |settings| settings.font_size = FONT_SIZES[ix.row]);
                }
            }),
        ];
        cx.spawn(async move |this, cx| {
            let state = cx.background_spawn(async move { toolchain_state() }).await;
            let _ = this.update(cx, |this, cx| {
                this.toolchain = Some(state);
                cx.notify();
            });
        })
        .detach();
        Self {
            focus: cx.focus_handle(),
            language,
            theme,
            font_size,
            themes,
            toolchain: None,
            _changes: changes,
        }
    }
}

/// Whether the private Rust is installed (D-017), where, and the room it takes.
fn toolchain_state() -> String {
    let Ok(toolchain) = rustbot_toolchain::Toolchain::locate() else {
        return t("settings-toolchain-missing");
    };
    if !toolchain.is_installed() {
        return t("settings-toolchain-missing");
    }
    let data = toolchain.data_dir();
    let bytes = dir_size(&data.join("rustup")) + dir_size(&data.join("cargo"));
    #[allow(clippy::cast_precision_loss)] // a size shown to one decimal
    let size = format!("{:.1} GB", bytes as f64 / 1e9);
    t_with(
        "settings-toolchain-installed",
        &[
            ("version", rustbot_core::RUST_VERSION),
            ("path", &data.display().to_string()),
            ("size", &size),
        ],
    )
}

/// The bytes of the files under `path`, not following symbolic links.
fn dir_size(path: &Path) -> u64 {
    let Ok(entries) = std::fs::read_dir(path) else {
        return 0;
    };
    entries
        .flatten()
        .map(|entry| match entry.metadata() {
            Ok(meta) if meta.is_dir() => dir_size(&entry.path()),
            Ok(meta) => meta.len(),
            Err(_) => 0,
        })
        .sum()
}

impl Render for SettingsPanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        // The user's themes load in the background and change while the app runs.
        let themes = theme_names(cx);
        if themes != self.themes {
            let labels = theme_labels(&themes);
            self.themes = themes;
            self.theme
                .update(cx, |list, cx| list.set_items(labels, window, cx));
        }
        let theme = cx.theme();
        let row = |label: String, select: &Choice| {
            h_flex()
                .gap(px(12.))
                .child(div().w(px(160.)).child(label))
                .child(div().w(px(260.)).child(Select::new(select)))
        };
        let hint = |text: String| {
            div()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(text)
        };
        let heading = |text: String| div().font_weight(FontWeight::SEMIBOLD).child(text);
        let folder = settings::config_dir()
            .map_or_else(String::new, |dir| dir.join("themes").display().to_string());
        v_flex()
            .size_full()
            .p(px(16.))
            .gap(px(12.))
            .bg(theme.background)
            .text_color(theme.foreground)
            .child(heading(t("settings-appearance")))
            .child(row(t("settings-language"), &self.language))
            .child(row(t("settings-theme"), &self.theme))
            .child(hint(t_with("settings-themes-folder", &[("path", &folder)])))
            .child(row(t("settings-font-size"), &self.font_size))
            .child(heading(t("settings-toolchain")))
            .child(hint(
                self.toolchain
                    .clone()
                    .unwrap_or_else(|| t("settings-toolchain-measuring")),
            ))
    }
}

impl gpui_kit::component::dock::BasePanel for SettingsPanel {
    fn panel_name(&self) -> &'static str {
        "settings"
    }

    fn closable(&self, _: &gpui_kit::App) -> bool {
        true
    }
}

impl gpui_kit::component::dock::Panel for SettingsPanel {
    fn tab_name(&self, _: &gpui_kit::App) -> Option<SharedString> {
        Some(t("settings").into())
    }

    fn title(&mut self, _: &mut Window, _: &mut Context<'_, Self>) -> impl IntoElement {
        SharedString::from(t("settings"))
    }
}

impl gpui_kit::EventEmitter<gpui_kit::component::dock::PanelEvent> for SettingsPanel {}

impl gpui_kit::Focusable for SettingsPanel {
    fn focus_handle(&self, _: &gpui_kit::App) -> FocusHandle {
        self.focus.clone()
    }
}
