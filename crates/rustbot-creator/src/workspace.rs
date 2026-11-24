//! The main window: the welcome screen, or the open project's dock (D-044) over a status
//! bar.

use std::path::{Path, PathBuf};

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::dock::{DockArea, DockLayout, DockPlacement, DockSkin, panel_handle};
use gpui_kit::component::{ActiveTheme as _, Sizable as _};
use gpui_kit::component::{h_flex, v_flex};
use gpui_kit::{
    AppContext as _, Context, Entity, FontWeight, InteractiveElement as _, IntoElement,
    ParentElement as _, PathPromptOptions, Render, StatefulInteractiveElement as _, Styled as _,
    Window, div, px,
};
use rustbot_core::Project;

use crate::guide::{Close, Guide};
use crate::i18n::{t, t_with};
use crate::panels::{Editor, Logs, OpenWizard, Problems, ProjectTree, Properties, SettingsPanel};
use crate::run::Run;
use crate::state::AppState;
use crate::wizard::{Done, Wizard};

pub(crate) struct Workspace {
    open: Option<(Entity<AppState>, Entity<DockArea>)>,
    error: Option<String>,
    /// "New project" shows the templates (D-115).
    choosing: bool,
    /// Settings (D-130), made when first opened: a tab of the dock, or the whole window
    /// before a project is open (`showing_settings`).
    settings: Option<Entity<SettingsPanel>>,
    showing_settings: bool,
    /// The first-run wizard (D-128), over everything while it is open.
    wizard: Option<Entity<Wizard>>,
    /// The Developer Portal guide (D-129), over the project while it is open.
    guide: Option<Entity<Guide>>,
    /// The wizard ended at the templates, so the guide follows the new project (D-131).
    guide_next: bool,
}

impl Workspace {
    pub(crate) fn new(
        path: Option<PathBuf>,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> Self {
        crate::measure::first_frame(window);
        // A theme that follows the system follows it while the app runs (D-127).
        crate::settings::apply(Some(window), cx);
        cx.observe_window_appearance(window, |_, window, cx| {
            if crate::settings::get(cx).theme.is_empty() {
                gpui_kit::component::Theme::sync_system_appearance(Some(window), cx);
            }
        })
        .detach();
        let mut this = Self {
            open: None,
            error: None,
            choosing: false,
            settings: None,
            showing_settings: false,
            wizard: None,
            guide: None,
            guide_next: false,
        };
        // A project named on the command line is opened at once; otherwise the wizard
        // comes first until it was finished or skipped.
        if let Some(path) = path {
            this.open_project(&path, window, cx);
        } else if !crate::settings::get(cx).first_run_done {
            this.open_wizard(window, cx);
        }
        this
    }

    fn open_project(&mut self, root: &Path, window: &mut Window, cx: &mut Context<'_, Self>) {
        match Project::open(root) {
            Ok(project) => self.show(project, window, cx),
            Err(e) => self.error = Some(t_with("error-open", &[("error", &e.to_string())])),
        }
        cx.notify();
    }

    fn create_project(
        &mut self,
        root: &Path,
        template: usize,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        self.choosing = false;
        let template = &crate::templates::TEMPLATES[template];
        match crate::templates::create(root, template) {
            Ok(project) => {
                // Made here, so trusted (D-064).
                if let Ok(toolchain) = rustbot_toolchain::Toolchain::locate() {
                    let _ = crate::trust::trust(toolchain.data_dir(), &project.root);
                }
                self.show(project, window, cx);
                if std::mem::take(&mut self.guide_next) {
                    self.open_guide(window, cx);
                }
            }
            Err(e) => self.error = Some(t_with("error-create", &[("error", &e.to_string())])),
        }
        cx.notify();
    }

    fn show(&mut self, project: Project, window: &mut Window, cx: &mut Context<'_, Self>) {
        let state = cx.new(|cx| AppState::new(project, cx));
        cx.observe(&state, |_, _, cx| cx.notify()).detach();
        cx.subscribe_in(
            &state,
            window,
            |this, _, _: &crate::state::OpenGuide, window, cx| {
                this.open_guide(window, cx);
            },
        )
        .detach();
        let tree = cx.new(|cx| ProjectTree::new(state.clone(), cx));
        let properties = cx.new(|cx| Properties::new(state.clone(), window, cx));
        let editor = cx.new(|cx| Editor::new(state.clone(), cx));
        let logs = cx.new(|cx| Logs::new(state.clone(), cx));
        let problems = cx.new(|cx| Problems::new(state.clone(), cx));

        let (dock, _) = DockSkin::dock_area("main", Some(1), window, cx);
        dock.update(cx, |dock, cx| {
            let side = DockLayout::v_split()
                .child(DockLayout::tabs().panel_view(panel_handle(tree), cx), None)
                .child(
                    DockLayout::tabs().panel_view(panel_handle(properties), cx),
                    None,
                );
            dock.set_dock(DockPlacement::Left, side, window, cx);
            dock.set_dock_size(DockPlacement::Left, px(280.), window, cx);
            let center = DockLayout::tabs().panel_view(panel_handle(editor), cx);
            dock.set_center(center, window, cx);
            let bottom = DockLayout::tabs()
                .panel_view(panel_handle(logs), cx)
                .panel_view(panel_handle(problems), cx);
            dock.set_dock(DockPlacement::Bottom, bottom, window, cx);
            dock.set_dock_size(DockPlacement::Bottom, px(200.), window, cx);
        });
        self.open = Some((state, dock));
        self.error = None;
    }

    fn open_wizard(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let has_project = self.open.is_some();
        let wizard = cx.new(|cx| Wizard::new(has_project, window, cx));
        cx.subscribe(&wizard, |this, _, done: &Done, cx| {
            crate::settings::change(cx, |settings| settings.first_run_done = true);
            this.wizard = None;
            this.showing_settings = false;
            this.choosing = done.choose_template && this.open.is_none();
            this.guide_next = this.choosing;
            cx.notify();
        })
        .detach();
        self.wizard = Some(wizard);
        cx.notify();
    }

    /// Opens the Developer Portal guide for the open project (D-129, D-133).
    fn open_guide(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let Some((state, _)) = &self.open else { return };
        let state = state.clone();
        let guide = cx.new(|cx| Guide::new(state, window, cx));
        cx.subscribe(&guide, |this, _, _: &Close, cx| {
            this.guide = None;
            cx.notify();
        })
        .detach();
        self.guide = Some(guide);
        cx.notify();
    }

    /// Opens Settings: as a tab in the dock's centre, or over the welcome screen.
    fn open_settings(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let panel = if let Some(panel) = &self.settings {
            panel.clone()
        } else {
            let panel = cx.new(|cx| SettingsPanel::new(window, cx));
            cx.subscribe_in(&panel, window, |this, _, _: &OpenWizard, window, cx| {
                this.open_wizard(window, cx);
            })
            .detach();
            self.settings = Some(panel.clone());
            panel
        };
        match &self.open {
            Some((_, dock)) => dock.update(cx, |dock, cx| {
                dock.add_panel_view(panel_handle(panel), DockPlacement::Center, None, window, cx);
            }),
            None => self.showing_settings = true,
        }
        cx.notify();
    }

    fn pick_folder(window: &mut Window, cx: &mut Context<'_, Self>) {
        let paths = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some(t("pick-project-folder").into()),
        });
        cx.spawn_in(window, async move |this, cx| {
            let Ok(Ok(Some(mut paths))) = paths.await else {
                return;
            };
            let Some(path) = paths.pop() else { return };
            let _ = this.update_in(cx, |this, window, cx| {
                this.open_project(&path, window, cx);
            });
        })
        .detach();
    }

    fn pick_new_folder(template: usize, window: &mut Window, cx: &mut Context<'_, Self>) {
        let home = std::env::var_os("HOME").map_or_else(|| PathBuf::from("/"), PathBuf::from);
        let path = cx.prompt_for_new_path(&home, Some("my-bot"));
        cx.spawn_in(window, async move |this, cx| {
            let Ok(Ok(Some(path))) = path.await else {
                return;
            };
            let _ = this.update_in(cx, |this, window, cx| {
                this.create_project(&path, template, window, cx);
            });
        })
        .detach();
    }
}

/// A project named after its folder, from the "Empty" template: a `/ping` command to start
/// from.
#[cfg(test)]
pub(crate) fn new_project(root: &Path) -> Result<Project, rustbot_core::Error> {
    crate::templates::create(root, &crate::templates::TEMPLATES[0])
}

/// The template cards "New project" shows (D-115), then the folder dialog.
fn template_cards(cx: &mut Context<'_, Workspace>) -> impl IntoElement {
    let theme = cx.theme();
    let cards = crate::templates::TEMPLATES
        .iter()
        .enumerate()
        .map(|(ix, template)| {
            let title = t(&format!("template-{}", template.id));
            let about = t(&format!("template-{}-about", template.id));
            v_flex()
                .id(("template", ix))
                .w(px(260.))
                .gap(px(6.))
                .p(px(12.))
                .border_1()
                .border_color(theme.border)
                .rounded(px(6.))
                .cursor_pointer()
                .hover(|style| style.bg(theme.secondary_hover))
                .child(div().font_weight(FontWeight::SEMIBOLD).child(title))
                .child(
                    div()
                        .text_sm()
                        .text_color(theme.muted_foreground)
                        .child(about),
                )
                .on_click(cx.listener(move |_, _, window, cx| {
                    Workspace::pick_new_folder(ix, window, cx);
                }))
        })
        .collect::<Vec<_>>();
    v_flex()
        .size_full()
        .items_center()
        .justify_center()
        .gap(px(16.))
        .bg(theme.background)
        .text_color(theme.foreground)
        .child(div().text_xl().child(t("choose-template")))
        .child(
            h_flex()
                .flex_wrap()
                .justify_center()
                .max_w(px(840.))
                .gap(px(12.))
                .children(cards),
        )
        .child(
            div()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(t("template-fill-hint")),
        )
        .child(
            Button::new("templates-back")
                .label(t("back"))
                .on_click(cx.listener(|this, _, _, cx| {
                    this.choosing = false;
                    cx.notify();
                })),
        )
}

/// Settings over the welcome screen, before a project is open (D-130).
fn settings_screen(
    panel: Entity<SettingsPanel>,
    cx: &mut Context<'_, Workspace>,
) -> impl IntoElement {
    v_flex()
        .size_full()
        .bg(cx.theme().background)
        .child(div().flex_1().min_h_0().child(panel))
        .child(
            h_flex()
                .p(px(16.))
                .child(
                    Button::new("settings-back")
                        .label(t("back"))
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.showing_settings = false;
                            cx.notify();
                        })),
                ),
        )
}

impl Render for Workspace {
    fn render(&mut self, _: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        crate::measure::frame_start();
        if let Some(wizard) = &self.wizard {
            return div().size_full().child(wizard.clone()).into_any_element();
        }
        if let Some(guide) = &self.guide {
            return div().size_full().child(guide.clone()).into_any_element();
        }
        if self.open.is_none() && self.choosing {
            return template_cards(cx).into_any_element();
        }
        if let (None, true, Some(panel)) = (&self.open, self.showing_settings, &self.settings) {
            return settings_screen(panel.clone(), cx).into_any_element();
        }
        let theme = cx.theme();
        let Some((state, dock)) = &self.open else {
            return v_flex()
                .size_full()
                .items_center()
                .justify_center()
                .gap(px(16.))
                .bg(theme.background)
                .text_color(theme.foreground)
                .child(div().text_2xl().child("RustBot Creator"))
                .child(
                    div()
                        .text_color(theme.muted_foreground)
                        .child(t("welcome-tagline")),
                )
                .child(
                    h_flex()
                        .gap(px(8.))
                        .child(
                            Button::new("new-project")
                                .primary()
                                .label(t("new-project"))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.choosing = true;
                                    this.error = None;
                                    cx.notify();
                                })),
                        )
                        .child(
                            Button::new("open-project")
                                .label(t("open-project"))
                                .on_click(
                                    cx.listener(|_, _, window, cx| Self::pick_folder(window, cx)),
                                ),
                        )
                        .child(Button::new("open-settings").label(t("settings")).on_click(
                            cx.listener(|this, _, window, cx| {
                                this.open_settings(window, cx);
                            }),
                        )),
                )
                .children(
                    self.error
                        .clone()
                        .map(|error| div().text_color(theme.danger).child(error)),
                )
                .into_any_element();
        };
        v_flex()
            .size_full()
            .bg(theme.background)
            .text_color(theme.foreground)
            .child(div().flex_1().min_h_0().child(dock.clone()))
            .child(
                h_flex()
                    .h(px(24.))
                    .px(px(8.))
                    .gap(px(16.))
                    .justify_between()
                    .bg(theme.status_bar)
                    .border_t_1()
                    .border_color(theme.status_bar_border)
                    .text_xs()
                    .child(run_controls(state, cx))
                    .child(
                        h_flex()
                            .gap(px(16.))
                            .child(format!("Rust {}", rustbot_core::RUST_VERSION))
                            .child(t("status-language"))
                            .child(
                                Button::new("status-settings")
                                    .ghost()
                                    .xsmall()
                                    .label(format!("⚙ {}", t("settings")))
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.open_settings(window, cx);
                                    })),
                            ),
                    ),
            )
            .child(crate::measure::frame_end())
            .into_any_element()
    }
}

/// Run/Stop and the bot's state, on the left of the status bar (D-071).
fn run_controls(state: &Entity<AppState>, cx: &mut Context<'_, Workspace>) -> impl IntoElement {
    let (button, label) = match &state.read(cx).run {
        Run::Stopped => {
            let state = state.clone();
            let button = Button::new("run")
                .ghost()
                .xsmall()
                .label(format!("▶ {}", t("run")))
                .on_click(move |_, window, cx| crate::run::start(state.clone(), window, cx));
            (Some(button), t("status-stopped"))
        }
        Run::Busy(text) => (None, text.clone()),
        Run::Running(_) => {
            let state = state.clone();
            let button = Button::new("stop")
                .ghost()
                .xsmall()
                .label(format!("■ {}", t("stop")))
                .on_click(move |_, _, cx| crate::run::stop(&state, cx));
            (Some(button), t("status-running"))
        }
    };
    h_flex().gap(px(8.)).children(button).child(label)
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_new_project_starts_with_a_valid_ping() {
        let dir = std::env::temp_dir().join(format!("rustbot-new-{}", std::process::id()));
        let project = super::new_project(&dir.join("my-bot")).unwrap();
        assert_eq!(project.manifest.bot.name, "my-bot");
        let registry = rustbot_core::Registry::with_core();
        assert_eq!(registry.validate(&project.graphs[0].graph), Vec::new());
        assert_eq!(rustbot_core::Project::open(&project.root).unwrap(), project);
        std::fs::remove_dir_all(dir).unwrap();
    }

    /// Run first asks to trust a project the app did not make (D-064); Cancel builds
    /// nothing.
    #[gpui_kit::test]
    fn run_asks_before_building_a_project_from_elsewhere(cx: &mut gpui_kit::TestAppContext) {
        use gpui_kit::test::TestWindowExt as _;
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("bot");
        super::new_project(&root).unwrap();
        cx.update(gpui_kit::init);
        let (workspace, cx) =
            cx.add_window_view(|window, cx| super::Workspace::new(Some(root.clone()), window, cx));
        cx.update(|window, cx| {
            window.render_frame(cx);
            window.click("run", cx);
        });
        cx.run_until_parked();
        assert!(cx.has_pending_prompt());
        cx.simulate_prompt_answer(&crate::i18n::t("cancel"));
        cx.run_until_parked();
        cx.read(|cx| {
            let (state, _) = workspace.read(cx).open.as_ref().unwrap();
            let state = state.read(cx);
            assert!(matches!(state.run, crate::run::Run::Stopped));
            assert_eq!(
                state.logs.last().unwrap().text,
                crate::i18n::t("run-cancelled")
            );
        });
        assert!(!root.join("generated").exists());
    }

    /// Settings open over the welcome screen, and as a tab once a project is open (D-130).
    #[gpui_kit::test]
    fn settings_open_from_the_welcome_screen_and_the_status_bar(cx: &mut gpui_kit::TestAppContext) {
        use gpui_kit::test::TestWindowExt as _;
        let dir = tempfile::tempdir().unwrap();
        cx.update(|cx| {
            gpui_kit::init(cx);
            crate::settings::init(dir.path().join("settings.toml"), cx);
        });
        let (workspace, cx) =
            cx.add_window_view(|window, cx| super::Workspace::new(None, window, cx));
        // The wizard's "done" is handled after the update that clicked it.
        cx.update(|window, cx| {
            window.render_frame(cx);
            window.click("wizard-skip", cx);
        });
        cx.update(|window, cx| {
            window.render_frame(cx);
            window.click("open-settings", cx);
            window.render_frame(cx);
        });
        cx.read(|cx| {
            let workspace = workspace.read(cx);
            assert!(workspace.showing_settings && workspace.settings.is_some());
        });
        let root = dir.path().join("bot");
        super::new_project(&root).unwrap();
        workspace.update_in(cx, |workspace, window, cx| {
            workspace.open_project(&root, window, cx);
        });
        cx.update(|window, cx| {
            window.render_frame(cx);
            window.click("status-settings", cx);
            window.render_frame(cx);
        });
        cx.read(|cx| assert!(workspace.read(cx).open.is_some()));
    }

    /// The first start shows the wizard (D-128). Going through it to "Choose a template"
    /// shows the templates and remembers that it is done, so it does not come again.
    #[gpui_kit::test]
    fn the_wizard_comes_first_and_ends_at_the_templates(cx: &mut gpui_kit::TestAppContext) {
        use gpui_kit::test::TestWindowExt as _;
        let dir = tempfile::tempdir().unwrap();
        let settings = dir.path().join("settings.toml");
        cx.update(|cx| {
            gpui_kit::init(cx);
            crate::settings::init(settings.clone(), cx);
        });
        let (workspace, cx) =
            cx.add_window_view(|window, cx| super::Workspace::new(None, window, cx));
        cx.update(|window, cx| {
            window.render_frame(cx);
            for button in ["wizard-next", "wizard-next", "wizard-template"] {
                window.click(button, cx);
                window.render_frame(cx);
            }
        });
        cx.read(|cx| {
            let workspace = workspace.read(cx);
            assert!(workspace.wizard.is_none() && workspace.choosing);
        });
        assert!(
            std::fs::read_to_string(&settings)
                .unwrap()
                .contains("first-run-done = true")
        );
        let (again, cx) = cx.add_window_view(|window, cx| super::Workspace::new(None, window, cx));
        cx.read(|cx| assert!(again.read(cx).wizard.is_none()));
    }

    /// "Connect to Discord…" in the bot settings opens the guide over the project, and
    /// "Finish" closes it (D-133).
    #[gpui_kit::test]
    fn the_bot_settings_open_the_discord_guide(cx: &mut gpui_kit::TestAppContext) {
        use gpui_kit::test::TestWindowExt as _;
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("bot");
        super::new_project(&root).unwrap();
        cx.update(gpui_kit::init);
        let (workspace, cx) =
            cx.add_window_view(|window, cx| super::Workspace::new(Some(root.clone()), window, cx));
        cx.update(|window, cx| {
            window.render_frame(cx);
            window.click("connect-discord", cx);
        });
        cx.update(|window, cx| {
            window.render_frame(cx);
            assert!(workspace.read(cx).guide.is_some());
            window.click("guide-done", cx);
        });
        cx.read(|cx| assert!(workspace.read(cx).guide.is_none()));
    }

    #[test]
    fn the_language_is_known() {
        assert!(["en", "pl"].contains(&crate::i18n::language()));
    }
}
