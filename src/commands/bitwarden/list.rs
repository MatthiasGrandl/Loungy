use std::{collections::HashMap, path::PathBuf, sync::OnceLock, time::Duration};

use async_std::{
    channel,
    process::{Command, Output},
    task::sleep,
};
use bonsaidb::{
    core::schema::{Collection, SerializedCollection},
    local::Database,
};
use gpui::*;
use log::*;
use serde::{Deserialize, Serialize};

use url::Url;

use crate::{
    commands::{RootCommand, RootCommandBuilder},
    components::{
        list::{Accessory, AsyncListItems, Item, ItemBuilder, ListBuilder, ListItem},
        shared::{Icon, Img},
    },
    db::Db,
    paths::paths,
    platform::{auto_fill, close_and_paste},
    state::{Action, Shortcut, StateModel, StateViewBuilder, StateViewContext},
    window::Window,
};

use super::accounts::{
    BitwardenAccountFormBuilder, BitwardenAccountListBuilder, BitwardenPasswordPromptBuilder,
};

#[derive(Clone)]
pub struct BitwardenListBuilder {
    view: View<AsyncListItems>,
}

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
                context.actions.clone().set_dropdown("", options, cx);
            }
        }

        context.actions.update_global(
            vec![Action::new(
                Img::list_icon(Icon::UserSearch, None),
                "List Accounts",
                Some(Shortcut::cmd(",")),
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

#[derive(Serialize, Deserialize, Clone)]
pub(super) struct BitwardenUri {
    uri: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub(super) struct BitwardenLoginItem {
    username: String,
    password: String,
    totp: Option<String>,
    uris: Vec<BitwardenUri>,
}

impl BitwardenLoginItem {
    pub fn fields(&self) -> Vec<&str> {
        let mut fields = vec!["username", "password"];
        if self.totp.is_some() {
            fields.push("totp");
        }
        fields
    }
    pub async fn get_field(
        &self,
        field: &str,
        id: &str,
        account: &mut BitwardenAccount,
        cx: &mut AsyncWindowContext,
    ) -> anyhow::Result<String> {
        match field {
            "username" => Ok(self.username.clone()),
            "password" => Ok(self.password.clone()),
            "totp" => {
                let output = account.auth_command(vec!["get", "totp", id], cx).await?;
                Ok(String::from_utf8(output.stdout)?)
            }
            _ => Err(anyhow::anyhow!("Invalid field")),
        }
    }
    pub fn get_label(&self, field: &str) -> (&str, Img, Shortcut) {
        match field {
            "username" => (
                "Username",
                Img::list_icon(Icon::User, None),
                Shortcut::cmd("u"),
            ),
            "password" => (
                "Password",
                Img::list_icon(Icon::Lock, None),
                Shortcut::cmd("p"),
            ),
            "totp" => (
                "TOTP 2FA",
                Img::list_icon(Icon::Clock, None),
                Shortcut::cmd("t"),
            ),
            _ => panic!("Unknown field {}", field),
        }
    }
    pub fn get_action(&self, field: &str, id: &str, account: &BitwardenAccount) -> Action {
        let (label, img, shortcut) = self.get_label(field);
        let id = id.to_string();
        let field = field.to_string();
        let account = account.clone();
        let login = self.clone();
        Action::new(
            img,
            label,
            Some(shortcut),
            move |actions, cx| {
                let id = id.clone();
                let field = field.clone();
                let mut account = account.clone();
                let login = login.clone();
                let mut actions = actions.clone();
                cx.spawn(move |mut cx| async move {
                    if let Ok(value) = login.get_field(&field, &id, &mut account, &mut cx).await {
                        let _ = cx.update_window(cx.window_handle(), |_, cx| {
                            close_and_paste(value.as_str(), true, cx);
                        });
                    } else {
                        actions
                            .toast
                            .error(format!("Failed to get {}", field), &mut cx);
                    }
                })
                .detach();
            },
            false,
        )
    }
    pub fn get_actions(&self, id: &str, account: &BitwardenAccount) -> Vec<Action> {
        self.fields()
            .iter()
            .map(|field| self.get_action(field, id, account))
            .collect()
    }
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub(super) enum BitwardenItem {
    Login {
        id: String,
        name: String,
        notes: Option<String>,
        login: BitwardenLoginItem,
    },
    Other {
        id: String,
    },
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

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(super) enum BitwardenVaultStatus {
    Unauthenticated,
    Locked,
    Unlocked,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(super) struct BitwardenStatus {
    server_url: String,
    status: BitwardenVaultStatus,
}

impl BitwardenAccount {
    pub fn path(&self) -> PathBuf {
        paths().data.join("bitwarden").join(self.id.clone())
    }
    pub async fn command(&self, args: Vec<&str>) -> anyhow::Result<Output> {
        let mut env: HashMap<String, String> = HashMap::new();
        env.insert(
            "PATH".to_string(),
            "/opt/homebrew/bin:/usr/local/bin/bw".to_string(),
        );
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
        debug!("Status: {}", String::from_utf8(status.stdout.clone())?);
        let status: BitwardenStatus = serde_json::from_slice(&status.stdout)?;

        match status.status {
            BitwardenVaultStatus::Unlocked => {
                return Ok(());
            }
            BitwardenVaultStatus::Unauthenticated => {
                if !self
                    .command(vec!["login", "--apikey", "--nointeraction"])
                    .await?
                    .status
                    .success()
                {
                    return Err(anyhow::anyhow!("Failed to login"));
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
        let result = self.clone().push_into(db());
        if let Err(result) = result {
            error!("Failed to save account: {:?}", result.error);
        }
        Ok(())
    }
}

pub struct Markdown {
    pub text: String,
}

impl Render for Markdown {
    fn render(&mut self, _: &mut ViewContext<Self>) -> impl IntoElement {
        div().child(self.text.clone())
    }
}

pub struct BitwardenCommandBuilder;

pub(super) fn db() -> &'static Database {
    static DB: OnceLock<Database> = OnceLock::new();
    DB.get_or_init(Db::init_collection::<BitwardenAccount>)
}

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
                            let _ = account.auth_command(vec!["sync"], &mut cx).await;
                        }
                        first = false;

                        let mut items: Vec<Item> = vec![];
                        if let Ok(output) = account
                            .auth_command(vec!["list", "items", "--nointeraction"], &mut cx)
                            .await
                        {
                            let parsed: Vec<BitwardenItem> = serde_json::from_slice(&output.stdout)
                                .map_err(|e| {
                                    error!("Failed to parse items: {}", e);
                                    e
                                })
                                .unwrap_or_default();

                            for item in parsed {
                                if let BitwardenItem::Login {
                                    id,
                                    name,
                                    notes: _,
                                    login,
                                } = item
                                {
                                    let domain = login.uris.first().and_then(|uri| {
                                        Url::parse(&if !uri.uri.starts_with("http") {
                                            format!("https://{}", uri.uri)
                                        } else {
                                            uri.uri.clone()
                                        })
                                        .ok()
                                        .and_then(|url| url.domain().map(|d| d.to_owned()))
                                    });

                                    // TODO: ideally replace this with a custom favicon scraper
                                    let img = match domain {
                                        Some(domain) => Img::list_url(format!(
                                            "https://icons.bitwarden.net/{}/icon.png",
                                            domain
                                        )),
                                        None => Img::list_icon(Icon::Globe, None),
                                    };

                                    let mut keywords = vec![name.clone()];
                                    keywords.append(
                                        &mut login.uris.iter().map(|uri| uri.uri.clone()).collect(),
                                    );
                                    let mut actions = vec![Action::new(
                                        Img::list_icon(Icon::PaintBucket, None),
                                        "Autofill",
                                        None,
                                        {
                                            //
                                            let id = id.clone();
                                            let login = login.clone();
                                            let account = account.clone();
                                            move |_, cx| {
                                                Window::close(cx);
                                                let id = id.clone();
                                                let login = login.clone();
                                                let mut account = account.clone();
                                                cx.spawn(move |mut cx| async move {
                                                    Window::wait_for_close(&mut cx).await;
                                                    let mut prev = "".to_string();
                                                    // Timeout after 2 minutes, could probably be lower, but TOTP takes a while sometimes
                                                    let max_tries = 1200;
                                                    let mut tries = 0;
                                                    for field in login.fields() {
                                                        loop {
                                                            let value = login
                                                                .get_field(
                                                                    field,
                                                                    &id,
                                                                    &mut account,
                                                                    &mut cx,
                                                                )
                                                                .await
                                                                .unwrap();
                                                            match auto_fill(
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
                                                                        error!(
                                                                            "Autofill timed out"
                                                                        );
                                                                        return;
                                                                    }
                                                                    sleep(Duration::from_millis(
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
                                    // let preview = cx.update_window::<StateItem, _>(cx.window_handle(), |_, cx| {
                                    //     StateItem::init(BitwardenAccountListBuilder, false, cx)
                                    // }).ok();
                                    actions.append(&mut login.get_actions(&id, &account));
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
                                        .build(),
                                    );
                                }
                            }
                        } else {
                            error!("Failed to list items");
                        }
                        let id = account.id.clone();
                        if let Some(view) = view.upgrade() {
                            let _ = view.update(&mut cx, move |list: &mut AsyncListItems, cx| {
                                list.update(id.clone(), items, cx);
                            });
                            sleep(Duration::from_secs(500)).await;
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
            Box::new(move |_, cx| {
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
            }),
        )
    }
}
