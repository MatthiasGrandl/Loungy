use gpui::*;

use crate::icon::Icon;
use crate::list::Img;
use crate::state::{StateItem, StateModel};
use crate::theme::Theme;

pub struct Workspace {
    state: StateModel,
}

impl Workspace {
    pub fn build(cx: &mut WindowContext) -> View<Self> {
        let view = cx.new_view(|cx| {
            let state = StateModel::init(cx);
            cx.set_global(state.clone());
            Workspace { state }
        });
        view
    }
}

impl Render for Workspace {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let stack: &Vec<StateItem> = self.state.inner.read(cx).stack.as_ref();
        let item = stack.last().unwrap();
        let mut back = div();
        if stack.len() > 1 {
            back = div()
                .on_mouse_down(MouseButton::Left, move |_, cx| {
                    cx.update_global::<StateModel, _>(|this, cx| {
                        this.pop(cx);
                    });
                })
                .child(Img::list_icon(Icon::ArrowLeft, None));
        }
        div()
            .child(item.loading.clone())
            .full()
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
                    .text_lg()
                    .p_2()
                    .w_full()
                    .border_b_1()
                    .border_color(theme.mantle),
            )
            .child(div().child(item.view.clone()).p_2())
            .child(
                div()
                    .absolute()
                    .bottom_0()
                    .left_0()
                    .right_0()
                    .bg(theme.mantle)
                    .w_full()
                    .border_t_1()
                    .border_color(theme.crust)
                    .px_4()
                    .py_2()
                    .text_color(theme.subtext0)
                    .text_xs()
                    .flex()
                    .child(
                        div()
                            .mr_2()
                            .on_mouse_down(MouseButton::Left, |_ev, cx| {
                                Theme::change(catppuccin::Flavour::Latte, cx);
                            })
                            .child("Latte"),
                    )
                    .child(
                        div()
                            .mr_2()
                            .on_mouse_down(MouseButton::Left, |_ev, cx| {
                                Theme::change(catppuccin::Flavour::Mocha, cx);
                            })
                            .child("Mocha"),
                    )
                    .child(
                        div()
                            .mr_2()
                            .on_mouse_down(MouseButton::Left, |_ev, cx| {
                                Theme::change(catppuccin::Flavour::Frappe, cx);
                            })
                            .child("Frappe"),
                    )
                    .child(
                        div()
                            .mr_2()
                            .on_mouse_down(MouseButton::Left, |_ev, cx| {
                                Theme::change(catppuccin::Flavour::Macchiato, cx);
                            })
                            .child("Macchiato"),
                    )
                    .child(item.actions.inner.clone()),
            )
    }
}
