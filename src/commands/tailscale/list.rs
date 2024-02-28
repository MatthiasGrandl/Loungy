use std::{collections::HashMap, process::Command, sync::mpsc::Receiver, time::Duration};

use gpui::*;
use serde::Deserialize;
use time::OffsetDateTime;

use crate::{
    commands::{RootCommand, RootCommandBuilder},
    components::{
        list::{Accessory, Item, List, ListBuilder, ListItem},
        shared::{Icon, Img},
    },
    query::{TextInput, TextInputWeak},
    state::{Action, ActionsModel, Shortcut, StateModel, StateViewBuilder},
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
    #[serde(with = "time::serde::iso8601")]
    last_seen: OffsetDateTime,
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
impl StateViewBuilder for TailscaleListBuilder {
    fn build(
        &self,
        query: &TextInputWeak,
        actions: &ActionsModel,
        update_receiver: Receiver<bool>,
        cx: &mut WindowContext,
    ) -> AnyView {
        query.set_placeholder("Search for peers...", cx);
        actions.clone().set_dropdown(
            "online",
            vec![("online", "Hide Offline"), ("offline", "Show Offline")],
            cx,
        );
        ListBuilder::new()
            .build(
                query,
                &actions,
                |this, _, cx| {
                    let offline = "offline"
                        .to_string()
                        .eq(&this.actions.get_dropdown_value(cx));
                    let theme = cx.global::<Theme>().clone();
                    let status = Command::new("tailscale")
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
                                    format!("Last seen: {}", p.last_seen.date().to_string()),
                                    theme.surface0,
                                ),
                            };
                            let ip = p.tailscale_ips.first().unwrap();
                            let ipv6 = p.tailscale_ips.last().unwrap();
                            let url = format!("https://{}", &ip);
                            Some(Item::new(
                                p.id.clone(),
                                vec![name],
                                cx.new_view(|_| {
                                    ListItem::new(
                                        Some(Img::list_dot(color)),
                                        name,
                                        Some(p.os.to_string()),
                                        vec![Accessory::Tag { tag, img: None }],
                                    )
                                })
                                .into(),
                                None,
                                vec![
                                    Action::new(
                                        Img::list_icon(Icon::ArrowUpRightFromSquare, None),
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
                                        Img::list_icon(Icon::Clipboard, None),
                                        "Copy IPv4",
                                        Some(Shortcut::cmd("c")),
                                        {
                                            let ip = ip.clone();
                                            move |this, cx| {
                                                cx.write_to_clipboard(ClipboardItem::new(
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
                                        Img::list_icon(Icon::Clipboard, None),
                                        "Copy IPv6",
                                        Some(Shortcut::new(Keystroke {
                                            modifiers: Modifiers {
                                                command: true,
                                                shift: true,
                                                ..Modifiers::default()
                                            },
                                            key: "c".to_string(),
                                            ime_key: None,
                                        })),
                                        {
                                            let ip = ipv6.clone();
                                            move |this, cx| {
                                                cx.write_to_clipboard(ClipboardItem::new(
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
                                ],
                                None,
                                None,
                                None,
                            ))
                        })
                        .collect();
                    items.sort_unstable_by_key(|i| i.keywords.first().unwrap().clone());
                    Ok(Some(items))
                },
                None,
                Some(Duration::from_secs(10)),
                update_receiver,
                cx,
            )
            .into()
    }
}

pub struct TailscaleCommandBuilder;

impl RootCommandBuilder for TailscaleCommandBuilder {
    fn build(&self, _cx: &mut WindowContext) -> RootCommand {
        RootCommand::new(
            "tailscale",
            "Search Peers",
            "Tailscale",
            Icon::Waypoints,
            vec!["VPN"],
            None,
            Box::new(|_, cx| {
                StateModel::update(|this, cx| this.push(TailscaleListBuilder, cx), cx);
            }),
        )
    }
}
