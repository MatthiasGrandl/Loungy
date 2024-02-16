use std::{collections::HashMap, path::PathBuf, sync::mpsc::Receiver, time::Duration};

use async_std::{
    channel,
    process::{Command, Output},
};
use gpui::*;
use log::*;
use serde::{Deserialize, Serialize};
use swift_rs::SRString;
use url::Url;

use crate::{
    commands::{RootCommand, RootCommandBuilder},
    db::Db,
    icon::Icon,
    list::{Accessory, AsyncListItems, Img, Item, List, ListItem},
    paths::Paths,
    query::TextInput,
    state::{Action, ActionsModel, Shortcut, StateModel, StateViewBuilder},
    swift::{autofill, keytap},
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
    fn build(
        &self,
        query: &TextInput,
        actions: &ActionsModel,
        update_receiver: Receiver<bool>,
        cx: &mut WindowContext,
    ) -> AnyView {
        query.set_placeholder("Search your vault...", cx);
        actions.update_global(
            vec![Action::new(
                Img::list_icon(Icon::UserSearch, None),
                "List Accounts",
                Some(Shortcut::cmd(",")),
                |_, cx| {
                    cx.update_global::<StateModel, _>(|model, cx| {
                        model.push(BitwardenAccountListBuilder {}, cx)
                    });
                },
                false,
            )],
            cx,
        );
        AsyncListItems::loader(&self.view, &actions, cx);
        let view = self.view.clone();
        List::new(
            query,
            &actions,
            move |_, _, cx| Ok(Some(view.read(cx).items.clone())),
            None,
            Some(Duration::from_secs(1)),
            update_receiver,
            true,
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
                Window::close(cx);
                cx.spawn(move |mut cx| async move {
                    // TODO: add a better way to make sure the window is closed
                    // Also the window closing and spawning should be handled by the keytap function as that is always going to be wanted
                    cx.background_executor()
                        .timer(Duration::from_millis(250))
                        .await;
                    if let Ok(value) = login.get_field(&field, &id, &mut account, &mut cx).await {
                        unsafe {
                            keytap(SRString::from(value.as_str()));
                        }
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

#[derive(Serialize, Deserialize, Clone)]
pub(super) struct BitwardenAccount {
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
    pub fn path(&self, cx: &WindowContext) -> PathBuf {
        cx.global::<Paths>()
            .data
            .join("bitwarden")
            .join(self.id.clone())
    }
    pub fn path_async(&self, cx: &mut AsyncWindowContext) -> anyhow::Result<PathBuf> {
        cx.read_global::<Paths, anyhow::Result<PathBuf>>(|this, _| {
            Ok(this.data.join("bitwarden").join(self.id.clone()))
        })?
    }
    pub async fn command(
        &self,
        args: Vec<&str>,
        cx: &mut AsyncWindowContext,
    ) -> anyhow::Result<Output> {
        let mut env: HashMap<String, String> = HashMap::new();
        env.insert(
            "PATH".to_string(),
            "/opt/homebrew/bin:/usr/local/bin/bw".to_string(),
        );
        env.insert(
            "BITWARDENCLI_APPDATA_DIR".to_string(),
            self.path_async(cx).unwrap().to_string_lossy().to_string(),
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
        self.command(args, cx).await
    }
    pub async fn unlock(&mut self, cx: &mut AsyncWindowContext) -> anyhow::Result<()> {
        // TODO: if there is no password, we need to prompt for it
        let status = self
            .command(vec!["status", "--raw", "--nointeraction"], cx)
            .await?;
        debug!("Status: {}", String::from_utf8(status.stdout.clone())?);
        let status: BitwardenStatus = serde_json::from_slice(&status.stdout)?;

        match status.status {
            BitwardenVaultStatus::Unlocked => {
                return Ok(());
            }
            BitwardenVaultStatus::Unauthenticated => {
                if !self
                    .command(vec!["login", "--apikey", "--nointeraction"], cx)
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
            let _ = cx.update_global::<StateModel, _>(|model, cx| {
                model.push(
                    BitwardenPasswordPromptBuilder {
                        password: s,
                        account: self.clone(),
                    },
                    cx,
                );
            });
            let (password, remember) = r.recv().await?;
            if remember {
                self.password = Some(password.clone());
            }
            password
        };
        let output = self
            .command(vec!["unlock", &password, "--raw", "--nointeraction"], cx)
            .await?;

        if output.stdout.len() < 1 {
            self.session = None;
            return Err(anyhow::anyhow!("Failed to unlock account"));
        };
        let session = String::from_utf8(output.stdout)?;
        self.session = Some(session);
        let _ = cx.update_global::<Db, _>(|db, _| {
            // TODO: This is a bit yolo, might overwrite other accounts
            let mut accounts = db
                .get::<HashMap<String, BitwardenAccount>>("bitwarden")
                .unwrap_or_default();
            accounts.insert(self.id.clone(), self.clone());
            let _ = db.set::<HashMap<String, BitwardenAccount>>("bitwarden", &accounts);
        });
        Ok(())
    }
}

pub struct BitwardenCommandBuilder;

impl RootCommandBuilder for BitwardenCommandBuilder {
    fn build(&self, cx: &mut WindowContext) -> RootCommand {
        let view = cx.new_view(|cx| {
            cx.spawn(move |view, mut cx| async move {
                let mut first = true;
                let mut old_count = 0;
                let mut last_update = std::time::Instant::now();
                loop {
                    if view.upgrade().is_none() {
                        break;
                    }
                    let mut accounts = cx
                        .read_global::<Db, _>(|db, _| {
                            db.get::<HashMap<String, BitwardenAccount>>("bitwarden")
                                .unwrap_or_default()
                        })
                        .unwrap_or_default();
                    if accounts.len() == 0 || (accounts.len() == old_count && last_update.elapsed().as_secs() < 500) {
                        cx.background_executor()
                            .timer(Duration::from_millis(250))
                            .await;
                        continue;
                    }
                    old_count = accounts.len();
                    last_update = std::time::Instant::now();
                    if !first {
                        for account in accounts.values_mut() {
                            let _ = account.auth_command(vec!["sync"], &mut cx).await;
                        }
                    }
                    first = false;

                    let mut items: Vec<Item> = vec![];
                    for mut account in accounts.values().cloned() {
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
                                match item {
                                    BitwardenItem::Login {
                                        id,
                                        name,
                                        notes: _,
                                        login,
                                    } => {
                                        let domain = login
                                            .uris
                                            .first()
                                            .map(|uri| {
                                                Url::parse(&if !uri.uri.starts_with("http") {
                                                    format!("https://{}", uri.uri)
                                                } else {
                                                    uri.uri.clone()
                                                })
                                                .ok()
                                                .map(|url| url.domain().map(|d| d.to_owned()))
                                                .flatten()
                                            })
                                            .flatten();

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
                                            &mut login
                                                .uris
                                                .iter()
                                                .map(|uri| uri.uri.clone())
                                                .collect(),
                                        );
                                        let mut actions = vec![
                                            Action::new(
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
                                                            let mut prev = SRString::from("");
                                                            // Timeout after 2 minutes, could probably be lower, but TOTP takes a while sometimes
                                                            let max_tries = 1200;
                                                            let mut tries = 0;
                                                            for field in login.fields() {
                                                                loop {
                                                                    let value = login
                                                                    .get_field(
                                                                        field, &id, &mut account,
                                                                        &mut cx,
                                                                    )
                                                                    .await.unwrap();
                                                                match unsafe {
                                                                    autofill(
                                                                        SRString::from(
                                                                            value.as_str(),
                                                                        ),
                                                                        field.eq("password"),
                                                                        &prev,
                                                                    )
                                                                } {
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
                                                                        cx.background_executor().timer(Duration::from_millis(100)).await;
                                                                    }
                                                                }
                                                                }
                                                            }
                                                        })
                                                        .detach();
                                                    }
                                                },
                                                false,
                                            )
                                        ];
                                        actions.append(&mut login.get_actions(&id, &account));
                                        items.push(Item::new(
                                            keywords,
                                            cx.new_view(|_| {
                                                ListItem::new(
                                                    Some(img),
                                                    name.clone(),
                                                    None,
                                                    vec![Accessory::new(
                                                        login.username.clone(),
                                                        None,
                                                    )],
                                                )
                                            })
                                            .unwrap()
                                            .into(),
                                            None,
                                            actions,
                                            None,
                                        ));
                                    }
                                    _ => {}
                                }
                            }
                        } else {
                            error!("Failed to list items");
                        }
                    }
                    if let Some(view) = view.upgrade() {
                        let _ = view.update(&mut cx, move |list: &mut AsyncListItems, cx| {
                            list.update(items, cx);
                        });
                    } else {
                        break;
                    }
                }
            })
            .detach();
            AsyncListItems { items: vec![], initialized: false }
        });
        RootCommand::new(
            "Search Vault",
            "Bitwarden",
            Icon::Lock,
            vec!["Passwords"],
            None,
            Box::new(move |_, cx| {
                let view = view.clone();
                cx.update_global::<StateModel, _>(|model, cx| {
                    let accounts = cx
                        .global::<Db>()
                        .get::<HashMap<String, BitwardenAccount>>("bitwarden")
                        .unwrap_or_default();
                    if accounts.is_empty() {
                        model.push(BitwardenAccountFormBuilder {}, cx);
                    } else {
                        model.push(BitwardenListBuilder { view }, cx);
                    };
                });
            }),
        )
    }
}
