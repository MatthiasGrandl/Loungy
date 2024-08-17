use crate::exports::loungy::command::command::{Callback, Guest, Metadata};
use crate::loungy::command::host;

wit_bindgen::generate!({
    path: "../../wit",
    world: "loungy",
});

struct Component;

impl Guest for Component {
    fn init() -> Metadata {
        Metadata {
            id: "root".to_string(),
            title: "Root".to_string(),
            subtitle: "Lists all apps and commands".to_string(),
            icon: "test".to_string(),
            keywords: vec![],
        }
    }
    fn run() {
        eprintln!("HELLO WORLD: {:?}", host::get_commands());
    }
    fn call(name: String, payload: Callback) {
        eprintln!("Callback: {:?} {:?}", name, payload);
    }
}

export!(Component);
