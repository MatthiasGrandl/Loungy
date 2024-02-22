use std::{sync::mpsc::Receiver, time::Duration};

use gpui::*;
use swift_rs::SRData;

use crate::{
    commands::{RootCommand, RootCommandBuilder},
    components::{
        list::{Accessory, Item, List, ListItem},
        shared::{Icon, Img},
    },
    query::{TextInput, TextInputWeak},
    state::{Action, ActionsModel, Shortcut, StateModel, StateViewBuilder},
    swift::{menu_item_select, menu_items, MenuItem},
};

#[derive(Clone)]
pub struct MenuListBuilder;
impl StateViewBuilder for MenuListBuilder {
    fn build(
        &self,
        query: &TextInputWeak,
        actions: &ActionsModel,
        update_receiver: Receiver<bool>,
        cx: &mut WindowContext,
    ) -> AnyView {
        query.set_placeholder("Search for menu items...", cx);
        List::new(
            query,
            &actions,
            |_, _, cx| {
                let data = unsafe { menu_items() };
                if let Ok(items) = serde_json::from_slice::<Vec<MenuItem>>(data.as_slice()) {
                    Ok(Some(
                        items
                            .into_iter()
                            .map(|item| {
                                let mut path = item.path.clone();
                                let name = path.pop().unwrap();
                                let subtitle = path.join(" -> ");
                                let actions = if let Some(indices) = item.path_indices {
                                    let indices = indices.clone();
                                    vec![Action::new(
                                        Img::list_icon(Icon::BookOpen, None),
                                        "Select Menu Item",
                                        None,
                                        move |this, cx| {
                                            let data = serde_json::to_vec(&indices).unwrap();
                                            unsafe {
                                                menu_item_select(SRData::from(data.as_slice()))
                                            };
                                            this.toast.success("Menu item selected", cx);
                                        },
                                        false,
                                    )]
                                } else {
                                    vec![]
                                };
                                let accessories = if let Some(shortcut) = item.shortcut {
                                    vec![Accessory::Shortcut(Shortcut::new(shortcut))]
                                } else {
                                    vec![]
                                };

                                Item::new(
                                    vec![name.clone(), subtitle.clone()],
                                    cx.new_view(|_| {
                                        ListItem::new(None, name, Some(subtitle), accessories)
                                    })
                                    .into(),
                                    None,
                                    actions,
                                    None,
                                    None,
                                    None,
                                )
                            })
                            .collect(),
                    ))
                } else {
                    Err(anyhow::Error::msg("Failed to deserialize menu list"))
                }
            },
            None,
            Some(Duration::from_secs(10)),
            update_receiver,
            true,
            cx,
        )
        .into()
    }
}

pub struct MenuCommandBuilder;

impl RootCommandBuilder for MenuCommandBuilder {
    fn build(&self, _cx: &mut WindowContext) -> RootCommand {
        RootCommand::new(
            "macos_menu",
            "Search Menu Items",
            "Navigation",
            Icon::Library,
            vec!["MacOS", "Apple"],
            None,
            Box::new(|_, cx| {
                StateModel::update(|this, cx| this.push(MenuListBuilder, cx), cx);
            }),
        )
    }
}
