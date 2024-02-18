use std::{collections::HashMap, str::FromStr, time::Duration};

use bonsaidb::{
    core::schema::{Collection, SerializedCollection},
    local::Database,
};
use global_hotkey::{
    hotkey::{Code, HotKey, Modifiers},
    GlobalHotKeyEvent, GlobalHotKeyManager,
};
use gpui::*;
use serde::{Deserialize, Serialize};

use crate::{
    commands::RootCommands,
    db::Db,
    state::{Actions, CloneableFn},
    window::Window,
};

pub struct HotkeyManager {
    manager: GlobalHotKeyManager,
    hotkeys: Vec<HotKey>,
    map: HashMap<u32, Box<dyn CloneableFn>>,
    db: Database,
}

impl Global for HotkeyManager {}

impl HotkeyManager {
    pub fn init(cx: &mut WindowContext) {
        let manager = GlobalHotKeyManager::new().unwrap();
        let receiver = GlobalHotKeyEvent::receiver().clone();
        // Fallback hotkey
        let mut mods = Modifiers::empty();

        mods.set(Modifiers::CONTROL, true);
        mods.set(Modifiers::ALT, true);
        mods.set(Modifiers::META, true);
        let hotkey = HotKey::new(Some(mods), Code::Space);

        manager.register(hotkey).unwrap();
        Db::new::<CommandHotkeys, HotkeyManager>(
            move |db| HotkeyManager {
                manager,
                hotkeys: vec![],
                map: HashMap::new(),
                db,
            },
            cx,
        );
        Self::update(cx);
        cx.spawn(|mut cx| async move {
            loop {
                if let Ok(event) = receiver.try_recv() {
                    if event.state == global_hotkey::HotKeyState::Released {
                        let _ = cx.update_global::<HotkeyManager, _>(|manager, cx| {
                            if let Some(action) = manager.map.get(&event.id) {
                                action(&mut Actions::default(cx), cx);
                            }
                            Window::open(cx);
                        });
                    }
                }
                cx.background_executor()
                    .timer(Duration::from_millis(50))
                    .await;
            }
        })
        .detach();
    }
    pub fn update(cx: &mut WindowContext) {
        cx.update_global::<HotkeyManager, _>(|manager, cx| {
            let commands = cx.global::<RootCommands>();
            let hotkeys = CommandHotkeys::all(&manager.db).query().unwrap_or_default();
            let _ = manager.manager.unregister_all(&manager.hotkeys);
            manager.hotkeys.clear();
            for hotkey in hotkeys {
                let hotkey = hotkey.contents;
                let known = commands.commands.get(hotkey.id.as_str());
                if let Some(known) = known {
                    let hotkey = HotKey::new(Some(hotkey.mods), hotkey.code);

                    manager.hotkeys.push(hotkey);
                    manager.map.insert(hotkey.id(), known.action.clone());
                }
            }
            let _ = manager.manager.register_all(&manager.hotkeys);
        });
    }
    pub fn set(id: &str, keystroke: Keystroke, cx: &mut WindowContext) -> anyhow::Result<()> {
        // This is annoying and will break for most hotkeys
        let mut mods = Modifiers::default();
        mods.set(Modifiers::ALT, keystroke.modifiers.alt);
        mods.set(Modifiers::META, keystroke.modifiers.command);
        mods.set(Modifiers::CONTROL, keystroke.modifiers.control);
        mods.set(Modifiers::SHIFT, keystroke.modifiers.shift);
        // if key is lowercase letter
        eprintln!("{:?}", keystroke.key);
        let code = {
            if keystroke.key.len() == 1 {
                let char = keystroke.key.chars().next().unwrap();
                if char.is_ascii_lowercase() {
                    Code::from_str(format!("Key{}", char.to_uppercase()).as_str())
                } else if char.is_ascii_uppercase() {
                    mods.set(Modifiers::SHIFT, true);
                    Code::from_str(format!("Key{}", char).as_str())
                } else if char.is_numeric() {
                    Code::from_str(format!("Digit{}", char).as_str())
                } else {
                    Code::from_str(format!("{}", char).as_str())
                }
            } else {
                let capitalized = keystroke.key[0..1].to_uppercase() + &keystroke.key[1..];
                Code::from_str(capitalized.as_str())
            }
        }?;

        CommandHotkeys {
            id: id.to_string(),
            mods,
            code,
        }
        .push_into(&cx.global::<HotkeyManager>().db)?;
        Self::update(cx);
        Ok(())
    }
    pub fn unset(id: &str, cx: &mut WindowContext) -> anyhow::Result<()> {
        let db = cx.global::<HotkeyManager>().db.clone();
        if let Some(hk) = CommandHotkeys::get(&id.to_string(), &db).unwrap() {
            hk.delete(&db)?;
        }
        Self::update(cx);
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Collection, Debug)]
#[collection(name = "command-hotkeys")]
pub struct CommandHotkeys {
    #[natural_id]
    id: String,
    mods: Modifiers,
    code: Code,
}
