use gpui::*;
use std::{
    cmp::Reverse, collections::HashMap, fs, process::Command, sync::mpsc::Receiver, time::Duration,
};

use regex::Regex;

#[cfg(target_os = "macos")]
use crate::swift::get_application_data;
use crate::{
    commands::{RootCommand, RootCommandBuilder},
    components::{
        list::{Accessory, Item, List, ListItem},
        shared::{Icon, Img},
    },
    paths::paths,
    query::{TextInput, TextInputWeak},
    state::{Action, ActionsModel, StateModel, StateViewBuilder},
    theme::Theme,
};

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
    let kb = bytes / 1000;
    let mb = kb / 1000;
    let gb = mb as f32 / 1000.0;
    if gb >= 1.0 {
        format!("{:.2} GB", gb)
    } else if mb > 0 {
        format!("{} MB", mb)
    } else if kb > 0 {
        format!("{} KB", kb)
    } else {
        format!("{} B", bytes)
    }
}

#[derive(Clone)]
pub struct ProcessListBuilder;
impl StateViewBuilder for ProcessListBuilder {
    fn build(
        &self,
        query: &TextInputWeak,
        actions: &ActionsModel,
        update_receiver: Receiver<bool>,
        cx: &mut WindowContext,
    ) -> AnyView {
        query.set_placeholder("Search for running processes...", cx);
        actions.clone().set_dropdown(
            "memory",
            vec![("memory", "Sort by Memory"), ("cpu", "Sort by CPU")],
            cx,
        );

        List::new(
            query,
            &actions,
            |this, _, cx| {
                let lavender = cx.global::<Theme>().lavender.clone();
                let cache_dir = paths().cache.clone();
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

                let sort_by_cpu = "cpu".to_string().eq(&this.actions.get_dropdown_value(cx));
                if sort_by_cpu {
                    parsed.sort_unstable_by_key(|p| Reverse(p.cpu as u64));
                } else {
                    parsed.sort_unstable_by_key(|p| Reverse(p.mem));
                }

                let re = Regex::new(r"(.+\.(?:prefPane|app))(?:/.*)?$").unwrap();
                Ok(Some(
                    parsed
                        .iter()
                        .map(|p| {
                            let path = re
                                .captures(p.name.as_str())
                                .and_then(|caps| caps.get(1))
                                .map(|m| String::from(m.as_str()))
                                .unwrap_or_default();

                            #[cfg(target_os = "macos")]
                            let (name, image) = match unsafe {
                                get_application_data(
                                    &cache_dir.to_str().unwrap().into(),
                                    &path.as_str().into(),
                                )
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
                            #[cfg(target_os = "linux")]
                            let (name, image) = (
                                p.name.split("/").last().unwrap().to_string(),
                                Img::list_icon(Icon::Cpu, None),
                            );

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
                                                format!("{: >8}", format_bytes(p.mem * 1024)),
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
                                    {
                                        let pid = p.pid.to_string();
                                        move |this, cx| {
                                            if Command::new("kill")
                                                .arg("-9")
                                                .arg(pid.clone())
                                                .output()
                                                .is_err()
                                            {
                                                this.toast.error("Failed to kill process", cx);
                                            } else {
                                                this.toast.success("Killed process", cx);
                                            }
                                            this.update();
                                        }
                                    },
                                    false,
                                )],
                                None,
                            )
                        })
                        .collect(),
                ))
            },
            None,
            Some(Duration::from_secs(5)),
            update_receiver,
            true,
            cx,
        )
        .into()
    }
}

pub struct ProcessCommandBuilder;

impl RootCommandBuilder for ProcessCommandBuilder {
    fn build(&self, _cx: &mut WindowContext) -> RootCommand {
        RootCommand::new(
            "task_manager",
            "Search Processes",
            "Task Manager",
            Icon::Cpu,
            vec!["Kill", "Memory", "CPU"],
            None,
            Box::new(|_, cx| {
                StateModel::update(|this, cx| this.push(ProcessListBuilder, cx), cx);
            }),
        )
    }
}
