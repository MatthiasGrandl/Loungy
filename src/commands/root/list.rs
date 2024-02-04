use std::{collections::HashMap, fs, path::PathBuf, sync::Arc};
use swift_rs::{swift, SRObject, SRString};

use gpui::*;

use crate::{
    lazy::LazyMutex,
    list::{Accessory, Action, Img, ImgMask, ImgSize, ImgSource, Item, List, ListItem},
    nucleo::fuzzy_match,
    paths::Paths,
    query::TextEvent,
    workspace::Query,
};

use super::numbat::Numbat;

static LIST: LazyMutex<Vec<Item>> = LazyMutex::new(Vec::<Item>::new);

#[repr(C)]
pub(super) struct AppData {
    pub(super) id: SRString,
    pub(super) name: SRString,
}

swift!(pub(super) fn get_application_data(cache_dir: &SRString, input: &SRString) -> Option<SRObject<AppData>>);

fn update(cx: &mut WindowContext) {
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
                let app = Item::new(
                    vec![name.clone()],
                    cx.new_view(|_cx| {
                        ListItem::new(
                            Some(Img::new(
                                ImgSource::Base(ImageSource::File(Arc::new(icon_path))),
                                ImgMask::None,
                                ImgSize::Medium,
                            )),
                            name.clone(),
                            None,
                            vec![Accessory::new(tag, None)],
                        )
                    })
                    .into(),
                    None,
                    Vec::<Action>::new(),
                    None,
                );
                apps.insert(bundle_id, app);
            }
        }
    }
    let mut apps: Vec<Item> = apps.values().cloned().collect();
    apps.sort_unstable_by_key(|a| a.keywords[0].clone());

    *LIST.lock() = apps;
}

pub struct Root {
    list: View<List>,
}

impl Render for Root {
    fn render(&mut self, _cx: &mut ViewContext<Self>) -> impl IntoElement {
        self.list.clone()
    }
}

fn list_items(list: &View<List>, query: &str, cx: &mut ViewContext<Root>) {
    list.update(cx, |this, cx| {
        let items = LIST.lock().clone();
        let mut items = fuzzy_match(query, items, false);
        if items.len() == 0 {
            if let Some(calc) = Numbat::init(query) {
                items.push(Item::new(
                    Vec::<String>::new(),
                    cx.new_view(|_cx| calc).into(),
                    None,
                    Vec::<Action>::new(),
                    None,
                ));
            }
        }
        this.items = items;
        cx.notify();
    });
}

impl Root {
    pub fn build(cx: &mut WindowContext) -> View<Self> {
        update(cx);
        cx.new_view(|cx| {
            let list = List::new(cx);
            let clone = list.clone();
            cx.update_global::<Query, _>(|query, cx| {
                query.inner.update(cx, |model, cx| {
                    model.reset(cx);
                    model.placeholder = "Search apps and commands...".to_string();
                    cx.notify();
                });
                list_items(&clone, "", cx);
                cx.subscribe(
                    &query.inner,
                    move |_subscriber, _emitter, event, cx| match event {
                        TextEvent::Input { text } => {
                            list_items(&clone, text.as_str(), cx);
                        }
                        _ => {}
                    },
                )
                .detach();
            });
            Root { list }
        })
    }
}
