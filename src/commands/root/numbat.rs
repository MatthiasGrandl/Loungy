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

use gpui::*;
use numbat::{
    markup::{Formatter, PlainTextFormatter},
    module_importer::BuiltinModuleImporter,
    pretty_print::PrettyPrint,
    Context,
};

use crate::{
    components::{list::ItemComponent, shared::Icon},
    query::{TextEvent, TextInputWeak},
    theme::Theme,
};

#[derive(Clone)]

pub struct NumbatResult {
    equation: String,
    pub result: String,
    unit: String,
    type_id: String,
}

#[derive(Clone)]
pub struct Numbat {
    pub result: Option<NumbatResult>,
}

fn rephraser(s: &str) -> String {
    let mut s = s.to_string();
    if s.contains(" and ") {
        s = s.replace(" and ", " + ");
    }
    if s.contains(" from now") {
        s = format!("now() + {}", s.replace(" from now", ""));
    }
    s
}

impl Numbat {
    pub fn init(query: &TextInputWeak, cx: &mut WindowContext) -> View<Numbat> {
        let importer = BuiltinModuleImporter::default();
        let mut ctx = Context::new(importer);
        ctx.load_currency_module_on_demand(true);
        Context::prefetch_exchange_rates();
        let _ = ctx.interpret("use prelude", numbat::resolver::CodeSource::Text);

        cx.new_view(move |cx| {
            if let Some(query) = query.view.upgrade() {
                cx.subscribe(&query, move |subscriber: &mut Numbat, _, event, cx| {
                    if let TextEvent::Input { text } = event {
                        let result =
                            ctx.interpret(&rephraser(text), numbat::resolver::CodeSource::Text);
                        let formatter = PlainTextFormatter {};
                        subscriber.result = match result {
                            Ok((statements, result)) => {
                                let s: Vec<String> = statements
                                    .iter()
                                    .map(|s| formatter.format(&s.pretty_print(), false))
                                    .collect();
                                let s = s.join(" ");
                                let result = &result.to_markup(
                                    statements.last(),
                                    ctx.dimension_registry(),
                                    true,
                                    true,
                                );
                                let mut value: Option<String> = None;
                                let mut type_id: Option<String> = None;
                                let mut unit: Option<String> = None;
                                for part in &result.0 {
                                    match part.1 {
                                        numbat::markup::FormatType::String => {
                                            value = Some(part.2.clone())
                                        }
                                        numbat::markup::FormatType::Value => {
                                            value = Some(part.2.clone());
                                        }
                                        numbat::markup::FormatType::TypeIdentifier => {
                                            type_id = Some(part.2.clone());
                                        }
                                        numbat::markup::FormatType::Unit => {
                                            unit = Some(part.2.clone());
                                        }
                                        _ => {}
                                    }
                                }
                                value.map(|value| NumbatResult {
                                    result: value,
                                    unit: unit.unwrap_or_default(),
                                    type_id: type_id.unwrap_or_default(),
                                    equation: s.replace('➞', "to"),
                                })
                            }
                            Err(_e) => None,
                        };
                        cx.notify();
                    }
                })
                .detach();
            }

            Numbat { result: None }
        })
    }
}

#[derive(Clone)]
pub struct NumbatWrapper {
    pub inner: View<Numbat>,
}

impl ItemComponent for NumbatWrapper {
    fn render(&self, _selected: bool, _cx: &WindowContext) -> AnyElement {
        self.inner.clone().into_any_element()
    }
}

impl Render for Numbat {
    fn render(&mut self, cx: &mut gpui::ViewContext<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        if self.result.is_none() {
            return div();
        }
        let result = self.result.as_ref().unwrap().clone();
        let len = result
            .equation
            .len()
            .max(format!("{} {}", result.result, result.unit).len());

        if len > 30 {
            div().text_sm()
        } else if len > 25 {
            div()
        } else if len > 20 {
            div().text_lg()
        } else {
            div().text_xl()
        }
        .flex()
        .font_weight(FontWeight::SEMIBOLD)
        .relative()
        .child(
            div()
                .w_1_2()
                .h_24()
                .flex()
                .items_center()
                .justify_center()
                .child(result.equation),
        )
        .child(
            div()
                .w_1_2()
                .h_24()
                .flex()
                .items_center()
                .justify_center()
                .child(format!("{} {}", result.result, result.unit)),
        )
        .child(
            div()
                .absolute()
                .flex()
                .items_center()
                .justify_center()
                .inset_0()
                .child(
                    svg()
                        .path(Icon::MoveRight.path())
                        .size_12()
                        .text_color(theme.surface0),
                ),
        )
    }
}
