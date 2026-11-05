// SPDX-License-Identifier: GPL-3.0-or-later
//! The user's settings: what init.rhai gave `forge.plugin.config("org-modern", { ... })`,
//! as JSON, turned into [`Options`]. A key that isn't known or has the
//! wrong kind of value is an error that names it, so a typo isn't silent.

use crate::scan::Options;
use serde_json::Value;

/// Every setting, for the error that lists them.
const KNOWN: &[&str] = &[
    "star", "cycle_stars", "hide_stars", "list", "checkbox", "rule", "todo", "priority", "tag",
    "timestamp", "table", "progress", "block_name", "keyword", "target", "todo_keywords",
    "done_keywords",
];

fn bad(key: &str, wants: &str) -> String {
    format!("org-modern: {key} must be {wants}")
}

fn flag(key: &str, v: &Value) -> Result<bool, String> {
    v.as_bool().ok_or_else(|| bad(key, "true or false"))
}

fn text(key: &str, v: &Value) -> Result<String, String> {
    v.as_str().map(String::from).ok_or_else(|| bad(key, "text"))
}

fn words(key: &str, v: &Value) -> Result<Vec<String>, String> {
    v.as_array()
        .ok_or_else(|| bad(key, "a list of text"))?
        .iter()
        .map(|w| text(key, w))
        .collect()
}

/// The options the settings in `json` (an object) describe, over the defaults.
pub fn parse(json: &str) -> Result<Options, String> {
    let mut o = Options::default();
    let v: Value = serde_json::from_str(json).map_err(|e| format!("org-modern: settings: {e}"))?;
    let Some(map) = v.as_object() else {
        return Err("org-modern: settings must be a table".into());
    };
    for (key, v) in map {
        match key.as_str() {
            "star" => match v {
                Value::Bool(b) => o.star = *b,
                Value::Array(_) => {
                    let list = words("star", v)?;
                    if list.is_empty() {
                        return Err(bad("star", "false or a list of at least one symbol"));
                    }
                    o.star_symbols = list;
                }
                _ => return Err(bad("star", "false or a list of symbols")),
            },
            "cycle_stars" => o.cycle_stars = flag(key, v)?,
            "hide_stars" => o.hide_leading_stars = flag(key, v)?,
            "list" => match v {
                Value::Bool(b) => o.list = *b,
                Value::Object(m) => {
                    for (bullet, sym) in m {
                        let i = match bullet.as_str() {
                            "+" => 0,
                            "-" => 1,
                            "*" => 2,
                            other => {
                                return Err(format!(
                                    "org-modern: list has no bullet {other:?} (use \"+\", \"-\" or \"*\")"
                                ));
                            }
                        };
                        o.list_symbols[i] = text("list bullets", sym)?;
                    }
                }
                _ => return Err(bad("list", "false or a table like { [\"-\"] = \"–\" }")),
            },
            "checkbox" => o.checkbox = flag(key, v)?,
            "rule" => match v {
                Value::Bool(b) => o.rule = *b,
                Value::String(s) => o.rule_text = Some(s.clone()),
                _ => return Err(bad("rule", "false or the text to draw")),
            },
            "todo" => o.todo = flag(key, v)?,
            "priority" => o.priority = flag(key, v)?,
            "tag" => o.tag = flag(key, v)?,
            "timestamp" => o.timestamp = flag(key, v)?,
            "table" => o.table = flag(key, v)?,
            "target" => o.target = flag(key, v)?,
            "progress" => match v {
                Value::Bool(true) => {}
                Value::Bool(false) => o.progress = 0,
                Value::Number(n) => match n.as_u64() {
                    Some(w) if (1..=200).contains(&w) => o.progress = w as usize,
                    _ => return Err(bad("progress", "false or a bar width from 1 to 200")),
                },
                _ => return Err(bad("progress", "false or a bar width")),
            },
            "block_name" => match v {
                Value::Bool(b) => o.block_name = *b,
                Value::Object(m) => {
                    for (name, pair) in m {
                        let labels = words("block_name entries", pair)?;
                        let [open, close] = <[String; 2]>::try_from(labels).map_err(|_| {
                            format!("org-modern: block_name {name:?} must be {{ \"open\", \"close\" }}")
                        })?;
                        o.block_labels.push((name.to_lowercase(), open, close));
                    }
                }
                _ => return Err(bad("block_name", "false or a table like { src = { \"»\", \"«\" } }")),
            },
            "keyword" => match v {
                Value::Bool(b) => o.keyword = *b,
                Value::Object(m) => {
                    for (name, icon) in m {
                        o.keyword_icons.push((name.to_lowercase(), text("keyword icons", icon)?));
                    }
                }
                _ => return Err(bad("keyword", "false or a table like { title = \"T\" }")),
            },
            "todo_keywords" => o.todo_keywords = words(key, v)?,
            "done_keywords" => o.done_keywords = words(key, v)?,
            other => {
                return Err(format!(
                    "org-modern: unknown setting {other:?} (known: {})",
                    KNOWN.join(", ")
                ));
            }
        }
    }
    Ok(o)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scan::scan;

    fn shown(opt: &Options, text: &str) -> String {
        let mut ops = scan(text, opt);
        ops.sort_by_key(|o| std::cmp::Reverse(o.start));
        let mut c: Vec<char> = text.chars().collect();
        for o in ops {
            c.splice(o.start as usize..o.end as usize, o.text.unwrap_or_default().chars());
        }
        c.into_iter().collect()
    }

    #[test]
    fn nothing_given_is_the_defaults() {
        assert_eq!(parse("{}").unwrap(), Options::default());
    }

    #[test]
    fn settings_change_what_is_shown() {
        let o = parse(
            r#"{"star":["◉","○"],"list":{"+":"➤"},"progress":false,"priority":false,
                "rule":"────","block_name":{"SRC":["»","«"]},"keyword":{"title":"T"},
                "todo_keywords":["TODO","PROJ"]}"#,
        )
        .unwrap();
        assert_eq!(shown(&o, "** x"), " ○ x");
        assert_eq!(shown(&o, "*** x"), "  ○ x", "past the last symbol, the last one");
        assert_eq!(shown(&o, "+ x"), "➤ x");
        assert_eq!(shown(&o, "- x"), "– x", "bullets not given keep their default");
        assert_eq!(o.progress, 0);
        assert!(!o.priority);
        assert_eq!(shown(&o, "-----------"), "────");
        assert_eq!(shown(&o, "#+begin_src rust"), "» rust");
        assert_eq!(shown(&o, "#+END_SRC"), "«");
        assert_eq!(shown(&o, "#+begin_quote"), "quote", "no label: the name, styled");
        assert_eq!(shown(&o, "#+title: Notes"), "T Notes");
        assert_eq!(shown(&o, "#+author: Me"), "author: Me", "no icon: only `#+` hidden");
        assert!(o.todo_keywords.contains(&"PROJ".to_string()));
    }

    #[test]
    fn mistakes_name_the_setting() {
        let e = |j: &str| parse(j).unwrap_err();
        assert!(e(r#"{"stars":true}"#).contains("unknown setting \"stars\""));
        assert!(e(r#"{"star":"x"}"#).contains("star must be"));
        assert!(e(r#"{"star":[]}"#).contains("at least one"));
        assert!(e(r#"{"list":{"x":"y"}}"#).contains("no bullet \"x\""));
        assert!(e(r#"{"block_name":{"src":["a"]}}"#).contains("block_name \"src\""));
        assert!(e(r#"{"progress":0}"#).contains("progress must be"));
        assert!(e("[1]").contains("must be a table"));
        assert!(e("nope").contains("settings"));
    }
}
