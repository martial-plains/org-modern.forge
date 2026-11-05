// SPDX-License-Identifier: GPL-3.0-or-later
//! org-modern: a modern look for Org buffers.
//!
//! A port of Daniel Mendler's Emacs package of the same name
//! (GPL-3.0-or-later). The text in the buffer never changes; the plugin
//! only tells Forge to show some of it differently (`set-display`):
//!
//! - heading stars as one symbol per level, leading stars blank
//! - TODO / DONE keywords, priorities `[#A]` and tags as padded labels
//! - timestamps as date and time labels
//! - progress cookies `[2/4]` / `[50%]` as bars
//! - list bullets, checkboxes, horizontal rules, tables drawn with lines
//! - `#+begin_` / `#+end_` and `#+` hidden, block names styled
//! - targets `<<x>>` and radio targets `<<<x>>>` marked
//!
//! The prettified text stays on the line point is on, as in Emacs; point is
//! drawn at the character it reaches. Skipped from the original: the fringe block borders and
//! footnote superscripts.
//!
//! - `M-x org-modern-mode` toggles it in this buffer.
//! - `M-x global-org-modern-mode` turns it on in every Org buffer, now and
//!   later.
//! - `M-x org-modern-toggle` turns one look on or off, e.g. `table`.
//!
//! The faces are `org-modern-todo`, `org-modern-tag`, ... which a theme
//! can restyle (`forge.theme`).
//!
//!   cargo build --release --target wasm32-wasip2
//!   cp target/wasm32-wasip2/release/org_modern.wasm ~/.config/forge/plugins/org-modern.wasm

pub mod config;
pub mod scan;

use forge_plugin_sdk::{
    Buffer, Event, EventKind, Position, Replacement, Span, command, completing_read, decorations,
    extend_submenu, forge_plugin, menu_item, message, modes, on,
};
use scan::{FEATURES, Op, Options, scan as scan_text};
use std::cell::{Cell, RefCell};

pub const MODE: &str = "org-modern-mode";

/// Buffers bigger than this aren't restyled (all of it is scanned on each
/// change).
const MAX_BYTES: usize = 2_000_000;

thread_local! {
    static GLOBAL: Cell<bool> = const { Cell::new(false) };
    static OPTIONS: RefCell<Options> = RefCell::new(Options::default());
}

fn is_on(b: &Buffer) -> bool {
    b.minor_modes().iter().any(|m| m == MODE)
}

fn eligible(b: &Buffer) -> bool {
    b.major_mode() == "org-mode"
}

fn replacement(o: &Op) -> Replacement {
    Replacement {
        span: Span {
            start: Position { line: o.line, column: o.start },
            end: Position { line: o.line, column: o.end },
        },
        text: o.text.clone(),
        face: o.face.map(String::from),
    }
}

fn restyle(b: &Buffer) -> Result<(), String> {
    let text = b.text();
    if text.len() > MAX_BYTES {
        return Err(format!("{} is too big for org-modern", b.name()));
    }
    let ops = OPTIONS.with_borrow(|o| scan_text(&text, o));
    let items: Vec<Replacement> = ops.iter().map(replacement).collect();
    decorations::set_display(b, &items)
}

fn enable(b: &Buffer) -> Result<(), String> {
    b.enable_minor_mode(MODE)?;
    restyle(b)
}

fn disable(b: &Buffer) {
    b.disable_minor_mode(MODE);
    let _ = decorations::set_display(b, &[]);
}

fn warn(r: Result<(), String>) {
    if let Err(e) = r {
        forge_plugin_sdk::warn(&e);
    }
}

fn init() -> Result<(), String> {
    // The user's settings (`forge.plugin.config("org-modern", { ... })`);
    // a mistake in them stops the plugin loading, naming the setting.
    OPTIONS.set(config::parse(&forge_plugin_sdk::config_json())?);
    modes::define_minor_mode(MODE)?;
    modes::set_lighter(MODE, "")?;
  
    command(
        MODE,
        "Toggle the modern look of Org in this buffer. With a prefix argument, \
         a positive one turns it on and zero or a negative one turns it off.",
        |prefix| {
            let b = Buffer::current();
            let want = match prefix {
                None => !is_on(&b),
                Some(n) => n > 0,
            };
            if want {
                enable(&b)?;
                message("Org Modern on");
            } else {
                disable(&b);
                message("Org Modern off");
            }
            Ok(())
        },
    )?;

    command(
        "global-org-modern-mode",
        "Toggle the modern look in every Org buffer, now and when Org files are opened. \
         With a prefix argument, a positive one turns it on and zero or a negative one turns it off.",
        |prefix| {
            let on = match prefix {
                None => !GLOBAL.get(),
                Some(n) => n > 0,
            };
            GLOBAL.set(on);
            for b in Buffer::all() {
                if on && eligible(&b) {
                    enable(&b)?;
                } else if !on && is_on(&b) {
                    disable(&b);
                }
            }
            message(if on { "Global Org Modern on" } else { "Global Org Modern off" });
            Ok(())
        },
    )?;

    command(
        "org-modern-toggle",
        "Turn one look of org-modern on or off (star, list, checkbox, rule, todo, priority, tag, \
         timestamp, table, progress, block-name, keyword, target).",
        |_| {
            let names: Vec<String> = FEATURES
                .iter()
                .map(|f| {
                    let on = OPTIONS.with_borrow(|o| o.get(f)).unwrap_or(false);
                    format!("{f} ({})", if on { "on" } else { "off" })
                })
                .collect();
            completing_read("Toggle org-modern look: ", &names, true, |answer| {
                let Some(answer) = answer else { return };
                let name = answer.split(' ').next().unwrap_or("");
                let now = OPTIONS.with_borrow_mut(|o| {
                    let on = !o.get(name)?;
                    o.set(name, on).then_some(on)
                });
                let Some(now) = now else { return };
                for b in Buffer::all().into_iter().filter(is_on) {
                    warn(restyle(&b));
                }
                message(&format!("org-modern {name} {}", if now { "on" } else { "off" }));
            });
            Ok(())
        },
    )?;

    // Under Options, next to Show/Hide (Forge has no View menu: Emacs doesn't).
    extend_submenu(
        "Options",
        "Org Modern",
        &[
            menu_item("This Buffer", MODE),
            menu_item("All Org Buffers", "global-org-modern-mode"),
            menu_item("Toggle a Look...", "org-modern-toggle"),
        ],
    )?;

    on(EventKind::BufferOpened, |e| {
        if let Event::BufferOpened(id) = e
            && GLOBAL.get()
            && let Some(b) = Buffer::from_id(*id)
            && eligible(&b)
        {
            warn(enable(&b));
        }
    });
    on(EventKind::BufferChanged, |e| {
        if let Event::BufferChanged(id) = e
            && let Some(b) = Buffer::from_id(*id).filter(is_on)
        {
            warn(restyle(&b));
        }
    });
    // Turned on or off some other way (init.rhai, another plugin), or the
    // buffer became Org.
    on(EventKind::ModeChanged, |e| {
        if let Event::ModeChanged(id) = e
            && let Some(b) = Buffer::from_id(*id)
        {
            if is_on(&b) {
                warn(restyle(&b));
            } else if GLOBAL.get() && eligible(&b) {
                warn(enable(&b));
            } else {
                let _ = decorations::set_display(&b, &[]);
            }
        }
    });
    Ok(())
}

forge_plugin!(init);
