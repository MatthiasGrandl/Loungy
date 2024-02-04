use gpui::*;
use numbat::{
    markup::{Formatter, PlainTextFormatter},
    module_importer::BuiltinModuleImporter,
    pretty_print::PrettyPrint,
    Context,
};

use crate::{
    lazy::LazyMutex,
    theme::{self, Theme},
};

struct Ctx {
    ctx: Context,
}
impl Default for Ctx {
    fn default() -> Ctx {
        let importer = BuiltinModuleImporter::default();
        let mut ctx = Context::new(importer);
        ctx.load_currency_module_on_demand(true);
        Context::prefetch_exchange_rates();
        let _ = ctx.interpret("use prelude", numbat::resolver::CodeSource::Text);
        Ctx { ctx }
    }
}
static CTX: LazyMutex<Ctx> = LazyMutex::new(Ctx::default);

#[derive(Clone)]
pub struct Numbat {
    result: Option<String>,
    unit: Option<String>,
    type_id: Option<String>,
    equation: String,
}

impl Numbat {
    pub fn init(query: &str) -> Option<Self> {
        let c = &mut *CTX.lock();
        let ctx = &mut c.ctx;
        let result = ctx.interpret(query, numbat::resolver::CodeSource::Text);
        let formatter = PlainTextFormatter {};
        match result {
            Ok((statements, result)) => {
                let s: Vec<String> = statements
                    .iter()
                    .map(|s| {
                        let s = formatter.format(&s.pretty_print(), false);
                        s
                    })
                    .collect();
                let s = s.join(" ");
                let result = &result.to_markup(statements.last(), ctx.dimension_registry(), true);
                let mut value: Option<String> = None;
                let mut type_id: Option<String> = None;
                let mut unit: Option<String> = None;
                for part in &result.0 {
                    match part.1 {
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

                return Some(Self {
                    result: value,
                    unit,
                    type_id,
                    equation: s.replace("➞", "to"),
                });
            }
            _ => None,
        }
    }
}

impl Render for Numbat {
    fn render(&mut self, cx: &mut gpui::ViewContext<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        div()
            .flex()
            .text_2xl()
            .relative()
            .child(
                div()
                    .w_1_2()
                    .h_24()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(self.equation.clone()),
            )
            .child(
                div()
                    .w_1_2()
                    .h_24()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(format!(
                        "{} {}",
                        self.result.as_ref().unwrap_or(&String::from("")),
                        self.unit.as_ref().unwrap_or(&String::from(""))
                    )),
            )
            .child(
                div()
                    .absolute()
                    .flex()
                    .items_center()
                    .justify_center()
                    .inset_0()
                    .text_color(theme.surface0)
                    .child("->"),
            )
    }
}
