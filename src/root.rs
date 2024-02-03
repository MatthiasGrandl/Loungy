use gpui::*;

use crate::{query::Query, theme::Theme};

#[derive(IntoElement, Clone)]
pub struct RootCommand {}

impl RenderOnce for RootCommand {
    fn render(self, cx: &mut WindowContext) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let query = cx.global::<Query>();

        let mut bg_hover = theme.mantle;
        bg_hover.fade_out(0.5);
        let selected = 0;
        let children = query.inner.split_whitespace().enumerate().map(|(i, s)| {
            let (bg, bg_hover) = if i == selected {
                (theme.mantle, theme.mantle)
            } else {
                (transparent_black(), bg_hover)
            };
            div()
                .p_2()
                .bg(bg)
                .hover(|s| s.bg(bg_hover).border_color(theme.crust))
                .border_1()
                .rounded_xl()
                .child(String::from(s))
        });
        div().children(children)
    }
}
