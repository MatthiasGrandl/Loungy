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

use std::ops::Range;

use gpui::*;
use log::debug;

use crate::theme::Theme;

#[derive(IntoElement, Clone)]
pub struct TextInput {
    pub focus_handle: FocusHandle,
    pub view: View<TextView>,
}

impl TextInput {
    pub fn new(cx: &mut WindowContext) -> Self {
        let focus_handle = cx.focus_handle();
        let view = TextView::init(cx, &focus_handle);
        Self { focus_handle, view }
    }
    pub fn downgrade(&self) -> TextInputWeak {
        TextInputWeak {
            focus_handle: self.focus_handle.clone(),
            view: self.view.downgrade(),
        }
    }
}

#[derive(Clone)]
pub struct TextInputWeak {
    pub focus_handle: FocusHandle,
    pub view: WeakView<TextView>,
}

impl TextInputWeak {
    pub fn get_text(&self, cx: &WindowContext) -> String {
        if let Some(view) = self.view.upgrade() {
            return view.read(cx).text.clone();
        }
        "".to_string()
    }
    pub fn set_placeholder<C: VisualContext>(&self, placeholder: impl ToString, cx: &mut C) {
        if let Some(view) = self.view.upgrade() {
            cx.update_view(&view, |editor: &mut TextView, cx| {
                editor.placeholder = placeholder.to_string();
                cx.notify();
            });
        }
    }
    pub fn set_text<C: VisualContext>(&self, text: impl ToString, cx: &mut C) {
        if let Some(view) = self.view.upgrade() {
            cx.update_view(&view, |editor: &mut TextView, cx| {
                editor.set_text(text, cx);
            });
        }
    }
    pub fn set_masked<C: VisualContext>(&self, masked: bool, cx: &mut C) {
        if let Some(view) = self.view.upgrade() {
            cx.update_view(&view, |editor: &mut TextView, cx| {
                editor.set_masked(masked, cx);
            });
        }
    }
    pub fn has_focus(&self, cx: &WindowContext) -> bool {
        if let Some(fh) = cx.focused() {
            return fh.eq(&self.focus_handle);
        }
        false
    }
}

pub struct TextView {
    pub text: String,
    pub selection: Range<usize>,
    pub word_click: (usize, u16),
    pub placeholder: String,
    pub masked: bool,
}

impl TextView {
    pub fn init(cx: &mut WindowContext, focus_handle: &FocusHandle) -> View<Self> {
        let m = Self {
            text: "".to_string(),
            selection: 0..0,
            word_click: (0, 0),
            placeholder: "Type here...".to_string(),
            masked: false,
        };
        let view = cx.new_view(|cx| {
            #[cfg(debug_assertions)]
            cx.on_release(|_, _, _| debug!("Text Input released"))
                .detach();
            cx.on_blur(focus_handle, |_: &mut TextView, cx| {
                cx.emit(TextEvent::Blur);
            })
            .detach();
            cx.on_focus(focus_handle, |view, cx| {
                view.select_all(cx);
            })
            .detach();
            m
        });
        cx.subscribe(&view, |subscriber, emitter: &TextEvent, cx| {
            if let TextEvent::Input { text: _ } = emitter {
                subscriber.update(cx, |editor, _cx| {
                    editor.word_click = (0, 0);
                });
            }
        })
        .detach();
        view
    }
    pub fn set_text(&mut self, text: impl ToString, cx: &mut ViewContext<Self>) {
        self.text = text.to_string();
        self.selection = self.text.len()..self.text.len();
        cx.notify();
        cx.emit(TextEvent::Input {
            text: self.text.clone(),
        });
    }
    pub fn set_masked(&mut self, masked: bool, cx: &mut ViewContext<Self>) {
        self.masked = masked;
        cx.notify();
    }

    pub fn reset(&mut self, cx: &mut ViewContext<Self>) {
        self.text = "".to_string();
        self.selection = 0..0;
        cx.notify();
        cx.emit(TextEvent::Input {
            text: self.text.clone(),
        });
    }
    pub fn char_range_to_text_range(&self, text: &str) -> Range<usize> {
        let start = text
            .chars()
            .take(self.selection.start)
            .collect::<String>()
            .len();
        let end = text
            .chars()
            .take(self.selection.end)
            .collect::<String>()
            .len();
        start..end
    }
    pub fn select_all(&mut self, cx: &mut ViewContext<Self>) {
        self.selection = 0..self.text.chars().count();
        cx.notify();
    }
    pub fn word_ranges(&self) -> Vec<Range<usize>> {
        let mut words = Vec::new();
        let mut last_was_boundary = true;
        let mut word_start = 0;
        let s = self.text.clone();

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
}

pub enum TextEvent {
    Input { text: String },
    Blur,
    Back,
    KeyDown(KeyDownEvent),
}

impl EventEmitter<TextEvent> for TextView {}

impl RenderOnce for TextInput {
    fn render(self, cx: &mut WindowContext) -> impl IntoElement {
        cx.focus(&self.focus_handle);
        //let theme = cx.global::<Theme>();
        let clone = self.view.clone();
        div()
            .track_focus(&self.focus_handle)
            .on_key_down(move |ev, cx| {
                self.view.update(cx, |editor, cx| {
                    let prev = editor.text.clone();
                    cx.emit(TextEvent::KeyDown(ev.clone()));
                    let keystroke = &ev.keystroke.key;
                    let chars = editor.text.chars().collect::<Vec<char>>();
                    #[cfg(target_os = "macos")]
                    let m = ev.keystroke.modifiers.platform;
                    #[cfg(not(target_os = "macos"))]
                    let m = ev.keystroke.modifiers.control;

                    let ime_key = &ev.keystroke.ime_key;

                    if m {
                        match keystroke.as_str() {
                            "a" => {
                                editor.selection = 0..chars.len();
                            }
                            "c" => {
                                if !editor.masked {
                                    let selected_text =
                                        chars[editor.selection.clone()].iter().collect();
                                    cx.write_to_clipboard(ClipboardItem::new_string(selected_text));
                                }
                            }
                            "v" => {
                                let clipboard = cx.read_from_clipboard();
                                if let Some(clipboard) = clipboard {
                                    let Some(text) = clipboard.text() else {
                                        return;
                                    };
                                    editor.text.replace_range(
                                        editor.char_range_to_text_range(&editor.text),
                                        &text,
                                    );
                                    let i = editor.selection.start + text.chars().count();
                                    editor.selection = i..i;
                                }
                            }
                            "x" => {
                                let selected_text =
                                    chars[editor.selection.clone()].iter().collect();
                                cx.write_to_clipboard(ClipboardItem::new_string(selected_text));
                                editor.text.replace_range(
                                    editor.char_range_to_text_range(&editor.text),
                                    "",
                                );
                                editor.selection.end = editor.selection.start;
                            }
                            _ => {}
                        }
                    } else if let Some(ime_key) = ime_key {
                        editor
                            .text
                            .replace_range(editor.char_range_to_text_range(&editor.text), ime_key);
                        let i = editor.selection.start + ime_key.chars().count();
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
                                if editor.text.is_empty() && !ev.is_held {
                                    cx.emit(TextEvent::Back);
                                } else if editor.selection.start == editor.selection.end
                                    && editor.selection.start > 0
                                {
                                    let i = (editor.selection.start - 1).min(chars.len());
                                    editor.text = chars[0..i].iter().collect::<String>()
                                        + &(chars[editor.selection.end.min(chars.len())..]
                                            .iter()
                                            .collect::<String>());
                                    editor.selection = i..i;
                                } else {
                                    editor.text.replace_range(
                                        editor.char_range_to_text_range(&editor.text),
                                        "",
                                    );
                                    editor.selection.end = editor.selection.start;
                                }
                            }
                            "enter" => {
                                if ev.keystroke.modifiers.shift {
                                    editor.text.insert(
                                        editor.char_range_to_text_range(&editor.text).start,
                                        '\n',
                                    );
                                    let i = editor.selection.start + 1;
                                    editor.selection = i..i;
                                }
                            }
                            _ => {}
                        };
                    }
                    if prev != editor.text {
                        cx.emit(TextEvent::Input {
                            text: editor.text.clone(),
                        });
                    }
                    cx.notify();
                });
            })
            .rounded_xl()
            .p_2()
            //.border_2()
            //.border_color(transparent_black())
            //.focus(|style| style.border_color(theme.lavender))
            .child(clone)
    }
}

impl Render for TextView {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        let mut text = self.text.clone();
        let mut selection_style = HighlightStyle::default();
        let mut color = theme.lavender;
        color.fade_out(0.8);
        selection_style.background_color = Some(color);

        if self.masked {
            text = "•".repeat(text.len());
        }
        let mut highlights = vec![(self.char_range_to_text_range(&text), selection_style)];

        let mut style = TextStyle {
            color: theme.text,
            font_family: theme.font_sans.clone(),
            ..TextStyle::default()
        };
        if text.is_empty() {
            text = self.placeholder.to_string();
            style.color = theme.subtext0;
            highlights = vec![];
        }

        let styled_text = StyledText::new(text + " ").with_highlights(&style, highlights);
        let view = cx.view().clone();
        InteractiveText::new("text", styled_text).on_click(self.word_ranges(), move |ev, cx| {
            view.update(cx, |editor, cx| {
                let (index, mut count) = editor.word_click;
                if index == ev {
                    count += 1;
                } else {
                    count = 1;
                }
                match count {
                    2 => {
                        let word_ranges = editor.word_ranges();
                        editor.selection = word_ranges.get(ev).unwrap().clone();
                    }
                    3 => {
                        // Should select the line
                    }
                    4 => {
                        count = 0;
                        editor.selection = 0..editor.text.len();
                    }
                    _ => {}
                }
                editor.word_click = (ev, count);
                cx.notify();
            });
        })
    }
}
