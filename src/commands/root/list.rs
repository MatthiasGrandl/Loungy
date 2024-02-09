use std::{collections::HashMap, fs, path::PathBuf};

use gpui::*;

use crate::{
    icon::Icon,
    list::{Accessory, Img, Item, List, ListItem},
    nucleo::fuzzy_match,
    paths::Paths,
    query::{TextEvent, TextInput},
    state::{Action, ActionsModel, Loading, StateModel, StateView},
    swift::get_application_data,
};

use super::{menu::MenuBuilder, numbat::Numbat, process::ProcessBuilder, theme::ThemeListBuilder};

struct Root {
    model: Model<Vec<Item>>,
    list: View<List>,
    numbat: View<Numbat>,
}

impl Root {
    fn update(&mut self, cx: &mut WindowContext) {
        let cache_dir = cx.global::<Paths>().cache.clone();
        fs::create_dir_all(cache_dir.clone()).unwrap();

        let applications_folders = vec![
            PathBuf::from("/Applications"),
            PathBuf::from("/System/Applications/Utilities"),
            PathBuf::from("/System/Applications"),
            PathBuf::from("/System/Library/CoreServices/Applications"),
            PathBuf::from("/Library/PreferencePanes"),
            PathBuf::from("/System/Library/ExtensionKit/Extensions"),
        ];
        // iterate this folder
        // for each .app file, create an App struct
        // return a vector of App structs
        // list all files in applications_folder
        let mut apps = HashMap::<String, Item>::new();

        for applications_folder in applications_folders {
            for entry in applications_folder
                .read_dir()
                .expect("Unable to read directory")
            {
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
                            Box::new(move |cx| {
                                cx.hide();
                                let id = id.clone();
                                let mut command = std::process::Command::new("open");
                                if ex {
                                    command.arg(format!("x-apple.systempreferences:{}", id));
                                } else {
                                    command.arg("-b");
                                    command.arg(id);
                                }
                                let _ = command.spawn();
                            }),
                            false,
                        )],
                        None,
                    );
                    apps.insert(bundle_id, app);
                }
            }
        }
        let mut apps: Vec<Item> = apps.values().cloned().collect();
        apps.sort_unstable_by_key(|a| a.keywords[0].clone());

        self.model.update(cx, |model, cx| {
            *model = apps;
            cx.notify();
        });
    }
    fn list(&mut self, query: &str, cx: &mut WindowContext) {
        self.list.update(cx, |this, cx| {
            let mut items = self.model.read(cx).clone();
            items.append(&mut vec![
                Item::new(
                    vec!["Search Process", "Kill", "Task Manager", "Memory", "CPU"],
                    cx.new_view(|_| {
                        ListItem::new(
                            Some(Img::list_icon(Icon::Cpu, None)),
                            "Search Process",
                            Some("Task Manager".to_string()),
                            Vec::<Accessory>::new(),
                        )
                    })
                    .into(),
                    None,
                    vec![Action::new(
                        Img::list_icon(Icon::Skull, None),
                        "Search Process",
                        None,
                        Box::new(|cx| {
                            cx.update_global::<StateModel, _>(|this, cx| {
                                this.push(ProcessBuilder {}, cx);
                            });
                        }),
                        false,
                    )],
                    Some(3),
                ),
                Item::new(
                    vec!["Menu Items", "Navigation"],
                    cx.new_view(|_| {
                        ListItem::new(
                            Some(Img::list_icon(Icon::Library, None)),
                            "Search Menu Items",
                            Some("Navigation".to_string()),
                            Vec::<Accessory>::new(),
                        )
                    })
                    .into(),
                    None,
                    vec![Action::new(
                        Img::list_icon(Icon::Library, None),
                        "Search Menu Items",
                        None,
                        Box::new(|cx| {
                            cx.update_global::<StateModel, _>(|this, cx| {
                                this.push(MenuBuilder {}, cx);
                            });
                        }),
                        false,
                    )],
                    Some(3),
                ),
                Item::new(
                    vec!["Search Themes", "Customization"],
                    cx.new_view(|_| {
                        ListItem::new(
                            Some(Img::list_icon(Icon::Palette, None)),
                            "Search Themes",
                            Some("Customization".to_string()),
                            Vec::<Accessory>::new(),
                        )
                    })
                    .into(),
                    None,
                    vec![Action::new(
                        Img::list_icon(Icon::Palette, None),
                        "Search Themes",
                        None,
                        Box::new(|cx| {
                            cx.update_global::<StateModel, _>(|this, cx| {
                                this.push(ThemeListBuilder {}, cx);
                            });
                        }),
                        false,
                    )],
                    Some(3),
                ),
            ]);

            let mut items = fuzzy_match(query, items, false);
            if items.len() == 0 {
                if let Some(result) = self.numbat.read(cx).result.clone() {
                    items.push(Item::new(
                        Vec::<String>::new(),
                        self.numbat.clone().into(),
                        None,
                        vec![Action::new(
                            Img::list_icon(Icon::Copy, None),
                            "Copy",
                            None,
                            Box::new(move |cx| {
                                cx.write_to_clipboard(ClipboardItem::new(
                                    result.result.to_string(),
                                ));
                                cx.hide();
                            }),
                            false,
                        )],
                        None,
                    ));
                }
            }
            this.items = items;
            cx.notify();
        });
    }
}

impl Render for Root {
    fn render(&mut self, _cx: &mut ViewContext<Self>) -> impl IntoElement {
        self.list.clone()
    }
}

pub struct RootBuilder;

impl StateView for RootBuilder {
    fn build(
        &self,
        query: &TextInput,
        actions: &ActionsModel,
        loading: &View<Loading>,
        cx: &mut WindowContext,
    ) -> AnyView {
        let list = List::new(query, Some(&actions), cx);
        let numbat = Numbat::init(&query, cx);
        let mut root = Root {
            list,
            model: cx.new_model(|_| Vec::<Item>::with_capacity(500)),
            numbat,
        };
        root.update(cx);
        root.list("", cx);
        query.set_placeholder("Search for apps and commands...", cx);
        cx.new_view(|cx| {
            cx.subscribe(
                &query.view,
                move |subscriber: &mut Root, _emitter, event, cx| match event {
                    TextEvent::Input { text } => {
                        subscriber.list(text.as_str(), cx);
                    }
                    _ => {}
                },
            )
            .detach();
            root
        })
        .into()
    }
}
