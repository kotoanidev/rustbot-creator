//! A code tab in the dock's centre (D-177): a file of the user's own code in `src/`,
//! saved as it is typed, or a read-only view of the code generated from the graphs,
//! which follows them.

use std::path::{Path, PathBuf};
use std::time::Duration;

use gpui_kit::component::button::Button;
use gpui_kit::component::input::{Editor, EditorState, InputEvent};
use gpui_kit::component::{ActiveTheme as _, Sizable as _, h_flex, v_flex};
use gpui_kit::{
    AppContext as _, Context, Entity, FocusHandle, IntoElement, ParentElement as _, Render,
    SharedString, Styled as _, Subscription, Task, Window, div, px, relative,
};

use crate::i18n::{t, t_with};
use crate::state::{AppState, SaveCode};

/// How long typing pauses before the file is saved.
const SAVE_DELAY: Duration = Duration::from_millis(400);

pub(crate) struct CodePanel {
    focus: FocusHandle,
    state: Entity<AppState>,
    /// The file, relative to the project folder: `src/…`, or `generated/…` to view.
    path: PathBuf,
    editor: Entity<EditorState>,
    /// A view of `generated/`, never saved.
    read_only: bool,
    /// What the file held when last read or saved, to tell a change made elsewhere.
    saved: String,
    /// Typed since the last save.
    dirty: bool,
    /// The file changed elsewhere, so the tab stopped saving until it is reloaded.
    stopped: bool,
    message: Option<String>,
    save: Option<Task<()>>,
    _changes: Vec<Subscription>,
}

impl CodePanel {
    pub(crate) fn new(
        state: Entity<AppState>,
        path: PathBuf,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> Self {
        let read_only = path.starts_with("generated");
        let (text, message) = if read_only {
            (generated(&state, &path, cx), None)
        } else {
            match crate::code::read(&state.read(cx).project.root, &path) {
                Ok(text) => (text, None),
                Err(e) => (String::new(), Some(read_failed(&path, &e))),
            }
        };
        let language = match path.extension().and_then(|e| e.to_str()) {
            Some("toml") => "toml",
            _ => "rust",
        };
        let editor = cx.new(|cx| {
            EditorState::new(window, cx)
                .language(language)
                .line_number(true)
                .default_value(text.clone())
        });
        let mut changes =
            vec![
                cx.subscribe_in(&editor, window, |this, _, event: &InputEvent, _, cx| {
                    if matches!(event, InputEvent::Change) && !this.read_only {
                        this.dirty = true;
                        this.schedule_save(cx);
                        cx.notify();
                    }
                }),
            ];
        // Run and the exports build what was typed, not what was last saved.
        changes.push(cx.subscribe(&state, |this, _, _: &SaveCode, cx| this.save_now(cx)));
        if read_only {
            changes.push(cx.observe_in(&state, window, |this, state, window, cx| {
                let text = generated(&state, &this.path, cx);
                if this.editor.read(cx).value().as_ref() != text {
                    this.editor
                        .update(cx, |editor, cx| editor.set_value(text, window, cx));
                }
            }));
        }
        cx.on_app_quit(|this, cx| {
            this.save_now(cx);
            async {}
        })
        .detach();
        Self {
            focus: cx.focus_handle(),
            state,
            path,
            editor,
            read_only,
            saved: text,
            dirty: false,
            stopped: false,
            message,
            save: None,
            _changes: changes,
        }
    }

    /// Puts the cursor on 0-based `line`.
    pub(crate) fn go_to(&self, line: u32, window: &mut Window, cx: &mut Context<'_, Self>) {
        self.editor.update(cx, |editor, cx| {
            editor.set_cursor_position(
                gpui_kit::component::input::Position::new(line, 0),
                window,
                cx,
            );
        });
    }

    fn schedule_save(&mut self, cx: &mut Context<'_, Self>) {
        self.save = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(SAVE_DELAY).await;
            let _ = this.update(cx, Self::save_now);
        }));
    }

    /// Writes what was typed, unless the file changed elsewhere since it was read.
    fn save_now(&mut self, cx: &mut Context<'_, Self>) {
        self.save = None;
        if self.read_only || self.stopped || !self.dirty {
            return;
        }
        let text = self.editor.read(cx).value().to_string();
        let root = self.state.read(cx).project.root.clone();
        match crate::code::read(&root, &self.path) {
            Ok(disk) if disk != self.saved => {
                self.stopped = true;
                self.message = Some(t("code-changed-elsewhere"));
            }
            _ => match crate::code::write(&root, &self.path, &text) {
                Ok(()) => {
                    self.saved = text;
                    self.dirty = false;
                    self.message = None;
                }
                Err(e) => {
                    let path = self.path.display().to_string();
                    self.message = Some(t_with(
                        "code-save-failed",
                        &[("path", &path), ("error", &e.to_string())],
                    ));
                }
            },
        }
        cx.notify();
    }

    /// "Reload": the file as it is now, in place of what was typed.
    fn reload(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let root = self.state.read(cx).project.root.clone();
        match crate::code::read(&root, &self.path) {
            Ok(text) => {
                self.editor
                    .update(cx, |editor, cx| editor.set_value(text.clone(), window, cx));
                self.saved = text;
                self.dirty = false;
                self.stopped = false;
                self.message = None;
            }
            Err(e) => self.message = Some(read_failed(&self.path, &e)),
        }
        cx.notify();
    }

    fn name(&self) -> SharedString {
        let name = self
            .path
            .file_name()
            .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
        if self.dirty {
            format!("{name} ●").into()
        } else {
            name.into()
        }
    }
}

/// A generated file's text as the graphs make it now, or why there is none.
fn generated(state: &Entity<AppState>, path: &Path, cx: &gpui_kit::App) -> String {
    let state = state.read(cx);
    match rustbot_codegen::generate(&state.project, &state.registry) {
        Ok(output) => output.files.get(path).cloned().unwrap_or_default(),
        Err(_) => format!("// {}\n", t("code-generated-invalid")),
    }
}

fn read_failed(path: &Path, error: &std::io::Error) -> String {
    let path = path.display().to_string();
    t_with(
        "code-read-failed",
        &[("path", &path), ("error", &error.to_string())],
    )
}

impl gpui_kit::component::dock::BasePanel for CodePanel {
    fn panel_name(&self) -> &'static str {
        "CodePanel"
    }

    fn closable(&self, _: &gpui_kit::App) -> bool {
        true
    }
}

impl gpui_kit::component::dock::Panel for CodePanel {
    fn tab_name(&self, _: &gpui_kit::App) -> Option<SharedString> {
        Some(self.name())
    }

    fn title(&mut self, _: &mut Window, _: &mut Context<'_, Self>) -> impl IntoElement {
        self.name()
    }
}

impl gpui_kit::EventEmitter<gpui_kit::component::dock::PanelEvent> for CodePanel {}

impl gpui_kit::Focusable for CodePanel {
    fn focus_handle(&self, _: &gpui_kit::App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for CodePanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let theme = cx.theme();
        let banner = if self.read_only {
            Some(
                div()
                    .px(px(8.))
                    .py(px(4.))
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .child(t("code-read-only")),
            )
        } else {
            self.message.clone().map(|message| {
                h_flex()
                    .px(px(8.))
                    .py(px(4.))
                    .gap(px(8.))
                    .text_xs()
                    .text_color(theme.danger)
                    .child(message)
                    .children(self.stopped.then(|| {
                        Button::new("code-reload")
                            .xsmall()
                            .label(t("code-reload"))
                            .on_click(cx.listener(|this, _, window, cx| this.reload(window, cx)))
                    }))
            })
        };
        v_flex().size_full().children(banner).child(
            div().flex_1().min_h_0().child(
                Editor::new(&self.editor)
                    .readonly(self.read_only)
                    .bordered(false)
                    .h(relative(1.)),
            ),
        )
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::Duration;

    use gpui_kit::test::TestWindowExt as _;
    use gpui_kit::{AppContext as _, TestAppContext};

    use super::CodePanel;
    use crate::state::AppState;

    /// What is typed is saved after a pause; a change made elsewhere stops the saving
    /// until "Reload" (D-177).
    #[gpui_kit::test]
    fn typing_is_saved_until_the_file_changes_elsewhere(cx: &mut TestAppContext) {
        let dir = tempfile::tempdir().unwrap();
        let project = crate::workspace::new_project(&dir.path().join("bot")).unwrap();
        let lib = project.root.join("src/lib.rs");
        std::fs::create_dir_all(project.root.join("src")).unwrap();
        std::fs::write(&lib, "").unwrap();
        cx.update(gpui_kit::init);
        let (panel, cx) = cx.add_window_view(|window, cx| {
            let app = cx.new(|cx| AppState::new(project, cx));
            CodePanel::new(app, PathBuf::from("src/lib.rs"), window, cx)
        });
        let typed = |text: &str, cx: &mut gpui_kit::VisualTestContext| {
            cx.update(|window, cx| {
                let editor = panel.read(cx).editor.clone();
                editor.update(cx, |editor, cx| editor.focus(window, cx));
                window.render_frame(cx);
            });
            cx.update(|window, cx| window.input(text, cx));
            cx.executor().advance_clock(Duration::from_secs(1));
            cx.run_until_parked();
        };
        typed("pub fn a() {}", cx);
        assert_eq!(std::fs::read_to_string(&lib).unwrap(), "pub fn a() {}");
        assert!(!cx.read(|cx| panel.read(cx).dirty));

        std::fs::write(&lib, "// changed elsewhere\n").unwrap();
        typed(" ", cx);
        assert_eq!(
            std::fs::read_to_string(&lib).unwrap(),
            "// changed elsewhere\n"
        );
        assert!(cx.read(|cx| panel.read(cx).stopped));
        cx.update(|window, cx| panel.update(cx, |panel, cx| panel.reload(window, cx)));
        cx.read(|cx| {
            let panel = panel.read(cx);
            assert!(!panel.stopped);
            assert_eq!(
                panel.editor.read(cx).value().as_ref(),
                "// changed elsewhere\n"
            );
        });
    }

    /// The generated code is shown read-only, as the graphs make it now.
    #[gpui_kit::test]
    fn generated_code_is_shown_read_only(cx: &mut TestAppContext) {
        let dir = tempfile::tempdir().unwrap();
        let project = crate::workspace::new_project(&dir.path().join("bot")).unwrap();
        cx.update(gpui_kit::init);
        let (panel, cx) = cx.add_window_view(|window, cx| {
            let app = cx.new(|cx| AppState::new(project, cx));
            CodePanel::new(app, PathBuf::from("generated/src/main.rs"), window, cx)
        });
        cx.run_until_parked();
        cx.read(|cx| {
            let panel = panel.read(cx);
            assert!(panel.read_only);
            assert!(panel.editor.read(cx).value().contains("async fn main()"));
        });
    }
}
