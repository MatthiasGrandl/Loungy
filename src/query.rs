use std::{
    ops::Range,
    sync::{Arc, Mutex},
};

use gpui::*;

use crate::{
    keymap::query::{Input, MoveDown, MoveUp},
    theme::Theme,
};

#[derive(IntoElement, Clone)]
pub struct TextInput {
    pub text_display_view: View<TextDisplay>,
    focus_handle: FocusHandle,
}

impl TextInput {
    pub fn new(cx: &mut WindowContext, initial_text: String) -> Self {
        cx.set_global(Query {
            inner: String::from(initial_text.clone()),
        });
        let i = initial_text.len();
        Self {
            text_display_view: cx.new_view(|_cx| TextDisplay {
                text: initial_text,
                selection: i..i,
                word_click: Arc::new(Mutex::new((0, 0))),
            }),
            focus_handle: cx.focus_handle(),
        }
    }
}

pub struct Query {
    pub inner: String,
}

impl Global for Query {}

impl RenderOnce for TextInput {
    fn render(self, cx: &mut WindowContext) -> impl IntoElement {
        cx.focus(&self.focus_handle);

        let theme = cx.global::<Theme>();

        let text_display_view = self.text_display_view.clone();

        div()
            .key_context("query")
            .track_focus(&self.focus_handle)
            .on_key_down(move |ev, cx| {
                //eprintln!("Key down: {:?}", ev);
                text_display_view.update(cx, |editor, cx| {
                    let keystroke = &ev.keystroke.key;
                    if ev.keystroke.modifiers.command {
                        match keystroke.as_str() {
                            "a" => {
                                editor.selection = 0..editor.text.len();
                            }
                            "c" => {
                                let selected_text =
                                    editor.text[editor.selection.clone()].to_string();
                                cx.write_to_clipboard(ClipboardItem::new(selected_text));
                            }
                            "v" => {
                                let clipboard = cx.read_from_clipboard();
                                if let Some(clipboard) = clipboard {
                                    let text = clipboard.text();
                                    editor.text.replace_range(editor.selection.clone(), &text);
                                    let i = editor.selection.start + text.len();
                                    editor.selection = i..i;
                                }
                            }
                            "x" => {
                                let selected_text =
                                    editor.text[editor.selection.clone()].to_string();
                                cx.write_to_clipboard(ClipboardItem::new(selected_text));
                                editor.text.replace_range(editor.selection.clone(), "");
                                editor.selection.end = editor.selection.start;
                            }
                            _ => {}
                        }
                    } else if let Some(ime_key) = &ev.keystroke.ime_key {
                        editor.text.replace_range(editor.selection.clone(), ime_key);
                        let i = editor.selection.start + ime_key.len();
                        editor.selection = i..i;
                    } else {
                        match keystroke.as_str() {
                            "left" => {
                                if editor.selection.start > 0 {
                                    let i = if editor.selection.start == editor.selection.end {
                                        editor.selection.start - 1
                                    } else {
                                        editor.selection.start
                                    };
                                    editor.selection = i..i;
                                }
                            }
                            "right" => {
                                if editor.selection.end < editor.text.len() {
                                    let i = if editor.selection.start == editor.selection.end {
                                        editor.selection.end + 1
                                    } else {
                                        editor.selection.end
                                    };
                                    editor.selection = i..i;
                                }
                            }
                            "backspace" => {
                                if editor.selection.start == editor.selection.end
                                    && editor.selection.start > 0
                                {
                                    let mut start = editor.text[..editor.selection.start].chars();
                                    start.next_back();
                                    let start = start.as_str();
                                    let i = start.len();
                                    editor.text =
                                        start.to_owned() + &editor.text[editor.selection.end..];
                                    editor.selection = i..i;
                                } else {
                                    editor.text.replace_range(editor.selection.clone(), "");
                                    editor.selection.end = editor.selection.start;
                                }
                            }
                            "enter" => {
                                editor.text.insert(editor.selection.start, '\n');
                                let i = editor.selection.start + 1;
                                editor.selection = i..i;
                            }
                            "escape" => {
                                cx.blur();
                                cx.hide();
                            }
                            keystroke_str => {
                                //eprintln!("Unhandled keystroke {keystroke_str}")
                            }
                        };
                    }
                    cx.set_global(Query {
                        inner: editor.text.clone(),
                    });
                    cx.dispatch_action(Box::new(Input));
                    cx.notify();
                });
            })
            .p_4()
            .w_full()
            .border_b_1()
            .border_color(theme.mantle)
            .text_color(theme.text)
            .focus(|style| style.border_color(theme.lavender))
            .child(self.text_display_view)
    }
}

#[derive(Clone)]
pub struct TextDisplay {
    // TODO: Use Arc<String>? Other places we can reduce clones?
    pub text: String,
    pub selection: Range<usize>,
    pub word_click: Arc<Mutex<(usize, u16)>>,
}

fn split_into_words(s: &str) -> Vec<Range<usize>> {
    let mut words = Vec::new();
    let mut last_was_boundary = true;
    let mut word_start = 0;

    for (i, c) in s.char_indices() {
        if c.is_alphanumeric() || c == '_' {
            if last_was_boundary {
                word_start = i;
            }
            last_was_boundary = false;
        } else {
            if !last_was_boundary {
                words.push(word_start..i);
            }
            last_was_boundary = true;
        }
    }

    // Check if the last characters form a word and push it if so
    if !last_was_boundary {
        words.push(word_start..s.len());
    }

    words
}

impl Render for TextDisplay {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let mut text = self.text.clone();
        let mut selection_style = HighlightStyle::default();
        let mut color = theme.lavender;
        color.fade_out(0.8);
        selection_style.background_color = Some(color);

        let word_ranges = split_into_words(text.as_str());
        let word_ranges_clone = word_ranges.clone();

        let sel = self.selection.clone();
        let mut highlights = vec![(sel, selection_style)];

        let mut style = TextStyle::default();
        style.color = theme.text;
        if text.len() == 0 {
            text = "Type here...".to_string();
            style.color = theme.subtext0;
            highlights = vec![];
        }

        let styled_text = StyledText::new(text + " ").with_highlights(&style, highlights);
        let view = cx.view().clone();
        let clicked = self.word_click.clone();

        InteractiveText::new("text", styled_text).on_click(word_ranges, move |ev, cx| {
            let mut c = clicked.lock().unwrap();
            if c.0 == ev {
                *c = (ev, c.1 + 1);
            } else {
                *c = (ev, 1);
            }

            match c.1 {
                2 => {
                    cx.update_view(&view, |editor, cx| {
                        editor.selection = word_ranges_clone[ev].clone();
                        cx.notify();
                    });
                }
                3 => {
                    // Should select the line
                }
                4 => {
                    *c = (0, 0);
                    cx.update_view(&view, |editor, cx| {
                        editor.selection = 0..editor.text.len();
                        cx.notify();
                    });
                }
                _ => {}
            }
        })
    }
}
