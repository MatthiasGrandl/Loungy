use gpui::*;

use crate::{
    list::{Action, Item, List, ListItem},
    nucleo::fuzzy_match,
    query::TextEvent,
    workspace::Query,
};

pub struct Root {
    list: View<List>,
}

impl Render for Root {
    fn render(&mut self, _cx: &mut gpui::ViewContext<Self>) -> impl gpui::IntoElement {
        self.list.clone()
    }
}

impl Root {
    pub fn build(cx: &mut gpui::WindowContext) -> gpui::View<Self> {
        cx.new_view(|cx| {
            let list = List::new(cx);
            let clone = list.clone();
            cx.update_global::<Query, _>(|query, cx| {
                cx.subscribe(
                    &query.inner,
                    move |_subscriber, _emitter, event, cx| match event {
                        TextEvent::Input { text } => {
                            clone.update(cx, |this, cx| {
                                let items: Vec<Item> = (0..10000)
                                    .map(|i| {
                                        let title = format!("Item {}", i);
                                        Item::new(
                                            vec![title.clone()],
                                            cx.new_view(|_cx| ListItem {
                                                title: title.clone(),
                                            })
                                            .into(),
                                            None,
                                            Vec::<Action>::new(),
                                            None,
                                            false,
                                        )
                                    })
                                    .collect();
                                this.items = fuzzy_match(text, items, false);
                                cx.notify();
                            });
                        }
                        _ => {}
                    },
                )
                .detach();
            });
            Root { list }
        })
    }
}
