use gpui::*;

use crate::{
    query::{Query, QueryEvent, QueryModel, QueryMovement},
    theme::Theme,
};

#[derive(IntoElement, Clone)]
pub struct RootCommand {
    selected: usize,
}

impl RootCommand {
    pub fn new(cx: &mut WindowContext) -> Self {
        cx.update_global::<Query, _>(|query, cx| {
            cx.subscribe(&query.inner, |_subscriber, emitter: &QueryEvent, cx| {
                match emitter {
                    QueryEvent::Input { text: _ } => {
                        cx.update_global::<RootCommand, _>(|this, _| {
                            this.selected = 0;
                        });
                    }
                    QueryEvent::Movement(QueryMovement::Up) => {
                        cx.update_global::<RootCommand, _>(|this, _| {
                            if this.selected > 0 {
                                this.selected -= 1;
                            }
                        });
                    }
                    QueryEvent::Movement(QueryMovement::Down) => {
                        cx.update_global::<RootCommand, _>(|this, _| {
                            this.selected += 1;
                        });
                    }
                }
                cx.refresh();
            })
            .detach();
        });
        Self { selected: 0 }
    }
}

impl Global for RootCommand {}

impl RenderOnce for RootCommand {
    fn render(self, cx: &mut WindowContext) -> impl IntoElement {
        let query = cx.global::<Query>();
        let theme = cx.global::<Theme>();

        let selected = cx.global::<RootCommand>().selected;

        let text = query.inner.read(cx).text.clone();

        let mut bg_hover = theme.mantle;
        bg_hover.fade_out(0.5);
        let children = text.split_whitespace().enumerate().map(|(i, s)| {
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
