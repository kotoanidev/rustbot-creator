//! Problems in the project's graphs, in the bottom panel.

use gpui_kit::component::ActiveTheme as _;
use gpui_kit::{
    Context, Entity, FocusHandle, InteractiveElement as _, IntoElement, ParentElement as _, Render,
    StatefulInteractiveElement as _, Styled as _, TestSupportExt as _, Window, div, px,
};
use rustbot_core::registry::{OptionProblem, Problem, ProblemKind};
use rustbot_core::template::TemplateError;

use crate::i18n::{t, t_with};
use crate::state::AppState;

pub(crate) struct Problems {
    focus: FocusHandle,
    state: Entity<AppState>,
}

super::dock_panel!(Problems, "Problems", "panel-problems");

impl Problems {
    pub(crate) fn new(state: Entity<AppState>, cx: &mut Context<'_, Self>) -> Self {
        cx.observe(&state, |_, _, cx| cx.notify()).detach();
        Self {
            focus: cx.focus_handle(),
            state,
        }
    }
}

impl Render for Problems {
    fn render(&mut self, _: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let state = self.state.read(cx);
        // Problems block Run; warnings, after them, do not (D-085). A click shows the node
        // (D-090).
        let lines: Vec<(String, bool, usize, Option<rustbot_core::NodeId>)> = [false, true]
            .into_iter()
            .flat_map(|warning| {
                state
                    .project
                    .graphs
                    .iter()
                    .enumerate()
                    .flat_map(move |(ix, file)| {
                        let found = if warning {
                            state.registry.warnings(&file.graph)
                        } else {
                            state.registry.validate(&file.graph)
                        };
                        found.into_iter().map(move |problem| {
                            let text = format!("{}: {}", file.name, describe(&problem));
                            (text, warning, ix, problem.node)
                        })
                    })
            })
            .collect();
        if lines.is_empty() {
            return super::placeholder(t("problems-empty"), cx).into_any_element();
        }
        let theme = cx.theme();
        div()
            .size_full()
            .p(px(8.))
            .text_sm()
            .children(
                lines
                    .into_iter()
                    .enumerate()
                    .map(|(row, (line, warning, graph, node))| {
                        let state = self.state.clone();
                        div()
                            .id(("problem", row))
                            .test_support()
                            .px(px(4.))
                            .rounded(px(4.))
                            .cursor_pointer()
                            .hover(|line| line.bg(theme.list_hover))
                            .text_color(if warning {
                                theme.warning
                            } else {
                                theme.foreground
                            })
                            .on_click(move |_, _, cx| {
                                let node = node.clone();
                                state.update(cx, |state, cx| state.show_node(graph, node, cx));
                            })
                            .child(line)
                    }),
            )
            .into_any_element()
    }
}

/// The problem in the app's language.
pub(crate) fn describe(problem: &Problem) -> String {
    let pin = |pin: &rustbot_core::PinRef| pin.to_string();
    let text = match &problem.kind {
        ProblemKind::NoTrigger => t("problem-no-trigger"),
        ProblemKind::SeveralTriggers => t("problem-several-triggers"),
        ProblemKind::UnknownNodeType(kind) => {
            t_with("problem-unknown-node-type", &[("type", kind)])
        }
        ProblemKind::UnknownPin(p) => t_with("problem-unknown-pin", &[("pin", &pin(p))]),
        ProblemKind::WrongDirection(p) => t_with("problem-wrong-direction", &[("pin", &pin(p))]),
        ProblemKind::KindMismatch { from, to } => t_with(
            "problem-kind-mismatch",
            &[("from", &pin(from)), ("to", &pin(to))],
        ),
        ProblemKind::SeveralSources(p) => t_with("problem-several-sources", &[("pin", &pin(p))]),
        ProblemKind::MissingProp(prop) => t_with("problem-missing-prop", &[("prop", prop)]),
        ProblemKind::UnknownProp(prop) => t_with("problem-unknown-prop", &[("prop", prop)]),
        ProblemKind::WrongPropType(prop) => t_with("problem-wrong-prop-type", &[("prop", prop)]),
        ProblemKind::BadTemplate { prop, error } => {
            let id = match error {
                TemplateError::Unclosed(_) => "problem-template-unclosed",
                TemplateError::Stray(_) => "problem-template-stray",
                TemplateError::Empty(_) => "problem-template-empty",
            };
            t_with(id, &[("prop", prop)])
        }
        ProblemKind::UnknownVariable { prop, path } => t_with(
            "problem-unknown-variable",
            &[("prop", prop), ("path", path)],
        ),
        ProblemKind::WrongVariableType { prop, path } => {
            t_with("problem-variable-type", &[("prop", prop), ("path", path)])
        }
        ProblemKind::NoReply => t("problem-no-reply"),
        ProblemKind::EmptyMessage => t("problem-empty-message"),
        ProblemKind::BadColor(prop) => t_with("problem-bad-color", &[("prop", prop)]),
        ProblemKind::NoPermissionCheck { permission } => t_with(
            "problem-no-permission-check",
            &[(
                "permission",
                &crate::i18n::t_or(&format!("choice-permission-{permission}"), permission),
            )],
        ),
        ProblemKind::BadNumber { prop, max } => t_with(
            "problem-bad-number",
            &[("prop", prop), ("max", &max.to_string())],
        ),
        ProblemKind::BadOption { option, problem } => {
            let id = match problem {
                OptionProblem::Name => "problem-option-name",
                OptionProblem::Duplicate => "problem-option-duplicate",
                OptionProblem::Description => "problem-option-description",
                OptionProblem::Type => "problem-option-type",
                OptionProblem::RequiredAfterOptional => "problem-option-order",
                OptionProblem::TooMany => "problem-option-too-many",
            };
            t_with(id, &[("option", &(option + 1).to_string())])
        }
    };
    match &problem.node {
        Some(node) => format!("{node}: {text}"),
        None => text,
    }
}

#[cfg(test)]
mod tests {
    use gpui_kit::test::TestWindowExt as _;
    use gpui_kit::{AppContext as _, TestAppContext};
    use rustbot_core::NodeId;

    use super::Problems;
    use crate::state::AppState;

    /// Clicking a problem opens its graph, selects its node and asks the editor to show it
    /// (D-090).
    #[gpui_kit::test]
    fn clicking_a_problem_selects_its_node(cx: &mut TestAppContext) {
        let dir = tempfile::tempdir().unwrap();
        let project = crate::workspace::new_project(&dir.path().join("bot")).unwrap();
        cx.update(gpui_kit::init);
        let mut state = None;
        let (_, cx) = cx.add_window_view(|_, cx| {
            let app = cx.new(|cx| AppState::new(project, cx));
            app.update(cx, |app, cx| {
                app.edit(false, cx, |file, _| {
                    let reply = file.graph.nodes.get_mut(&NodeId("n2".to_owned())).unwrap();
                    reply.props.insert("content".to_owned(), "".into());
                });
            });
            state = Some(app.clone());
            Problems::new(app, cx)
        });
        let state = state.unwrap();
        cx.update(|window, cx| window.click(("problem", 0_usize), cx));
        cx.read(|cx| {
            let state = state.read(cx);
            assert_eq!(state.selected_node, Some(NodeId("n2".to_owned())));
            assert_eq!((state.open_graph, state.reveal), (Some(0), 1));
        });
    }
}
