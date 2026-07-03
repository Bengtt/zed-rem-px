# CSSREM for Zed

px ⇄ rem conversion as **code completion** for CSS, SCSS and Sass — inspired by
[vscode-cssrem](https://github.com/cipchk/vscode-cssrem).

Type a length and press <kbd>Ctrl</kbd>+<kbd>Space</kbd>: the first suggestion is
the converted value.

| You type | First suggestion | Accept inserts |
| -------- | ---------------- | -------------- |
| `16px`   | `1rem`           | `1rem`         |
| `14px`   | `0.875rem`       | `0.875rem`     |
| `1rem`   | `16px`           | `16px`         |
| `1.5rem` | `24px`           | `24px`         |

Conversion uses a configurable root font size (default **16px**).

## Why two parts?

Unlike VS Code, **Zed extensions cannot provide completions directly** — they can
only run a Language Server (LSP) over the LSP protocol. So this repo has:

- **`lsp/`** — `cssrem-lsp`, a tiny native language server that does the actual
  px⇄rem conversion and returns it as a completion item.
- **root** (`extension.toml`, `src/lib.rs`) — the Zed extension (compiled to
  WASM) that tells Zed to launch `cssrem-lsp` for CSS and SCSS buffers.

## Prerequisites

- [Rust toolchain](https://rustup.rs/) (`cargo`, `rustc`).
- The WASM target for the extension build (Zed adds it automatically, but you can
  pre-install it): `rustup target add wasm32-wasip1`.

## Install

### 1. Build & install the language server

From the repo root:

```sh
cargo install --path lsp
```

This puts `cssrem-lsp` on your Cargo bin PATH (`~/.cargo/bin`). The extension
finds it there automatically.

> Alternatively, build with `cargo build --release` inside `lsp/` and point Zed at
> the binary explicitly via settings (see Configuration).

### 2. Install the extension into Zed

1. Open Zed.
2. Command palette → **zed: install dev extension** (or Extensions panel →
   **Install Dev Extension**).
3. Select this repo's **root** folder.

Zed compiles the extension to WASM and loads it. Open a `.css` or `.scss` file,
type `16px`, press <kbd>Ctrl</kbd>+<kbd>Space</kbd>.

## Configuration

Set the root font size (and optionally an explicit binary path) in your Zed
`settings.json`:

```jsonc
{
  "lsp": {
    "cssrem": {
      "initialization_options": {
        "rootFontSize": 16
      },
      // Optional — only if cssrem-lsp is not on your PATH:
      "binary": {
        "path": "C:\\Users\\you\\.cargo\\bin\\cssrem-lsp.exe"
      }
    }
  }
}
```

## `.sass` (indented syntax)

The extension attaches to Zed's **built-in CSS and SCSS** languages, which cover
`.css` and `.scss`. Zed does not ship an indented-`.sass` language out of the
box, so to get completions in `.sass` files you need a Sass language available in
Zed (e.g. via a Sass language extension). Once a language named `Sass` exists,
add it to `languages` in `extension.toml`:

```toml
[language_servers.cssrem]
name = "cssrem"
languages = ["CSS", "SCSS", "Sass"]
```

The conversion logic itself is syntax-agnostic — it only inspects the token
before the cursor — so it works for any of these once Zed routes the buffer to
`cssrem`.

## How it works

The server watches open documents. On a completion request it reads the text
immediately before the cursor; if it matches `<number>px` or `<number>rem`
(e.g. `-0.5rem`, `.5px`, `24px`), it returns one completion whose `textEdit`
replaces that token with the converted value. `preselect` + a low `sortText`
make it the top entry; `filterText` keeps it matching what you already typed.

## License

MIT (add a LICENSE file as you prefer).
