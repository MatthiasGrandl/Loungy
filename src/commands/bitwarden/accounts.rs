use std::{fs, sync::mpsc::Receiver, time::Duration};

use async_std::channel::Sender;
use bonsaidb::core::schema::SerializedCollection;
use gpui::*;
use log::error;

use crate::{
    commands::{RootCommand, RootCommandBuilder},
    components::form::{Form, Input, InputKind},
    components::list::{Accessory, Item, List, ListItem},
    components::shared::{Icon, Img},
    query::TextInput,
    state::{Action, ActionsModel, Shortcut, StateModel, StateViewBuilder},
};

use super::list::{db, BitwardenAccount};

#[derive(Clone)]
pub(super) struct BitwardenPasswordPromptBuilder {
    pub(super) account: BitwardenAccount,
    pub(super) password: Sender<(String, bool)>,
}

impl StateViewBuilder for BitwardenPasswordPromptBuilder {
    fn build(
        &self,
        query: &TextInput,
        actions: &ActionsModel,
        _: Receiver<bool>,
        cx: &mut WindowContext,
    ) -> AnyView {
        let password = self.password.clone();
        Form::new(
            vec![
                Input::new(
                    "password",
                    "Password",
                    InputKind::TextField {
                        placeholder: format!("Enter password for {}", self.account.id),
                        value: "".to_string(),
                        validate: Some(|v| v.is_empty().then(|| "Password is required")),
                        password: true,
                    },
                    cx,
                ),
                Input::new(
                    "remember_password",
                    "Remember Password?",
                    InputKind::TextField {
                        placeholder: "(y)es|(n)o".to_string(),
                        value: "no".to_string(),
                        validate: Some(|v| match v.to_lowercase().as_str() {
                            "y" | "yes" | "n" | "no" => None,
                            _ => Some("Invalid response"),
                        }),
                        password: false,
                    },
                    cx,
                ),
            ],
            move |values, _, cx| {
                let password = password.clone();
                cx.spawn(|mut cx| async move {
                    let remember = match values["remember_password"]
                        .value::<String>()
                        .to_lowercase()
                        .as_str()
                    {
                        "y" | "yes" => true,
                        _ => false,
                    };
                    if password
                        .send((values["password"].value::<String>(), remember))
                        .await
                        .is_err()
                    {
                        eprintln!("Failed to send password back");
                    }
                    StateModel::update_async(
                        |this, cx| {
                            this.pop(cx);
                        },
                        &mut cx,
                    );
                })
                .detach();
            },
            &query,
            &actions,
            cx,
        )
        .into()
    }
}

impl EventEmitter<Self> for BitwardenPasswordPromptBuilder {}

#[derive(Clone)]
pub struct BitwardenAccountFormBuilder;
impl StateViewBuilder for BitwardenAccountFormBuilder {
    fn build(
        &self,
        query: &TextInput,
        actions: &ActionsModel,
        _: Receiver<bool>,
        cx: &mut WindowContext,
    ) -> AnyView {
        Form::new(
            vec![
                Input::new(
                    "instance",
                    "Instance URL",
                    InputKind::TextField {
                        placeholder: "Enter the bitwarden instance URL...".to_string(),
                        value: "https://bitwarden.com".to_string(),
                        validate: Some(|v| {
                            if v.is_empty() {
                                return Some("Instance URL is required");
                            };
                            if url::Url::parse(&v).is_err() {
                                return Some("Invalid URL");
                            }
                            None
                        }),
                        password: false,
                    },
                    cx,
                ),
                Input::new(
                    "id",
                    "Identifier",
                    InputKind::TextField {
                        placeholder: "Enter an account identifier...".to_string(),
                        value: "".to_string(),
                        validate: Some(|v| v.is_empty().then(|| "Identifier is required")),
                        password: false,
                    },
                    cx,
                ),
                Input::new(
                    "client_id",
                    "Client ID",
                    InputKind::TextField {
                        placeholder: "Enter client_id...".to_string(),
                        value: "".to_string(),
                        validate: Some(|v| v.is_empty().then(|| "Client ID is required")),
                        password: false,
                    },
                    cx,
                ),
                Input::new(
                    "client_secret",
                    "Client Secret",
                    InputKind::TextField {
                        placeholder: "Enter client_secret...".to_string(),
                        value: "".to_string(),
                        validate: Some(|v| v.is_empty().then(|| "Client Secret is required")),
                        password: true,
                    },
                    cx,
                ),
            ],
            |values, actions, cx| {
                if BitwardenAccount::get(&values["id"].value::<String>(), db())
                    .unwrap()
                    .is_some()
                {
                    actions
                        .clone()
                        .toast
                        .error("Account Identifier already used", cx);
                }
                let mut actions = actions.clone();
                cx.spawn(|mut cx| async move {
                    actions.toast.loading("Saving account...", &mut cx);

                    let mut account = BitwardenAccount {
                        instance: values["instance"].value::<String>(),
                        id: values["id"].value::<String>(),
                        client_id: values["client_id"].value::<String>(),
                        client_secret: values["client_secret"].value::<String>(),
                        password: None,
                        session: None,
                    };

                    let _ = account
                        .command(vec!["config", "server", &account.instance])
                        .await;

                    if account.unlock(&mut cx).await.is_err() {
                        actions.toast.error("Failed to unlock account", &mut cx);
                        return;
                    }

                    actions
                        .toast
                        .success("Vault unlocked successfully", &mut cx);

                    StateModel::update_async(
                        |this, cx| {
                            this.replace(BitwardenAccountListBuilder, cx);
                        },
                        &mut cx,
                    );
                })
                .detach();
            },
            &query,
            &actions,
            cx,
        )
        .into()
    }
}

#[derive(Clone)]
pub struct BitwardenAccountListBuilder;
impl StateViewBuilder for BitwardenAccountListBuilder {
    fn build(
        &self,
        query: &TextInput,
        actions: &ActionsModel,
        update_receiver: Receiver<bool>,
        cx: &mut WindowContext,
    ) -> AnyView {
        query.set_placeholder("Search your accounts...", cx);
        actions.update_global(
            vec![Action::new(
                Img::list_icon(Icon::PlusSquare, None),
                "Add Account",
                Some(Shortcut::cmd("n")),
                |_, cx| {
                    StateModel::update(|this, cx| this.push(BitwardenAccountFormBuilder, cx), cx);
                },
                false,
            )],
            cx,
        );
        List::new(
            query,
            &actions,
            |_, _, cx| {
                let accounts = BitwardenAccount::all(db()).descending().query().unwrap();

                let items: Vec<Item> = accounts
                    .into_iter()
                    .map(|account| {
                        let account = account.contents;
                        Item::new(
                            vec![account.id.clone()],
                            cx.new_view({
                                let id = account.id.clone();
                                let instance = account.instance.clone();
                                move |_| {
                                    ListItem::new(
                                        Some(Img::list_icon(Icon::User, None)),
                                        id,
                                        None,
                                        vec![Accessory::new(instance, None)],
                                    )
                                }
                            })
                            .into(),
                            None,
                            vec![
                                Action::new(
                                    Img::list_icon(Icon::Pen, None),
                                    "Edit",
                                    None,
                                    {
                                        // TODO:
                                        move |actions, cx| {
                                            actions.toast.error("Not implemented", cx);
                                        }
                                    },
                                    false,
                                ),
                                Action::new(
                                    Img::list_icon(Icon::Delete, None),
                                    "Delete",
                                    None,
                                    {
                                        //
                                        let path = account.path();
                                        let id = account.id.clone();
                                        move |actions, cx| {
                                            if let Err(err) = fs::remove_dir_all(path.clone()) {
                                                error!("Failed to delete account: {}", err);
                                                actions.toast.error("Failed to delete account", cx);
                                            }
                                            if let Some(account) =
                                                BitwardenAccount::get(&id, db()).unwrap()
                                            {
                                                if let Err(err) = account.delete(db()) {
                                                    error!("Failed to delete account: {}", err);
                                                    actions
                                                        .toast
                                                        .error("Failed to delete account", cx);
                                                }
                                            };
                                            StateModel::update(|this, cx| this.reset(cx), cx);
                                        }
                                    },
                                    false,
                                ),
                            ],
                            None,
                        )
                    })
                    .collect();
                Ok(Some(items))
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

pub struct BitwardenAccountCommandBuilder;

impl RootCommandBuilder for BitwardenAccountCommandBuilder {
    fn build(&self, _: &mut WindowContext) -> RootCommand {
        RootCommand::new(
            "bitwarden_accounts",
            "Search Vault",
            "Bitwarden",
            Icon::Vault,
            vec!["Passwords"],
            None,
            Box::new(|_, cx| {
                StateModel::update(|this, cx| this.push(BitwardenAccountListBuilder, cx), cx);
            }),
        )
    }
}
