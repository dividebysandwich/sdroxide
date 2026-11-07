# Translating sdroxide

The interface's text lives here, one [Fluent](https://projectfluent.org) file
per language:

```
i18n/
  en-US/sdroxide.ftl   the English reference — every message is defined here
  es/sdroxide.ftl      a translation (any BCP 47 tag: es, de, pt-BR, …)
```

Files are embedded in the binary when it is built, so a new or changed file
takes effect on the next build. The language is picked in Settings → UI →
Language; "System" follows the operating system (or the browser, for the web
client).

## Adding a language

1. Make a folder named after the language's tag — `de`, `pt-BR` — and copy
   `en-US/sdroxide.ftl` into it.
2. Change `language-name` to the language's name *in that language*
   (`Deutsch`, `Português (Brasil)`). The picker lists every language under
   its own name.
3. Translate the values. Leave the ids (left of `=`) and the `{ $name }`
   placeables as they are.
4. Run `cargo test -p sdroxide-ui i18n` (see *Checks* below).

A translation does not have to be complete. Any message it leaves out is shown
in English, so a file can start with the most-seen text and fill in over
time. Delete the messages you have not translated rather than leaving them in
English, so that an updated English text reaches your users.

## What a message looks like

```fluent
# A comment for the translator, about the message below it.
settings-ui-theme = Theme

# An attribute: the hover help that goes with the label above it.
settings-ui-language = Language
    .hint = The language of the interface. …

# A placeable: a value the program fills in. Move it wherever your
# grammar needs it; do not translate or rename it.
settings-dialling = Dialling { $url }…

# A plural. English has two forms; use the forms your language has
# (zero, one, two, few, many, other) — Fluent picks by the CLDR rules.
windows-n-channels = { $n } { $n ->
        [one] channel
       *[other] channels
    }
```

Selectors are also used where a word depends on a value — a month, a region,
"more" or "less". Translate each branch; keep the keys in square brackets.

## Rules

- **Write capitals where the English has them.** Many captions are shown in
  capitals (`SETTINGS`, `PEAK HOLD`). The program does not change the case of
  translated text — it would get Turkish `i`/`İ` and German `ß` wrong — so a
  caption that should read in capitals is written in capitals.
- **Keep it short where the English is short.** Chip and button labels sit in
  rows that are measured to the point. A label much longer than the English
  may not fit; an abbreviation is better than a clipped word. Hover help and
  notes have room.
- **Leave radio terms that are not words alone** when your operators use them
  as they are: mode names (USB, FT8), Q-codes, CTCSS/DCS, SNR, dBFS, units.
  They are kept out of the file where they are identifiers, and left
  translatable where a language might want its own word.
- **Text that goes on the air is not here.** CQ macros, JS8 directed messages,
  callsigns and anything else transmitted stays as it is, whatever the
  interface language.

## Not covered by these files

- **The manual behind F1** is a separate document and is English only.
- **Voice announcements** use an English voice; their settings are
  translated, the speech is not.
- **Scripts.** The interface fonts cover Latin, Greek and Cyrillic. A subset of
  Noto Sans SC covers the GB 2312 characters of Simplified Chinese (`zh-CN`),
  and a subset of Noto Sans JP covers the JIS X 0208 characters and kana of
  Japanese (`ja`). A script beyond those needs a font added first, and egui does
  not lay out right-to-left text, so Arabic and Hebrew are not possible yet. A
  character outside these subsets would show as a box; extend the subset
  (`assets/fonts`) if a translation needs one.
- **Numbers** are written the way the program formats them (a dot for the
  decimal point, no grouping), in every language.

## Checks

`cargo test -p sdroxide-ui i18n` checks, for every file:

- it parses;
- it defines no message twice (Fluent would silently keep the first);
- every message it defines exists in English, and uses no placeable the
  English does not pass (it would be shown as its own name);
- `language-name` is set;

and, for English, that every message is used somewhere in the program.

## Testing the layout

A debug build lists **Ƥşḗŭḓǿ (layout test)** in the language picker. It draws
every message accented and a third longer, in brackets:

```
⟦Ŧhḗmḗ~~⟧
```

Text that is *not* accented has not gone through the message file. A label
whose closing `⟧` is missing has been clipped, and a longer language will be
too.

## For developers

Text is drawn with `tr!` (see `src/i18n.rs`):

```rust
ui.label(tr!("settings-ui-theme"));
ui.label(tr!("settings-ui-language", "hint"));          // an attribute
ui.label(tr!("settings-dialling", url = server.url()));  // with values
```

`tr!` checks the id, and the names of the values, against the English file
when the crate is compiled: a typo is a build error. Values are borrowed, like
`format!`'s, and may be text or numbers; pass a number as a number so a
translation can choose its plural form by it.

Values from `sdroxide-types` (a theme, a sort order, a relay role) have an
English `label()` that is also written to files and logs. Draw them with
`.tr()` from the `Tr` trait (`src/i18n/labels.rs`), never with `.label()`.

When adding text: add the English message to `en-US/sdroxide.ftl` under the
section for the file it is used in, name it after where it is shown
(`settings-ui-…`, `topbar-…`), and write the casing it is shown in.
