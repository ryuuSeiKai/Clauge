<p align="center">
  <img src="src-tauri/icons/clauge-mark.svg" alt="Clauge" width="96" />
</p>

<h1 align="center">Clauge</h1>

<p align="center">
  <strong>The Claude Code workspace built for developers who refuse to wait.</strong><br/>
  Parallel sessions. Smart purposes. Git built in. 6 MB.
</p>

<p align="center">
  <a href="https://github.com/ansxuman/Clauge/releases/latest"><img src="https://img.shields.io/github/v/release/ansxuman/Clauge?style=flat-square&color=1dc880&label=latest" alt="Release"></a>
  <a href="https://github.com/ansxuman/Clauge/blob/main/LICENSE"><img src="https://img.shields.io/badge/license-Apache%202.0-7c5cf8?style=flat-square" alt="License"></a>
  <a href="https://github.com/ansxuman/Clauge/stargazers"><img src="https://img.shields.io/github/stars/ansxuman/Clauge?style=flat-square&color=f5a623" alt="Stars"></a>
  <img src="https://img.shields.io/badge/built%20with-Rust%20%2B%20Tauri-CE422B?style=flat-square" alt="Rust + Tauri">
  <img src="https://img.shields.io/badge/binary-6%20MB-4f94d4?style=flat-square" alt="6 MB">
</p>

<p align="center">
  <a href="https://clauge.ssh-i.in">Website</a> ·
  <a href="https://clauge.ssh-i.in/changelog.html">Changelog</a> ·
  <a href="https://github.com/ansxuman/Clauge/releases/latest"><strong>Download for macOS →</strong></a> ·
  <a href="https://github.com/ansxuman/Clauge/issues">Report Bug</a> ·
  <a href="https://buymeacoffee.com/ansxuman">Buy me a coffee</a>
</p>

---

## Why Clauge?

Most Claude Code workflows bottleneck on a single session. You start a feature, need to review a PR, hit a bug — and now you're either context-switching in the same terminal or juggling multiple windows with no organization.

Clauge is a macOS app built specifically for Claude Code. It gives every task its own session, its own purpose, its own terminal — all organized by project, all running in parallel. Built with Rust and Tauri, the entire app fits in 6 MB.

**Wait less. Ship more.**

---

## What's a Purpose?

Every Clauge session has a purpose that shapes Claude's focus from the very first message. No more prompting Claude to "act like a code reviewer" mid-conversation.

| Purpose | What Claude focuses on |
|---|---|
| **Brainstorming** | Architecture, tradeoffs, multiple approaches — before writing a line |
| **Development** | Clean, tested, pattern-consistent code shipped in small verified steps |
| **Code Review** | Bugs, security holes, edge cases — with file and line references |
| **PR Review** | End-to-end pull request analysis: what changed, what's good, what needs work |
| **Debugging** | Root cause, not band-aids — reproduce, trace, verify the fix actually works |
| **Custom** | Import an existing Claude Code session or define your own mode |

---

## Features

### Parallel Sessions with Zero Conflicts

Run as many Claude Code sessions as you need on the same project. Each session is automatically isolated with git worktree support so they never overwrite each other. Auto-detects existing sessions and notifies you before creating duplicates.

### Full Git Integration — Without Leaving the App

- Branch indicator with ahead/behind count in the status bar
- Color-coded file changes (modified, added, deleted) with inline diff viewer
- Selective staging with per-file checkboxes
- Commit, push, pull, stash, pop — all from the UI
- Branch switching, commit history browsing, and per-session git identity (different name/email per session)

### Usage Dashboard

Know exactly where your API spend is going — without leaving your workflow.

- Total cost, API call count, cache hit rate, session count
- Daily activity chart with spending trends
- Per-model breakdown: Opus, Sonnet, Haiku
- Per-project cost with session counts
- Tool usage (Read, Edit, Bash, etc.) and shell command patterns
- Live session and weekly limits with configurable refresh
- Connect to claude.ai for real-time tracking

### Context Manager

Create reusable snippets — coding guidelines, architectural decisions, system prompts — and attach them to any session. Contexts are written to `CLAUDE.md` with safe markers that don't conflict with your existing content. Add or remove contexts while a session is running.

### Plugin Manager

Browse, install, enable, and disable Claude Code plugins without touching the terminal. One-click management from inside the app.

### Embedded Terminal

GPU-accelerated (WebGL) terminal with full color support, scrollback, resize, and per-session shell panel (`Cmd+L`). Drag-to-resize. File drag-and-drop pastes the path. No separate terminal tab required.

### macOS Polish

- Notification chime when Claude needs your input (repeats until the window is focused)
- Dock icon bounces on action-required prompts
- Close-to-tray behavior so your sessions keep running
- Auto-launch on login
- Background auto-update
- Dark/light themes with 6 accent colors

---

## Clauge vs. Alternatives

| | **Clauge** | Alternatives |
|---|:---:|:---:|
| **Binary size** | **6 MB** | ~455 MB |
| **Memory footprint** | Low | High |
| **Claude Code–specific** | ✅ Built for it | ❌ Generic agent wrapper |
| **Purpose modes** | ✅ 6 built-in | ❌ None |
| **Usage & cost dashboard** | ✅ Full analytics | ❌ Not available |
| **Context Manager** | ✅ CLAUDE.md integration | ❌ Not available |
| **Plugin Manager** | ✅ Built-in marketplace | ❌ Not available |
| **Per-session git identity** | ✅ | ❌ |
| **Inline diff viewer** | ✅ | ✅ |
| **Parallel sessions** | ✅ | ✅ |
| **Git worktree isolation** | ✅ | ✅ |
| **Notification system** | ✅ Sound + dock bounce | ✅ |
| **License** | Apache 2.0 (free forever) | Proprietary / paid tiers |

---

## Keyboard Shortcuts

| Shortcut | Action |
|---|---|
| `Cmd+N` | New session |
| `Cmd+1–9` | Switch to session |
| `Cmd+B` | Toggle sidebar |
| `Cmd+L` | Toggle shell panel |

---

## Download

<a href="https://github.com/ansxuman/Clauge/releases/latest"><strong>Download for macOS →</strong></a>

---

## Development

**Requires:** [Bun](https://bun.sh), [Rust](https://rustup.rs) 1.77+, [Tauri CLI](https://tauri.app) v2

```bash
git clone https://github.com/ansxuman/Clauge.git
cd Clauge
bun install
bun run tauri dev
```

## Tech Stack

| | |
|---|---|
| **Frontend** | SvelteKit, Svelte 5 |
| **Backend** | Rust, Tauri v2 |
| **Terminal** | xterm.js (WebGL renderer), portable-pty |

## Contributing

See [CONTRIBUTING.md](.github/CONTRIBUTING.md). Issues and PRs welcome.

## Support

<a href="https://www.buymeacoffee.com/ansxuman" target="_blank"><img src="https://cdn.buymeacoffee.com/buttons/v2/default-yellow.png" alt="Buy Me A Coffee" height="40"></a>

## License

[Apache License 2.0](LICENSE)
