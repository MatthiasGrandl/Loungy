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

use std::{collections::HashMap, sync::OnceLock, time::Duration};

use async_std::task::sleep;
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
    state::{Actions, CloneableFn, StateModel},
    window::Window,
};

pub struct HotkeyManager {
    manager: GlobalHotKeyManager,
    hotkeys: Vec<HotKey>,
    map: HashMap<u32, Box<dyn CloneableFn>>,
}

impl Global for HotkeyManager {}

fn db() -> &'static Database {
    static DB: OnceLock<Database> = OnceLock::new();
    DB.get_or_init(Db::init_collection::<CommandHotkeys>)
}

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

        cx.set_global::<HotkeyManager>(HotkeyManager {
            manager,
            hotkeys: vec![],
            map: HashMap::new(),
        });

        Self::update(cx);
        cx.spawn(|mut cx| async move {
            loop {
                if let Ok(event) = receiver.try_recv() {
                    if event.state == global_hotkey::HotKeyState::Released {
                        let _ = cx.update_global::<HotkeyManager, _>(|manager, cx| {
                            if let Some(action) = manager.map.get(&event.id) {
                                StateModel::update(|this, cx| this.reset(cx), cx);
                                action(&mut Actions::default(cx), cx);
                            }
                            Window::open(cx);
                        });
                    }
                }
                sleep(Duration::from_millis(50)).await;
                // cx.background_executor()
                //     .timer(Duration::from_millis(50))
                //     .await;
            }
        })
        .detach();
    }
    pub fn update(cx: &mut WindowContext) {
        cx.update_global::<HotkeyManager, _>(|manager, cx| {
            let commands = cx.global::<RootCommands>();
            let hotkeys = CommandHotkeys::all(db()).query().unwrap_or_default();
            let _ = manager.manager.unregister_all(&manager.hotkeys);
            manager.hotkeys.clear();
            for hotkey in hotkeys {
                let hotkey = hotkey.contents;
                let known = commands.commands.get(hotkey.id.as_str());
                if let Some(known) = known {
                    let hotkey = HotKey::try_from(hotkey.hotkey).unwrap();

                    manager.hotkeys.push(hotkey);
                    manager.map.insert(hotkey.id(), known.action.clone());
                }
            }
            let _ = manager.manager.register_all(&manager.hotkeys);
        });
    }
    pub fn set(id: &str, keystroke: Keystroke, cx: &mut WindowContext) -> anyhow::Result<()> {
        // This is annoying and will break for most hotkeys
        let mut tokens = Vec::<&str>::new();
        if keystroke.modifiers.alt {
            tokens.push("alt");
        }
        if keystroke.modifiers.command {
            tokens.push("command");
        }
        if keystroke.modifiers.control {
            tokens.push("control");
        }
        if keystroke.modifiers.shift
            || (keystroke.key.len() == 1
                && keystroke.key.chars().next().unwrap().is_ascii_uppercase())
        {
            tokens.push("shift");
        }
        tokens.push(keystroke.key.as_str());
        let hotkey = tokens.join("+");

        HotKey::try_from(hotkey.clone())?;

        CommandHotkeys {
            id: id.to_string(),
            hotkey,
        }
        .overwrite_into(&id.to_string(), db())?;
        Self::update(cx);
        Ok(())
    }
    pub fn unset(id: &str, cx: &mut WindowContext) -> anyhow::Result<()> {
        if let Some(hk) = CommandHotkeys::get(&id.to_string(), db())? {
            hk.delete(db())?;
        }
        Self::update(cx);
        Ok(())
    }
    pub fn get(id: &str) -> Option<Keystroke> {
        CommandHotkeys::get(&id.to_string(), db()).ok()?.map(|hk| {
            hk.contents
                .hotkey
                .split('+')
                .fold(Keystroke::default(), |mut keystroke, token| {
                    match token {
                        "alt" => keystroke.modifiers.alt = true,
                        "command" => keystroke.modifiers.command = true,
                        "control" => keystroke.modifiers.control = true,
                        "shift" => keystroke.modifiers.shift = true,
                        _ => keystroke.key = token.to_string(),
                    }
                    keystroke
                })
        })
    }
}

#[derive(Serialize, Deserialize, Collection, Debug)]
#[collection(name = "command-hotkeys")]
pub struct CommandHotkeys {
    #[natural_id]
    id: String,
    hotkey: String,
}
