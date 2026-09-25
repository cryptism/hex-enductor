//! Regenerates crates/map-core/src/link_icons.rs from game-icons.net's
//! source SVGs (github.com/game-icons/icons, CC BY 3.0 — see CREDITS.md)
//! — port of scripts/sync-link-icons.ts. Only the `LINK_ICONS` array in
//! that file is generated; the types, lookup and tests around it are
//! hand-written and left alone.

use std::fs;
use std::path::Path;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum Category {
    Settlement,
    Landmark,
    Ruin,
    Hazard,
    Waypoint,
}

#[derive(Clone, Copy, Debug)]
enum Author {
    Delapouite,
    Lorc,
}

use Author::*;
use Category::*;

struct IconSource {
    slug: &'static str,
    label: &'static str,
    category: Category,
    author: Author,
}

const fn src(
    slug: &'static str,
    label: &'static str,
    category: Category,
    author: Author,
) -> IconSource {
    IconSource {
        slug,
        label,
        category,
        author,
    }
}

/// To add an icon: add it here, then `cargo run -p hexen-cli -- sync-link-icons`.
const ICON_SOURCES: &[IconSource] = &[
    src("village", "Village", Settlement, Delapouite),
    src("medieval-gate", "Gate", Settlement, Delapouite),
    src("hill-fort", "Hill fort", Settlement, Delapouite),
    src("watchtower", "Watchtower", Settlement, Delapouite),
    src("windmill", "Windmill", Settlement, Delapouite),
    src("well", "Well", Settlement, Delapouite),
    src("tavern-sign", "Tavern", Settlement, Delapouite),
    src("church", "Church", Landmark, Delapouite),
    src("greek-temple", "Temple", Landmark, Delapouite),
    src("castle", "Castle", Landmark, Lorc),
    src("lighthouse", "Lighthouse", Landmark, Delapouite),
    src("obelisk", "Obelisk", Landmark, Delapouite),
    src("crystal-shrine", "Shrine", Landmark, Delapouite),
    src("oasis", "Oasis", Landmark, Delapouite),
    src("waterfall", "Waterfall", Landmark, Delapouite),
    src("castle-ruins", "Castle ruins", Ruin, Delapouite),
    src("ancient-ruins", "Ancient ruins", Ruin, Delapouite),
    src("crypt-entrance", "Crypt entrance", Ruin, Delapouite),
    src("cave-entrance", "Cave entrance", Ruin, Delapouite),
    src("dungeon-gate", "Dungeon gate", Ruin, Delapouite),
    src("graveyard", "Graveyard", Ruin, Delapouite),
    src("tombstone", "Tombstone", Ruin, Lorc),
    src("rune-stone", "Rune stone", Ruin, Lorc),
    src("swamp", "Swamp", Hazard, Delapouite),
    src("quicksand", "Quicksand", Hazard, Lorc),
    src("spider-web", "Spider web", Hazard, Lorc),
    src("thorny-vine", "Thorny vine", Hazard, Lorc),
    src("sandstorm", "Sandstorm", Hazard, Delapouite),
    src("volcano", "Volcano", Hazard, Lorc),
    src("lightning-storm", "Lightning storm", Hazard, Lorc),
    src("poison-gas", "Poison gas", Hazard, Lorc),
    src("campfire", "Campfire", Waypoint, Lorc),
    src("compass", "Compass", Waypoint, Lorc),
    src("footprint", "Footprint", Waypoint, Lorc),
    src("pin", "Pin", Waypoint, Delapouite),
    src("wooden-sign", "Signpost", Waypoint, Lorc),
];

const START: &str = "pub const LINK_ICONS: &[LinkIconDef] = &[\n";
const END: &str = "\n];\n";

impl Author {
    fn dir(self) -> &'static str {
        match self {
            Delapouite => "delapouite",
            Lorc => "lorc",
        }
    }
}

/// The source files are a black background rect plus a white icon path:
/// keep only the icon, recoloured to `currentColor` so it can be tinted
/// with CSS `color`.
fn icon_svg(slug: &str, raw: &str) -> Result<String, String> {
    let paths: Vec<&str> = raw
        .split("<path")
        .skip(1)
        .map(|p| p.split_once("/>").map_or(p, |(p, _)| p))
        .collect();
    let [_, icon] = paths.as_slice() else {
        return Err(format!(
            "Unexpected SVG shape for {slug} (expected 2 <path> elements, got {})",
            paths.len()
        ));
    };
    let icon = icon.replacen("fill=\"#fff\"", "fill=\"currentColor\"", 1);
    Ok(format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 512 512\"><path{icon}/></svg>"
    ))
}

fn render_entry(source: &IconSource, svg: &str) -> String {
    format!(
        "    LinkIconDef {{\n        slug: \"{}\",\n        label: \"{}\",\n        category: IconCategory::{:?},\n        author: IconAuthor::{:?},\n        svg: r#\"{svg}\"#,\n    }},",
        source.slug, source.label, source.category, source.author
    )
}

/// `existing` with its LINK_ICONS body replaced by `entries`.
fn splice(existing: &str, entries: &[String]) -> Result<String, String> {
    let start = existing
        .find(START)
        .ok_or("couldn't find LINK_ICONS in the target file")?
        + START.len();
    let end = start
        + existing[start..]
            .find(END)
            .ok_or("couldn't find the end of LINK_ICONS")?;
    Ok(format!(
        "{}{}{}",
        &existing[..start],
        entries.join("\n"),
        &existing[end..]
    ))
}

pub fn sync(out: &Path) -> Result<(), String> {
    let mut sources: Vec<&IconSource> = ICON_SOURCES.iter().collect();
    sources.sort_by(|a, b| a.category.cmp(&b.category).then(a.slug.cmp(b.slug)));

    let mut entries = Vec::new();
    for source in sources {
        let url = format!(
            "https://raw.githubusercontent.com/game-icons/icons/master/{}/{}.svg",
            source.author.dir(),
            source.slug
        );
        let raw = ureq::get(&url)
            .call()
            .map_err(|e| format!("{url}: {e}"))?
            .body_mut()
            .read_to_string()
            .map_err(|e| format!("{url}: {e}"))?;
        entries.push(render_entry(source, &icon_svg(source.slug, &raw)?));
    }

    let existing = fs::read_to_string(out).map_err(|e| format!("{}: {e}", out.display()))?;
    fs::write(out, splice(&existing, &entries)?).map_err(|e| format!("{}: {e}", out.display()))?;
    println!("Wrote {} icons to {}", entries.len(), out.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const CURRENT: &str = include_str!("../../../crates/map-core/src/link_icons.rs");

    #[test]
    fn strips_the_background_and_recolours_the_icon() {
        let raw = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512"><path d="M0 0h512v512H0z"/><path fill="#fff" d="M1 2z"/></svg>"##;
        assert_eq!(
            icon_svg("x", raw).unwrap(),
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512"><path fill="currentColor" d="M1 2z"/></svg>"#
        );
        assert!(icon_svg("x", "<svg><path/></svg>").is_err());
    }

    /// Re-rendering the icons already in link_icons.rs (with their SVGs as
    /// they are) reproduces the file byte for byte — so a sync only ever
    /// changes what upstream changed.
    #[test]
    fn rendering_the_current_icons_reproduces_the_file() {
        let mut sources: Vec<&IconSource> = ICON_SOURCES.iter().collect();
        sources.sort_by(|a, b| a.category.cmp(&b.category).then(a.slug.cmp(b.slug)));
        let entries: Vec<String> = sources
            .iter()
            .map(|source| {
                let marker = format!("slug: \"{}\"", source.slug);
                let at = CURRENT
                    .find(&marker)
                    .unwrap_or_else(|| panic!("{} missing from link_icons.rs", source.slug));
                let svg_start = at + CURRENT[at..].find("svg: r#\"").unwrap() + "svg: r#\"".len();
                let svg_end = svg_start + CURRENT[svg_start..].find("\"#").unwrap();
                render_entry(source, &CURRENT[svg_start..svg_end])
            })
            .collect();
        assert_eq!(splice(CURRENT, &entries).unwrap(), CURRENT);
    }
}
