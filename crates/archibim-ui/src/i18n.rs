//! Language packs: every word on screen comes from here (TOOLS.md, rule 14).

use std::{collections::HashMap, sync::OnceLock};

type Pack = HashMap<&'static str, &'static str>;

const EN: &str = include_str!("../lang/en.txt");
const FR: &str = include_str!("../lang/fr.txt");

fn parse(text: &'static str) -> Pack {
    text.lines()
        .filter(|l| !l.trim_start().starts_with('#'))
        .filter_map(|l| l.split_once('='))
        .map(|(k, v)| (k.trim(), v.trim()))
        .collect()
}

/// The text for `key` in the user's language, set by `ARCHIBIM_LANG` (`en`
/// or `fr`, French by default); English when the pack lacks it, the key
/// itself as a last resort.
// ponytail: two packs built in; load packs from files when a third language
// comes.
pub fn t(key: &'static str) -> &'static str {
    static PACKS: OnceLock<(Pack, Pack)> = OnceLock::new();
    let (chosen, en) = PACKS.get_or_init(|| {
        let lang = std::env::var("ARCHIBIM_LANG").unwrap_or_default();
        let chosen = if lang == "en" { EN } else { FR };
        (parse(chosen), parse(EN))
    });
    chosen
        .get(key)
        .or_else(|| en.get(key))
        .copied()
        .unwrap_or(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_pack_has_the_same_keys() {
        let (en, fr) = (parse(EN), parse(FR));
        let mut a: Vec<_> = en.keys().collect();
        let mut b: Vec<_> = fr.keys().collect();
        a.sort();
        b.sort();
        assert_eq!(a, b);
    }
}
