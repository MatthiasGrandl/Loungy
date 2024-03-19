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

use std::cmp::Reverse;

use nucleo::pattern::{Atom, AtomKind, CaseMatching, Normalization};
use nucleo::{Config, Matcher, Utf32Str};

use crate::components::list::Item;
use crate::state::LazyMutex;

pub static MATCHER: LazyMutex<nucleo::Matcher> = LazyMutex::new(nucleo::Matcher::default);

pub fn fuzzy_match<T: Score>(pattern: &str, items: Vec<T>, path: bool) -> Vec<T> {
    let mut matcher = MATCHER.lock();
    matcher.config = Config::DEFAULT;
    if path {
        matcher.config.set_match_paths();
    }
    let pattern = Atom::new(
        pattern,
        CaseMatching::Smart,
        Normalization::Smart,
        AtomKind::Fuzzy,
        false,
    );
    let mut buf = Vec::new();
    let mut items: Vec<(T, u16)> = items
        .into_iter()
        .filter_map(|item| item.score(&pattern, &mut buf, &mut matcher))
        .collect();
    items.sort_by_key(|(_, score)| Reverse(*score));
    items.into_iter().map(|item| item.0).collect()
}

pub trait Score {
    fn score(
        &self, // Use a reference to self, to avoid moving `self`
        pattern: &Atom,
        buf: &mut Vec<char>,
        matcher: &mut Matcher,
    ) -> Option<(Self, u16)>
    where
        Self: Sized; // Use associated type Self
}

impl Score for Item {
    fn score(
        &self, // Use a reference to self, to avoid moving `self`
        pattern: &Atom,
        buf: &mut Vec<char>,
        matcher: &mut Matcher,
    ) -> Option<(Self, u16)>
    where
        Self: Sized,
    {
        let keywords = self.keywords.clone();
        let scores = keywords
            .into_iter()
            .map(|needle| pattern.score(Utf32Str::new(&needle, buf), matcher));

        // Consider using `filter_map` to avoid the inner `unwrap_or(None)`
        let highest = scores.flatten().max();

        let weight = self.weight.unwrap_or(1);
        highest.map(|score| ((*self).clone(), score * weight)) // Cloning self to avoid borrowing issues
    }
}
