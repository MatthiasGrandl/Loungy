use std::{collections::HashMap, path::PathBuf, sync::mpsc::Receiver, time::Duration};

use gpui::*;

use crate::{
    commands::{RootCommand, RootCommandBuilder, RootCommands},
    components::{
        list::{nucleo::fuzzy_match, Accessory, Item, ListBuilder, ListItem},
        shared::{Icon, Img},
    },
    platform::get_app_data,
    query::TextInputWeak,
    state::{Action, ActionsModel, StateViewBuilder},
    window::Window,
};

use super::numbat::Numbat;

#[derive(Clone)]
pub struct RootListBuilder;

impl StateViewBuilder for RootListBuilder {
    fn build(
        &self,
        query: &TextInputWeak,
        actions: &ActionsModel,
        update_receiver: Receiver<bool>,
        cx: &mut WindowContext,
    ) -> AnyView {
        query.set_placeholder("Search for apps and commands...", cx);
        let numbat = Numbat::init(query, cx);
        let commands = RootCommands::list(cx);
        ListBuilder::new()
            .build(
                query,
                actions,
                |_, _, cx| {
                    {
                        let user_dir = PathBuf::from("/Users")
                            .join(whoami::username())
                            .join("Applications");

                        #[cfg(target_os = "macos")]
                        let applications_folders = vec![
                            PathBuf::from("/Applications"),
                            PathBuf::from("/Applications/Chromium Apps"),
                            PathBuf::from("/System/Applications/Utilities"),
                            PathBuf::from("/System/Applications"),
                            PathBuf::from("/System/Library/CoreServices/Applications"),
                            PathBuf::from("/Library/PreferencePanes"),
                            PathBuf::from("/System/Library/ExtensionKit/Extensions"),
                            user_dir.clone(),
                            user_dir.clone().join("Chromium Apps.localized"),
                            // Not sure about the correct path for PWAs
                            user_dir.clone().join("Chrome Apps.localized"),
                            user_dir.clone().join("Brave Apps.localized"),
                        ];
                        #[cfg(target_os = "linux")]
                        let applications_folders = vec![
                            PathBuf::from("/usr/share/applications"),
                            PathBuf::from("/usr/local/share/applications"),
                            PathBuf::from("/home")
                                .join(whoami::username())
                                .join(".local/share/applications"),
                        ];

                        let mut apps = HashMap::<String, Item>::new();

                        for applications_folder in applications_folders {
                            let dir = applications_folder.read_dir();
                            if dir.is_err() {
                                continue;
                            }
                            for entry in dir.unwrap().flatten() {
                                let path = entry.path();

                                // search for .icns in Contents/Resources
                                let data = get_app_data(&path);
                                if data.is_none() {
                                    continue;
                                }
                                let data = data.unwrap();
                                let app = Item::new(
                                    data.id.clone(),
                                    vec![data.name.clone()],
                                    cx.new_view(|_cx| {
                                        ListItem::new(
                                            Some(data.icon.clone()),
                                            data.name.clone(),
                                            None,
                                            vec![Accessory::new(data.tag.clone(), None)],
                                        )
                                    })
                                    .into(),
                                    None,
                                    vec![Action::new(
                                        Img::list_icon(Icon::ArrowUpRightFromSquare, None),
                                        format!("Open {}", data.tag.clone()),
                                        None,
                                        {
                                            let id = data.id.clone();

                                            #[cfg(target_os = "macos")]
                                            {
                                                let ex = data.tag == "System Setting";
                                                move |_, cx| {
                                                    Window::close(cx);
                                                    let id = id.clone();
                                                    let mut command =
                                                        std::process::Command::new("open");
                                                    if ex {
                                                        command.arg(format!(
                                                            "x-apple.systempreferences:{}",
                                                            id
                                                        ));
                                                    } else {
                                                        command.arg("-b");
                                                        command.arg(id);
                                                    }
                                                    let _ = command.spawn();
                                                }
                                            }
                                            #[cfg(target_os = "linux")]
                                            {
                                                move |_, cx| {
                                                    Window::close(cx);
                                                    let mut command =
                                                        std::process::Command::new("gtk-launch");
                                                    command.arg(id.clone());
                                                    let _ = command.spawn();
                                                }
                                            }
                                        },
                                        false,
                                    )],
                                    None,
                                    None,
                                    None,
                                );
                                apps.insert(data.id, app);
                            }
                        }
                        let mut apps: Vec<Item> = apps.values().cloned().collect();
                        apps.sort_unstable_by_key(|a| a.keywords[0].clone());
                        Ok(Some(apps))
                    }
                },
                Some(Box::new(move |this, cx| {
                    let mut items = this.items_all.clone();
                    items.append(&mut commands.clone());
                    let query = this.query.view.upgrade();
                    if query.is_none() {
                        return vec![];
                    }
                    let query = query.unwrap().read(cx).text.clone();
                    let mut items = fuzzy_match(&query, items, false);
                    if items.is_empty() {
                        if let Some(result) = numbat.read(cx).result.clone() {
                            items.push(Item::new(
                                "Numbat",
                                Vec::<String>::new(),
                                numbat.clone().into(),
                                None,
                                vec![Action::new(
                                    Img::list_icon(Icon::Copy, None),
                                    "Copy",
                                    None,
                                    {
                                        move |this, cx: &mut WindowContext| {
                                            cx.write_to_clipboard(ClipboardItem::new(
                                                result.result.to_string(),
                                            ));
                                            this.toast.floating(
                                                "Copied to clipboard",
                                                Some(Icon::Clipboard),
                                                cx,
                                            );
                                            Window::close(cx);
                                        }
                                    },
                                    false,
                                )],
                                None,
                                None,
                                None,
                            ));
                        }
                    }
                    items
                })),
                Some(Duration::from_secs(60)),
                update_receiver,
                cx,
            )
            .into()
    }
}

pub struct LoungyCommandBuilder;

impl RootCommandBuilder for LoungyCommandBuilder {
    fn build(&self, _cx: &mut WindowContext) -> RootCommand {
        RootCommand::new(
            "loungy",
            "Loungy",
            "Preferences",
            Icon::Rocket,
            vec!["Settings"],
            None,
            Box::new(|actions, cx| {
                actions.toast.error("Preferences not yet implemented", cx);
            }),
        )
    }
}
