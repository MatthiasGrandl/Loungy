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

use std::str::FromStr;

use matrix_sdk::ruma::OwnedMxcUri;
use url::Url;

pub(super) fn mxc_to_http(
    server: Url,
    url: OwnedMxcUri,
    thumb: bool,
) -> Result<Url, anyhow::Error> {
    let (server_name, media_id) = url.parts()?;
    let (t, q) = if thumb {
        ("thumbnail", "?width=50&height=50&method=crop")
    } else {
        ("download", "")
    };
    let path = format!(
        "{}_matrix/media/v3/{}/{}/{}{}",
        server, t, server_name, media_id, q
    );
    Ok(Url::from_str(&path)?)
}
