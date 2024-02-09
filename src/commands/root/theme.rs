use gpui::*;

use crate::{
    db::Db,
    icon::Icon,
    list::{Img, Item, List, ListItem},
    nucleo::fuzzy_match,
    query::{TextEvent, TextInput},
    state::{Action, ActionsModel, Loading, Shortcut, StateView},
    theme::{Theme, ThemeSettings},
};

#[derive(Clone)]
struct ThemeList {
    list: View<List>,
    query: TextInput,
    model: Model<Vec<Item>>,
}

impl ThemeList {
    fn update(&mut self, cx: &mut WindowContext) {
        let themes = Theme::list(cx);
        let items: Vec<Item> = themes
            .into_iter()
            .map(|theme| {
                let clone = theme.clone();
                let n1 = theme.name.clone();
                let n2 = theme.name.clone();
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
                            Box::new(move |cx| {
                                cx.update_global::<Theme, _>(|this, _| {
                                    *this = clone.clone();
                                });
                                cx.refresh();
                            }),
                            false,
                        ),
                        Action::new(
                            Img::list_icon(Icon::Sun, None),
                            "Default Light Theme",
                            Some(Shortcut::cmd("l")),
                            Box::new(move |cx| {
                                cx.update_global::<Db, _>(|this, _| {
                                    let mut settings =
                                        this.get::<ThemeSettings>("theme").unwrap_or_default();
                                    settings.light = n1.clone().to_string();
                                    // TODO: Catch error with future toast API
                                    let _ = this.set::<ThemeSettings>("theme", &settings);
                                });
                                cx.refresh();
                            }),
                            false,
                        ),
                        Action::new(
                            Img::list_icon(Icon::Moon, None),
                            "Default Dark Theme",
                            Some(Shortcut::cmd("d")),
                            Box::new(move |cx| {
                                cx.update_global::<Db, _>(|this, _| {
                                    let mut settings =
                                        this.get::<ThemeSettings>("theme").unwrap_or_default();
                                    settings.dark = n2.clone().to_string();
                                    // TODO: Catch error with future toast API
                                    let _ = this.set::<ThemeSettings>("theme", &settings);
                                });
                                cx.refresh();
                            }),
                            false,
                        ),
                    ],
                    None,
                )
            })
            .collect();
        self.model.update(cx, |model, cx| {
            *model = items;
            cx.notify();
        });
        self.list(cx);
    }
    fn list(&mut self, cx: &mut WindowContext) {
        let query = self.query.view.read(cx).text.clone();
        self.list.update(cx, |this, cx| {
            let items = self.model.read(cx).clone();
            let items = fuzzy_match(&query, items, false);
            this.items = items;
            cx.notify();
        });
    }
}

impl Render for ThemeList {
    fn render(&mut self, _cx: &mut ViewContext<Self>) -> impl IntoElement {
        self.list.clone()
    }
}

pub struct ThemeListBuilder {}
impl StateView for ThemeListBuilder {
    fn build(
        &self,
        query: &TextInput,
        actions: &ActionsModel,
        _loading: &View<Loading>,
        cx: &mut WindowContext,
    ) -> AnyView {
        let mut comp = ThemeList {
            list: List::new(query, Some(actions), cx),
            query: query.clone(),
            model: cx.new_model(|_| Vec::<Item>::new()),
        };
        query.set_placeholder("Search for themes...", cx);
        comp.update(cx);

        cx.new_view(|cx| {
            cx.subscribe(
                &query.view,
                move |subscriber: &mut ThemeList, _emitter, event, cx| match event {
                    TextEvent::Input { text: _ } => {
                        subscriber.list(cx);
                    }
                    _ => {}
                },
            )
            .detach();
            comp
        })
        .into()
    }
}
