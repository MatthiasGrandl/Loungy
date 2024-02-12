use std::{
    collections::HashMap,
    process::Command,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::Receiver,
    },
    time::Duration,
};

use gpui::*;
use serde::Deserialize;
use time::OffsetDateTime;

use crate::{
    commands::{RootCommand, RootCommandBuilder},
    icon::Icon,
    list::{Accessory, Img, Item, List, ListItem},
    query::TextInput,
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

static OFFLINE: AtomicBool = AtomicBool::new(false);

#[derive(Clone)]
pub struct TailscaleListBuilder;
impl StateViewBuilder for TailscaleListBuilder {
    fn build(
        &self,
        query: &TextInput,
        actions: &ActionsModel,
        update_receiver: Receiver<bool>,
        cx: &mut WindowContext,
    ) -> AnyView {
        query.set_placeholder("Search for peers...", cx);
        List::new(
            query,
            &actions,
            |this, cx| {
                let offline = OFFLINE.load(Ordering::Relaxed);
                {
                    let filter_action = if offline {
                        Action::new(
                            Img::list_icon(Icon::EyeOff, None),
                            "Hide Offline",
                            Some(Shortcut::simple("tab")),
                            move |this, _| {
                                OFFLINE.store(false, Ordering::Relaxed);
                                this.update();
                            },
                            false,
                        )
                    } else {
                        Action::new(
                            Img::list_icon(Icon::Eye, None),
                            "Show Offline",
                            Some(Shortcut::simple("tab")),
                            move |this, _| {
                                OFFLINE.store(true, Ordering::Relaxed);
                                this.update();
                            },
                            false,
                        )
                    };
                    this.actions.update_global(vec![filter_action], cx);
                }
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
                                            cx.write_to_clipboard(ClipboardItem::new(ip.clone()));
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
                                            cx.write_to_clipboard(ClipboardItem::new(ip.clone()));
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
                        ))
                    })
                    .collect();
                items.sort_unstable_by_key(|i| i.keywords.first().unwrap().clone());
                Ok(items)
            },
            None,
            Some(Duration::from_secs(10)),
            update_receiver,
            true,
            cx,
        )
        .into()
    }
}

pub struct TailscaleCommandBuilder;

impl RootCommandBuilder for TailscaleCommandBuilder {
    fn build(&self, _cx: &mut WindowContext) -> RootCommand {
        RootCommand::new(
            "Search Peers",
            "Tailscale",
            Icon::Palette,
            vec!["VPN"],
            None,
            Box::new(|_, cx| {
                cx.update_global::<StateModel, _>(|model, cx| {
                    model.push(TailscaleListBuilder {}, cx)
                });
            }),
        )
    }
}
