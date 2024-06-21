use async_std::{io::ReadExt, io::WriteExt, os::unix::net::UnixStream};
use clap::Command;

use crate::commands::RootCommands;

use super::{
    server::{get_command, CommandPayload, TopLevelCommand},
    SOCKET_PATH,
};

pub async fn client_connect() -> anyhow::Result<()> {
    let mut stream = UnixStream::connect(SOCKET_PATH).await?;

    let mut buf = vec![0; 8096];
    let n = stream.read(&mut buf).await?;
    let root_commands: RootCommands = serde_json::from_slice(&buf[..n])?;

    let command: Command = get_command(&root_commands);

    let matches = command.get_matches();

    let payload: CommandPayload = CommandPayload {
        action: matches
            .get_one::<TopLevelCommand>("Action")
            .ok_or(anyhow::anyhow!("Action not found"))?
            .clone(),
        command: matches.get_one::<String>("Command").cloned(),
    };

    let bytes = serde_json::to_vec(&payload)?;

    stream.write_all(&bytes).await?;

    Ok(())
}
