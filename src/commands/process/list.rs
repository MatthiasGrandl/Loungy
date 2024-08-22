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

use gpui::*;
use std::{
    cmp::Reverse, collections::HashMap, fs, path::PathBuf, process::Command, time::Duration,
};

use regex::Regex;

use crate::{
    command,
    commands::{RootCommand, RootCommandBuilder},
    components::{
        list::{Accessory, ItemBuilder, ListBuilder, ListItem},
        shared::{Icon, Img, ImgMask, ImgSize},
    },
    paths::paths,
    platform::{get_application_data, AppData},
    state::{Action, CommandTrait, StateModel, StateViewBuilder, StateViewContext},
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
command!(ProcessListBuilder);

impl StateViewBuilder for ProcessListBuilder {
    fn build(&self, context: &mut StateViewContext, cx: &mut WindowContext) -> AnyView {
        context
            .query
            .set_placeholder("Search for running processes...", cx);
        context.actions.set_dropdown(
            "memory",
            vec![("memory", "Sort by Memory"), ("cpu", "Sort by CPU")],
            cx,
        );

        ListBuilder::new()
            .interval(Duration::from_secs(5))
            .build(
                |this, _, cx| {
                    let theme = cx.global::<Theme>().clone();
                    let cache_dir = paths().cache.join("apps");
                    if !cache_dir.exists() {
                        fs::create_dir_all(cache_dir.clone()).unwrap();
                    }

                    let ps = Command::new("ps")
                        .arg("-eo")
                        .arg("pid,ppid,pcpu,rss,comm")
                        .output()
                        .expect("failed to get process list")
                        .stdout;

                    let parsed: Vec<Process> = String::from_utf8(ps)
                        .unwrap()
                        .split('\n')
                        .skip(1)
                        .filter_map(|line| Process::parse(line).ok())
                        .collect();

                    let mut aggregated = HashMap::<u64, Process>::new();
                    parsed.iter().for_each(|p| {
                        if p.ppid == 1 {
                            aggregated.insert(p.pid, p.clone());
                        } else if let Some(parent) = aggregated.get(&p.ppid) {
                            let mut parent = parent.clone();
                            parent.cpu += p.cpu;
                            parent.mem += p.mem;
                            aggregated.insert(p.ppid, parent);
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

                                let data =
                                    get_application_data(&PathBuf::from(path)).unwrap_or(AppData {
                                        id: "".to_string(),
                                        name: p.name.split('/').last().unwrap().to_string(),
                                        icon: Img::default().icon(Icon::Cpu),
                                        icon_path: PathBuf::new(),
                                        keywords: vec![],
                                        tag: "".to_string(),
                                    });
                                ItemBuilder::new(p.pid, {
                                    let (m, c) = if sort_by_cpu {
                                        (theme.subtext0, theme.lavender)
                                    } else {
                                        (theme.lavender, theme.subtext0)
                                    };
                                    ListItem::new(
                                        Some(data.icon),
                                        data.name.clone(),
                                        None,
                                        vec![
                                            Accessory::new(
                                                format!("{: >8}", format_bytes(p.mem * 1024)),
                                                Some(
                                                    Img::default()
                                                        .icon(Icon::MemoryStick)
                                                        .icon_color(m)
                                                        .mask(ImgMask::None)
                                                        .size(ImgSize::SM),
                                                ),
                                            ),
                                            Accessory::new(
                                                format!("{: >6.2}%", p.cpu),
                                                Some(
                                                    Img::default()
                                                        .icon(Icon::Cpu)
                                                        .icon_color(c)
                                                        .mask(ImgMask::None)
                                                        .size(ImgSize::SM),
                                                ),
                                            ),
                                        ],
                                    )
                                })
                                .keywords(vec![data.name.clone()])
                                .actions(vec![Action::new(
                                    Img::default().icon(Icon::Skull),
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
                                )])
                                .build()
                            })
                            .collect(),
                    ))
                },
                context,
                cx,
            )
            .into()
    }
}

pub struct ProcessCommandBuilder;
command!(ProcessCommandBuilder);

impl RootCommandBuilder for ProcessCommandBuilder {
    fn build(&self, _cx: &mut WindowContext) -> RootCommand {
        RootCommand::new(
            "task_manager",
            "Search Processes",
            "Task Manager",
            Icon::Cpu,
            vec!["Kill", "Memory", "CPU"],
            None,
            |_, cx| {
                StateModel::update(|this, cx| this.push(ProcessListBuilder, cx), cx);
            },
        )
    }
}
