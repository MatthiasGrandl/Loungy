use std::path::Path;

use anyhow::anyhow;
use async_std::{
    io::{ReadExt, WriteExt},
    os::unix::net::{UnixListener, UnixStream},
};
use clap::{command, Arg, ValueEnum};
use gpui::AsyncWindowContext;
use serde::{Deserialize, Serialize};

use crate::{
    commands::RootCommands,
    state::{Actions, StateModel},
    window::Window,
};

use super::SOCKET_PATH;

pub async fn setup_socket() -> anyhow::Result<UnixListener> {
    if Path::new(SOCKET_PATH).exists() {
        if UnixStream::connect(SOCKET_PATH).await.is_ok() {
            return Err(anyhow!("Server already running"));
        }
        std::fs::remove_file(SOCKET_PATH)?;
    };
    let listener = UnixListener::bind(SOCKET_PATH).await?;
    log::info!("Listening on socket: {}", SOCKET_PATH);

    Ok(listener)
}

pub async fn start_server(
    listener: UnixListener,
    mut cx: AsyncWindowContext,
) -> anyhow::Result<()> {
    let commands = cx.read_global::<RootCommands, _>(|commands, _| commands.clone())?;
    loop {
        let (stream, _) = listener.accept().await?;
        cx.spawn(|cx| handle_client(stream, commands.clone(), cx))
            .detach();
    }
}

async fn handle_client(
    mut stream: UnixStream,
    commands: RootCommands,
    mut cx: AsyncWindowContext,
) -> anyhow::Result<()> {
    // Send available commands to the client
    let bytes = serde_json::to_vec(&commands)?;
    stream.write_all(&bytes).await?;

    let mut buf = vec![0; 1024];
    let n = stream.read(&mut buf).await?;

    let matches: CommandPayload = serde_json::from_slice(&buf[..n])?;

    let _ = cx.update::<anyhow::Result<()>>(|cx| {
        match matches.action {
            TopLevelCommand::Toggle => {
                Window::toggle(cx);
            }
            TopLevelCommand::Show => {
                Window::open(cx);
            }
            TopLevelCommand::Hide => {
                Window::close(cx);
            }
            TopLevelCommand::Quit => {
                cx.quit();
            }
            TopLevelCommand::Command => {
                let Some(c) = matches.command else {
                    return Err(anyhow!("No command provided"));
                };
                let Some((_, command)) = commands.commands.iter().find(|(k, _)| {
                    let split = k.split("::").collect::<Vec<_>>();
                    c.eq(split[2])
                }) else {
                    return Err(anyhow!("Command not found"));
                };

                let state = cx.global::<StateModel>();
                let state = state.inner.read(cx);
                let mut is_active = false;
                if let Some(active) = state.stack.last() {
                    is_active = active.id.eq(&command.id);
                };
                if !is_active {
                    StateModel::update(
                        |this, cx| {
                            this.reset(cx);
                        },
                        cx,
                    );
                    (command.action)(&mut Actions::default(cx), cx);
                    Window::open(cx);
                } else {
                    Window::toggle(cx);
                }
            }
            TopLevelCommand::Pipe => {}
        }
        Ok(())
    });
    Ok(())
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CommandPayload {
    pub action: TopLevelCommand,
    pub command: Option<String>,
}

#[derive(Clone, Debug, ValueEnum, Serialize, Deserialize)]
pub enum TopLevelCommand {
    Toggle,
    Show,
    Hide,
    Quit,
    Command,
    Pipe,
}

impl From<TopLevelCommand> for clap::builder::OsStr {
    fn from(cmd: TopLevelCommand) -> Self {
        match cmd {
            TopLevelCommand::Toggle => "toggle".into(),
            TopLevelCommand::Show => "show".into(),
            TopLevelCommand::Hide => "hide".into(),
            TopLevelCommand::Quit => "quit".into(),
            TopLevelCommand::Command => "command".into(),
            TopLevelCommand::Pipe => "pipe".into(),
        }
    }
}

pub fn get_command(commands: &RootCommands) -> clap::Command {
    command!()
        .arg(
            Arg::new("Action")
                .value_parser(clap::builder::EnumValueParser::<TopLevelCommand>::new())
                .required(true),
        )
        .arg(
            Arg::new("Command")
                .required_if_eq("Action", TopLevelCommand::Command)
                .value_parser(
                    commands
                        .commands
                        .keys()
                        .map(|key| {
                            let split = key.split("::").collect::<Vec<_>>();
                            split[2].to_string()
                        })
                        .collect::<Vec<_>>(),
                ),
        )
        .arg(
            Arg::new("Delimeter")
                .long("Delimeter")
                .short('d')
                .required_if_eq("Action", TopLevelCommand::Pipe)
                .default_value(" "),
        )
}
