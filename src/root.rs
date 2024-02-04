use std::any::TypeId;

use gpui::*;

use crate::{
    query::{
        actions::{Input, MoveDown, MoveUp},
        Query,
    },
    theme::Theme,
};

#[derive(IntoElement, Clone)]
pub struct RootCommand {
    selected: usize,
}

impl RootCommand {
    pub fn new() -> Self {
        Self { selected: 0 }
    }
}

impl Global for RootCommand {}

impl RenderOnce for RootCommand {
    fn render(self, cx: &mut WindowContext) -> impl IntoElement {
        cx.on_action(TypeId::of::<MoveDown>(), move |_action, phase, cx| {
            if phase == DispatchPhase::Capture {
                return;
            }
            cx.update_global::<RootCommand, _>(|this, _| {
                this.selected += 1;
            });
            cx.refresh();
        });
        cx.on_action(TypeId::of::<MoveUp>(), move |_action, phase, cx| {
            if phase == DispatchPhase::Capture {
                return;
            }
            eprintln!("MoveUp");
            cx.update_global::<RootCommand, _>(|this, _| {
                if this.selected > 0 {
                    this.selected -= 1;
                }
            });
            cx.refresh();
        });
        cx.on_action(TypeId::of::<Input>(), move |_action, phase, cx| {
            if phase == DispatchPhase::Capture {
                return;
            }
            cx.update_global::<RootCommand, _>(|this, _| {
                this.selected = 0;
            });
            cx.refresh();
        });
        let theme = cx.global::<Theme>();
        let query = cx.global::<Query>();
        let selected = cx.global::<RootCommand>().selected;
        eprintln!("selected: {:?}", selected);

        let mut bg_hover = theme.mantle;
        bg_hover.fade_out(0.5);
        let children = query.inner.split_whitespace().enumerate().map(|(i, s)| {
            if i == selected {
                div().border_color(theme.crust).bg(theme.mantle)
            } else {
                div().hover(|s| s.bg(bg_hover))
            }
            .p_2()
            .border_1()
            .rounded_xl()
            .child(String::from(s))
        });
        div().children(children)
    }
}
