use gpui::*;
use std::{cmp::Reverse, collections::HashMap, fs, process::Command, sync::atomic::AtomicBool};

use regex::Regex;

use crate::{
    icon::Icon,
    list::{Accessory, Img, Item, List, ListItem},
    nucleo::fuzzy_match,
    paths::Paths,
    query::{TextEvent, TextInput},
    state::{ActionsModel, StateView},
};

use super::list::get_application_data;

static CPU: AtomicBool = AtomicBool::new(false);

#[derive(Clone)]
struct Process {
    pid: u64,
    ppid: u64,
    cpu: f32,
    mem: u64,
    name: String,
}

impl Process {
    fn parse(line: &str) -> anyhow::Result<Self> {
        let split: Vec<&str> = line.split_whitespace().collect();
        if split.len() < 5 {
            return Err(anyhow::anyhow!("invalid line"));
        }

        Ok(Process {
            pid: split[0].parse()?,
            ppid: split[1].parse()?,
            cpu: split[2].parse()?,
            mem: split[3].parse()?,
            name: split[4].to_string(),
        })
    }
}

fn format_bytes(bytes: u64) -> String {
    let kb = bytes / 1024;
    let mb = kb / 1024;
    let gb = mb / 1024;
    if gb > 0 {
        format!("{} GB", gb)
    } else if mb > 0 {
        format!("{} MB", mb)
    } else if kb > 0 {
        format!("{} KB", kb)
    } else {
        format!("{} B", bytes)
    }
}

struct ProcessList {
    list: View<List>,
    model: Model<Vec<Item>>,
}

impl ProcessList {
    fn update(&mut self, cx: &mut WindowContext) {
        let cache_dir = cx.global::<Paths>().cache.clone();
        fs::create_dir_all(cache_dir.clone()).unwrap();
        let ps = Command::new("ps")
            .arg("-eo")
            .arg("pid,ppid,pcpu,rss,comm")
            .output()
            .expect("failed to get process list")
            .stdout;

        let parsed: Vec<Process> = String::from_utf8(ps)
            .unwrap()
            .split("\n")
            .skip(1)
            .filter_map(|line| Process::parse(line).ok())
            .collect();

        let mut aggregated = HashMap::<u64, Process>::new();
        parsed.iter().for_each(|p| {
            if p.ppid == 1 {
                aggregated.insert(p.pid, p.clone());
            } else {
                if let Some(parent) = aggregated.get(&p.ppid) {
                    let mut parent = parent.clone();
                    parent.cpu += p.cpu;
                    parent.mem += p.mem;
                    aggregated.insert(p.ppid, parent);
                }
            }
        });
        let mut parsed = aggregated.values().cloned().collect::<Vec<Process>>();

        if CPU.load(std::sync::atomic::Ordering::Relaxed) {
            parsed.sort_unstable_by_key(|p| Reverse(p.cpu as u64));
        } else {
            parsed.sort_unstable_by_key(|p| Reverse(p.mem));
        }

        let re = Regex::new(r"(.+\.(?:prefPane|app))(?:/.*)?$").unwrap();
        let items: Vec<Item> = parsed
            .iter()
            .map(|p| {
                let path = re
                    .captures(p.name.as_str())
                    .and_then(|caps| caps.get(1))
                    .map(|m| String::from(m.as_str()))
                    .unwrap_or_default();

                let (name, image) = match unsafe {
                    get_application_data(&cache_dir.to_str().unwrap().into(), &path.as_str().into())
                } {
                    Some(d) => {
                        let mut icon_path = cache_dir.clone();
                        icon_path.push(format!("{}.png", d.id.to_string()));
                        (d.name.to_string(), Img::list_file(icon_path))
                    }
                    None => {
                        let mut icon_path = cache_dir.clone();
                        icon_path.push("com.apple.Terminal.png");
                        (
                            p.name.split("/").last().unwrap().to_string(),
                            Img::list_file(icon_path),
                        )
                    }
                };
                // let sort_action = match CPU.load(std::sync::atomic::Ordering::Relaxed) {
                //     true => Action::new(
                //         "plugin:root|toggle_process_sort",
                //         "Sort by Memory",
                //         Some(Image::Icon {
                //             icon: Icon::MemoryStick,
                //             mask: Some(ImageMask::RoundedRectangle),
                //         }),
                //         Some(Shortcut::new("Tab", vec![])),
                //         None,
                //     ),
                //     false => Action::new(
                //         "plugin:root|toggle_process_sort",
                //         "Sort by CPU",
                //         Some(Image::Icon {
                //             icon: Icon::Cpu,
                //             mask: Some(ImageMask::RoundedRectangle),
                //         }),
                //         Some(Shortcut::new("Tab", vec![])),
                //         None,
                //     ),
                // };
                Item::new(
                    vec![name.clone()],
                    cx.new_view(|_| {
                        ListItem::new(
                            Some(image),
                            name.clone(),
                            None,
                            vec![
                                Accessory::new(
                                    format_bytes(p.mem * 1024),
                                    Some(Img::accessory_icon(Icon::MemoryStick)),
                                ),
                                Accessory::new(
                                    format!("{:.2}%", p.cpu),
                                    Some(Img::accessory_icon(Icon::Cpu)),
                                ),
                            ],
                        )
                    })
                    .into(),
                    None,
                    vec![],
                    None,
                )
                // Item::new(
                //     p.pid.to_string(),
                //     name,
                //     Vec::<String>::new(),
                //     vec![
                //         // Action::new(
                //         //     "plugin:root|kill_process",
                //         //     "Kill Process",
                //         //     Some(Image::Icon {
                //         //         icon: Icon::Skull,
                //         //         mask: Some(ImageMask::RoundedRectangle),
                //         //     }),
                //         //     None,
                //         //     None,
                //         // ),
                //         // sort_action,
                //     ],
                //     Component::ListItem {
                //         subtitle: None,
                //         icon: Some(image),
                //         accessories: vec![
                //             Accessory {
                //                 icon: Some(Image::Icon {
                //                     icon: Icon::MemoryStick,
                //                     mask: None,
                //                 }),
                //                 tag: format_bytes(p.mem * 1024),
                //             },
                //             Accessory {
                //                 icon: Some(Image::Icon {
                //                     icon: Icon::Cpu,
                //                     mask: None,
                //                 }),
                //                 tag: format!("{:.2}%", p.cpu),
                //             },
                //         ],
                //     },
                //     None,
                //     None,
                //     None,
                // )
            })
            .collect();

        self.model.update(cx, |model, cx| {
            *model = items;
            cx.notify();
        });
    }
    fn list(&mut self, query: &str, cx: &mut WindowContext) {
        self.list.update(cx, |this, cx| {
            let items = self.model.read(cx).clone();
            let items = fuzzy_match(&query, items, false);
            this.items = items;
            cx.notify();
        });
    }
}

impl Render for ProcessList {
    fn render(&mut self, _cx: &mut ViewContext<Self>) -> impl IntoElement {
        self.list.clone()
    }
}

pub struct ProcessBuilder {}
impl StateView for ProcessBuilder {
    fn build(&self, query: &TextInput, actions: &ActionsModel, cx: &mut WindowContext) -> AnyView {
        let mut comp = ProcessList {
            list: List::new(query, actions, cx),
            model: cx.new_model(|_cx| Vec::<Item>::with_capacity(100)),
        };
        comp.update(cx);
        comp.list("", cx);
        query.set_placeholder("Search for running processes...", cx);
        cx.new_view(|cx| {
            cx.subscribe(
                &query.view,
                move |subscriber: &mut ProcessList, _emitter, event, cx| match event {
                    TextEvent::Input { text } => {
                        subscriber.list(text.as_str(), cx);
                    }
                    _ => {}
                },
            )
            .detach();
            comp
        })
        .into()
    }
}

// #[tauri::command]
// pub(super) fn toggle_process_sort(handle: AppHandle) {
//     CPU.store(
//         !CPU.load(std::sync::atomic::Ordering::Relaxed),
//         std::sync::atomic::Ordering::Relaxed,
//     );
//     _ = handle.emit_all("plugin:root_list_processes", "");
// }

// #[tauri::command]
// pub(super) fn kill_process(id: String, handle: AppHandle) {
//     info!("killing process {}", id);
//     Command::new("kill").arg("-9").arg(id).output().unwrap();
//     _ = handle.emit_all("plugin:root_list_processes", "");
// }
