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

use std::time::Duration;

use gpui::*;
use serde::Deserialize;
use serde_json::Value;
use swift_rs::{swift, SRData};

use crate::{
    command,
    commands::{RootCommand, RootCommandBuilder},
    components::{
        list::{Accessory, ItemBuilder, ListBuilder, ListItem},
        shared::{Icon, Img},
    },
    state::{Action, CommandTrait, Shortcut, StateModel, StateViewBuilder, StateViewContext},
};

#[derive(Deserialize)]

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
command!(MenuListBuilder);
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
                    let result = serde_json::from_slice::<Vec<MenuItem>>(data.as_slice());
                    if let Ok(items) = result {
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
                                            Img::default().icon(Icon::BookOpen),
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
                                        vec![Accessory::Shortcut(Shortcut::from(&shortcut))]
                                    } else {
                                        vec![]
                                    };

                                    ItemBuilder::new(item.path.clone(), {
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
                        log::error!(
                            "Failed to deserialize menu list: {:?}",
                            result.err().unwrap()
                        );
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
command!(MenuCommandBuilder);

impl RootCommandBuilder for MenuCommandBuilder {
    fn build(&self, _cx: &mut WindowContext) -> RootCommand {
        RootCommand::new(
            "macos_menu",
            "Search Menu Items",
            "Navigation",
            Icon::Library,
            vec!["MacOS", "Apple"],
            None,
            |_, cx| {
                StateModel::update(|this, cx| this.push(MenuListBuilder, cx), cx);
            },
        )
    }
}
