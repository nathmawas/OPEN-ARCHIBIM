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
    label(key)
}

/// A name kept in the project: translated when it is a language pack key
/// (the names of the starting library), shown as it is when the user typed
/// it.
pub fn label(name: &str) -> &str {
    static PACKS: OnceLock<(Pack, Pack)> = OnceLock::new();
    let (chosen, en) = PACKS.get_or_init(|| {
        let lang = std::env::var("ARCHIBIM_LANG").unwrap_or_default();
        let chosen = if lang == "en" { EN } else { FR };
        (parse(chosen), parse(EN))
    });
    chosen
        .get(name)
        .or_else(|| en.get(name))
        .copied()
        .unwrap_or(name)
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

    fn keys(text: &str) -> Vec<&str> {
        text.lines()
            .filter(|l| !l.trim_start().starts_with('#'))
            .filter_map(|l| l.split_once('='))
            .map(|(k, _)| k.trim())
            .collect()
    }

    #[test]
    fn no_pack_repeats_a_key() {
        for text in [EN, FR] {
            let mut k = keys(text);
            let n = k.len();
            k.sort();
            k.dedup();
            assert_eq!(k.len(), n, "a key is repeated");
        }
    }

    #[test]
    fn every_library_material_and_family_has_a_name_in_both_packs() {
        let (en, fr) = (parse(EN), parse(FR));
        let mut names: Vec<&str> = archibim_model::STARTING_LIBRARY
            .iter()
            .map(|m| m.0)
            .collect();
        names.extend([
            "family.masonry",
            "family.insulation",
            "family.timber_frame",
            "family.metal_frame",
            "family.air",
            "family.membrane",
            "family.facade_finish",
            "family.cladding",
            "family.interior_lining",
            "family.roof_covering",
            "family.other",
        ]);
        for name in names {
            assert!(en.get(name).is_some_and(|v| !v.is_empty()), "en {name}");
            assert!(fr.get(name).is_some_and(|v| !v.is_empty()), "fr {name}");
        }
    }
}
