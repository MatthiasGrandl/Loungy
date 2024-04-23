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

use gpui::AppContext;
use time::{format_description, OffsetDateTime};

pub fn format_date(date: &OffsetDateTime, cx: &AppContext) -> String {
    let prefix = if date.day() == OffsetDateTime::now_utc().day() {
        "Today"
    } else if date.day()
        == OffsetDateTime::now_utc()
            .saturating_sub(time::Duration::days(1))
            .day()
    {
        "Yesterday"
    } else {
        "[day]. [month repr:short] [year]"
    };
    let format = format!("{}, [hour]:[minute]:[second]", prefix);
    let format = format_description::parse(&format).unwrap();

    date.checked_add(time::Duration::seconds(
        cx.local_timezone().whole_seconds() as i64
    ))
    .unwrap()
    .format(&format)
    .unwrap()
}
