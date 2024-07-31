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
use jiff::{fmt::strtime, tz::TimeZone, Timestamp, ToSpan};

pub fn format_date(date: Timestamp, _cx: &AppContext) -> String {
    let tz = TimeZone::system();
    let zoned = date.to_zoned(tz.clone());
    let zoned_now = Timestamp::now().to_zoned(tz.clone());
    let prefix = if zoned_now.day().eq(&zoned.day()) {
        "Today"
    } else if zoned_now
        .day()
        .eq(&zoned.checked_sub(ToSpan::day(1)).unwrap().day())
    {
        "Yesterday"
    } else {
        "%d. %b %Y"
    };
    let format = format!("{}, %H:%M:%S", prefix);

    strtime::format(format, zoned.datetime()).unwrap()
}
