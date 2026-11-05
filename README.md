# org-modern

A modern look for Org buffers in Forge. A port of
[org-modern](https://github.com/minad/org-modern) by Daniel Mendler
(GPL-3.0-or-later, as is this plugin).

The buffer text never changes. The plugin tells Forge to *show* stretches of
it differently, and the line point is on always shows the real text.

| Org | Shown as |
|---|---|
| `*** Heading` | `  ◈ Heading` (symbols `◉○◈◇✳`, leading stars blank) |
| `TODO`, `DONE`, `[#A]`, `:tag:` | padded labels (`org-modern-todo`, `-done`, `-priority`, `-tag`) |
| `<2024-03-01 Fri 10:30>` | date and time labels, active / inactive |
| `[2/4]`, `[50%]` | 12-cell progress bar |
| `-` `+` `*` bullets | `–` `◦` `•` |
| `[ ]` `[X]` `[-]` | `□` `☑` `□–` |
| `-----` | a drawn line |
| `\| a \| b \|` and `\|---+---\|` | box-drawing characters |
| `#+begin_src` / `#+end_src` | `src` (name styled `org-modern-block-name`) |
| `#+title:` | `title:` |
| `<<x>>`, `<<<x>>>` | ` ↪ x `, ` ⛯ x ` |

Not ported: fringe block borders and footnote superscripts.

## Use

- `M-x org-modern-mode`: this buffer.
- `M-x global-org-modern-mode`: every Org buffer, now and later
  (init.rhai: `forge.command.run("global-org-modern-mode", 1)`).
- `M-x org-modern-toggle`: switch one look (`star`, `list`, `checkbox`,
  `rule`, `todo`, `priority`, `tag`, `timestamp`, `table`, `progress`,
  `block-name`, `keyword`, `target`).

Faces `org-modern-*` can be restyled with `forge.theme`. Needs a Forge with
plugin API 0.1.12 (`set-display`, `config`).

## Settings

In init.rhai, with `forge.plugin.config`. Everything is optional; a key the
plugin doesn't know, or a value of the wrong kind, stops it loading and the
message names it. Changes apply the next time the plugin loads.

```rhai
forge.mode.hook("org-mode", "org-modern-mode");

forge.plugin.config("org-modern", #{
  star: ["◉", "○", "✸", "✿", "✤", "✜", "◆", "▶"],
  list: #{ "+": "➤", "-": "–", "*": "•" },
  progress: false,                       // or a bar width, e.g. 20
  priority: false,
  rule: "────────────────────────────────────",
  "block_name": #{                       // { on #+begin_, on #+end_ }
    src: ["»", "«"],
    example: ["»–", "–«"],
    quote: ["❝", "❞"],
    export: ["⏩", "⏪"],
  },
  keyword: #{                            // shown instead of #+name:
    title: "𝙏", subtitle: "𝙩", author: "𝘼", date: "𝘿",
    property: "☸", options: "⌥", startup: "⏻",
    include: "⇤", setupfile: "⇚",
    attr_org: "⒪", name: "⁍", header: "›",
    caption: "☰", results: "🠶",
  },
  "todo_keywords": ["TODO", "PROJ", "STRT", "HOLD", "WAIT"],
  "done_keywords": ["DONE", "KILL", "NO"],
});
```

| Setting | Value | Default |
|---|---|---|
| `star` | `false`, or symbols, one per level | `◉ ○ ◈ ◇ ✳` |
| `cycle_stars` | use the symbols over again for deep headings | `false` |
| `hide_stars` | leading stars show as blanks | `true` |
| `list` | `false`, or `#{ "+": .., "-": .., "*": .. }` | `◦ – •` |
| `rule` | `false`, or the text a rule is drawn as | a line as long as the rule |
| `progress` | `false`, or the bar width in cells (1 to 200) | `12` |
| `block_name` | `false`, or `{ name = { open, close } }`; other blocks keep their name, styled | on |
| `keyword` | `false`, or `{ name = icon }` for `#+name:`; others only lose the `#+` | on |
| `todo_keywords`, `done_keywords` | lists of words | Org's usual ones |
| `checkbox`, `todo`, `priority`, `tag`, `timestamp`, `table`, `target` | `true` / `false` | `true` |

The Emacs options that have no setting here: `org-modern-todo-faces`
(every TODO keyword uses `org-modern-todo`, every done one
`org-modern-done`; restyle them with `forge.theme`), the table line widths,
and text-property tricks like `(display (raise -0.14))`, which a terminal
can't do.

## Install with use_package

Add this to `~/.config/forge/init.rhai`:

```rhai
use_package("org-modern", #{
  vc: "https://github.com/martial-plains/forge-plugin-org-modern",
  pin: "v0.1.0",
  commands: ["org-modern-mode", "global-org-modern-mode"],
  hook: #{ "org-mode": "org-modern-mode" },
});
```

Forge asks before running the package build command the first time.

## Build

    cargo build --release --target wasm32-wasip2
    cp target/wasm32-wasip2/release/org_modern.wasm ~/.config/forge/plugins/org-modern.wasm

    cargo test --lib     # the scanner, natively


## Building

This plugin is built against the Forge plugin SDK (`forge-plugin-sdk`),
taken from the Forge repository named in `Cargo.toml` (change the `git =`
URL if Forge lives elsewhere).

    rustup target add wasm32-wasip2
    cargo build --release --target wasm32-wasip2

Copy the resulting `.wasm` to `~/.config/forge/plugins/`.

## License

GPL-3.0-or-later. See the license file in this directory.
