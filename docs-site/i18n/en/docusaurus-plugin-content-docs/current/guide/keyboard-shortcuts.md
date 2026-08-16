---
sidebar_position: 8
---

# Keyboard Shortcuts

The easiest way to understand NyaTerm shortcuts is to split them into two groups:

1. **App-level shortcuts** — toggle panels, create sessions, copy terminal content, and so on
2. **Shell-level keys** — keys that are sent to the remote or local shell, such as `Ctrl+C`

If you want to copy text from the terminal, use NyaTerm's app-level shortcuts rather than assuming shell shortcuts become copy actions.

## Conventions

- **Ctrl / Cmd** means `Ctrl` on Windows/Linux and `Cmd` on macOS
- `Ctrl+Tab` and `Ctrl+Shift+Tab` are kept as-is because that matches the current implementation

## Terminal operations

| Shortcut | Action |
|--------|------|
| `Ctrl / Cmd + Shift + C` | Copy |
| `Ctrl / Cmd + Shift + V` | Paste |
| `Ctrl / Cmd + Shift + X` | Paste selected text |
| `Ctrl / Cmd + Shift + F` | Find |
| `Ctrl / Cmd + Shift + K` | Clear screen |
| `Ctrl / Cmd + Shift + A` | Select all |

## Tabs and workspace

| Shortcut | Action |
|--------|------|
| `Ctrl / Cmd + Shift + N` | New session |
| `Ctrl / Cmd + Shift + S` | Open Command Palette / session quick switcher |
| ``Ctrl / Cmd + ` `` | New local terminal |
| `Ctrl / Cmd + Shift + W` | Close current tab |
| `Ctrl + Tab` | Next tab |
| `Ctrl + Shift + Tab` | Previous tab |
| `Ctrl / Cmd + 1-8` | Jump to a specific tab |
| `Ctrl / Cmd + 9` | Jump to the last tab |
| `Ctrl / Cmd + Shift + D` | Duplicate current session |
| `Ctrl / Cmd + Shift + M` | Multiplex current SSH session |
| `Ctrl / Cmd + Alt + D` | Duplicate current session with a startup command |
| `Ctrl / Cmd + Alt + M` | Multiplex SSH with a startup command |

## View and panels

| Shortcut | Action |
|--------|------|
| `Ctrl / Cmd + Shift + E` | Toggle left activity bar / panel |
| `Ctrl / Cmd + Shift + B` | Toggle right activity bar / panel |
| `Ctrl / Cmd + =` | Zoom in |
| `Ctrl / Cmd + -` | Zoom out |
| `Ctrl / Cmd + 0` | Reset zoom |

## Special actions

| Shortcut | Action |
|--------|------|
| `Ctrl / Cmd + Shift + L` | Lock screen |
| `Ctrl / Cmd + ,` | Open settings |
| `Ctrl / Cmd + Shift + G` | Manage synchronized input groups |
| `Ctrl / Cmd + Alt + C` | Copy selected saved connections in the Saved Connections panel |

## Usage tips

- If you often copy logs from the terminal, memorize `Ctrl / Cmd + Shift + C`
- If you frequently switch between remote and local sessions, `Ctrl / Cmd + Shift + N` and the new-local-terminal shortcut will be your fastest entry points
- If you rely on screen lock, remember `Ctrl / Cmd + Shift + L`

:::tip
Use the current app settings and UI as the source of truth for shortcuts. If a future version adds configurable shortcuts, prefer the in-app interaction settings over this page.
:::
