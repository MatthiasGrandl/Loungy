use gpui::*;

use crate::{
    query::{TextEvent, TextMovement},
    theme::Theme,
    workspace::Query,
};

#[derive(IntoElement, Clone)]
pub struct ListItem {
    text: String,
    selected: bool,
}

impl ListItem {
    pub fn new(text: String, selected: bool) -> Self {
        Self { text, selected }
    }
}

impl RenderOnce for ListItem {
    fn render(self, cx: &mut WindowContext) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let mut bg_hover = theme.mantle;
        bg_hover.fade_out(0.5);
        if self.selected {
            div().border_color(theme.crust).bg(theme.mantle)
        } else {
            div().hover(|s| s.bg(bg_hover))
        }
        .p_2()
        .border_1()
        .rounded_xl()
        .child(self.text)
    }
}

pub struct List {
    selected: usize,
    items: Vec<ListItem>,
}

impl Render for List {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        div().flex_1().children(
            self.items
                .clone()
                .into_iter()
                .enumerate()
                .skip(self.selected)
                .map(|(i, mut item)| {
                    item.selected = i == self.selected;
                    item.clone()
                }),
        )
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
            cx.subscribe(&query.inner, move |subscriber, emitter: &TextEvent, cx| {
                let clone = clone.clone();
                match emitter {
                    TextEvent::Input { text } => {
                        clone.update(cx, |this, cx| {
                            this.selected = 0;
                            this.items = text
                                .split_whitespace()
                                .map(|s| ListItem::new(s.to_string(), false))
                                .collect();
                            cx.notify();
                        });
                        // To update the root component of the workspace
                        // let test: AnyView = cx.new_view(|_cx| Test {}).into();
                        // cx.update_global::<State, _>(|state, cx| {
                        //     state.inner.update(cx, |state, _cx| {
                        //         state.root = test;
                        //     });
                        // });
                    }
                    TextEvent::Movement(TextMovement::Up) => {
                        clone.update(cx, |this, cx| {
                            if this.selected > 0 {
                                this.selected -= 1;
                                cx.notify();
                            }
                        });
                    }
                    TextEvent::Movement(TextMovement::Down) => {
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

pub struct Test {}
impl Render for Test {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        div().child("SNERZ!!!")
    }
}
