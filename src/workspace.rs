use gpui::*;

use crate::components::shared::{Icon, Img};
use crate::state::{ActiveLoaders, StateItem, StateModel};
use crate::theme::Theme;

pub struct Workspace {
    state: StateModel,
}

impl Workspace {
    pub fn build(cx: &mut WindowContext) -> View<Self> {
        let view = cx.new_view(|cx| {
            let state = StateModel::init(cx);
            Workspace { state }
        });
        view
    }
}

impl Render for Workspace {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let loader = cx
            .global::<ActiveLoaders>()
            .inner
            .read(cx)
            .first()
            .cloned()
            .map(|l| l.upgrade().map(|l| l.into_any_element()))
            .flatten()
            .unwrap_or(div().h_px().bg(theme.mantle).into_any_element());
        let stack: &Vec<StateItem> = self.state.inner.read(cx).stack.as_ref();
        let item = stack.last().unwrap();
        let view = stack.iter().filter(|item| item.workspace).last().unwrap();

        let mut back = div();
        if stack.len() > 1 {
            back = div()
                .ml_2()
                .on_mouse_down(MouseButton::Left, move |_, cx| {
                    StateModel::update(|this, cx| this.pop(cx), cx);
                })
                .child(Img::list_icon(Icon::ArrowLeft, None));
        }
        let a = item.actions.read(cx).clone();
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(theme.base)
            .text_color(theme.text)
            .font(theme.font_sans.clone())
            .child(
                div()
                    .flex()
                    .items_center()
                    .child(back)
                    .child(item.query.clone())
                    .child(a.dropdown.clone())
                    .p_2()
                    .w_full(),
            )
            .child(loader)
            .child(div().flex_1().size_full().p_2().child(view.view.clone()))
            .child(
                div()
                    .mt_auto()
                    .bg(theme.mantle)
                    .w_full()
                    .border_t_1()
                    .border_color(theme.crust)
                    .px_4()
                    .py_2()
                    .text_color(theme.subtext0)
                    .text_xs()
                    .flex()
                    .child(a.toast.state.clone())
                    .child(item.actions.clone()),
            )
    }
}
