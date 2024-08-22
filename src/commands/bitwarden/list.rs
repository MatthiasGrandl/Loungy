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

use std::{collections::HashMap, path::PathBuf, sync::OnceLock, time::Duration};

use async_std::{
    channel,
    process::{Command, Output},
};
use bonsaidb::{
    core::schema::{Collection, SerializedCollection},
    local::Database,
};
use gpui::*;
use log::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use url::Url;

use crate::{
    command,
    commands::{RootCommand, RootCommandBuilder},
    components::{
        list::{Accessory, AsyncListItems, Item, ItemBuilder, ListBuilder, ListItem},
        shared::{Icon, Img, ImgMask},
    },
    db::Db,
    paths::paths,
    platform::{autofill, close_and_paste},
    state::{Action, CommandTrait, Shortcut, StateModel, StateViewBuilder, StateViewContext},
    window::Window,
};

use super::accounts::{
    BitwardenAccountFormBuilder, BitwardenAccountListBuilder, BitwardenPasswordPromptBuilder,
};

#[derive(Clone)]
pub struct BitwardenListBuilder {
    view: View<AsyncListItems>,
}
command!(BitwardenListBuilder);
impl StateViewBuilder for BitwardenListBuilder {
    fn build(&self, context: &mut StateViewContext, cx: &mut WindowContext) -> AnyView {
        context.query.set_placeholder("Search your vault...", cx);
        if let Ok(accounts) = BitwardenAccount::all(db()).query() {
            if accounts.len() > 1 {
                let mut options = vec![("".to_string(), "Show All".to_string())];
                for account in accounts {
                    let id = account.contents.id.clone();
                    options.push((id.clone(), id));
                }
                context.actions.set_dropdown("", options, cx);
            }
        }

        context.actions.update_global(
            vec![Action::new(
                Img::default().icon(Icon::UserSearch),
                "List Accounts",
                Some(Shortcut::new(",").cmd()),
                |_, cx| {
                    StateModel::update(
                        |this, cx| {
                            this.push(BitwardenAccountListBuilder, cx);
                        },
                        cx,
                    );
                },
                false,
            )],
            cx,
        );
        AsyncListItems::loader(&self.view, &context.actions, cx);
        let view = self.view.clone();
        ListBuilder::new()
            .build(
                move |list, _, cx| {
                    let account = list.actions.get_dropdown_value(cx);
                    let items = view.read(cx).items.clone();
                    if account.is_empty() {
                        Ok(Some(items.values().flatten().cloned().collect()))
                    } else {
                        Ok(Some(items.get(&account).cloned().unwrap_or_default()))
                    }
                },
                context,
                cx,
            )
            .into()
    }
}

impl BitwardenLoginItem {
    pub fn fields(&self) -> Vec<&str> {
        let mut fields = vec!["username", "password"];
        if self.totp.is_some() {
            fields.push("totp");
        }
        fields
    }
    pub fn get_label(&self, field: &str) -> (&str, Img, Shortcut) {
        match field {
            "username" => (
                "Username",
                Img::default().icon(Icon::User),
                Shortcut::new("u").cmd(),
            ),
            "password" => (
                "Password",
                Img::default().icon(Icon::Lock),
                Shortcut::new("p").cmd(),
            ),
            "totp" => (
                "TOTP 2FA",
                Img::default().icon(Icon::Clock),
                Shortcut::new("t").cmd(),
            ),
            _ => panic!("Unknown field {}", field),
        }
    }
    pub fn get_action(&self, field: &str) -> Action {
        let (label, img, shortcut) = self.get_label(field);
        let field = field.to_string();
        Action::new(
            img,
            label,
            Some(shortcut),
            move |this, cx| {
                let Some(meta) = this.get_meta::<(Vec<String>, HashMap<String, String>)>(cx) else {
                    return;
                };
                let value = meta.1.get(&field).cloned().unwrap_or("".to_string());
                close_and_paste(value.as_str(), true, cx);
            },
            false,
        )
    }
    pub fn get_actions(&self) -> Vec<Action> {
        self.fields()
            .iter()
            .map(|field| self.get_action(field))
            .collect()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub(super) struct BitwardenUri {
    uri: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub(super) struct BitwardenLoginItem {
    username: String,
    password: String,
    totp: Option<String>,
    uris: Vec<BitwardenUri>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(untagged)]
pub(super) enum BitwardenItem {
    Login {
        id: String,
        name: String,
        notes: Option<String>,
        login: BitwardenLoginItem,
    },
    Other(Value),
}

#[derive(Serialize, Deserialize, Clone, Collection)]
#[collection(name = "bitwarden-accounts")]
pub(super) struct BitwardenAccount {
    #[natural_id]
    pub id: String,
    pub client_id: String,
    pub client_secret: String,
    pub instance: String,
    pub password: Option<String>,
    pub session: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub(super) enum BitwardenVaultStatus {
    Unauthenticated,
    Locked,
    Unlocked,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub(super) struct BitwardenStatus {
    server_url: Option<String>,
    status: BitwardenVaultStatus,
}

impl BitwardenAccount {
    pub fn path(&self) -> PathBuf {
        paths().data.join("bitwarden").join(self.id.clone())
    }
    pub async fn command(&self, args: Vec<&str>) -> anyhow::Result<Output> {
        let mut env: HashMap<String, String> = HashMap::new();
        env.insert("PATH".to_string(), paths().path_env.clone());
        env.insert(
            "BITWARDENCLI_APPDATA_DIR".to_string(),
            self.path().to_string_lossy().to_string(),
        );
        env.insert("BW_CLIENTID".to_string(), self.client_id.clone());
        env.insert("BW_CLIENTSECRET".to_string(), self.client_secret.clone());
        if let Some(session) = &self.session {
            env.insert("BW_SESSION".to_string(), session.clone());
        }

        Ok(Command::new("bw").args(args).envs(env).output().await?)
    }
    pub async fn auth_command(
        &mut self,
        args: Vec<&str>,
        cx: &mut AsyncWindowContext,
    ) -> anyhow::Result<Output> {
        self.unlock(cx).await?;
        self.command(args).await
    }
    pub async fn unlock(&mut self, cx: &mut AsyncWindowContext) -> anyhow::Result<()> {
        // TODO: if there is no password, we need to prompt for it
        let status = self
            .command(vec!["status", "--raw", "--nointeraction"])
            .await?;

        if !status.status.success() {
            return Err(anyhow::anyhow!(
                "Bitwarden Status Query Failed: {}",
                String::from_utf8_lossy(&status.stderr)
            ));
        }

        let status: BitwardenStatus = serde_json::from_slice(&status.stdout)?;

        match status.status {
            BitwardenVaultStatus::Unlocked => {
                return Ok(());
            }
            BitwardenVaultStatus::Unauthenticated => {
                self.command(vec!["config", "server", &self.instance])
                    .await?;
                let login = self
                    .command(vec!["login", "--apikey", "--nointeraction"])
                    .await?;
                if !login.status.success() {
                    return Err(anyhow::anyhow!(
                        "Bitwarden Failed To Login: {}",
                        String::from_utf8_lossy(&login.stderr)
                    ));
                }
            }
            _ => {}
        }

        let password = if let Some(password) = &self.password {
            password.clone()
        } else {
            let (s, r) = channel::unbounded::<(String, bool)>();
            StateModel::update_async(
                |this, cx| {
                    this.push(
                        BitwardenPasswordPromptBuilder {
                            password: s,
                            account: self.clone(),
                        },
                        cx,
                    )
                },
                cx,
            );
            let (password, remember) = r.recv().await?;
            if remember {
                self.password = Some(password.clone());
            }
            password
        };
        let output = self
            .command(vec!["unlock", &password, "--raw", "--nointeraction"])
            .await?;

        if output.stdout.is_empty() {
            self.session = None;
            return Err(anyhow::anyhow!("Failed to unlock account"));
        };
        let session = String::from_utf8(output.stdout)?;
        self.session = Some(session);
        let result = self.clone().overwrite_into(&self.id, db());
        if let Err(result) = result {
            error!("Failed to save account: {:?}", result.error);
        }
        Ok(())
    }
}

pub struct BitwardenCommandBuilder;

pub(super) fn db() -> &'static Database {
    static DB: OnceLock<Database> = OnceLock::new();
    DB.get_or_init(Db::init_collection::<BitwardenAccount>)
}

struct EntryModel {
    inner: Model<(Vec<String>, HashMap<String, String>)>,
}

impl EntryModel {
    pub fn new(
        account: &BitwardenAccount,
        item: &BitwardenItem,
        cx: &mut AsyncWindowContext,
    ) -> Self {
        let window = cx.window_handle();
        Self {
            inner: cx
                .new_model(|cx| match item {
                    BitwardenItem::Login { id, login, .. } => {
                        let mut map = HashMap::new();
                        map.insert("username".to_string(), login.username.clone());
                        map.insert("password".to_string(), login.password.clone());

                        if login.totp.is_some() {
                            let account = account.clone();
                            let id = id.clone();
                            cx.spawn(|model, mut cx| async move {
                                while let Some(model) = model.upgrade() {
                                    let Ok(output) = cx
                                        .update_window(window, |_, cx| {
                                            let mut cx = cx.to_async();
                                            let account = account.clone();
                                            let id = id.clone();
                                            async move {
                                                account
                                                    .clone()
                                                    .auth_command(vec!["get", "totp", &id], &mut cx)
                                                    .await
                                            }
                                        })
                                        .unwrap()
                                        .await
                                    else {
                                        continue;
                                    };
                                    let totp = String::from_utf8_lossy(&output.stdout);
                                    let _ = model.update(
                                        &mut cx,
                                        |map: &mut (Vec<String>, HashMap<String, String>), cx| {
                                            map.0 = vec![
                                                "username".to_string(),
                                                "password".to_string(),
                                                "totp".to_string(),
                                            ];
                                            map.1.insert("totp".to_string(), totp.to_string());
                                            cx.notify();
                                        },
                                    );
                                    cx.background_executor()
                                        .timer(Duration::from_secs(10))
                                        .await
                                }
                            })
                            .detach();
                        }
                        (vec!["username".to_string(), "password".to_string()], map)
                    }
                    BitwardenItem::Other { .. } => (vec![], HashMap::new()),
                })
                .unwrap(),
        }
    }
}
command!(BitwardenCommandBuilder);
impl RootCommandBuilder for BitwardenCommandBuilder {
    fn build(&self, cx: &mut WindowContext) -> RootCommand {
        let view = cx.new_view(|cx| {
            let accounts = BitwardenAccount::all(db()).query().unwrap_or_default();
            for account in accounts {
                let mut account = account.contents;
                cx.spawn(move |view, mut cx| async move {
                    let mut first = true;
                    loop {
                        if !first {
                            if let Err(sync) =
                                account.auth_command(vec!["sync", "-f"], &mut cx).await
                            {
                                error!("Failed to sync: {}", sync);
                            }
                        }
                        first = false;

                        let mut items: Vec<Item> = vec![];
                        let response = account
                            .auth_command(vec!["list", "items", "--nointeraction"], &mut cx)
                            .await;
                        let Ok(output) = response else {
                            error!("Failed to list items: {}", response.err().unwrap());
                            return;
                        };

                        let parsed: Vec<BitwardenItem> = serde_json::from_slice(&output.stdout)
                            .map_err(|e| {
                                error!("Failed to parse items: {}", e);
                                e
                            })
                            .unwrap_or_default();

                        for item in parsed {
                            let item_clone = item.clone();

                            if let BitwardenItem::Login {
                                id,
                                name,
                                notes: _,
                                login,
                            } = item
                            {
                                let mut img = login
                                    .uris
                                    .first()
                                    .and_then(|uri| {
                                        Url::parse(&if !uri.uri.starts_with("http") {
                                            format!("https://{}", uri.uri)
                                        } else {
                                            uri.uri.clone()
                                        })
                                        .ok()
                                        .and_then(|url| {
                                            cx.update_window(cx.window_handle(), |_, cx| {
                                                Img::default().mask(ImgMask::Rounded).favicon(
                                                    url,
                                                    Icon::Globe,
                                                    cx,
                                                )
                                            })
                                            .ok()
                                        })
                                    })
                                    .unwrap_or(Img::default().icon(Icon::Globe));

                                img.mask = ImgMask::Rounded;

                                let mut keywords = vec![name.clone()];
                                keywords.append(
                                    &mut login.uris.iter().map(|uri| uri.uri.clone()).collect(),
                                );
                                let meta = EntryModel::new(&account, &item_clone, &mut cx);
                                let mut actions = vec![Action::new(
                                    Img::default().icon(Icon::PaintBucket),
                                    "Autofill",
                                    None,
                                    {
                                        |this, cx| {
                                            Window::close(cx);
                                            let Some(meta) = this.get_meta_model::<(
                                                Vec<String>,
                                                HashMap<String, String>,
                                            )>(
                                            ) else {
                                                return;
                                            };
                                            cx.spawn(move |mut cx| async move {
                                                Window::wait_for_close(&mut cx).await;
                                                let mut prev = "".to_string();
                                                let max_tries = 900;
                                                let mut tries = 0;
                                                let Ok(keys) = cx
                                                    .read_model(&meta, |(keys, _), _| keys.clone())
                                                else {
                                                    return;
                                                };
                                                for field in keys {
                                                    loop {
                                                        let value = cx
                                                            .read_model(&meta, |(_, map), _| {
                                                                map.clone()
                                                            })
                                                            .map(|map| {
                                                                map.get(&field)
                                                                    .map(|s| s.to_string())
                                                            })
                                                            .ok()
                                                            .flatten()
                                                            .unwrap_or("".to_string());
                                                        match autofill(
                                                            value.as_str(),
                                                            field.eq("password"),
                                                            &prev,
                                                        ) {
                                                            Some(p) => {
                                                                prev = p;
                                                                break;
                                                            }
                                                            None => {
                                                                tries += 1;
                                                                if tries > max_tries {
                                                                    error!("Autofill timed out");
                                                                    return;
                                                                }
                                                                cx.background_executor()
                                                                    .timer(Duration::from_millis(
                                                                        100,
                                                                    ))
                                                                    .await;
                                                                //cx.background_executor().timer(Duration::from_millis(100)).await;
                                                            }
                                                        }
                                                    }
                                                }
                                            })
                                            .detach();
                                        }
                                    },
                                    false,
                                )];

                                let url = login.uris.first().and_then(|uri| {
                                    Url::parse(&if !uri.uri.starts_with("http") {
                                        format!("https://{}", uri.uri)
                                    } else {
                                        uri.uri.clone()
                                    })
                                    .ok()
                                });

                                if let Some(url) = url {
                                    actions.push(Action::new(
                                        Img::default().icon(Icon::Globe),
                                        "Open",
                                        Some(Shortcut::new("o").cmd()),
                                        {
                                            move |_, cx| {
                                                Window::close(cx);
                                                cx.open_url(url.as_str());
                                            }
                                        },
                                        false,
                                    ));
                                }

                                // let preview = cx.update_window::<StateItem, _>(cx.window_handle(), |_, cx| {
                                //     StateItem::init(BitwardenAccountListBuilder, false, cx)
                                // }).ok();
                                actions.append(&mut login.get_actions());
                                items.push(
                                    ItemBuilder::new(
                                        id.clone(),
                                        ListItem::new(
                                            Some(img),
                                            name.clone(),
                                            None,
                                            vec![Accessory::new(login.username.clone(), None)],
                                        ),
                                    )
                                    .keywords(keywords)
                                    .actions(actions)
                                    .meta(meta.inner.into_any())
                                    .build(),
                                );
                            }
                        }
                        let id = account.id.clone();
                        if let Some(view) = view.upgrade() {
                            let _ = view.update(&mut cx, move |list: &mut AsyncListItems, cx| {
                                list.update(id.clone(), items, cx);
                            });
                            cx.background_executor()
                                .timer(Duration::from_secs(500))
                                .await;
                        } else {
                            break;
                        }
                    }
                })
                .detach();
            }

            AsyncListItems::new()
        });
        RootCommand::new(
            "bitwarden",
            "Search Vault",
            "Bitwarden",
            Icon::Lock,
            vec!["Passwords"],
            None,
            move |_, cx| {
                let view = view.clone();
                let accounts = BitwardenAccount::all(db());
                if accounts.count().unwrap_or_default() == 0 {
                    StateModel::update(
                        |this, cx| this.push(BitwardenAccountFormBuilder {}, cx),
                        cx,
                    );
                } else {
                    StateModel::update(|this, cx| this.push(BitwardenListBuilder { view }, cx), cx);
                };
            },
        )
    }
}
