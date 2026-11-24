//! The first-run wizard (D-128): the language and look, then gcc and the private Rust with
//! consent, then a first project from a template. Every step can be skipped, and Settings
//! open it again.

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::{ActiveTheme as _, h_flex, v_flex};
use gpui_kit::{
    AppContext as _, AsyncWindowContext, Context, Entity, EventEmitter, FontWeight, IntoElement,
    ParentElement as _, Render, Styled as _, Window, div, px,
};

use crate::i18n::{t, t_with};
use crate::install::{self, Progress};
use crate::panels::SettingsPanel;

/// The wizard is over.
pub(crate) struct Done {
    /// The user wants to make a project from a template now.
    pub(crate) choose_template: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Step {
    Appearance,
    Toolchain,
    Project,
}

const STEPS: [Step; 3] = [Step::Appearance, Step::Toolchain, Step::Project];

/// What the toolchain step knows.
enum Setup {
    Checking,
    Found { gcc: bool, rust: bool },
    Installing { step: String, lines: Vec<String> },
    Failed(String),
}

/// How many lines of the installer's output the step shows.
const SHOWN_LINES: usize = 8;

pub(crate) struct Wizard {
    step: Step,
    appearance: Entity<SettingsPanel>,
    setup: Setup,
    /// A project is open, so the last step does not offer a new one.
    has_project: bool,
}

impl EventEmitter<Done> for Wizard {}

impl Wizard {
    pub(crate) fn new(has_project: bool, window: &mut Window, cx: &mut Context<'_, Self>) -> Self {
        let appearance = cx.new(|cx| SettingsPanel::appearance(window, cx));
        let mut wizard = Self {
            step: Step::Appearance,
            appearance,
            setup: Setup::Checking,
            has_project,
        };
        wizard.check(cx);
        wizard
    }

    /// Looks for gcc and the private Rust in the background.
    fn check(&mut self, cx: &mut Context<'_, Self>) {
        self.setup = Setup::Checking;
        cx.spawn(async move |this, cx| {
            let (gcc, rust) = cx
                .background_spawn(async {
                    let rust = rustbot_toolchain::Toolchain::locate()
                        .is_ok_and(|toolchain| toolchain.is_installed());
                    (rustbot_toolchain::c_compiler().is_some(), rust)
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                this.setup = Setup::Found { gcc, rust };
                cx.notify();
            });
        })
        .detach();
    }

    /// Installs what is missing, asking first, and shows the progress in the step.
    fn install(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let Ok(toolchain) = rustbot_toolchain::Toolchain::locate() else {
            self.setup = Setup::Failed(t("error-no-home"));
            return;
        };
        self.setup = Setup::Installing {
            step: String::new(),
            lines: Vec::new(),
        };
        let this = cx.entity().downgrade();
        window
            .spawn(cx, async move |cx| {
                let report = |cx: &mut AsyncWindowContext, progress: Progress| {
                    let _ = this.update(cx, |this, cx| {
                        if let Setup::Installing { step, lines } = &mut this.setup {
                            match progress {
                                Progress::Step(text) => *step = text,
                                Progress::Lines(new) => {
                                    lines.extend(new);
                                    let old = lines.len().saturating_sub(SHOWN_LINES);
                                    lines.drain(..old);
                                }
                            }
                        }
                        cx.notify();
                    });
                };
                let result = install::install(cx, &toolchain, &report).await;
                let _ = this.update(cx, |this, cx| match result {
                    Ok(()) => this.check(cx),
                    Err(error) => {
                        this.setup = Setup::Failed(error);
                        cx.notify();
                    }
                });
            })
            .detach();
        cx.notify();
    }

    fn go(&mut self, by: isize, cx: &mut Context<'_, Self>) {
        let at = STEPS.iter().position(|&s| s == self.step).unwrap_or(0);
        if let Some(&step) = at.checked_add_signed(by).and_then(|ix| STEPS.get(ix)) {
            self.step = step;
            cx.notify();
        }
    }

    fn toolchain_step(&self, cx: &mut Context<'_, Self>) -> gpui_kit::Div {
        let theme = cx.theme();
        let version = [("version", rustbot_core::RUST_VERSION)];
        let mut body = v_flex().gap(px(8.)).child(t("wizard-toolchain-about"));
        match &self.setup {
            Setup::Checking => body = body.child(t("wizard-checking")),
            Setup::Found { gcc, rust } => {
                body = body
                    .child(t(if *gcc {
                        "wizard-gcc-found"
                    } else {
                        "wizard-gcc-missing"
                    }))
                    .child(t_with(
                        if *rust {
                            "wizard-rust-found"
                        } else {
                            "wizard-rust-missing"
                        },
                        &version,
                    ));
                if !(*gcc && *rust) {
                    body = body.child(
                        h_flex().child(
                            Button::new("wizard-install")
                                .primary()
                                .label(t("wizard-install"))
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.install(window, cx);
                                })),
                        ),
                    );
                }
            }
            Setup::Installing { step, lines } => {
                body = body.child(step.clone()).child(
                    v_flex()
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .children(lines.iter().cloned()),
                );
            }
            Setup::Failed(error) => {
                body = body
                    .child(div().text_color(theme.danger).child(error.clone()))
                    .child(
                        h_flex().child(
                            Button::new("wizard-install")
                                .label(t("wizard-install"))
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.install(window, cx);
                                })),
                        ),
                    );
            }
        }
        body
    }

    /// Back, and Next, Skip or the last step's choices.
    fn buttons(&self, number: usize, cx: &mut Context<'_, Self>) -> gpui_kit::Div {
        let busy = matches!(self.setup, Setup::Installing { .. });
        let mut buttons = h_flex().gap(px(8.));
        if number > 1 && !busy {
            buttons = buttons.child(
                Button::new("wizard-back")
                    .label(t("back"))
                    .on_click(cx.listener(|this, _, _, cx| this.go(-1, cx))),
            );
        }
        buttons = match self.step {
            Step::Project if !self.has_project => buttons
                .child(
                    Button::new("wizard-later")
                        .label(t("wizard-later"))
                        .on_click(cx.listener(|_, _, _, cx| {
                            cx.emit(Done {
                                choose_template: false,
                            });
                        })),
                )
                .child(
                    Button::new("wizard-template")
                        .primary()
                        .label(t("wizard-template"))
                        .on_click(cx.listener(|_, _, _, cx| {
                            cx.emit(Done {
                                choose_template: true,
                            });
                        })),
                ),
            Step::Project => buttons.child(
                Button::new("wizard-finish")
                    .primary()
                    .label(t("wizard-finish"))
                    .on_click(cx.listener(|_, _, _, cx| {
                        cx.emit(Done {
                            choose_template: false,
                        });
                    })),
            ),
            Step::Toolchain if busy => buttons,
            Step::Toolchain if !self.ready() => buttons.child(
                Button::new("wizard-next")
                    .label(t("wizard-skip-step"))
                    .on_click(cx.listener(|this, _, _, cx| this.go(1, cx))),
            ),
            Step::Appearance | Step::Toolchain => buttons.child(
                Button::new("wizard-next")
                    .primary()
                    .label(t("wizard-next"))
                    .on_click(cx.listener(|this, _, _, cx| this.go(1, cx))),
            ),
        };
        buttons
    }

    /// Whether the toolchain step has nothing left to install.
    fn ready(&self) -> bool {
        matches!(
            self.setup,
            Setup::Found {
                gcc: true,
                rust: true
            }
        )
    }
}

impl Render for Wizard {
    fn render(&mut self, _: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let number = STEPS.iter().position(|&s| s == self.step).unwrap_or(0) + 1;
        let (name, body) = match self.step {
            Step::Appearance => (
                t("wizard-appearance"),
                v_flex()
                    .gap(px(12.))
                    .child(t("wizard-appearance-about"))
                    .child(self.appearance.clone()),
            ),
            Step::Toolchain => (t("wizard-toolchain"), self.toolchain_step(cx)),
            Step::Project => (
                t("wizard-project"),
                v_flex().child(t(if self.has_project {
                    "wizard-project-open"
                } else {
                    "wizard-project-about"
                })),
            ),
        };
        let buttons = self.buttons(number, cx);
        let theme = cx.theme();
        let count = STEPS.len().to_string();
        v_flex()
            .size_full()
            .items_center()
            .justify_center()
            .bg(theme.background)
            .text_color(theme.foreground)
            .child(
                v_flex()
                    .w(px(560.))
                    .gap(px(16.))
                    .child(div().text_2xl().child(t("wizard-title")))
                    .child(div().font_weight(FontWeight::SEMIBOLD).child(t_with(
                        "wizard-step",
                        &[
                            ("number", &number.to_string()),
                            ("count", &count),
                            ("name", &name),
                        ],
                    )))
                    .child(body)
                    .child(
                        h_flex()
                            .justify_between()
                            .child(
                                Button::new("wizard-skip")
                                    .ghost()
                                    .label(t("wizard-skip"))
                                    .on_click(cx.listener(|_, _, _, cx| {
                                        cx.emit(Done {
                                            choose_template: false,
                                        });
                                    })),
                            )
                            .child(buttons),
                    ),
            )
    }
}
