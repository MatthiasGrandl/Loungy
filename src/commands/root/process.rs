use gpui::*;
use std::{
    cmp::Reverse,
    collections::HashMap,
    fs,
    process::Command,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{channel, Sender},
    },
    time::Duration,
};

use regex::Regex;

use crate::{
    icon::Icon,
    list::{Accessory, Img, Item, List, ListItem},
    nucleo::fuzzy_match,
    paths::Paths,
    query::{TextEvent, TextInput},
    state::{Action, ActionsModel, Loading, Shortcut, StateView},
    swift::get_application_data,
    theme::Theme,
};

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
#[derive(Clone)]
struct ProcessList {
    list: View<List>,
    query: TextInput,
    model: Model<Vec<Item>>,
    sender: Sender<bool>,
    actions: ActionsModel,
}

impl ProcessList {
    fn update(&mut self, cx: &mut WindowContext) {
        let sort_by_cpu = CPU.load(Ordering::Relaxed);
        let s1 = self.sender.clone();
        let sort_action = if sort_by_cpu {
            Action::new(
                Img::list_icon(Icon::MemoryStick, None),
                "Sort by Memory",
                Some(Shortcut::simple("tab")),
                Box::new(move |_| {
                    CPU.store(false, Ordering::Relaxed);
                    let _ = s1.clone().send(true);
                }),
                false,
            )
        } else {
            Action::new(
                Img::list_icon(Icon::Cpu, None),
                "Sort by CPU",
                Some(Shortcut::simple("tab")),
                Box::new(move |_| {
                    CPU.store(true, Ordering::Relaxed);
                    let _ = s1.clone().send(true);
                }),
                false,
            )
        };
        self.actions.update_global(vec![sort_action], cx);
        let lavender = cx.global::<Theme>().lavender.clone();
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

        if sort_by_cpu {
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
                let pid = p.pid.to_string();
                let s1 = self.sender.clone();
                Item::new(
                    vec![name.clone()],
                    cx.new_view(|_cx| {
                        let (m, c) = if sort_by_cpu {
                            (None, Some(lavender))
                        } else {
                            (Some(lavender), None)
                        };
                        ListItem::new(
                            Some(image),
                            name.clone(),
                            None,
                            vec![
                                Accessory::new(
                                    format!("{: >6}", format_bytes(p.mem * 1024)),
                                    Some(Img::accessory_icon(Icon::MemoryStick, m)),
                                ),
                                Accessory::new(
                                    format!("{: >6.2}%", p.cpu),
                                    Some(Img::accessory_icon(Icon::Cpu, c)),
                                ),
                            ],
                        )
                    })
                    .into(),
                    None,
                    vec![Action::new(
                        Img::list_icon(Icon::Skull, None),
                        "Kill Process",
                        None,
                        Box::new(move |_| {
                            let _ = Command::new("kill").arg("-9").arg(pid.clone()).output();
                            let _ = s1.clone().send(true);
                        }),
                        false,
                    )],
                    None,
                )
            })
            .collect();

        self.model.update(cx, |model, _cx| {
            *model = items;
        });
        self.list(cx);
    }
    fn list(&mut self, cx: &mut WindowContext) {
        let query = self.query.view.read(cx).text.clone();
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
    fn build(
        &self,
        query: &TextInput,
        actions: &ActionsModel,
        loading: &View<Loading>,
        cx: &mut WindowContext,
    ) -> AnyView {
        let (s, r) = channel::<bool>();
        let mut comp = ProcessList {
            list: List::new(query, Some(actions), cx),
            query: query.clone(),
            model: cx.new_model(|_| Vec::<Item>::with_capacity(500)),
            sender: s,
            actions: actions.clone(),
        };
        query.set_placeholder("Search for running processes...", cx);
        comp.update(cx);
        let mut acomp = comp.clone();
        cx.new_view(|cx| {
            cx.spawn(|view, mut cx| async move {
                let mut last = std::time::Instant::now();
                loop {
                    if view.upgrade().is_none() {
                        break;
                    }
                    // if message in channel or last update was more than 5s ago
                    if last.elapsed().as_secs() > 5 || r.try_recv().is_ok() {
                        let _ = cx.update(|cx| {
                            acomp.update(cx);
                            last = std::time::Instant::now();
                        });
                    }
                    cx.background_executor()
                        .timer(Duration::from_millis(50))
                        .await;
                }
            })
            .detach();
            cx.subscribe(
                &query.view,
                move |subscriber: &mut ProcessList, _emitter, event, cx| match event {
                    TextEvent::Input { text: _ } => {
                        subscriber.list(cx);
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
