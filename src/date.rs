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

use std::sync::OnceLock;

use bonsaidb::core::num_traits::ToPrimitive;
use time::{format_description, OffsetDateTime};
use tz::TimeZone;

fn try_local_offset() -> anyhow::Result<i32> {
    Ok(TimeZone::local()?
        .find_current_local_time_type()?
        .ut_offset())
}

pub fn get_offset() -> &'static i32 {
    static OFFSET: OnceLock<i32> = OnceLock::new();
    OFFSET.get_or_init(|| try_local_offset().unwrap_or(0))
}

pub fn format_date(date: &OffsetDateTime) -> String {
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

    date.checked_add(time::Duration::seconds(get_offset().to_i64().unwrap()))
        .unwrap()
        .format(&format)
        .unwrap()
}
