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

use std::{collections::HashMap, process::Command, time::Duration};

use gpui::*;
use jiff::Timestamp;
use serde::Deserialize;

use crate::{
    command,
    commands::{RootCommand, RootCommandBuilder},
    components::{
        list::{Accessory, Item, ItemBuilder, ListBuilder, ListItem},
        shared::{Icon, Img},
    },
    date::format_date,
    paths::paths,
    state::{Action, CommandTrait, Shortcut, StateModel, StateViewBuilder, StateViewContext},
    theme::Theme,
};

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Peer {
    #[serde(rename = "ID")]
    id: String,
    host_name: String,
    #[serde(rename = "DNSName")]
    dns_name: String,
    #[serde(rename = "OS")]
    os: String,
    #[serde(rename = "TailscaleIPs")]
    tailscale_ips: Vec<String>,
    tags: Vec<String>,
    rx_bytes: u64,
    tx_bytes: u64,
    created: String,
    last_seen: Timestamp,
    online: bool,
    active: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Status {
    peer: HashMap<String, Peer>,
}

#[derive(Clone)]
pub struct TailscaleListBuilder;

command!(TailscaleListBuilder);
impl StateViewBuilder for TailscaleListBuilder {
    fn build(&self, context: &mut StateViewContext, cx: &mut WindowContext) -> AnyView {
        context.query.set_placeholder("Search for peers...", cx);
        context.actions.set_dropdown(
            "online",
            vec![("online", "Hide Offline"), ("offline", "Show Offline")],
            cx,
        );
        ListBuilder::new()
            .interval(Duration::from_secs(10))
            .build(
                |this, _, cx| {
                    let offline = "offline"
                        .to_string()
                        .eq(&this.actions.get_dropdown_value(cx));
                    let theme = cx.global::<Theme>().clone();
                    let mut env: HashMap<String, String> = HashMap::new();
                    env.insert("PATH".to_string(), paths().path_env.clone());
                    let status = Command::new("tailscale")
                        .envs(env)
                        .arg("status")
                        .arg("--json")
                        .output()?
                        .stdout;
                    let json = serde_json::from_slice::<Status>(&status)?;

                    let mut items: Vec<Item> = json
                        .peer
                        .values()
                        .filter_map(|p| {
                            if !offline && !p.online {
                                return None;
                            }
                            let name = p.dns_name.split('.').next().unwrap();
                            let (tag, color) = match p.online {
                                true => ("Connected".to_string(), theme.green),
                                false => (
                                    format!("Last seen: {}", format_date(p.last_seen, cx)),
                                    theme.surface0,
                                ),
                            };
                            let ip = p.tailscale_ips.first().unwrap();
                            let ipv6 = p.tailscale_ips.last().unwrap();
                            let url = format!("https://{}", &ip);
                            Some(
                                ItemBuilder::new(
                                    p.id.clone(),
                                    ListItem::new(
                                        Some(Img::default().dot(color)),
                                        name,
                                        Some(p.os.to_string()),
                                        vec![Accessory::Tag { tag, img: None }],
                                    ),
                                )
                                .actions(vec![
                                    Action::new(
                                        Img::default().icon(Icon::ArrowUpRightFromSquare),
                                        "Open",
                                        None,
                                        move |this, cx| {
                                            cx.open_url(&url.clone());
                                            this.toast.floating(
                                                "Opened peer in browser",
                                                Some(Icon::ArrowUpRightFromSquare),
                                                cx,
                                            )
                                        },
                                        false,
                                    ),
                                    Action::new(
                                        Img::default().icon(Icon::Clipboard),
                                        "Copy IPv4",
                                        Some(Shortcut::new("c").cmd()),
                                        {
                                            let ip = ip.clone();
                                            move |this, cx| {
                                                cx.write_to_clipboard(ClipboardItem::new_string(
                                                    ip.clone(),
                                                ));
                                                this.toast.floating(
                                                    "Copied IPv4 to Clipboard",
                                                    Some(Icon::Clipboard),
                                                    cx,
                                                )
                                            }
                                        },
                                        false,
                                    ),
                                    Action::new(
                                        Img::default().icon(Icon::Clipboard),
                                        "Copy IPv6",
                                        Some(Shortcut::new("c").cmd().shift()),
                                        {
                                            let ip = ipv6.clone();
                                            move |this, cx| {
                                                cx.write_to_clipboard(ClipboardItem::new_string(
                                                    ip.clone(),
                                                ));
                                                this.toast.floating(
                                                    "Copied IPv6 to Clipboard",
                                                    Some(Icon::Clipboard),
                                                    cx,
                                                )
                                            }
                                        },
                                        false,
                                    ),
                                ])
                                .keywords(vec![name])
                                .build(),
                            )
                        })
                        .collect();
                    items.sort_unstable_by_key(|i| i.get_keywords().first().unwrap().clone());
                    Ok(Some(items))
                },
                context,
                cx,
            )
            .into()
    }
}

pub struct TailscaleCommandBuilder;
command!(TailscaleCommandBuilder);

impl RootCommandBuilder for TailscaleCommandBuilder {
    fn build(&self, _cx: &mut WindowContext) -> RootCommand {
        RootCommand::new(
            "tailscale",
            "Search Peers",
            "Tailscale",
            Icon::Waypoints,
            vec!["VPN"],
            None,
            |_, cx| {
                StateModel::update(|this, cx| this.push(TailscaleListBuilder, cx), cx);
            },
        )
    }
}
