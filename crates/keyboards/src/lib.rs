#![forbid(unsafe_code)]

use serde::Deserialize;
use std::collections::HashSet;

const BUILTIN_LAYOUTS: &str = include_str!("../data/layouts.json");

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct Layout {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub rtl: bool,
    pub rows: Vec<Vec<String>>,
}

#[derive(Deserialize)]
struct LayoutFile {
    layouts: Vec<Layout>,
}

pub fn builtins() -> Result<Vec<Layout>, String> {
    parse_and_validate(BUILTIN_LAYOUTS)
}

pub fn parse_and_validate(json: &str) -> Result<Vec<Layout>, String> {
    let parsed: LayoutFile =
        serde_json::from_str(json).map_err(|error| format!("invalid keyboard data: {error}"))?;
    let mut ids = HashSet::new();
    for layout in &parsed.layouts {
        if layout.id.trim().is_empty() || layout.name.trim().is_empty() {
            return Err("keyboard id and name must not be empty".into());
        }
        if !ids.insert(layout.id.as_str()) {
            return Err(format!("duplicate keyboard id: {}", layout.id));
        }
        if layout.rows.is_empty() || layout.rows.iter().any(Vec::is_empty) {
            return Err(format!("keyboard {} has an empty row", layout.id));
        }
        if layout.rows.iter().flatten().any(|key| {
            key.is_empty() || key.chars().count() > 16 || key.chars().any(char::is_control)
        }) {
            return Err(format!(
                "keyboard {} contains an unsafe key label",
                layout.id
            ));
        }
    }
    Ok(parsed.layouts)
}

pub fn find<'a>(layouts: &'a [Layout], id: &str) -> Option<&'a Layout> {
    layouts.iter().find(|layout| layout.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_builtins_are_valid_and_unique() {
        let layouts = builtins().expect("built-in keyboards must validate");
        assert!(layouts.len() >= 20);
        assert!(find(&layouts, "devanagari").is_some());
        assert!(find(&layouts, "chinese").is_some());
        assert!(find(&layouts, "japanese").is_some());
    }

    #[test]
    fn duplicate_ids_are_rejected() {
        let input = r#"{"layouts":[{"id":"x","name":"x","rows":[["a"]]},{"id":"x","name":"y","rows":[["b"]]}]}"#;
        assert!(parse_and_validate(input).unwrap_err().contains("duplicate"));
    }
}
