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

use std::{fs, time::Duration};

use async_std::channel::Sender;
use bonsaidb::core::schema::SerializedCollection;
use gpui::*;
use log::error;

use crate::{
    command,
    commands::{RootCommand, RootCommandBuilder},
    components::{
        form::{Form, Input, InputKind},
        list::{Accessory, Item, ItemBuilder, ListBuilder, ListItem},
        shared::{Icon, Img},
    },
    state::{Action, CommandTrait, Shortcut, StateModel, StateViewBuilder, StateViewContext},
};

use super::list::{db, BitwardenAccount};

#[derive(Clone)]
pub(super) struct BitwardenPasswordPromptBuilder {
    pub(super) account: BitwardenAccount,
    pub(super) password: Sender<(String, bool)>,
}

command!(BitwardenPasswordPromptBuilder);

impl StateViewBuilder for BitwardenPasswordPromptBuilder {
    fn build(&self, context: &mut StateViewContext, cx: &mut WindowContext) -> AnyView {
        let password = self.password.clone();
        Form::new(
            vec![
                Input::new(
                    "password",
                    "Password",
                    InputKind::TextField {
                        placeholder: format!("Enter password for {}", self.account.id),
                        value: "".to_string(),
                        validate: Some(|v| v.is_empty().then_some("Password is required")),
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
                    let remember = matches!(
                        values["remember_password"]
                            .value::<String>()
                            .to_lowercase()
                            .as_str(),
                        "y" | "yes"
                    );
                    if password
                        .send((values["password"].value::<String>(), remember))
                        .await
                        .is_err()
                    {
                        log::error!("Failed to send password back");
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
            context,
            cx,
        )
        .into()
    }
}

impl EventEmitter<Self> for BitwardenPasswordPromptBuilder {}

#[derive(Clone)]
pub struct BitwardenAccountFormBuilder;
command!(BitwardenAccountFormBuilder);
impl StateViewBuilder for BitwardenAccountFormBuilder {
    fn build(&self, context: &mut StateViewContext, cx: &mut WindowContext) -> AnyView {
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
                            if url::Url::parse(v).is_err() {
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
                        validate: Some(|v| v.is_empty().then_some("Identifier is required")),
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
                        validate: Some(|v| v.is_empty().then_some("Client ID is required")),
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
                        validate: Some(|v| v.is_empty().then_some("Client Secret is required")),
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
            context,
            cx,
        )
        .into()
    }
}

#[derive(Clone)]
pub struct BitwardenAccountListBuilder;
command!(BitwardenAccountListBuilder);
impl StateViewBuilder for BitwardenAccountListBuilder {
    fn build(&self, context: &mut StateViewContext, cx: &mut WindowContext) -> AnyView {
        context.query.set_placeholder("Search your accounts...", cx);
        context.actions.update_global(
            vec![Action::new(
                Img::default().icon(Icon::PlusSquare),
                "Add Account",
                Some(Shortcut::new("n").cmd()),
                |_, cx| {
                    StateModel::update(|this, cx| this.push(BitwardenAccountFormBuilder, cx), cx);
                },
                false,
            )],
            cx,
        );
        ListBuilder::new()
            .interval(Duration::from_secs(10))
            .build(
                |_, _, _| {
                    let accounts = BitwardenAccount::all(db()).descending().query().unwrap();

                    let items: Vec<Item> = accounts
                        .into_iter()
                        .map(|account| {
                            let account = account.contents;
                            ItemBuilder::new(account.id.clone(), {
                                let id = account.id.clone();
                                let instance = account.instance.clone();
                                ListItem::new(
                                    Some(Img::default().icon(Icon::User)),
                                    id,
                                    None,
                                    vec![Accessory::new(instance, None)],
                                )
                            })
                            .keywords(vec![account.id.clone()])
                            .actions(vec![
                                Action::new(
                                    Img::default().icon(Icon::Pen),
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
                                    Img::default().icon(Icon::Delete),
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
                            ])
                            .build()
                        })
                        .collect();
                    Ok(Some(items))
                },
                context,
                cx,
            )
            .into()
    }
}

pub struct BitwardenAccountCommandBuilder;
command!(BitwardenAccountCommandBuilder);
impl RootCommandBuilder for BitwardenAccountCommandBuilder {
    fn build(&self, _: &mut WindowContext) -> RootCommand {
        RootCommand::new(
            "bitwarden_accounts",
            "Search Vault",
            "Bitwarden",
            Icon::Vault,
            vec!["Passwords"],
            None,
            |_, cx| {
                StateModel::update(|this, cx| this.push(BitwardenAccountListBuilder, cx), cx);
            },
        )
    }
}
