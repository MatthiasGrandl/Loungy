use std::time::Duration;

use gpui::*;

use crate::{
    commands::{RootCommand, RootCommandBuilder},
    components::{
        list::{Item, ListBuilder, ListItem},
        shared::{Icon, Img},
    },
    db::db,
    state::{Action, Shortcut, StateModel, StateViewBuilder, StateViewContext},
    theme::{Theme, ThemeSettings},
};

#[derive(Clone)]
pub struct ThemeListBuilder;
impl StateViewBuilder for ThemeListBuilder {
    fn build(&self, context: &mut StateViewContext, cx: &mut WindowContext) -> AnyView {
        context.query.set_placeholder("Search for themes...", cx);
        ListBuilder::new()
            .interval(Duration::from_secs(10))
            .build(
                |_, _, cx| {
                    let themes = Theme::list();
                    Ok(Some(
                        themes
                            .into_iter()
                            .map(|theme| {
                                Item::new(
                                    theme.name.clone(),
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
                                                    let mut settings = db()
                                                        .get::<ThemeSettings>("theme")
                                                        .unwrap_or_default();
                                                    settings.light = name.clone().to_string();
                                                    if db()
                                                        .set::<ThemeSettings>("theme", &settings)
                                                        .is_err()
                                                    {
                                                        this.toast.error(
                                                            "Failed to change light theme",
                                                            cx,
                                                        );
                                                    } else {
                                                        this.toast
                                                            .success("Changed light theme", cx);
                                                    }

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
                                                    let mut settings = db()
                                                        .get::<ThemeSettings>("theme")
                                                        .unwrap_or_default();
                                                    settings.dark = name.clone().to_string();
                                                    if db()
                                                        .set::<ThemeSettings>("theme", &settings)
                                                        .is_err()
                                                    {
                                                        this.toast.error(
                                                            "Failed to change dark theme",
                                                            cx,
                                                        );
                                                    } else {
                                                        this.toast
                                                            .success("Changed dark theme", cx);
                                                    }
                                                    cx.refresh();
                                                }
                                            },
                                            false,
                                        ),
                                    ],
                                    None,
                                    None,
                                    None,
                                )
                            })
                            .collect(),
                    ))
                },
                None,
                context,
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
                StateModel::update(|this, cx| this.push(ThemeListBuilder, cx), cx);
            }),
        )
    }
}
