use std::{collections::HashMap, fs, path::PathBuf, sync::mpsc::Receiver, time::Duration};

use gpui::*;

#[cfg(target_os = "macos")]
use crate::swift::get_application_data;
use crate::{
    commands::{RootCommand, RootCommandBuilder, RootCommands},
    components::{
        list::{nucleo::fuzzy_match, Accessory, Item, List, ListItem},
        shared::{Icon, Img},
    },
    paths::{paths, Paths},
    query::{TextInput, TextInputWeak},
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
        let numbat = Numbat::init(&query, cx);
        let commands = RootCommands::list(cx);
        List::new(
            query,
            &actions,
            |_, _, cx| {
                #[cfg(target_os = "macos")]
                {
                    let cache_dir = paths().cache.clone();
                    fs::create_dir_all(cache_dir.clone()).unwrap();

                    let user_dir = PathBuf::from("/Users")
                        .join(whoami::username())
                        .join("Applications");

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
                    // iterate this folder
                    // for each .app file, create an App struct
                    // return a vector of App structs
                    // list all files in applications_folder
                    let mut apps = HashMap::<String, Item>::new();

                    for applications_folder in applications_folders {
                        let dir = applications_folder.read_dir();
                        if dir.is_err() {
                            continue;
                        }
                        for entry in dir.unwrap() {
                            if let Ok(entry) = entry {
                                let path = entry.path();
                                let extension = match path.extension() {
                                    Some(ext) => ext,
                                    None => continue,
                                };
                                let ex = extension.to_str().unwrap() == "appex";
                                let tag = match ex {
                                    true => "System Setting",
                                    false => "Application",
                                };
                                // search for .icns in Contents/Resources
                                let (bundle_id, name) = match unsafe {
                                    get_application_data(
                                        &cache_dir.to_str().unwrap().into(),
                                        &path.to_str().unwrap().into(),
                                    )
                                } {
                                    Some(d) => (d.id.to_string(), d.name.to_string()),
                                    None => continue,
                                };
                                let mut icon_path = cache_dir.clone();
                                icon_path.push(format!("{}.png", bundle_id.clone()));
                                let id = bundle_id.clone();
                                let app = Item::new(
                                    vec![name.clone()],
                                    cx.new_view(|_cx| {
                                        ListItem::new(
                                            Some(Img::list_file(icon_path)),
                                            name.clone(),
                                            None,
                                            vec![Accessory::new(tag, None)],
                                        )
                                    })
                                    .into(),
                                    None,
                                    vec![Action::new(
                                        Img::list_icon(Icon::ArrowUpRightFromSquare, None),
                                        format!("Open {}", tag),
                                        None,
                                        move |_, cx| {
                                            Window::close(cx);
                                            let id = id.clone();
                                            let mut command = std::process::Command::new("open");
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
                                        },
                                        false,
                                    )],
                                    None,
                                    None,
                                    None,
                                );
                                apps.insert(bundle_id, app);
                            }
                        }
                    }
                    let mut apps: Vec<Item> = apps.values().cloned().collect();
                    apps.sort_unstable_by_key(|a| a.keywords[0].clone());
                    Ok(Some(apps))
                }
                #[cfg(target_os = "linux")]
                {
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
                        for entry in dir.unwrap() {
                            if entry.is_err() {
                                continue;
                            }
                            let entry = entry.unwrap();
                            let path = entry.path();
                            let name = path
                                .components()
                                .last()
                                .unwrap()
                                .as_os_str()
                                .to_string_lossy()
                                .to_string();
                            let app = Item::new(
                                vec![name.clone()],
                                cx.new_view(|_cx| {
                                    ListItem::new(
                                        Some(Img::list_icon(Icon::AppWindow, None)),
                                        name.clone(),
                                        None,
                                        vec![Accessory::new("Application", None)],
                                    )
                                })
                                .into(),
                                None,
                                vec![Action::new(
                                    Img::list_icon(Icon::ArrowUpRightFromSquare, None),
                                    "Open Application",
                                    None,
                                    {
                                        let name = name.clone();
                                        move |_, cx| {
                                            Window::close(cx);
                                            let mut command =
                                                std::process::Command::new("gtk-launch");
                                            command.arg(name.clone());
                                            let _ = command.spawn();
                                        }
                                    },
                                    false,
                                )],
                                None,
                            );
                            apps.insert(name.to_string(), app);
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
                if items.len() == 0 {
                    if let Some(result) = numbat.read(cx).result.clone() {
                        items.push(Item::new(
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
            true,
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
