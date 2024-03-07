use std::time::Duration;

use gpui::*;
use serde::Deserialize;
use serde_json::Value;
use swift_rs::{swift, SRData};

use crate::{
    commands::{RootCommand, RootCommandBuilder},
    components::{
        list::{Accessory, ItemBuilder, ListBuilder, ListItem},
        shared::{Icon, Img},
    },
    state::{Action, Shortcut, StateModel, StateViewBuilder, StateViewContext},
};

#[derive(Deserialize)]
#[allow(dead_code)]
pub struct MenuItem {
    path: Vec<String>,
    #[serde(alias = "pathIndices")]
    path_indices: Option<Value>,
    shortcut: Option<Keystroke>,
}

// Function to list menu items
swift!( pub fn menu_items() -> SRData);

// Function to click a menu item
swift!( pub fn menu_item_select(data: SRData));

#[derive(Clone)]
pub struct MenuListBuilder;
impl StateViewBuilder for MenuListBuilder {
    fn build(&self, context: &mut StateViewContext, cx: &mut WindowContext) -> AnyView {
        context
            .query
            .set_placeholder("Search for menu items...", cx);
        ListBuilder::new()
            .interval(Duration::from_secs(10))
            .build(
                |_, _, _cx| {
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

                                    ItemBuilder::new(path.clone(), {
                                        ListItem::new(
                                            None,
                                            name.clone(),
                                            Some(subtitle.clone()),
                                            accessories,
                                        )
                                    })
                                    .keywords(vec![name.clone(), subtitle.clone()])
                                    .actions(actions)
                                    .build()
                                })
                                .collect(),
                        ))
                    } else {
                        Err(anyhow::Error::msg("Failed to deserialize menu list"))
                    }
                },
                context,
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
