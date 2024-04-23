/*
 *
 *  This source file is part of the Loungy open source project
 *
 *  Copyright (c) 2024 Loungy, Matthias Grandl and the Loungy project contributors
 *  Licensed under MIT License
 *
 *  See https://github.com/MatthiasGrandl/Loungy/blob/main/LICENSE.md for license information
 *
 */

use gpui::*;

use crate::components::shared::{Icon, Img};
use crate::loader::ActiveLoaders;
use crate::state::{StateItem, StateModel};
use crate::theme::Theme;

pub struct Workspace {
    state: StateModel,
}

impl Workspace {
    pub fn build(cx: &mut WindowContext) -> View<Self> {
        cx.new_view(|cx| {
            let state = StateModel::init(cx);
            Workspace { state }
        })
    }
}

impl Render for Workspace {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
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
                .child(Img::default().icon(Icon::ArrowLeft));
        }
        let a = item.actions.read(cx).clone();

        div()
            .rounded_xl()
            .border()
            .border_color(theme.crust)
            .size_full()
            .flex()
            .flex_col()
            .bg({
                let mut bg = theme.base;
                bg.fade_out(0.1);
                bg
            })
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
            .child(ActiveLoaders {})
            .child(div().flex_1().size_full().p_2().child(view.view.clone()))
            .child(
                div()
                    .mt_auto()
                    .bg({
                        let mut bg = theme.mantle;
                        bg.fade_out(
                            1.0 - theme
                                .window_background
                                .clone()
                                .unwrap_or_default()
                                .opacity(),
                        );
                        bg
                    })
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
