//! The user's settings (D-130), kept in `~/.config/rustbot-creator/settings.toml`, and
//! applying them: the language, the theme (D-127) and the font size.

use std::path::{Path, PathBuf};

use gpui_kit::component::{Theme, ThemeRegistry};
use gpui_kit::{App, Global, Window, px};

/// What the user chose. A missing or unknown value is the default, so an older or newer
/// file still loads.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub(crate) struct Settings {
    /// `en` or `pl`; empty for the system's.
    pub(crate) language: String,
    /// A theme's name; empty to follow the system with the built-in light and dark.
    pub(crate) theme: String,
    /// The text size in pixels, one of [`FONT_SIZES`].
    pub(crate) font_size: u16,
    /// Whether the first-run wizard was finished or skipped (D-128).
    pub(crate) first_run_done: bool,
}

impl Default for Settings {
    fn default() -> Self {
        DEFAULT.clone()
    }
}

/// The settings before any are read, e.g. in tests.
static DEFAULT: Settings = Settings {
    language: String::new(),
    theme: String::new(),
    font_size: 16,
    first_run_done: false,
};

/// The text sizes Settings offer.
pub(crate) const FONT_SIZES: [u16; 7] = [12, 13, 14, 15, 16, 18, 20];

/// The settings and their file, for the whole app.
struct Store {
    path: PathBuf,
    settings: Settings,
}

impl Global for Store {}

/// The app's settings folder, `~/.config/rustbot-creator`, which also holds the user's
/// themes.
/// The user's home folder, where the file dialogs start.
pub(crate) fn home() -> PathBuf {
    directories::UserDirs::new()
        .map_or_else(|| PathBuf::from("/"), |dirs| dirs.home_dir().to_owned())
}

pub(crate) fn config_dir() -> Option<PathBuf> {
    directories::BaseDirs::new().map(|dirs| dirs.config_dir().join("rustbot-creator"))
}

/// The settings in `path`, or the defaults when there are none. A damaged file is
/// reported and replaced by the defaults the next time a setting changes.
fn load(path: &Path) -> Settings {
    match std::fs::read_to_string(path) {
        Ok(text) => toml::from_str(&text).unwrap_or_else(|error| {
            eprintln!("{}: {error}; using the default settings", path.display());
            Settings::default()
        }),
        Err(_) => Settings::default(),
    }
}

/// Writes a new file and renames it over the old one, so a crash leaves one of them whole.
fn save(path: &Path, settings: &Settings) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let text = toml::to_string(settings).map_err(std::io::Error::other)?;
    let temporary = path.with_extension("toml.tmp");
    std::fs::write(&temporary, text)?;
    std::fs::rename(temporary, path)
}

/// Reads the settings from `path` and applies them. Called once, before the window opens.
pub(crate) fn init(path: PathBuf, cx: &mut App) {
    let settings = load(&path);
    cx.set_global(Store { path, settings });
    apply(None, cx);
}

/// The current settings; the defaults before [`init`].
pub(crate) fn get(cx: &App) -> &Settings {
    cx.try_global::<Store>()
        .map_or(&DEFAULT, |store| &store.settings)
}

/// Changes the settings, saves them and applies them.
pub(crate) fn change(cx: &mut App, edit: impl FnOnce(&mut Settings)) {
    let store = cx.global_mut::<Store>();
    edit(&mut store.settings);
    if let Err(error) = save(&store.path, &store.settings) {
        eprintln!("cannot save {}: {error}", store.path.display());
    }
    apply(None, cx);
}

/// Puts the settings into effect: the language, the theme and the font size. `window`
/// tells the system's light or dark when the theme follows it.
pub(crate) fn apply(window: Option<&mut Window>, cx: &mut App) {
    let settings = get(cx).clone();
    crate::i18n::set_language(&settings.language);
    gpui_kit::component::set_locale(crate::i18n::language());
    let registry = ThemeRegistry::global(cx);
    let chosen = registry.themes().get(settings.theme.as_str()).cloned();
    let (light, dark) = (
        registry.default_light_theme().clone(),
        registry.default_dark_theme().clone(),
    );
    if let Some(config) = chosen {
        let mode = config.mode;
        Theme::update(cx, |theme| {
            if mode.is_dark() {
                theme.dark_theme = config;
            } else {
                theme.light_theme = config;
            }
        });
        Theme::change(mode, None, cx);
    } else {
        Theme::update(cx, |theme| {
            theme.light_theme = light;
            theme.dark_theme = dark;
        });
        Theme::sync_system_appearance(window, cx);
    }
    Theme::update(cx, |theme| {
        theme.font_size = px(f32::from(settings.font_size));
    });
    cx.refresh_windows();
}

#[cfg(test)]
mod tests {
    use super::{Settings, load, save};

    #[test]
    fn settings_last_and_a_damaged_file_gives_the_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config/settings.toml");
        assert_eq!(load(&path), Settings::default());
        let chosen = Settings {
            language: "pl".to_owned(),
            theme: "Default Dark".to_owned(),
            font_size: 14,
            first_run_done: true,
        };
        save(&path, &chosen).unwrap();
        assert_eq!(load(&path), chosen);
        // A newer app's setting is ignored, and a missing one is the default.
        std::fs::write(&path, "theme = \"Nord\"\nzoom = 2\n").unwrap();
        assert_eq!(
            load(&path),
            Settings {
                theme: "Nord".to_owned(),
                ..Settings::default()
            }
        );
        std::fs::write(&path, "font-size = \"big\"").unwrap();
        assert_eq!(load(&path), Settings::default());
    }

    /// A change is saved and the font size reaches the theme.
    #[gpui_kit::test]
    fn a_change_is_saved_and_applied(cx: &mut gpui_kit::TestAppContext) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.toml");
        cx.update(|cx| {
            gpui_kit::init(cx);
            super::init(path.clone(), cx);
            super::change(cx, |settings| settings.font_size = 18);
            assert_eq!(
                gpui_kit::component::Theme::global(cx).font_size,
                gpui_kit::px(18.)
            );
        });
        assert_eq!(load(&path).font_size, 18);
    }
}
