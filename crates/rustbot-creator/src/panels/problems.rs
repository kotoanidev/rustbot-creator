//! Problems in the project's graphs, in the bottom panel.

use gpui_kit::component::ActiveTheme as _;
use gpui_kit::{
    Context, Entity, FocusHandle, IntoElement, ParentElement as _, Render, Styled as _, Window,
    div, px,
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
        // Problems block Run; warnings, after them, do not (D-085).
        let lines: Vec<(String, bool)> = [false, true]
            .into_iter()
            .flat_map(|warning| {
                state.project.graphs.iter().flat_map(move |file| {
                    let found = if warning {
                        state.registry.warnings(&file.graph)
                    } else {
                        state.registry.validate(&file.graph)
                    };
                    found.into_iter().map(move |problem| {
                        (format!("{}: {}", file.name, describe(&problem)), warning)
                    })
                })
            })
            .collect();
        if lines.is_empty() {
            return super::placeholder(t("problems-empty"), cx).into_any_element();
        }
        div()
            .size_full()
            .p(px(8.))
            .text_sm()
            .children(lines.into_iter().map(|(line, warning)| {
                let theme = cx.theme();
                div()
                    .text_color(if warning {
                        theme.warning
                    } else {
                        theme.foreground
                    })
                    .child(line)
            }))
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
