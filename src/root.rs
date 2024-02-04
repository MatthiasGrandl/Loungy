use gpui::*;

use crate::{
    query::{Query, QueryEvent, QueryModel, QueryMovement},
    theme::Theme,
};

pub struct List {
    selected: usize,
    items: Vec<String>,
}

impl Render for List {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let selected = self.selected;
        let mut bg_hover = theme.mantle;
        bg_hover.fade_out(0.5);
        let items = self.items.iter().enumerate().skip(selected).map(|(i, s)| {
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

        div().children(items)
    }
}

impl List {
    pub fn new(cx: &mut WindowContext) -> View<Self> {
        let view = cx.new_view(|_cx| Self {
            selected: 0,
            items: vec![],
        });
        let clone = view.clone();
        cx.update_global::<Query, _>(|query, cx| {
            cx.subscribe(&query.inner, move |subscriber, emitter: &QueryEvent, cx| {
                let clone = clone.clone();
                match emitter {
                    QueryEvent::Input { text } => {
                        clone.update(cx, |this, cx| {
                            this.selected = 0;
                            this.items = text.split_whitespace().map(String::from).collect();
                            cx.notify();
                        });
                    }
                    QueryEvent::Movement(QueryMovement::Up) => {
                        clone.update(cx, |this, cx| {
                            if this.selected > 0 {
                                this.selected -= 1;
                                cx.notify();
                            } else {
                                subscriber.update(cx, |editor, cx| {
                                    editor.reset(cx);
                                });
                            }
                        });
                    }
                    QueryEvent::Movement(QueryMovement::Down) => {
                        clone.update(cx, |this, cx| {
                            this.selected += 1;
                            cx.notify();
                        });
                    }
                }
            })
            .detach();
        });
        view
    }
}
