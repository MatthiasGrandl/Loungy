use gpui::*;
use swift_rs::SRData;

use crate::{
    icon::Icon,
    list::{Accessory, Img, Item, List, ListItem},
    nucleo::fuzzy_match,
    query::{TextEvent, TextInput},
    state::{Action, ActionsModel, Shortcut, StateView},
    swift::{menu_item_select, menu_items, MenuItem},
};

#[derive(Clone)]
struct MenuList {
    list: View<List>,
    query: TextInput,
    model: Model<Vec<Item>>,
}

impl MenuList {
    fn update(&mut self, cx: &mut WindowContext) {
        let data = unsafe { menu_items() };
        if let Ok(items) = serde_json::from_slice::<Vec<MenuItem>>(data.as_slice()) {
            let items: Vec<Item> = items
                .into_iter()
                .map(|item| {
                    let mut path = item.path.clone();
                    let name = path.pop().unwrap();
                    let subtitle = path.join(" -> ");
                    let actions = if let Some(indices) = item.path_indices {
                        let indices = indices.clone();
                        vec![Action::new(
                            Img::list_icon(Icon::BookOpen),
                            "Select Menu Item",
                            None,
                            Box::new(move |_| {
                                let data = serde_json::to_vec(&indices).unwrap();
                                unsafe { menu_item_select(SRData::from(data.as_slice())) };
                            }),
                            false,
                        )]
                    } else {
                        vec![]
                    };
                    let accessories = if let Some(shortcut) = item.shortcut {
                        eprintln!("shortcut: {:?}", shortcut);
                        vec![Accessory::Shortcut(Shortcut::new(shortcut))]
                    } else {
                        vec![]
                    };

                    Item::new(
                        vec![name.clone(), subtitle.clone()],
                        cx.new_view(|_| ListItem::new(None, name, Some(subtitle), accessories))
                            .into(),
                        None,
                        actions,
                        None,
                    )
                })
                .collect();
            self.model.update(cx, |this, _| {
                *this = items;
            });
            self.list(cx);
        };
    }
    fn list(&mut self, cx: &mut WindowContext) {
        let query = self.query.view.read(cx).text.clone();
        self.list.update(cx, |this, cx| {
            let items = self.model.read(cx).clone();
            let items = fuzzy_match(&query, items, false);
            this.items = items;
            cx.notify();
        });
    }
}

impl Render for MenuList {
    fn render(&mut self, _cx: &mut ViewContext<Self>) -> impl IntoElement {
        self.list.clone()
    }
}

pub struct MenuBuilder {}
impl StateView for MenuBuilder {
    fn build(&self, query: &TextInput, actions: &ActionsModel, cx: &mut WindowContext) -> AnyView {
        let mut comp = MenuList {
            list: List::new(query, Some(actions), cx),
            query: query.clone(),
            model: cx.new_model(|_| Vec::<Item>::new()),
        };
        query.set_placeholder("Search for menu items...", cx);
        comp.update(cx);

        cx.new_view(|cx| {
            cx.subscribe(
                &query.view,
                move |subscriber: &mut MenuList, _emitter, event, cx| match event {
                    TextEvent::Input { text: _ } => {
                        subscriber.list(cx);
                    }
                    _ => {}
                },
            )
            .detach();
            comp
        })
        .into()
    }
}
