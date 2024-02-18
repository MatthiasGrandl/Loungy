use std::{collections::HashMap, time::Duration};

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
}

#[derive(Serialize, Deserialize, Collection)]
#[collection(name = "command-hotkeys")]
pub struct CommandHotkeys {
    #[natural_id]
    id: String,
    mods: Modifiers,
    code: Code,
}
