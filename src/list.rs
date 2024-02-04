use gpui::*;

use crate::{
    query::{TextEvent, TextMovement},
    theme::Theme,
    workspace::Query,
};

#[derive(Clone)]
pub struct ListItem {
    pub title: String,
}

impl Render for ListItem {
    fn render(&mut self, _cx: &mut ViewContext<Self>) -> impl IntoElement {
        div().child(self.title.clone())
    }
}

#[derive(Clone)]
pub struct Action {
    label: String,
}

#[derive(IntoElement, Clone)]
pub struct Item {
    pub keywords: Vec<String>,
    component: AnyView,
    preview: Option<AnyView>,
    actions: Vec<Action>,
    pub weight: Option<u16>,
    selected: bool,
}

impl Item {
    pub fn new(
        keywords: Vec<impl ToString>,
        component: AnyView,
        preview: Option<AnyView>,
        actions: Vec<Action>,
        weight: Option<u16>,
        selected: bool,
    ) -> Self {
        Self {
            keywords: keywords.into_iter().map(|s| s.to_string()).collect(),
            component,
            preview,
            actions,
            weight,
            selected,
        }
    }
}

impl RenderOnce for Item {
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
        .child(self.component)
    }
}

pub struct List {
    selected: usize,
    pub items: Vec<Item>,
}

impl Render for List {
    fn render(&mut self, _cx: &mut ViewContext<Self>) -> impl IntoElement {
        div().flex_1().children(
            self.items
                .clone()
                .into_iter()
                .enumerate()
                .skip(self.selected)
                .map(|(i, mut item)| {
                    item.selected = i == self.selected;
                    item
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
            cx.subscribe(&query.inner, move |_subscriber, emitter: &TextEvent, cx| {
                let clone = clone.clone();
                match emitter {
                    TextEvent::Input { text: _ } => {
                        clone.update(cx, |this, cx| {
                            this.selected = 0;
                            cx.notify();
                        });
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
