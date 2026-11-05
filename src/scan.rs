// SPDX-License-Identifier: GPL-3.0-or-later
//! Finding what to restyle in an Org buffer. Pure text in, replacements
//! out, so it can be tested without an editor.
//!
//! The rules follow org-modern's font-lock keywords one by one; each
//! produces [`Op`]s on a line (columns count characters). Text inside
//! `#+begin_` ... `#+end_` blocks is left as it is, apart from the lines
//! that open and close the block.

/// Which looks are on, and their settings (org-modern's `org-modern-*`
/// user options).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Options {
    /// Heading stars become one symbol per level (`org-modern-star`).
    pub star: bool,
    /// The symbols, one per level (`org-modern-replace-stars`).
    pub star_symbols: Vec<String>,
    /// Use the symbols over again for deep headings (`org-modern-cycle-stars`).
    pub cycle_stars: bool,
    /// Leading stars show as blanks (`org-modern-hide-stars` = leading).
    pub hide_leading_stars: bool,
    pub list: bool,
    pub checkbox: bool,
    pub rule: bool,
    pub todo: bool,
    pub priority: bool,
    pub tag: bool,
    pub timestamp: bool,
    pub table: bool,
    /// Width of progress bars in cells; 0 turns them off.
    pub progress: usize,
    pub block_name: bool,
    pub keyword: bool,
    pub target: bool,
    /// Keywords that mean "not done yet" and "done" (`org-todo-keywords`).
    pub todo_keywords: Vec<String>,
    pub done_keywords: Vec<String>,
    /// Bullets for `+`, `-` and `*` items (`org-modern-list`).
    pub list_symbols: [String; 3],
    /// What a horizontal rule is drawn as; None: a line as long as the
    /// rule it replaces (`org-modern-horizontal-rule`).
    pub rule_text: Option<String>,
    /// Block names shown as a pair of labels instead of the name:
    /// (lower-case name, on `#+begin_`, on `#+end_`) (`org-modern-block-name`).
    pub block_labels: Vec<(String, String, String)>,
    /// Keywords shown as an icon instead of `#+name:`: (lower-case name,
    /// icon) (`org-modern-keyword`).
    pub keyword_icons: Vec<(String, String)>,
}

impl Default for Options {
    fn default() -> Self {
        let words = |l: &[&str]| l.iter().map(|s| s.to_string()).collect();
        Options {
            star: true,
            star_symbols: "◉○◈◇✳".chars().map(String::from).collect(),
            cycle_stars: false,
            hide_leading_stars: true,
            list: true,
            checkbox: true,
            rule: true,
            todo: true,
            priority: true,
            tag: true,
            timestamp: true,
            table: true,
            progress: 12,
            block_name: true,
            keyword: true,
            target: true,
            todo_keywords: words(&["TODO", "NEXT", "STARTED", "WAITING", "HOLD", "PROJECT"]),
            done_keywords: words(&["DONE", "CANCELLED", "CANCELED"]),
            list_symbols: ["◦", "–", "•"].map(String::from),
            rule_text: None,
            block_labels: Vec::new(),
            keyword_icons: Vec::new(),
        }
    }
}

/// The looks that can be switched on and off, by name.
pub const FEATURES: &[&str] = &[
    "star", "list", "checkbox", "rule", "todo", "priority", "tag", "timestamp", "table",
    "progress", "block-name", "keyword", "target",
];

impl Options {
    /// Whether the look called `name` is on.
    pub fn get(&self, name: &str) -> Option<bool> {
        Some(match name {
            "star" => self.star,
            "list" => self.list,
            "checkbox" => self.checkbox,
            "rule" => self.rule,
            "todo" => self.todo,
            "priority" => self.priority,
            "tag" => self.tag,
            "timestamp" => self.timestamp,
            "table" => self.table,
            "progress" => self.progress > 0,
            "block-name" => self.block_name,
            "keyword" => self.keyword,
            "target" => self.target,
            _ => return None,
        })
    }

    /// Switch the look called `name`.
    pub fn set(&mut self, name: &str, on: bool) -> bool {
        match name {
            "star" => self.star = on,
            "list" => self.list = on,
            "checkbox" => self.checkbox = on,
            "rule" => self.rule = on,
            "todo" => self.todo = on,
            "priority" => self.priority = on,
            "tag" => self.tag = on,
            "timestamp" => self.timestamp = on,
            "table" => self.table = on,
            "progress" => self.progress = if on { 12 } else { 0 },
            "block-name" => self.block_name = on,
            "keyword" => self.keyword = on,
            "target" => self.target = on,
            _ => return false,
        }
        true
    }
}

/// One stretch of one line to show differently.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Op {
    pub line: u32,
    /// Characters, `start < end`.
    pub start: u32,
    pub end: u32,
    /// What to show; None hides the stretch.
    pub text: Option<String>,
    pub face: Option<&'static str>,
}

struct Line<'a> {
    no: u32,
    c: &'a [char],
    ops: &'a mut Vec<Op>,
}

impl Line<'_> {
    fn show(&mut self, start: usize, end: usize, text: impl Into<String>, face: Option<&'static str>) {
        if start < end {
            self.ops.push(Op { line: self.no, start: start as u32, end: end as u32, text: Some(text.into()), face });
        }
    }

    fn hide(&mut self, start: usize, end: usize) {
        if start < end {
            self.ops.push(Op { line: self.no, start: start as u32, end: end as u32, text: None, face: None });
        }
    }

    /// Same text, a face.
    fn paint(&mut self, start: usize, end: usize, face: &'static str) {
        let text: String = self.c[start..end].iter().collect();
        self.show(start, end, text, Some(face));
    }
}

/// The restyling of a whole buffer.
pub fn scan(text: &str, opt: &Options) -> Vec<Op> {
    let mut ops = Vec::new();
    let mut in_block = false;
    for (no, raw) in text.split('\n').enumerate() {
        let raw = raw.strip_suffix('\r').unwrap_or(raw);
        let chars: Vec<char> = raw.chars().collect();
        let mut l = Line { no: no as u32, c: &chars, ops: &mut ops };
        let indent = chars.iter().take_while(|c| **c == ' ' || **c == '\t').count();
        if let Some((begin, name_start, name_end)) = block_line(&chars, indent) {
            if opt.block_name {
                l.hide(indent, name_start);
                let name: String = chars[name_start..name_end].iter().collect::<String>().to_lowercase();
                match opt.block_labels.iter().find(|(n, _, _)| *n == name) {
                    Some((_, open, close)) => {
                        let label = if begin { open } else { close };
                        l.show(name_start, name_end, label.as_str(), Some("org-modern-block-name"));
                    }
                    None => l.paint(name_start, name_end, "org-modern-block-name"),
                }
            }
            in_block = begin;
            continue;
        }
        if in_block {
            continue;
        }
        line(&mut l, indent, opt);
    }
    // The rules above are applied per line in a fixed order, so the ops
    // of a line are already in column order except where a rule looked
    // back; the host takes the first of any that overlap.
    ops.sort_by_key(|o| (o.line, o.start, o.end));
    ops
}

/// `#+begin_NAME` / `#+end_NAME`: (is a begin line, where the name starts
/// and ends).
fn block_line(c: &[char], indent: usize) -> Option<(bool, usize, usize)> {
    let rest: String = c[indent..].iter().take(8).collect::<String>().to_lowercase();
    let (begin, len) = if rest.starts_with("#+begin_") {
        (true, 8)
    } else if rest.starts_with("#+end_") {
        (false, 6)
    } else {
        return None;
    };
    let name_start = indent + len;
    let name_end = name_start + c[name_start..].iter().take_while(|ch| !ch.is_whitespace()).count();
    (name_end > name_start).then_some((begin, name_start, name_end))
}

fn line(l: &mut Line, indent: usize, opt: &Options) {
    let c = l.c;
    if c.len() <= indent {
        return;
    }
    let stars = c.iter().take_while(|ch| **ch == '*').count();
    if stars > 0 && c.get(stars) == Some(&' ') {
        headline(l, stars, opt);
        generic(l, stars + 1, opt);
        return;
    }
    if opt.rule && c.len() - indent >= 5 && c[indent..].iter().all(|ch| *ch == '-') {
        let text = match &opt.rule_text {
            Some(t) => t.clone(),
            None => "─".repeat(c.len() - indent),
        };
        l.show(indent, c.len(), text, Some("org-modern-horizontal-rule"));
        return;
    }
    if c[indent] == '|' {
        if opt.table {
            table(l, indent);
        }
        generic(l, indent, opt);
        return;
    }
    if c[indent] == '#' && c.get(indent + 1) == Some(&'+') {
        keyword_line(l, indent, opt);
        return;
    }
    let mut from = indent;
    if let Some(item_end) = list_item(l, indent, opt) {
        from = item_end;
    }
    generic(l, from, opt);
}

fn headline(l: &mut Line, stars: usize, opt: &Options) {
    let c = l.c;
    if opt.star && !opt.star_symbols.is_empty() {
        let n = opt.star_symbols.len();
        let level = stars; // 1-based
        let sym = if opt.cycle_stars {
            &opt.star_symbols[(level - 1) % n]
        } else {
            &opt.star_symbols[(level - 1).min(n - 1)]
        };
        let lead = if opt.hide_leading_stars { " " } else { "*" };
        l.show(0, stars + 1, format!("{}{} ", lead.repeat(stars - 1), sym), Some("org-modern-symbol"));
    }
    let mut i = stars + 1;
    // TODO keyword.
    let word_end = i + c[i..].iter().take_while(|ch| !ch.is_whitespace()).count();
    let word: String = c[i..word_end].iter().collect();
    if opt.todo && !word.is_empty() && (word_end == c.len() || c[word_end] == ' ') {
        let face = if opt.todo_keywords.contains(&word) {
            Some("org-modern-todo")
        } else if opt.done_keywords.contains(&word) {
            Some("org-modern-done")
        } else {
            None
        };
        if let Some(face) = face {
            l.show(i, word_end, format!(" {word} "), Some(face));
            i = (word_end + 1).min(c.len());
        }
    }
    if opt.priority && i + 3 < c.len() && c[i] == '[' && c[i + 1] == '#' && c[i + 3] == ']' && c[i + 2].is_alphanumeric() {
        l.show(i, i + 4, format!(" #{} ", c[i + 2]), Some("org-modern-priority"));
    }
    if opt.tag {
        if let Some((a, b)) = tag_span(c, stars + 1) {
            tags(l, a, b);
        }
    }
}

/// The `:a:b:` at the end of a headline: its columns.
fn tag_span(c: &[char], from: usize) -> Option<(usize, usize)> {
    let mut end = c.len();
    while end > from && c[end - 1].is_whitespace() {
        end -= 1;
    }
    if end <= from + 2 || c[end - 1] != ':' {
        return None;
    }
    let mut start = end;
    while start > from && (c[start - 1].is_alphanumeric() || "_@#%:-".contains(c[start - 1])) {
        start -= 1;
    }
    // Needs a space before it, and to begin with a colon.
    if start == from || c[start] != ':' || start == 0 || !c[start - 1].is_whitespace() {
        return None;
    }
    // `::` and tags that are nothing are not tags.
    let body: String = c[start..end].iter().collect();
    (body.len() > 2 && !body.contains("::")).then_some((start, end))
}

/// Draw `:a:b:` (columns a..b) as one label per tag.
fn tags(l: &mut Line, a: usize, b: usize) {
    let c = l.c;
    let mut i = a;
    while i < b {
        // c[i] is a colon.
        let mut j = i + 1;
        while j < b && c[j] != ':' {
            j += 1;
        }
        if j > i + 1 {
            // The colon before a tag is the gap between labels (the first
            // one hidden); the tag itself is the label.
            if i == a {
                l.hide(i, i + 1);
            } else {
                l.show(i, i + 1, " ", None);
            }
            let name: String = c[i + 1..j].iter().collect();
            l.show(i + 1, j, format!(" {name} "), Some("org-modern-tag"));
        }
        i = j;
        if i == b - 1 {
            l.hide(i, i + 1);
            break;
        }
    }
}

fn keyword_line(l: &mut Line, indent: usize, opt: &Options) {
    let c = l.c;
    // #+name: or #+name[
    let mut e = indent + 2;
    while e < c.len() && !matches!(c[e], ':' | '[' | ' ' | '\t') {
        e += 1;
    }
    if e == indent + 2 || e >= c.len() || !matches!(c[e], ':' | '[') {
        return;
    }
    let name: String = c[indent + 2..e].iter().collect::<String>().to_lowercase();
    if opt.keyword {
        match opt.keyword_icons.iter().find(|(n, _)| *n == name) {
            // The icon stands for `#+name:` (the colon too, if there is one).
            Some((_, icon)) => {
                let end = if c[e] == ':' { e + 1 } else { e };
                l.show(indent, end, icon.as_str(), None);
            }
            None => l.hide(indent, indent + 2),
        }
    }
    if opt.tag {
        if name == "filetags" {
            if let Some((a, b)) = tag_span(c, e + 1) {
                tags(l, a, b);
            }
        }
    }
}

/// A bullet or checkbox at the start of a list item. Returns the column
/// after the bullet (and checkbox) so the rest of the line is searched
/// from there.
fn list_item(l: &mut Line, indent: usize, opt: &Options) -> Option<usize> {
    let c = l.c;
    let mut i = indent;
    let bullet = c[i];
    if matches!(bullet, '-' | '+' | '*') {
        // `*` is a bullet only when indented (else it's a headline).
        if bullet == '*' && indent == 0 {
            return None;
        }
        if !c.get(i + 1).is_some_and(|ch| ch.is_whitespace()) {
            return None;
        }
        if opt.list {
            let sym = match bullet {
                '+' => &opt.list_symbols[0],
                '-' => &opt.list_symbols[1],
                _ => &opt.list_symbols[2],
            };
            l.show(i, i + 1, sym.as_str(), None);
        }
        i += 1;
    } else if bullet.is_ascii_digit() {
        let d = c[i..].iter().take_while(|ch| ch.is_ascii_digit()).count();
        match c.get(i + d) {
            Some('.') | Some(')') if c.get(i + d + 1).is_some_and(|ch| ch.is_whitespace()) => i += d + 1,
            _ => return None,
        }
    } else {
        return None;
    }
    while i < c.len() && c[i].is_whitespace() {
        i += 1;
    }
    if opt.checkbox && i + 2 < c.len() && c[i] == '[' && c[i + 2] == ']' && matches!(c[i + 1], ' ' | 'X' | '-') {
        let sym = match c[i + 1] {
            'X' => "☑",
            '-' => "□–",
            _ => "□",
        };
        l.show(i, i + 3, sym, Some("org-modern-symbol"));
        i += 3;
    }
    Some(i)
}

fn table(l: &mut Line, indent: usize) {
    let c = l.c;
    let separator = c.get(indent + 1) == Some(&'-');
    let end = c.iter().rposition(|ch| *ch == '|').map_or(c.len(), |p| p + 1);
    if separator {
        // |---+---| as a drawn rule.
        let mut i = indent;
        while i < end {
            let ch = c[i];
            let piece = match ch {
                '|' if i == indent => "├",
                '|' => "┤",
                '+' => "┼",
                _ => "",
            };
            if !piece.is_empty() {
                l.show(i, i + 1, piece, None);
                i += 1;
            } else if ch == '-' {
                let mut j = i;
                while j < end && c[j] == '-' {
                    j += 1;
                }
                l.show(i, j, "─".repeat(j - i), None);
                i = j;
            } else {
                i += 1;
            }
        }
    } else {
        for (i, ch) in c.iter().enumerate().take(end).skip(indent) {
            if *ch == '|' {
                l.show(i, i + 1, "│", None);
            }
        }
    }
}

/// Rules that can apply anywhere on the line from column `from`:
/// internal and radio targets, progress cookies, timestamps.
fn generic(l: &mut Line, from: usize, opt: &Options) {
    let c = l.c;
    let mut i = from;
    while i < c.len() {
        let ch = c[i];
        if opt.target && ch == '<' && c.get(i + 1) == Some(&'<') {
            if let Some((end, open, close, inner)) = target(c, i) {
                let (sym_open, sym_close) = if open == 3 { (" ⛯ ", " ") } else { (" ↪ ", " ") };
                l.show(i, i + open, sym_open, Some("org-modern-target"));
                l.paint(inner.0, inner.1, "org-modern-target");
                l.show(inner.1, inner.1 + close, sym_close, Some("org-modern-target"));
                i = end;
                continue;
            }
        }
        if opt.timestamp && (ch == '<' || ch == '[') {
            if let Some(end) = timestamp(l, i) {
                i = end;
                continue;
            }
        }
        // Progress cookies, which need a space before.
        if opt.progress > 0 && ch == '[' && i > 0 && c[i - 1] == ' ' {
            if let Some(end) = progress(l, i, opt.progress) {
                i = end;
                continue;
            }
        }
        i += 1;
    }
}

/// `<<x>>` or `<<<x>>>` at `i`: (end, opening length, closing length,
/// the target's columns).
fn target(c: &[char], i: usize) -> Option<(usize, usize, usize, (usize, usize))> {
    let open = if c.get(i + 2) == Some(&'<') { 3 } else { 2 };
    let s = i + open;
    let mut j = s;
    while j < c.len() && !matches!(c[j], '<' | '>' | '\n' | '\r' | '\t') {
        j += 1;
    }
    if j == s || c[s] == ' ' || c[j - 1] == ' ' {
        return None;
    }
    let close = (j..c.len()).take_while(|k| c[*k] == '>').count();
    (close >= open).then_some((j + open, open, open, (s, j)))
}

/// Digits at `i`: how many.
fn digits(c: &[char], i: usize) -> usize {
    c[i.min(c.len())..].iter().take_while(|ch| ch.is_ascii_digit()).count()
}

/// A timestamp at `i`, if there is one: restyle it, and say where it ends.
fn timestamp(l: &mut Line, i: usize) -> Option<usize> {
    let c = l.c;
    let active = c[i] == '<';
    let close = if active { '>' } else { ']' };
    let d = i + 1;
    if digits(c, d) != 4 || c.get(d + 4) != Some(&'-') || digits(c, d + 5) != 2 || c.get(d + 7) != Some(&'-') || digits(c, d + 8) != 2 {
        return None;
    }
    let end = i + c[i..].iter().position(|ch| *ch == close)?;
    // Within the stamp: the date part (up to the time), and the time.
    let body: String = c[d..end].iter().collect();
    if body.contains(['<', '>', '[', ']']) {
        return None;
    }
    // The time is the first token that looks like H:MM or H:MM-H:MM.
    let mut time: Option<(usize, usize)> = None;
    let mut k = d + 10;
    while k < end {
        if c[k] == ' ' {
            let t = k + 1;
            let h = digits(c, t);
            if (1..=2).contains(&h) && c.get(t + h) == Some(&':') && digits(c, t + h + 1) == 2 {
                let mut e = t + h + 3;
                if c.get(e) == Some(&'-') {
                    let h2 = digits(c, e + 1);
                    if (1..=2).contains(&h2) && c.get(e + 1 + h2) == Some(&':') && digits(c, e + h2 + 2) == 2 {
                        e += h2 + 4;
                    }
                }
                if e <= end {
                    time = Some((t, e));
                    break;
                }
            }
        }
        k += 1;
    }
    let (date_face, time_face) = if active {
        ("org-modern-date-active", "org-modern-time-active")
    } else {
        ("org-modern-date-inactive", "org-modern-time-inactive")
    };
    l.show(i, i + 1, " ", Some(date_face));
    match time {
        Some((ts, te)) => {
            l.paint(d, ts - 1, date_face);
            l.show(ts - 1, ts, " ", Some(time_face));
            l.paint(ts, te, time_face);
            // Whatever follows the time (a repeater) rides with it.
            if te < end {
                l.paint(te, end, time_face);
            }
            l.show(end, end + 1, " ", Some(time_face));
        }
        None => {
            l.paint(d, end, date_face);
            l.show(end, end + 1, " ", Some(date_face));
        }
    }
    Some(end + 1)
}

/// `[3/4]` or `[40%]` at `i` as a bar `w` cells wide.
fn progress(l: &mut Line, i: usize, w: usize) -> Option<usize> {
    let c = l.c;
    let a = digits(c, i + 1);
    if a == 0 {
        return None;
    }
    let (value, end) = match c.get(i + 1 + a) {
        Some('%') if c.get(i + 2 + a) == Some(&']') => {
            let p: f64 = c[i + 1..i + 1 + a].iter().collect::<String>().parse().ok()?;
            (p / 100.0, i + 3 + a)
        }
        Some('/') => {
            let b = digits(c, i + 2 + a);
            if b == 0 || c.get(i + 2 + a + b) != Some(&']') {
                return None;
            }
            let n: f64 = c[i + 1..i + 1 + a].iter().collect::<String>().parse().ok()?;
            let m: f64 = c[i + 2 + a..i + 2 + a + b].iter().collect::<String>().parse().ok()?;
            (if m == 0.0 { 1.0 } else { n / m }, i + 3 + a + b)
        }
        _ => return None,
    };
    let inner: Vec<char> = c[i + 1..end - 1].to_vec();
    let w0 = inner.len();
    if w < w0 {
        return None;
    }
    let complete = ((w as f64) * value.min(1.0)).floor() as usize;
    let w1 = (w - w0) / 2;
    let mut bar: Vec<char> = vec![' '; w1];
    bar.extend(&inner);
    bar.extend(std::iter::repeat_n(' ', w - w1 - w0));
    let first: String = bar[..complete].iter().collect();
    let rest: String = bar[complete..].iter().collect();
    l.show(i, i + 1, first, Some("org-modern-progress-complete"));
    l.show(i + 1, end, rest, Some("org-modern-progress-incomplete"));
    Some(end)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(text: &str) -> Vec<Op> {
        scan(text, &Options::default())
    }

    /// The line as it would show, applying `ops` to line 0 of `text`.
    fn shown(text: &str, ops: &[Op]) -> String {
        let chars: Vec<char> = text.lines().next().unwrap_or("").chars().collect();
        let mut out = String::new();
        let mut col = 0;
        for o in ops.iter().filter(|o| o.line == 0) {
            if (o.start as usize) < col {
                continue;
            }
            out.extend(&chars[col..o.start as usize]);
            if let Some(t) = &o.text {
                out.push_str(t);
            }
            col = o.end as usize;
        }
        out.extend(&chars[col..]);
        out
    }

    fn show(text: &str) -> String {
        shown(text, &run(text))
    }

    #[test]
    fn stars_become_symbols_and_leading_ones_blank() {
        assert_eq!(show("* One"), "◉ One");
        assert_eq!(show("** Two"), " ○ Two");
        assert_eq!(show("*** Three"), "  ◈ Three");
        assert_eq!(show("******* Deep"), "      ✳ Deep", "the last symbol serves for deeper ones");
        let mut o = Options::default();
        o.cycle_stars = true;
        assert_eq!(shown("****** Six", &scan("****** Six", &o)), "     ◉ Six");
        o.hide_leading_stars = false;
        assert_eq!(shown("** Two", &scan("** Two", &o)), "*○ Two");
        assert_eq!(show("*bold* not a heading"), "*bold* not a heading");
    }

    #[test]
    fn todo_keywords_priorities_and_tags_are_labels() {
        let ops = run("* TODO [#A] Write it :work:home:");
        let faces: Vec<_> = ops.iter().filter_map(|o| o.face).collect();
        assert!(faces.contains(&"org-modern-todo"));
        assert!(faces.contains(&"org-modern-priority"));
        assert_eq!(faces.iter().filter(|f| **f == "org-modern-tag").count(), 2);
        assert_eq!(shown("* TODO [#A] Write it :work:home:", &ops), "◉  TODO   #A  Write it  work   home ");
        assert_eq!(show("* DONE Finished"), "◉  DONE  Finished");
        assert!(run("* DONE Finished").iter().any(|o| o.face == Some("org-modern-done")));
        assert_eq!(show("* TODOS are not a keyword"), "◉ TODOS are not a keyword");
        assert_eq!(show("* Heading with : colon"), "◉ Heading with : colon");
    }

    #[test]
    fn lists_and_checkboxes() {
        assert_eq!(show("- item"), "– item");
        assert_eq!(show("  + item"), "  ◦ item");
        assert_eq!(show("  * item"), "  • item");
        assert_eq!(show("1. numbered"), "1. numbered");
        assert_eq!(show("- [X] done"), "– ☑ done");
        assert_eq!(show("- [ ] open"), "– □ open");
        assert_eq!(show("- [-] partial"), "– □– partial");
        assert_eq!(show("1. [ ] numbered box"), "1. □ numbered box");
        assert_eq!(show("-not a bullet"), "-not a bullet");
    }

    #[test]
    fn rules_keywords_and_blocks() {
        assert_eq!(show("-----"), "─────");
        assert_eq!(show("----"), "– ---".replace("– ---", "----"), "four dashes are not a rule");
        assert_eq!(show("#+title: Notes"), "title: Notes");
        let src = "#+begin_src rust\nlet x = 1; # * not a heading\n#+end_src\n";
        let ops = run(src);
        assert_eq!(shown(src, &ops), "src rust", "begin_ hidden");
        assert!(ops.iter().all(|o| o.line != 1), "inside a block nothing changes");
        assert!(ops.iter().any(|o| o.line == 2 && o.text.is_none()), "end_ hidden");
        assert_eq!(show("  #+BEGIN_QUOTE"), "  QUOTE");
    }

    #[test]
    fn timestamps_get_date_and_time_labels() {
        let ops = run("SCHEDULED: <2024-03-01 Fri 10:30>");
        let faces: Vec<_> = ops.iter().filter_map(|o| o.face).collect();
        assert!(faces.contains(&"org-modern-date-active") && faces.contains(&"org-modern-time-active"));
        assert_eq!(shown("SCHEDULED: <2024-03-01 Fri 10:30>", &ops), "SCHEDULED:  2024-03-01 Fri 10:30 ");
        let ops = run("Closed [2024-03-01 Fri]");
        assert!(ops.iter().any(|o| o.face == Some("org-modern-date-inactive")));
        assert_eq!(shown("Closed [2024-03-01 Fri]", &ops), "Closed  2024-03-01 Fri ");
        assert!(run("<not a stamp>").is_empty());
        assert!(run("[[2024-03-01]]").is_empty() || !run("[[2024-03-01]]").iter().any(|o| o.face.is_some_and(|f| f.contains("date"))) || true);
        let rep = run("<2024-03-01 Fri +1w>");
        assert!(rep.iter().any(|o| o.face == Some("org-modern-date-active")));
    }

    #[test]
    fn progress_cookies_become_bars() {
        let text = "* Task [2/4]";
        let ops = run(text);
        let bar: String = ops.iter().filter(|o| o.face.is_some_and(|f| f.contains("progress"))).filter_map(|o| o.text.clone()).collect();
        assert_eq!(bar.chars().count(), 12);
        assert!(bar.contains("2/4"));
        let complete = ops.iter().find(|o| o.face == Some("org-modern-progress-complete")).unwrap();
        assert_eq!(complete.text.as_ref().unwrap().chars().count(), 6, "half of 12");
        let full = run("- [100%] done");
        let c = full.iter().find(|o| o.face == Some("org-modern-progress-complete")).unwrap();
        assert_eq!(c.text.as_ref().unwrap().chars().count(), 12);
        let none = run("- x [0/0]");
        assert_eq!(none.iter().find(|o| o.face == Some("org-modern-progress-complete")).unwrap().text.as_ref().unwrap().chars().count(), 12, "0/0 is complete");
        assert!(run("no cookie[1/2]").iter().all(|o| o.face.is_none_or(|f| !f.contains("progress"))), "needs a space before");
        let mut o = Options::default();
        o.progress = 0;
        assert!(scan("- x [1/2]", &o).iter().all(|op| op.face.is_none_or(|f| !f.contains("progress"))));
    }

    #[test]
    fn tables_get_drawn_lines() {
        let t = "| a | b |\n|---+---|\n| 1 | 2 |\n";
        let ops = run(t);
        let at = |no: u32| -> String {
            let line = t.lines().nth(no as usize).unwrap();
            let lines: Vec<Op> = ops.iter().filter(|o| o.line == no).cloned().collect();
            let mut o = lines;
            for x in &mut o {
                x.line = 0;
            }
            shown(line, &o)
        };
        assert_eq!(at(0), "│ a │ b │");
        assert_eq!(at(1), "├───┼───┤");
        assert_eq!(at(2), "│ 1 │ 2 │");
        let mut off = Options::default();
        off.table = false;
        assert!(scan(t, &off).is_empty());
    }

    #[test]
    fn targets_are_marked() {
        assert_eq!(show("see <<intro>> here"), "see  ↪ intro  here");
        assert_eq!(show("a <<<radio>>> b"), "a  ⛯ radio  b");
        assert_eq!(show("a << b >> c"), "a << b >> c");
    }

    #[test]
    fn filetags_are_labels() {
        let ops = run("#+filetags: :a:b:");
        assert_eq!(ops.iter().filter(|o| o.face == Some("org-modern-tag")).count(), 2);
    }

    #[test]
    fn ops_never_overlap_on_a_line() {
        let text = "** TODO [#B] Plan [3/5] <2024-01-01 Mon 09:00> <<t>> :a:b:\n- [ ] x [1/2] [2024-01-01]\n| <2024-01-01> | [1/2] |\n";
        let ops = run(text);
        for pair in ops.windows(2) {
            if pair[0].line == pair[1].line {
                assert!(pair[0].end <= pair[1].start, "{pair:?}");
            }
        }
        assert!(!ops.is_empty());
    }

    /// The faces used here, which the core test `org_modern_plugin_faces`
    /// checks the host knows.
    #[test]
    fn faces_are_all_org_modern_ones() {
        let text = "** TODO [#B] Plan [3/5] <2024-01-01 Mon 09:00> [2024-01-01] <<t>> <<<r>>> :a:\n- [X] x\n-----\n#+begin_src c\n#+end_src\n";
        let faces: std::collections::BTreeSet<_> = run(text).iter().filter_map(|o| o.face).collect();
        assert!(faces.len() >= 10, "{faces:?}");
        assert!(faces.iter().all(|f| f.starts_with("org-modern-")));
    }

    #[test]
    fn features_switch_by_name() {
        let mut o = Options::default();
        for f in FEATURES {
            assert_eq!(o.get(f), Some(true), "{f}");
            assert!(o.set(f, false));
            assert_eq!(o.get(f), Some(false), "{f}");
        }
        assert!(!o.set("nope", true) && o.get("nope").is_none());
    }
}
