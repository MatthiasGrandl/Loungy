use std::ops::Range;

use gpui::*;

use crate::theme::Theme;

#[derive(IntoElement, Clone)]
pub struct TextInput {
    focus_handle: FocusHandle,
    view: View<TextDisplay>,
    pub model: Model<TextModel>,
}

impl TextInput {
    pub fn new(cx: &mut WindowContext, initial_text: String) -> Self {
        let model = TextModel::init(initial_text.clone(), cx);
        let clone = model.clone();
        let view = cx.new_view(move |cx| {
            let view = TextDisplay {
                model: clone.clone(),
            };
            cx.subscribe(&clone, |_subscriber, _emitter, event, cx| match event {
                TextEvent::Input { text: _ } => {
                    cx.notify();
                }
                _ => {}
            })
            .detach();
            view
        });
        Self {
            focus_handle: cx.focus_handle(),
            view,
            model,
        }
    }
}

pub struct TextModel {
    pub text: String,
    pub selection: Range<usize>,
    pub word_click: (usize, u16),
    pub placeholder: String,
}

impl TextModel {
    pub fn init(text: String, cx: &mut WindowContext) -> Model<Self> {
        let i = text.len();
        let m = Self {
            text,
            selection: i..i,
            word_click: (0, 0),
            placeholder: "Type here...".to_string(),
        };
        let model = cx.new_model(|_cx| m);
        cx.subscribe(
            &model,
            |subscriber, emitter: &TextEvent, cx| match emitter {
                TextEvent::Input { text: _ } => {
                    subscriber.update(cx, |editor, _cx| {
                        editor.word_click = (0, 0);
                    });
                }
                _ => {}
            },
        )
        .detach();
        model
    }
    pub fn reset(&mut self, cx: &mut ModelContext<Self>) {
        self.text = "".to_string();
        self.selection = 0..0;
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
    Movement(TextMovement),
    Submit {},
}
pub enum TextMovement {
    Up,
    Down,
}

impl EventEmitter<TextEvent> for TextModel {}

impl RenderOnce for TextInput {
    fn render(self, cx: &mut WindowContext) -> impl IntoElement {
        cx.focus(&self.focus_handle);

        let theme = cx.global::<Theme>();

        div()
            .track_focus(&self.focus_handle)
            .on_key_down(move |ev, cx| {
                self.model.update(cx, |editor, cx| {
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
                            "up" => {
                                cx.emit(TextEvent::Movement(TextMovement::Up));
                                return;
                            }
                            "down" => {
                                cx.emit(TextEvent::Movement(TextMovement::Down));
                                return;
                            }
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
                                } else {
                                    cx.emit(TextEvent::Submit {});
                                    return;
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
            .child(self.view)
    }
}

pub struct TextDisplay {
    model: Model<TextModel>,
}

impl Render for TextDisplay {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        let mut text = self.model.read(cx).text.clone();
        let mut selection_style = HighlightStyle::default();
        let mut color = theme.lavender;
        color.fade_out(0.8);
        selection_style.background_color = Some(color);

        let sel = self.model.read(cx).selection.clone();
        let mut highlights = vec![(sel, selection_style)];

        let mut style = TextStyle::default();
        style.color = theme.text;
        if text.len() == 0 {
            text = self.model.read(cx).placeholder.to_string();
            style.color = theme.subtext0;
            highlights = vec![];
        }

        let styled_text = StyledText::new(text + " ").with_highlights(&style, highlights);
        let clone = self.model.clone();
        InteractiveText::new("text", styled_text).on_click(
            self.model.read(cx).word_ranges(),
            move |ev, cx| {
                clone.update(cx, |editor, cx| {
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
            },
        )
    }
}
