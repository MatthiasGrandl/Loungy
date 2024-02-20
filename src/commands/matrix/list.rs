use bonsaidb::core::schema::SerializedCollection;
use gpui::*;

use crate::{
    commands::{RootCommand, RootCommandBuilder},
    components::{list::AsyncListItems, shared::Icon},
    state::StateModel,
};

use super::{
    account::AccountCreationBuilder,
    client::{db, Session},
};

pub struct MatrixCommandBuilder;

impl RootCommandBuilder for MatrixCommandBuilder {
    fn build(&self, cx: &mut WindowContext) -> RootCommand {
        let view = cx.new_view(|cx| {
            // cx.spawn(move |view, mut cx| async move {
            //     //
            // })
            // .detach();
            AsyncListItems {
                items: vec![],
                initialized: false,
            }
        });
        RootCommand::new(
            "matrix",
            "Search Rooms",
            "Matrix",
            Icon::MessageCircle,
            vec!["Chat", "Messages"],
            None,
            Box::new(move |_, cx| {
                let view = view.clone();
                let sessions = Session::all(db());
                if sessions.count().unwrap_or_default() == 0 {
                    StateModel::update(|this, cx| this.push(AccountCreationBuilder, cx), cx);
                } else {
                    unimplemented!();
                };
            }),
        )
    }
}
