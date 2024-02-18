use std::{sync::mpsc::Receiver, time::Duration};

use gpui::*;

use crate::{
    commands::{RootCommand, RootCommandBuilder},
    components::list::{Item, List, ListItem},
    components::shared::{Icon, Img},
    db::Db,
    query::TextInput,
    state::{Action, ActionsModel, Shortcut, StateModel, StateViewBuilder},
    theme::{Theme, ThemeSettings},
};

#[derive(Clone)]
pub struct ThemeListBuilder;
impl StateViewBuilder for ThemeListBuilder {
    fn build(
        &self,
        query: &TextInput,
        actions: &ActionsModel,
        update_receiver: Receiver<bool>,
        cx: &mut WindowContext,
    ) -> AnyView {
        query.set_placeholder("Search for themes...", cx);
        List::new(
            query,
            &actions,
            |_, _, cx| {
                let themes = Theme::list(cx);
                Ok(Some(
                    themes
                        .into_iter()
                        .map(|theme| {
                            Item::new(
                                vec![theme.name.clone()],
                                cx.new_view(|_| {
                                    ListItem::new(
                                        Some(Img::list_dot(theme.base)),
                                        theme.name.clone(),
                                        None,
                                        vec![],
                                    )
                                })
                                .into(),
                                None,
                                vec![
                                    Action::new(
                                        Img::list_icon(Icon::Palette, None),
                                        "Select Theme",
                                        None,
                                        {
                                            let theme = theme.clone();
                                            move |this, cx| {
                                                cx.update_global::<Theme, _>(|this, _| {
                                                    *this = theme.clone();
                                                });
                                                this.toast.success("Theme activated", cx);
                                                cx.refresh();
                                            }
                                        },
                                        false,
                                    ),
                                    Action::new(
                                        Img::list_icon(Icon::Sun, None),
                                        "Default Light Theme",
                                        Some(Shortcut::cmd("l")),
                                        {
                                            let name = theme.name.clone();
                                            move |this, cx| {
                                                cx.update_global::<Db, _>(|db, cx| {
                                                    let mut settings = db
                                                        .get::<ThemeSettings>("theme")
                                                        .unwrap_or_default();
                                                    settings.light = name.clone().to_string();
                                                    if db
                                                        .set::<ThemeSettings>("theme", &settings)
                                                        .is_err()
                                                    {
                                                        let _ = this.toast.error(
                                                            "Failed to change light theme",
                                                            cx,
                                                        );
                                                    } else {
                                                        let _ = this
                                                            .toast
                                                            .success("Changed light theme", cx);
                                                    }
                                                });

                                                cx.refresh();
                                            }
                                        },
                                        false,
                                    ),
                                    Action::new(
                                        Img::list_icon(Icon::Moon, None),
                                        "Default Dark Theme",
                                        Some(Shortcut::cmd("d")),
                                        {
                                            let name = theme.name.clone();
                                            move |this, cx| {
                                                cx.update_global::<Db, _>(|db, cx| {
                                                    let mut settings = db
                                                        .get::<ThemeSettings>("theme")
                                                        .unwrap_or_default();
                                                    settings.dark = name.clone().to_string();
                                                    if db
                                                        .set::<ThemeSettings>("theme", &settings)
                                                        .is_err()
                                                    {
                                                        let _ = this.toast.error(
                                                            "Failed to change dark theme",
                                                            cx,
                                                        );
                                                    } else {
                                                        let _ = this
                                                            .toast
                                                            .success("Changed dark theme", cx);
                                                    }
                                                });
                                                cx.refresh();
                                            }
                                        },
                                        false,
                                    ),
                                ],
                                None,
                            )
                        })
                        .collect(),
                ))
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

pub struct ThemeCommandBuilder;

impl RootCommandBuilder for ThemeCommandBuilder {
    fn build(&self, _cx: &mut WindowContext) -> RootCommand {
        RootCommand::new(
            "themes",
            "Search Themes",
            "Customization",
            Icon::Palette,
            vec!["Appearance"],
            None,
            Box::new(|_, cx| {
                cx.update_global::<StateModel, _>(|model, cx| model.push(ThemeListBuilder {}, cx));
            }),
        )
    }
}
