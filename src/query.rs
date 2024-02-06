use std::ops::Range;

use gpui::*;

use crate::{state::ActionsModel, theme::Theme};

#[derive(IntoElement, Clone)]
pub struct TextInput {
    focus_handle: FocusHandle,
    pub view: View<TextView>,
    actions: ActionsModel,
}

impl TextInput {
    pub fn new(actions: &ActionsModel, cx: &mut WindowContext) -> Self {
        let focus_handle = cx.focus_handle();
        let view = TextView::init(cx, &focus_handle);
        Self {
            focus_handle,
            view,
            actions: actions.clone(),
        }
    }
    pub fn set_placeholder(&self, placeholder: impl ToString, cx: &mut WindowContext) {
        self.view.update(cx, |editor, cx| {
            editor.placeholder = placeholder.to_string();
            cx.notify();
        });
    }
}

pub struct TextView {
    pub text: String,
    pub selection: Range<usize>,
    pub word_click: (usize, u16),
    pub placeholder: String,
}

impl TextView {
    pub fn init(cx: &mut WindowContext, focus_handle: &FocusHandle) -> View<Self> {
        let m = Self {
            text: "".to_string(),
            selection: 0..0,
            word_click: (0, 0),
            placeholder: "Type here...".to_string(),
        };
        let view = cx.new_view(|cx| {
            cx.on_blur(focus_handle, |_, cx| {
                cx.emit(TextEvent::Blur);
            })
            .detach();
            m
        });
        cx.subscribe(&view, |subscriber, emitter: &TextEvent, cx| match emitter {
            TextEvent::Input { text: _ } => {
                subscriber.update(cx, |editor, _cx| {
                    editor.word_click = (0, 0);
                });
            }
            _ => {}
        })
        .detach();
        view
    }
    pub fn reset(&mut self, cx: &mut ViewContext<Self>) {
        self.text = "".to_string();
        self.selection = 0..0;
        cx.notify();
        cx.emit(TextEvent::Input {
            text: self.text.clone(),
        });
    }
    pub fn select_all(&mut self, cx: &mut ViewContext<Self>) {
        self.selection = 0..self.text.len();
        cx.notify();
        cx.emit(TextEvent::Input {
            text: self.text.clone(),
        });
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
    KeyDown(KeyDownEvent),
}
pub enum TextMovement {
    Up,
    Down,
}

impl EventEmitter<TextEvent> for TextView {}

impl RenderOnce for TextInput {
    fn render(self, cx: &mut WindowContext) -> impl IntoElement {
        cx.focus(&self.focus_handle);
        let clone = self.view.clone();
        div()
            .track_focus(&self.focus_handle)
            .on_key_down(move |ev, cx| {
                if let Some(action) = self.actions.check(&ev.keystroke, cx) {
                    if ev.is_held {
                        return;
                    }
                    (action.action)(cx);
                    return;
                };

                self.view.update(cx, |editor, cx| {
                    cx.emit(TextEvent::KeyDown(ev.clone()));
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
                                if ev.keystroke.modifiers.shift {
                                    editor.text.insert(editor.selection.start, '\n');
                                    let i = editor.selection.start + 1;
                                    editor.selection = i..i;
                                }
                            }
                            "escape" => {
                                cx.hide();
                            }
                            keystroke_str => {
                                eprintln!("Unhandled keystroke {keystroke_str}")
                            }
                        };
                    }
                    cx.emit(TextEvent::Input {
                        text: editor.text.clone(),
                    });
                });
            })
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

        let sel = self.selection.clone();
        let mut highlights = vec![(sel, selection_style)];

        let mut style = TextStyle::default();
        style.color = theme.text;
        if text.len() == 0 {
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
