//! The centre: the open graph on a canvas (D-042). Nodes are dragged, wired pin to pin,
//! and added from a searchable list (D-067); Delete removes the selected node, Ctrl+Z and
//! Ctrl+Shift+Z undo and redo, and every change is autosaved (D-068).

use std::cell::Cell;
use std::rc::Rc;

use gpui_kit::component::ActiveTheme as _;
use gpui_kit::component::button::Button;
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::list::ListItem;
use gpui_kit::component::v_flex;
use gpui_kit::{
    AppContext as _, Bounds, Context, Entity, FocusHandle, Hsla, InteractiveElement as _,
    IntoElement, KeyDownEvent, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent,
    ParentElement as _, PathBuilder, Pixels, Point, Render, ScrollEvent, Styled as _, Subscription,
    Window, canvas, div, point, px,
};
use rustbot_core::registry::{Category, DataType, Direction, Pin, PinKind};
use rustbot_core::{GraphFile, NodeId, PinRef, Registry};

use crate::canvas::{self, Hit};
use crate::i18n::{t, t_or};
use crate::state::AppState;

pub(crate) struct Editor {
    focus: FocusHandle,
    state: Entity<AppState>,
    /// How far the view is scrolled: graph `[0, 0]` is drawn at the canvas origin + `pan`.
    pan: Point<Pixels>,
    /// Where the canvas is in the window, from the last frame.
    view: Rc<Cell<Bounds<Pixels>>>,
    drag: Drag,
    picker: Option<Picker>,
    frames: crate::measure::Frames,
}

super::dock_panel!(Editor, "Editor", "panel-editor");

enum Drag {
    None,
    /// Moving a node; `grab` is where it was held, relative to its corner.
    Node {
        id: NodeId,
        grab: [f32; 2],
        moved: bool,
    },
    Pan {
        last: Point<Pixels>,
    },
    /// A wire being pulled out of `from`; `to` is the pointer in graph coordinates.
    Wire {
        from: PinRef,
        pin: Pin,
        to: [f32; 2],
    },
}

/// The searchable node list (D-067). With `wire`, it lists only nodes that fit that pin
/// and wires the new node to it.
struct Picker {
    at: [f32; 2],
    query: Entity<InputState>,
    wire: Option<(PinRef, Pin)>,
    _changes: Subscription,
}

impl Editor {
    pub(crate) fn new(state: Entity<AppState>, cx: &mut Context<'_, Self>) -> Self {
        cx.observe(&state, |_, _, cx| cx.notify()).detach();
        Self {
            focus: cx.focus_handle(),
            state,
            pan: Point::default(),
            view: Rc::default(),
            drag: Drag::None,
            picker: None,
            frames: crate::measure::Frames::default(),
        }
    }

    /// A window position in graph coordinates.
    fn graph_point(&self, position: Point<Pixels>) -> [f32; 2] {
        let at = position - self.view.get().origin - self.pan;
        [at.x.into(), at.y.into()]
    }

    /// The part of the graph on screen, with a margin, as `[left, top, right, bottom]`.
    /// Only what touches it is drawn, so a large graph costs what is visible.
    fn visible(&self) -> Option<[f32; 4]> {
        let size = self.view.get().size;
        if size.width <= px(0.) || size.height <= px(0.) {
            return None;
        }
        let margin = 40.;
        let left = -f32::from(self.pan.x) - margin;
        let top = -f32::from(self.pan.y) - margin;
        Some([
            left,
            top,
            left + f32::from(size.width) + 2. * margin,
            top + f32::from(size.height) + 2. * margin,
        ])
    }

    fn hit(&self, at: [f32; 2], cx: &Context<'_, Self>) -> Option<(Hit, [f32; 2])> {
        let state = self.state.read(cx);
        let file = state.open_file()?;
        let hit = canvas::hit(file, &state.registry, at);
        let corner = match &hit {
            Hit::Node(id) => canvas::position(file, id),
            _ => [0., 0.],
        };
        Some((hit, corner))
    }

    fn select(&self, node: Option<NodeId>, cx: &mut Context<'_, Self>) {
        self.state.update(cx, |state, cx| {
            state.selected_node = node;
            cx.notify();
        });
    }

    fn mouse_down(
        &mut self,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        self.focus.focus(window, cx);
        self.picker = None;
        let at = self.graph_point(event.position);
        match self.hit(at, cx) {
            Some((Hit::Pin(from, pin), _)) => self.drag = Drag::Wire { from, pin, to: at },
            Some((Hit::Node(id), corner)) => {
                self.select(Some(id.clone()), cx);
                self.drag = Drag::Node {
                    id,
                    grab: [at[0] - corner[0], at[1] - corner[1]],
                    moved: false,
                };
            }
            _ => {
                self.select(None, cx);
                self.drag = Drag::Pan {
                    last: event.position,
                };
            }
        }
        cx.notify();
    }

    fn mouse_move(
        &mut self,
        event: &MouseMoveEvent,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        if event.pressed_button != Some(MouseButton::Left) && !matches!(self.drag, Drag::None) {
            // The button was let go outside the canvas.
            return self.finish_drag(event.position, window, cx);
        }
        let at = self.graph_point(event.position);
        match &mut self.drag {
            Drag::None => return,
            Drag::Node { id, grab, moved } => {
                let corner = [at[0] - grab[0], at[1] - grab[1]];
                let (id, undoable) = (id.clone(), !*moved);
                *moved = true;
                self.state.update(cx, |state, cx| {
                    state.edit(undoable, cx, |file, _| {
                        file.layout.nodes.insert(id, corner);
                    });
                });
            }
            Drag::Pan { last } => {
                self.pan += event.position - *last;
                *last = event.position;
            }
            Drag::Wire { to, .. } => *to = at,
        }
        cx.notify();
    }

    fn mouse_up(&mut self, event: &MouseUpEvent, window: &mut Window, cx: &mut Context<'_, Self>) {
        self.finish_drag(event.position, window, cx);
    }

    /// Ends a drag. A wire dropped on a fitting pin joins it; dropped on empty canvas, it
    /// opens the node list for nodes that fit.
    fn finish_drag(
        &mut self,
        position: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        if let Drag::Wire { from, pin, .. } = std::mem::replace(&mut self.drag, Drag::None) {
            let at = self.graph_point(position);
            match self.hit(at, cx) {
                Some((Hit::Pin(to, other), _))
                    if canvas::fits(&pin, &other) && to.node != from.node =>
                {
                    self.state.update(cx, |state, cx| {
                        state.edit(true, cx, |file, _| {
                            canvas::connect(file, (&from, &pin), &to);
                        });
                    });
                }
                Some((Hit::Empty, _)) => self.open_picker(at, Some((from, pin)), window, cx),
                _ => {}
            }
        }
        cx.notify();
    }

    fn open_picker(
        &mut self,
        at: [f32; 2],
        wire: Option<(PinRef, Pin)>,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let query = cx.new(|cx| InputState::new(window, cx).placeholder(t("picker-search")));
        let changes = cx.subscribe_in(&query, window, |this, _, event, window, cx| {
            if let InputEvent::PressEnter { .. } = event
                && let Some((kind, _)) = this.matches(cx).into_iter().next()
            {
                this.pick(&kind, cx);
            }
            let _ = window;
            cx.notify();
        });
        query.update(cx, |query, cx| query.focus(window, cx));
        self.picker = Some(Picker {
            at,
            query,
            wire,
            _changes: changes,
        });
        cx.notify();
    }

    /// The node types the picker lists: `(id, name)`, sorted by name.
    fn matches(&self, cx: &Context<'_, Self>) -> Vec<(String, String)> {
        let Some(picker) = &self.picker else {
            return Vec::new();
        };
        let query = picker.query.read(cx).value().to_lowercase();
        let mut found: Vec<(String, String)> = self
            .state
            .read(cx)
            .registry
            .iter()
            .filter(|node| {
                picker
                    .wire
                    .as_ref()
                    .is_none_or(|(_, pin)| canvas::fitting_pin(node, pin).is_some())
            })
            .map(|node| (node.id.clone(), node_name(&node.id)))
            .filter(|(id, name)| {
                name.to_lowercase().contains(&query) || id.to_lowercase().contains(&query)
            })
            .collect();
        found.sort_by(|a, b| a.1.cmp(&b.1));
        found
    }

    /// Adds a node from the picker, wired to the pin the picker was opened from.
    fn pick(&mut self, kind: &str, cx: &mut Context<'_, Self>) {
        let Some(picker) = self.picker.take() else {
            return;
        };
        let mut added = None;
        self.state.update(cx, |state, cx| {
            state.edit(true, cx, |file, registry| {
                // The new node's first row sits under the pointer, on the wire's side.
                let left = match &picker.wire {
                    Some((_, pin)) if pin.direction == Direction::In => canvas::WIDTH,
                    _ => 0.,
                };
                let at = [
                    picker.at[0] - left,
                    picker.at[1] - canvas::HEADER - canvas::ROW / 2.,
                ];
                let id = canvas::add_node(file, registry, kind, at);
                if let Some((from, pin)) = &picker.wire {
                    let target = registry
                        .get(kind)
                        .and_then(|node| canvas::fitting_pin(node, pin));
                    if let Some(target) = target {
                        let to = PinRef {
                            node: id.clone(),
                            pin: target.name.clone(),
                        };
                        canvas::connect(file, (from, pin), &to);
                    }
                }
                added = Some(id);
            });
        });
        self.select(added, cx);
    }

    fn key_down(&mut self, event: &KeyDownEvent, _: &mut Window, cx: &mut Context<'_, Self>) {
        let key = event.keystroke.key.as_str();
        let modifiers = event.keystroke.modifiers;
        if key == "escape" {
            self.picker = None;
            self.drag = Drag::None;
            return cx.notify();
        }
        // Typing in the node list is not a shortcut for the canvas.
        if self.picker.is_some() {
            return;
        }
        let state = self.state.clone();
        match key {
            "delete" | "backspace" => {
                let Some(node) = state.read(cx).selected_node.clone() else {
                    return;
                };
                state.update(cx, |state, cx| {
                    state.selected_node = None;
                    state.edit(true, cx, |file, _| canvas::remove_node(file, &node));
                });
            }
            "z" if modifiers.control => {
                state.update(cx, |state, cx| state.undo(modifiers.shift, cx));
            }
            "y" if modifiers.control => state.update(cx, |state, cx| state.undo(true, cx)),
            _ => return,
        }
        cx.stop_propagation();
    }

    fn render_node(
        &self,
        file: &GraphFile,
        registry: &Registry,
        id: &NodeId,
        cx: &Context<'_, Self>,
    ) -> impl IntoElement + use<> {
        let theme = cx.theme();
        let node = &file.graph.nodes[id];
        let kind = registry.get(&node.kind);
        let [x, y] = canvas::position(file, id);
        let [w, h] = canvas::size(kind);
        let selected = self.state.read(cx).selected_node.as_ref() == Some(id);
        let accent = match kind.map(|kind| kind.category) {
            Some(Category::Trigger) => theme.blue,
            Some(Category::Action) => theme.green,
            Some(Category::Logic) => theme.yellow,
            Some(Category::Data) => theme.magenta,
            None => theme.danger,
        };
        let title = kind.map_or_else(|| node.kind.clone(), |kind| node_name(&kind.id));
        let pins = [Direction::In, Direction::Out]
            .into_iter()
            .flat_map(|direction| {
                canvas::pins(kind, direction)
                    .into_iter()
                    .map(move |(pin, [px_x, px_y])| (pin.clone(), direction, px_x, px_y))
            });
        let pins: Vec<_> = pins
            .map(|(pin, direction, pin_x, pin_y)| {
                let label = div()
                    .absolute()
                    .top(px(pin_y - 8.))
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .child(t_or(&format!("pin-{}", pin.name), &pin.name));
                let label = if direction == Direction::In {
                    label.left(px(12.))
                } else {
                    label.right(px(12.))
                };
                let dot = div()
                    .absolute()
                    .left(px(pin_x - 5.))
                    .top(px(pin_y - 5.))
                    .size(px(10.))
                    .rounded_full()
                    .border_1()
                    .border_color(theme.background)
                    .bg(pin_color(pin.kind, cx));
                [label.into_any_element(), dot.into_any_element()]
            })
            .collect();
        div()
            .absolute()
            .left(self.pan.x + px(x))
            .top(self.pan.y + px(y))
            .w(px(w))
            .h(px(h))
            .rounded(px(6.))
            .border_1()
            .border_color(if selected { theme.ring } else { theme.border })
            .bg(theme.popover)
            .child(
                div()
                    .h(px(canvas::HEADER))
                    .px(px(10.))
                    .flex()
                    .items_center()
                    .rounded_t(px(6.))
                    .bg(accent.opacity(0.18))
                    .text_sm()
                    .child(title),
            )
            .children(pins.into_iter().flatten())
    }

    /// The wires, and the one being pulled, painted under the nodes. Painting also records
    /// where the canvas is in the window, for turning pointer positions into graph ones.
    fn render_wires(
        &self,
        file: &GraphFile,
        registry: &Registry,
        cx: &Context<'_, Self>,
    ) -> impl IntoElement + use<> {
        let mut wires: Vec<([f32; 2], [f32; 2], Hsla)> = file
            .graph
            .edges
            .iter()
            .filter_map(|edge| {
                let from = canvas::pin_position(file, registry, &edge.from)?;
                let to = canvas::pin_position(file, registry, &edge.to)?;
                let kind = registry
                    .get(&file.graph.nodes.get(&edge.from.node)?.kind)?
                    .pins
                    .iter()
                    .find(|pin| pin.name == edge.from.pin)?
                    .kind;
                Some((from, to, pin_color(kind, cx).opacity(0.8)))
            })
            .collect();
        if let Drag::Wire { from, pin, to } = &self.drag
            && let Some(start) = canvas::pin_position(file, registry, from)
        {
            let (a, b) = if pin.direction == Direction::Out {
                (start, *to)
            } else {
                (*to, start)
            };
            wires.push((a, b, cx.theme().ring));
        }
        let visible = self.visible();
        wires.retain(|(a, b, _)| {
            let bend = ((b[0] - a[0]).abs() * 0.5).max(40.);
            let min = [a[0].min(b[0]) - bend, a[1].min(b[1])];
            let max = [a[0].max(b[0]) + bend, a[1].max(b[1])];
            canvas::in_view(visible, min, max)
        });
        let view = self.view.clone();
        let pan = self.pan;
        canvas(
            move |bounds, _, _| view.set(bounds),
            move |bounds, (), window, _| {
                // One path per colour: the GPU rasterizes each path on its own, so dozens of
                // single-wire paths cost more than the rest of the frame.
                let mut paths: Vec<(Hsla, PathBuilder)> = Vec::new();
                for (a, b, color) in wires {
                    let a = bounds.origin + pan + point(px(a[0]), px(a[1]));
                    let b = bounds.origin + pan + point(px(b[0]), px(b[1]));
                    let bend = ((b.x - a.x).abs() * 0.5).max(px(40.));
                    let ix = paths
                        .iter()
                        .position(|(c, _)| *c == color)
                        .unwrap_or_else(|| {
                            paths.push((color, PathBuilder::stroke(px(2.))));
                            paths.len() - 1
                        });
                    let path = &mut paths[ix].1;
                    path.move_to(a);
                    path.cubic_bezier_to(b, point(a.x + bend, a.y), point(b.x - bend, b.y));
                }
                for (color, path) in paths {
                    if let Ok(path) = path.build() {
                        window.paint_path(path, color);
                    }
                }
            },
        )
        .absolute()
        .size_full()
    }

    fn render_picker(
        &self,
        picker: &Picker,
        cx: &mut Context<'_, Self>,
    ) -> impl IntoElement + use<> {
        let theme = cx.theme();
        let matches = self.matches(cx);
        let empty = matches.is_empty();
        let rows = matches
            .into_iter()
            .take(12)
            .enumerate()
            .map(|(ix, (kind, name))| {
                ListItem::new(ix)
                    .child(name)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.pick(&kind, cx);
                    }))
            });
        v_flex()
            .absolute()
            .left(self.pan.x + px(picker.at[0]))
            .top(self.pan.y + px(picker.at[1]))
            .w(px(240.))
            .p(px(4.))
            .gap(px(4.))
            .rounded(px(6.))
            .border_1()
            .border_color(theme.border)
            .bg(theme.popover)
            .shadow_md()
            // Clicks in the list are not clicks on the canvas.
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_mouse_up(MouseButton::Right, |_, _, cx| cx.stop_propagation())
            .child(Input::new(&picker.query))
            .children(rows)
            .children(empty.then(|| {
                div()
                    .p(px(6.))
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .child(t("picker-empty"))
            }))
    }
}

/// A node type's name in the app's language; a plugin's node without one shows its id.
pub(crate) fn node_name(id: &str) -> String {
    t_or(&format!("node-{}", id.replace('.', "-")), id)
}

fn pin_color(kind: PinKind, cx: &gpui_kit::App) -> Hsla {
    let theme = cx.theme();
    match kind {
        PinKind::Flow => theme.foreground,
        PinKind::Data(DataType::Text) => theme.cyan,
        PinKind::Data(DataType::Number) => theme.yellow,
        PinKind::Data(DataType::Bool) => theme.red,
        PinKind::Data(DataType::User) => theme.blue,
        PinKind::Data(DataType::Channel) => theme.green,
        PinKind::Data(DataType::Server) => theme.magenta,
    }
}

impl Render for Editor {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        self.frames.tick(window);
        let state = self.state.read(cx);
        let Some(file) = state.open_file() else {
            return super::placeholder(t("editor-empty"), cx).into_any_element();
        };
        let wire_layer = self.render_wires(file, &state.registry, cx);

        let visible = self.visible();
        let ids: Vec<NodeId> = file
            .graph
            .nodes
            .iter()
            .filter(|(id, node)| {
                let [x, y] = canvas::position(file, id);
                let [w, h] = canvas::size(state.registry.get(&node.kind));
                canvas::in_view(visible, [x, y], [x + w, y + h])
            })
            .map(|(id, _)| id.clone())
            .collect();
        let nodes: Vec<_> = ids
            .iter()
            .map(|id| {
                let state = self.state.read(cx);
                let file = state.open_file().expect("checked above");
                self.render_node(file, &state.registry, id, cx)
                    .into_any_element()
            })
            .collect();
        let picker = self
            .picker
            .as_ref()
            .map(|picker| self.render_picker(picker, cx).into_any_element());
        let add = div()
            .absolute()
            .top(px(8.))
            .left(px(8.))
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(
                Button::new("add-node")
                    .label("+")
                    .tooltip(t("canvas-add"))
                    .on_click(cx.listener(|this, _, window, cx| {
                        let at = [120. - f32::from(this.pan.x), 80. - f32::from(this.pan.y)];
                        this.open_picker(at, None, window, cx);
                    })),
            );

        let theme = cx.theme();
        div()
            .id("graph-canvas")
            .track_focus(&self.focus)
            .size_full()
            .relative()
            .overflow_hidden()
            .bg(theme.muted)
            .on_mouse_down(MouseButton::Left, cx.listener(Self::mouse_down))
            // On release: a press moves the focus to the canvas, away from the list's search.
            .on_mouse_up(
                MouseButton::Right,
                cx.listener(|this, event: &MouseUpEvent, window, cx| {
                    let at = this.graph_point(event.position);
                    this.open_picker(at, None, window, cx);
                }),
            )
            .on_mouse_move(cx.listener(Self::mouse_move))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::mouse_up))
            .on_scroll(cx.listener(|this, event: &ScrollEvent, _, cx| {
                this.pan += event.delta.pixel_delta(px(20.));
                cx.notify();
            }))
            .on_key_down(cx.listener(Self::key_down))
            .child(wire_layer)
            .children(nodes)
            .child(add)
            .children(picker)
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use gpui_kit::test::{TestEventExt as _, TestWindowExt as _};
    use gpui_kit::{
        AppContext as _, Entity, MouseButton, TestAppContext, VisualTestContext, point, px,
    };
    use rustbot_core::{NodeId, PinRef};

    use super::Editor;
    use crate::state::AppState;

    /// The editor alone in a window, on a new project's `/ping` graph: the slash command
    /// `n1` at [80, 120] wired to the reply `n2` at [380, 120].
    fn editor(
        cx: &mut TestAppContext,
    ) -> (Entity<AppState>, &mut VisualTestContext, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let project = crate::workspace::new_project(&dir.path().join("bot")).unwrap();
        cx.update(gpui_kit::init);
        let mut state = None;
        let (_, cx) = cx.add_window_view(|_, cx| {
            let app = cx.new(|cx| AppState::new(project, cx));
            state = Some(app.clone());
            Editor::new(app, cx)
        });
        cx.update(gpui_kit::test::TestWindowExt::render_frame);
        (state.unwrap(), cx, dir)
    }

    fn id(id: &str) -> NodeId {
        NodeId(id.to_owned())
    }

    #[gpui_kit::test]
    fn a_node_is_dragged(cx: &mut TestAppContext) {
        let (state, cx, _dir) = editor(cx);
        cx.update(|window, cx| {
            window.drag(point(px(150.), px(135.)), point(px(250.), px(235.)), cx);
        });
        cx.read(|cx| {
            let state = state.read(cx);
            assert_eq!(
                state.open_file().unwrap().layout.nodes[&id("n1")],
                [180., 220.]
            );
            assert_eq!(state.selected_node, Some(id("n1")));
        });
    }

    /// A wire pulled from the reply's `then` and dropped on empty canvas lists the nodes
    /// that fit; Enter adds the first and wires it.
    #[gpui_kit::test]
    fn a_dropped_wire_adds_a_wired_node(cx: &mut TestAppContext) {
        let (state, cx, _dir) = editor(cx);
        let then = point(px(380. + crate::canvas::WIDTH), px(120. + 30. + 12.));
        cx.update(|window, cx| window.drag(then, point(px(700.), px(400.)), cx));
        cx.update(|window, cx| window.press("enter", cx));
        cx.read(|cx| {
            let file = state.read(cx).open_file().unwrap();
            assert_eq!(file.graph.nodes[&id("n3")].kind, "core.reply");
            let from = PinRef {
                node: id("n2"),
                pin: "then".to_owned(),
            };
            let to = PinRef {
                node: id("n3"),
                pin: "run".to_owned(),
            };
            assert!(
                file.graph
                    .edges
                    .iter()
                    .any(|edge| edge.from == from && edge.to == to)
            );
        });
    }

    #[gpui_kit::test]
    fn delete_then_undo_and_redo(cx: &mut TestAppContext) {
        let (state, cx, _dir) = editor(cx);
        let reply = point(px(450.), px(135.));
        cx.update(|window, cx| {
            window.pointer_down(reply, MouseButton::Left, cx);
            window.pointer_up(reply, MouseButton::Left, cx);
            window.press("delete", cx);
        });
        let count = |cx: &mut VisualTestContext| {
            cx.read(|cx| {
                let file = state.read(cx).open_file().unwrap().clone();
                (file.graph.nodes.len(), file.graph.edges.len())
            })
        };
        assert_eq!(count(cx), (1, 0));
        cx.update(|window, cx| window.press("ctrl-z", cx));
        assert_eq!(count(cx), (2, 1));
        cx.update(|window, cx| window.press("ctrl-shift-z", cx));
        assert_eq!(count(cx), (1, 0));
    }

    /// A right-click opens the full list; typing filters it.
    #[gpui_kit::test]
    fn a_right_click_lists_and_searches_nodes(cx: &mut TestAppContext) {
        let (state, cx, _dir) = editor(cx);
        let at = point(px(600.), px(450.));
        cx.update(|window, cx| {
            window.pointer_down(at, MouseButton::Right, cx);
            window.pointer_up(at, MouseButton::Right, cx);
            window.render_frame(cx);
        });
        cx.update(|window, cx| window.input("slash", cx));
        cx.update(|window, cx| window.press("enter", cx));
        cx.read(|cx| {
            let file = state.read(cx).open_file().unwrap();
            assert_eq!(file.graph.nodes[&id("n3")].kind, "core.slash_command");
            assert_eq!(file.graph.edges.len(), 1);
        });
    }
}
