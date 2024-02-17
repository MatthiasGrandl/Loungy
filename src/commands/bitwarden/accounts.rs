use std::{collections::HashMap, fs, sync::mpsc::Receiver, time::Duration};

use async_std::channel::Sender;
use gpui::*;

use crate::{
    commands::{RootCommand, RootCommandBuilder},
    components::form::{Form, Input, InputKind},
    components::list::{Accessory, Item, List, ListItem},
    components::shared::{Icon, Img},
    db::Db,
    query::TextInput,
    state::{Action, ActionsModel, Shortcut, StateModel, StateViewBuilder},
};

use super::list::BitwardenAccount;

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
                    let _ = cx.update_global::<StateModel, _>(|model, cx| {
                        model.pop(cx);
                    });
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
                let accounts = cx
                    .global::<Db>()
                    .get::<HashMap<String, BitwardenAccount>>("bitwarden")
                    .unwrap_or_default();
                let id = values["id"].value::<String>();
                if accounts.get(&id).is_some() {
                    actions
                        .clone()
                        .toast
                        .error("Account Identifier already used", cx);
                    return;
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
                        .command(vec!["config", "server", &account.instance], &mut cx)
                        .await;

                    if account.unlock(&mut cx).await.is_err() {
                        actions.toast.error("Failed to unlock account", &mut cx);
                        return;
                    }

                    actions
                        .toast
                        .success("Vault unlocked successfully", &mut cx);

                    let _ = cx.update_global::<StateModel, _>(|model, cx| {
                        model.replace(BitwardenAccountListBuilder {}, cx);
                    });
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
                    cx.update_global::<StateModel, _>(|model, cx| {
                        model.push(BitwardenAccountFormBuilder {}, cx)
                    });
                },
                false,
            )],
            cx,
        );
        List::new(
            query,
            &actions,
            |_, _, cx| {
                let accounts = cx
                    .global::<Db>()
                    .get::<HashMap<String, BitwardenAccount>>("bitwarden")
                    .unwrap_or_default();

                let mut items: Vec<Item> = accounts
                    .values()
                    .cloned()
                    .into_iter()
                    .map(|account| {
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
                                        let path = account.path(cx);
                                        let id = account.id.clone();
                                        move |actions, cx| {
                                            if fs::remove_dir_all(path.clone()).is_err() {
                                                actions.toast.error("Failed to delete account", cx);
                                            }
                                            cx.update_global::<Db, _>(|db, cx| {
                                                let mut accounts = db
                                                    .get::<HashMap<String, BitwardenAccount>>(
                                                        "bitwarden",
                                                    )
                                                    .unwrap_or_default();
                                                accounts.remove(&id.clone());
                                                if db
                                                    .set::<HashMap<String, BitwardenAccount>>(
                                                        "bitwarden",
                                                        &accounts,
                                                    )
                                                    .is_err()
                                                {
                                                    actions
                                                        .toast
                                                        .error("Failed to delete account", cx);
                                                } else {
                                                    actions.toast.success(
                                                        "Successfully deleted account",
                                                        cx,
                                                    );
                                                    actions.update();
                                                }
                                            });
                                        }
                                    },
                                    false,
                                ),
                            ],
                            None,
                        )
                    })
                    .collect();
                items.sort_by_key(|i| i.keywords.first().unwrap().clone());
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
            "Search Vault",
            "Bitwarden",
            Icon::Vault,
            vec!["Passwords"],
            None,
            Box::new(|_, cx| {
                cx.update_global::<StateModel, _>(|model, cx| {
                    model.push(BitwardenAccountListBuilder {}, cx)
                });
            }),
        )
    }
}
