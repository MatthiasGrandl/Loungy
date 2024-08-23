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

use std::{collections::HashMap, rc::Rc};

use gpui::*;
use log::error;
use serde::{Deserialize, Serialize};

use crate::{
    command,
    components::{
        form::{Form, Input, InputKind},
        list::{Accessory, Item, ItemBuilder, ListItem},
        shared::{Icon, Img},
    },
    hotkey::HotkeyManager,
    state::{
        Action, ActionFn, CommandTrait, Shortcut, StateModel, StateViewBuilder, StateViewContext,
    },
};

use self::root::list;

#[cfg(feature = "bitwarden")]
mod bitwarden;
#[cfg(feature = "clipboard")]
mod clipboard;
#[cfg(feature = "matrix")]
mod matrix;
#[cfg(target_os = "macos")]
mod menu;
mod process;
pub mod root;
#[cfg(feature = "tailscale")]
mod tailscale;
mod theme;

fn def() -> Rc<dyn ActionFn> {
    Rc::new(|_, _| {})
}

#[derive(Clone, Serialize, Deserialize)]
pub struct RootCommand {
    pub id: String,
    title: String,
    subtitle: String,
    icon: Icon,
    keywords: Vec<String>,
    #[serde(skip)]
    shortcut: Option<Shortcut>,
    #[serde(skip, default = "def")]
    pub action: Rc<dyn ActionFn>,
}
impl RootCommand {
    pub fn new(
        id: impl ToString,
        title: impl ToString,
        subtitle: impl ToString,
        icon: Icon,
        keywords: Vec<impl ToString>,
        shortcut: Option<Shortcut>,
        action: impl ActionFn,
    ) -> Self {
        Self {
            id: id.to_string(),
            title: title.to_string(),
            subtitle: subtitle.to_string(),
            icon,
            keywords: keywords.into_iter().map(|s| s.to_string()).collect(),
            shortcut,
            action: Rc::new(action),
        }
    }
}

pub trait RootCommandBuilder: CommandTrait {
    fn build(&self, cx: &mut WindowContext) -> RootCommand;
}

#[derive(Clone, Serialize, Deserialize)]
pub struct RootCommands {
    pub commands: HashMap<String, RootCommand>,
}

impl RootCommands {
    pub fn init(cx: &mut WindowContext) {
        let commands: Vec<Box<dyn RootCommandBuilder>> = vec![
            Box::new(list::LoungyCommandBuilder),
            #[cfg(target_os = "macos")]
            Box::new(menu::list::MenuCommandBuilder),
            Box::new(process::list::ProcessCommandBuilder),
            Box::new(theme::list::ThemeCommandBuilder),
            #[cfg(feature = "tailscale")]
            Box::new(tailscale::list::TailscaleCommandBuilder),
            #[cfg(feature = "bitwarden")]
            Box::new(bitwarden::list::BitwardenCommandBuilder),
            #[cfg(feature = "matrix")]
            Box::new(matrix::list::MatrixCommandBuilder),
            #[cfg(feature = "clipboard")]
            Box::new(clipboard::list::ClipboardCommandBuilder),
        ];
        let mut map = HashMap::new();
        for command in commands {
            let id = command.command();
            let mut command = command.build(cx);
            command.id = id.clone();
            map.insert(id, command);
        }
        cx.set_global(Self { commands: map });
    }
    pub fn list(cx: &mut WindowContext) -> Vec<Item> {
        let commands = cx.global::<Self>().commands.clone();
        let items: Vec<Item> = commands
            .values()
            .map(|command| {
                let mut keywords = vec![command.title.clone(), command.subtitle.clone()];
                keywords.append(&mut command.keywords.clone());
                ItemBuilder::new(
                    command.id.clone(),
                    ListItem::new(
                        Some(Img::default().icon(command.icon.clone())),
                        command.title.clone(),
                        Some(command.subtitle.clone()),
                        command
                            .shortcut
                            .clone()
                            .map(|shortcut| vec![Accessory::shortcut(shortcut)])
                            .unwrap_or(vec![Accessory::new("Command", None)]),
                    ),
                )
                .keywords(keywords)
                .actions(vec![
                    Action::new_rc(
                        Img::default().icon(command.icon.clone()),
                        command.title.clone(),
                        None,
                        command.action.clone(),
                        false,
                    ),
                    Action::new(
                        Img::default().icon(Icon::Keyboard),
                        "Change Hotkey",
                        None,
                        {
                            let id = command.id.clone();
                            move |_, cx| {
                                let id = id.clone();
                                StateModel::update(
                                    |this, cx| this.push(HotkeyBuilder { id }, cx),
                                    cx,
                                );
                            }
                        },
                        false,
                    ),
                ])
                .weight(3)
                .build()
            })
            .collect();
        items
    }
}

impl Global for RootCommands {}

#[derive(Clone)]
pub struct HotkeyBuilder {
    id: String,
}
command!(HotkeyBuilder);
impl StateViewBuilder for HotkeyBuilder {
    fn build(&self, context: &mut StateViewContext, cx: &mut WindowContext) -> AnyView {
        let id = self.id.clone();
        let value = HotkeyManager::get(&id).map(Shortcut::new);
        Form::new(
            vec![Input::new(
                "hotkey",
                "Hotkey",
                InputKind::Shortcut {
                    tmp: value.clone(),
                    value,
                },
                cx,
            )],
            move |values, actions, cx| {
                let shortcut = values["hotkey"].value::<Option<Shortcut>>();
                if let Some(shortcut) = shortcut {
                    if let Err(err) = HotkeyManager::set(&id, shortcut.get(), cx) {
                        error!("Failed to set hotkey: {}", err);
                        actions.toast.error("Failed to set hotkey", cx);
                    } else {
                        actions.toast.success("Hotkey set", cx);
                    }
                } else if let Err(err) = HotkeyManager::unset(&id, cx) {
                    error!("Failed to unset hotkey: {}", err);
                    actions.toast.error("Failed to unset hotkey", cx);
                } else {
                    actions.toast.success("Hotkey unset", cx);
                }
                // let shortcut = values["hotkey"].value::<Option<Shortcut>>().unwrap();
                // if let Some(shortcut) = shortcut {
                //     let mut model = cx.global::<StateModel>();
                //     model.hotkey = shortcut;
                //     model.save(cx);
                // }
                //
            },
            context,
            cx,
        )
        .into()
    }
}
