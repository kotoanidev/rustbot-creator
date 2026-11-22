//! The main window: the welcome screen, or the open project's dock (D-044) over a status
//! bar.

use std::path::{Path, PathBuf};

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::dock::{DockArea, DockLayout, DockPlacement, DockSkin, panel_handle};
use gpui_kit::component::{ActiveTheme as _, Sizable as _};
use gpui_kit::component::{h_flex, v_flex};
use gpui_kit::{
    AppContext as _, Context, Entity, IntoElement, ParentElement as _, PathPromptOptions, Render,
    Styled as _, Window, div, px,
};
use rustbot_core::{GraphFile, GraphKind, Manifest, Project};

use crate::i18n::{t, t_with};
use crate::panels::{Editor, Logs, Problems, ProjectTree, Properties};
use crate::run::Run;
use crate::state::AppState;

pub(crate) struct Workspace {
    open: Option<(Entity<AppState>, Entity<DockArea>)>,
    error: Option<String>,
}

impl Workspace {
    pub(crate) fn new(
        path: Option<PathBuf>,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> Self {
        crate::measure::first_frame(window);
        let mut this = Self {
            open: None,
            error: None,
        };
        if let Some(path) = path {
            this.open_project(&path, window, cx);
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

    fn create_project(&mut self, root: &Path, window: &mut Window, cx: &mut Context<'_, Self>) {
        match new_project(root) {
            Ok(project) => {
                // Made here, so trusted (D-064).
                if let Ok(toolchain) = rustbot_toolchain::Toolchain::locate() {
                    let _ = crate::trust::trust(toolchain.data_dir(), &project.root);
                }
                self.show(project, window, cx);
            }
            Err(e) => self.error = Some(t_with("error-create", &[("error", &e.to_string())])),
        }
        cx.notify();
    }

    fn show(&mut self, project: Project, window: &mut Window, cx: &mut Context<'_, Self>) {
        let state = cx.new(|cx| AppState::new(project, cx));
        cx.observe(&state, |_, _, cx| cx.notify()).detach();
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

    fn pick_new_folder(window: &mut Window, cx: &mut Context<'_, Self>) {
        let home = std::env::var_os("HOME").map_or_else(|| PathBuf::from("/"), PathBuf::from);
        let path = cx.prompt_for_new_path(&home, Some("my-bot"));
        cx.spawn_in(window, async move |this, cx| {
            let Ok(Ok(Some(path))) = path.await else {
                return;
            };
            let _ = this.update_in(cx, |this, window, cx| {
                this.create_project(&path, window, cx);
            });
        })
        .detach();
    }
}

/// A project named after its folder, with a `/ping` command to start from.
pub(crate) fn new_project(root: &Path) -> Result<Project, rustbot_core::Error> {
    let name = root
        .file_name()
        .map_or_else(|| "bot".to_owned(), |n| n.to_string_lossy().into_owned());
    let mut project = Project::create(root, Manifest::new(&name))?;
    let ping = GraphFile {
        kind: GraphKind::Command,
        name: "ping".to_owned(),
        graph: serde_json::from_str(include_str!("../templates/ping.json"))
            .expect("the ping template is a valid graph"),
        layout: serde_json::from_str(include_str!("../templates/ping.layout.json"))
            .expect("the ping template is a valid layout"),
    };
    project.save_graph(&ping)?;
    project.graphs.push(ping);
    Ok(project)
}

impl Render for Workspace {
    fn render(&mut self, _: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
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
                                .on_click(cx.listener(|_, _, window, cx| {
                                    Self::pick_new_folder(window, cx);
                                })),
                        )
                        .child(
                            Button::new("open-project")
                                .label(t("open-project"))
                                .on_click(
                                    cx.listener(|_, _, window, cx| Self::pick_folder(window, cx)),
                                ),
                        ),
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
                            .child(t("status-language")),
                    ),
            )
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
            assert_eq!(state.logs.last().unwrap(), &crate::i18n::t("run-cancelled"));
        });
        assert!(!root.join("generated").exists());
    }

    #[test]
    fn the_language_is_known() {
        assert!(["en", "pl"].contains(&crate::i18n::language()));
    }
}
