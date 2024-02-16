use gpui::*;

use crate::{
    icon::Icon,
    list::{Accessory, Img, Item, ListItem},
    state::{Action, CloneableFn, Shortcut},
};

use self::root::{menu, process, theme};

#[cfg(feature = "bitwarden")]
pub mod bitwarden;
pub mod root;
#[cfg(feature = "tailscale")]
pub mod tailscale;

pub struct RootCommand {
    title: String,
    subtitle: String,
    icon: Icon,
    keywords: Vec<String>,
    shortcut: Option<Shortcut>,
    action: Box<dyn CloneableFn>,
}
impl RootCommand {
    pub fn new(
        title: impl ToString,
        subtitle: impl ToString,
        icon: Icon,
        keywords: Vec<impl ToString>,
        shortcut: Option<Shortcut>,
        action: Box<dyn CloneableFn>,
    ) -> Self {
        Self {
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

pub struct RootCommands;

impl RootCommands {
    pub fn list(cx: &mut WindowContext) -> Vec<Item> {
        let commands: Vec<Box<dyn RootCommandBuilder>> = vec![
            Box::new(menu::MenuCommandBuilder),
            Box::new(process::ProcessCommandBuilder),
            Box::new(theme::ThemeCommandBuilder),
            #[cfg(feature = "tailscale")]
            Box::new(tailscale::list::TailscaleCommandBuilder),
            #[cfg(feature = "bitwarden")]
            Box::new(bitwarden::list::BitwardenCommandBuilder),
        ];
        let items: Vec<Item> = commands
            .into_iter()
            .map(|c| {
                let command = c.build(cx);
                let mut keywords = vec![command.title.clone(), command.subtitle.clone()];
                keywords.append(&mut command.keywords.clone());
                Item::new(
                    keywords,
                    cx.new_view(|_| {
                        ListItem::new(
                            Some(Img::list_icon(command.icon.clone(), None)),
                            command.title.clone(),
                            Some(command.subtitle),
                            command
                                .shortcut
                                .and_then(|shortcut| Some(vec![Accessory::shortcut(shortcut)]))
                                .unwrap_or(vec![Accessory::new("Command", None)]),
                        )
                    })
                    .into(),
                    None,
                    vec![Action::new(
                        Img::list_icon(command.icon, None),
                        command.title,
                        None,
                        command.action,
                        false,
                    )],
                    Some(3),
                )
            })
            .collect();
        items
    }
}
