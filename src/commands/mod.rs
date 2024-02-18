use std::collections::HashMap;

use gpui::*;

use crate::{
    components::list::{Accessory, Item, ListItem},
    components::shared::{Icon, Img},
    state::{Action, CloneableFn, Shortcut},
};

use self::root::{menu, process, theme};

#[cfg(feature = "bitwarden")]
pub mod bitwarden;
pub mod root;
#[cfg(feature = "tailscale")]
pub mod tailscale;

#[derive(Clone)]
pub struct RootCommand {
    id: String,
    title: String,
    subtitle: String,
    icon: Icon,
    keywords: Vec<String>,
    shortcut: Option<Shortcut>,
    pub action: Box<dyn CloneableFn>,
}
impl RootCommand {
    pub fn new(
        id: impl ToString,
        title: impl ToString,
        subtitle: impl ToString,
        icon: Icon,
        keywords: Vec<impl ToString>,
        shortcut: Option<Shortcut>,
        action: Box<dyn CloneableFn>,
    ) -> Self {
        Self {
            id: id.to_string(),
            title: title.to_string(),
            subtitle: subtitle.to_string(),
            icon,
            keywords: keywords.into_iter().map(|s| s.to_string()).collect(),
            shortcut,
            action,
        }
    }
}

pub trait RootCommandBuilder {
    fn build(&self, cx: &mut WindowContext) -> RootCommand;
}

#[derive(Clone)]
pub struct RootCommands {
    pub commands: HashMap<String, RootCommand>,
}

impl RootCommands {
    pub fn init(cx: &mut WindowContext) {
        let commands: Vec<Box<dyn RootCommandBuilder>> = vec![
            Box::new(menu::MenuCommandBuilder),
            Box::new(process::ProcessCommandBuilder),
            Box::new(theme::ThemeCommandBuilder),
            #[cfg(feature = "tailscale")]
            Box::new(tailscale::list::TailscaleCommandBuilder),
            #[cfg(feature = "bitwarden")]
            Box::new(bitwarden::list::BitwardenCommandBuilder),
        ];
        let mut map = HashMap::new();
        for command in commands {
            let command = command.build(cx);
            map.insert(command.id.clone(), command);
        }
        cx.set_global(Self { commands: map });
    }
    pub fn list(cx: &mut WindowContext) -> Vec<Item> {
        let commands = cx.global::<Self>().commands.clone();
        let items: Vec<Item> = commands
            .values()
            .into_iter()
            .map(|command| {
                let mut keywords = vec![command.title.clone(), command.subtitle.clone()];
                keywords.append(&mut command.keywords.clone());
                Item::new(
                    keywords,
                    cx.new_view(|_| {
                        ListItem::new(
                            Some(Img::list_icon(command.icon.clone(), None)),
                            command.title.clone(),
                            Some(command.subtitle.clone()),
                            command
                                .shortcut
                                .clone()
                                .and_then(|shortcut| Some(vec![Accessory::shortcut(shortcut)]))
                                .unwrap_or(vec![Accessory::new("Command", None)]),
                        )
                    })
                    .into(),
                    None,
                    vec![Action::new(
                        Img::list_icon(command.icon.clone(), None),
                        command.title.clone(),
                        None,
                        command.action.clone(),
                        false,
                    )],
                    Some(3),
                )
            })
            .collect();
        items
    }
}

impl Global for RootCommands {}
