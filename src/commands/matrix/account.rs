use std::{str::FromStr, sync::mpsc::Receiver};

use async_compat::CompatExt;
use gpui::*;
use log::error;
use matrix_sdk::{matrix_auth::MatrixAuth, ruma::OwnedUserId, Client};

use crate::{
    components::form::{Form, Input, InputKind},
    paths::NAME,
    query::{TextInput, TextInputWeak},
    state::{ActionsModel, StateViewBuilder},
};

use super::client::Session;

#[derive(Clone)]
pub struct AccountCreationBuilder;

impl StateViewBuilder for AccountCreationBuilder {
    fn build(
        &self,
        query: &TextInputWeak,
        actions: &ActionsModel,
        _update_receiver: Receiver<bool>,
        cx: &mut WindowContext,
    ) -> AnyView {
        query.set_placeholder("Login...", cx);
        Form::new(
            vec![
                Input::new(
                    "username",
                    "Username",
                    InputKind::TextField {
                        placeholder: "@username:matrix.org".to_string(),
                        value: "".to_string(),
                        validate: Some(|v| v.is_empty().then(|| "Username is required")),
                        password: false,
                    },
                    cx,
                ),
                Input::new(
                    "password",
                    "Password",
                    InputKind::TextField {
                        placeholder: "Enter password...".to_string(),
                        value: "".to_string(),
                        validate: Some(|v| v.is_empty().then(|| "Password is required")),
                        password: true,
                    },
                    cx,
                ),
            ],
            move |values, actions, cx| {
                let username = values["username"].value::<String>();
                let password = values["password"].value::<String>();
                let actions = actions.clone();
                cx.spawn(move |mut cx| async move {
                    let mut actions_clone = actions.clone();
                    if let Err(err) = Session::login(username, password, actions, &mut cx)
                        .compat()
                        .await
                    {
                        error!("Failed to login: {}", err);
                        actions_clone
                            .toast
                            .error(&format!("Failed to login: {}", err), &mut cx);
                    }
                })
                .detach();

                //
            },
            &query,
            &actions,
            cx,
        )
        .into()
    }
}
