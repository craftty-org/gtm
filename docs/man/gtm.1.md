% gtm(1) gtm client manual
% prjctimg
% 2026

# NAME

gtm - terminal user interface and command-line client for the gtm music daemon

# SYNOPSIS

**gtm** [**\--socket**=*path*] [**\--cli**] [*command* [*args*]]

# DESCRIPTION

**gtm** is the client for **gtmd**(1). When invoked without the **\--cli** flag, it
opens a full-screen Terminal User Interface (TUI) with keyboard-driven
navigation. With **\--cli** (or **-c**), it acts as a command-line client for
scripting and headless control.

For the full keybinding reference, configuration options and setup, see the
project documentation.

# TUI MODE (default)

The TUI opens on the **Library** view; there are no tabs. Three panes are
available and **[** / **]** cycle between them.

## Library

Browse tracks through the sidebar: Library, Playlists, Spotify, Radio,
Top Charts, Podcasts. The left pane selects the view, the centre pane lists its
contents. **Library** is the whole library and every synced playlist in one
list, grouped five ways: **]** / **[** step between Tracks, Albums, Artists,
Genres and Folders. **Playlists** is playlists and the three history lists in
one row, grouped four ways by the same keys. The pair also steps the source
filter of an open picker. Keys: **Tab** / **Shift+Tab** (focus the next /
previous pane), **j**/**k** or **Up**/**Down** (navigate), **Enter** (drill down
or play), **/** (contextual search), **Alt+S** (cycle sort in the List context).

## Settings

Settings is a floating picker opened with **Alt+,**, not a tab. The left pane
selects a category, the right pane shows its options. Keys: **j**/**k**
(navigate), **Enter** (toggle/select). The System category renders 19 rows,
including **Daydream** — how long the TUI must be idle before the visualizer
takes over (see *Daydreaming*).

### Lyrics view

Press **l** to fetch lyrics for the current track (LRCLIB, an `.lrc`/`.srt`/
timed `.json` sidecar next to the audio file, or the offline cache). When
timestamps are available the active line is highlighted and auto-follows the
playback position; enhanced-LRC sources light up per word. **]** moves focus
into the lyrics pane, where **j**/**k**, **PageUp**/**PageDown**, **Home**/
**End** scroll manually, and **[** takes it back. Untimed lyrics
are shown as untimestamped lines and never get a highlight, and the pane says *not time
synced*.

## Daydreaming

With no keypress, paste or click for **daydream_secs** (default off) the TUI
shows the audio visualizer full screen, over the library view. Any input ends
it immediately. It only fires while something is playing — with playback
stopped the visualizer has nothing to draw — and it is suppressed while **z**
Zen is up, while a picker is open, and when the `visualizer` extension is
disabled.

Zen (**z**) and daydreaming are the same renderer: Zen on whichever of its
three surfaces was last selected, daydreaming always on the visualizer. **Tab**
and **Shift+Tab** cycle Zen's surfaces.

The idle threshold is a System setting row (`daydream_secs`); pressing Enter
cycles 15s, 30s, 1m, 3m and off. `0` disables it. There is no key toggle for
the visualizer itself: it is either on a surface or not.

## Discord presence

Rich Presence is set by the **daemon**, not the client, and needs the Discord
desktop application running on the same machine — the daemon connects to its
local IPC socket.

1. Create an application at <https://discord.com/developers/applications> and
   copy its numeric **Application ID**.
2. Paste it into **Alt+X** → Discord, or set `discord_app_id` in
   `$XDG_CONFIG_HOME/gtm/config.toml` (the TUI also writes this key).

The daemon re-reads that file every two seconds, so no restart is needed.
To turn it off, clear the field in the TUI or remove `discord_app_id` from config.

## Global Keys

| Key | Action |
|-----|--------|
| `Space` | Play / Pause |
| `n` / `p` | Next / Previous track |
| `+` / `-` | Volume up / down (5 per press) |
| `m` | Toggle mute |
| `r` | Cycle repeat mode |
| `S` | Toggle shuffle |
| `s` | Stop |
| `.` / `,` | Seek forward / backward (±5 s per press) |
| `Alt+r` | Radio Browser and cliamp (custom + top stations, tags/countries, search) |
| `Alt+T` | Cycle theme |
| `Alt+S` | Cycle library sort order |
| `Alt+O` | Play an HTTP(S) stream URL |
| `Alt+p` | Podcast feeds |
| `Alt+1` | Toggle mono |
| `Alt+/` | Search the library |
| `Alt+l` | Add the current track to a Spotify playlist |
| `Alt+,` | Settings |
| `Alt+.` | Pick a library to show |
| `Alt+a` | About |
| `Alt+c` | Theme picker |
| `Alt+e` | Equalizer |
| `Alt+n` | Notifications |
| `Alt+q` | Queue |
| `Alt+s` | Search Spotify (requires linking) |
| `Alt+v` | Visualizer preset |
| `Alt+x` | Setup walkthrough (Spotify, Last.fm, YouTube, Discord) |
| `Alt+y` | Search YouTube |
| `Alt+z` | Sleep timer |
| `l` | Fetch lyrics for current track |
| `i` | Track info — what the daemon knows about the current track |
| `:` | Command mode |
| `Q` / `Ctrl+Q` | Quit and stop the daemon |
| `q` | Quit the client |
| `Esc` | Close the top overlay, or pop a drill-down step |

# CLI MODE

With **\--cli** (or **-c**), **gtm** sends a single command to **gtmd**(1) and prints the result. Use **\--json** for machine-readable output where supported. Browsing, library management, integrations, and advanced setup are intentionally left to the TUI or IPC clients (see **gtmd-ipc**(1)).

## Response/output convention

- Success commands print `ok` (or a short confirmation) unless noted.
- Read commands print human-readable text by default; with **\--json** they emit the daemon response/state object for that command.
- `status --stream` continuously redraws a compact status line until interrupted.
- Errors are printed to stderr and exit non-zero.

## Essential commands

**play** *path* [*start_pos*]
:   Play a local file path (resolved relative to the client cwd) or resume from *start_pos* seconds.

**stream** *url*
:   Play an HTTP(S) stream URL.

**play-pause** | **pause** | **stop** | **next** | **prev**
:   Transport controls.

**seek** *position_secs*
:   Seek within the current track.

**volume** [*volume*]
:   Get current volume, or set 0-100.

**shuffle** | **mute**
:   Toggle shuffle or mute.

**repeat** {off|one|all}
:   Set repeat mode.

**status** [**--stream**]
:   Show playback state. `--stream` redraws continuously; `--json` returns daemon status state.

**queue**
:   Show queue. `--json` returns queue state.

**queue-add** *path*... [**--position** *index*]
:   Add files/directories to the queue (optionally at *index*).

**queue-remove** *index*
:   Remove a queue entry by index.

**queue-clear**
:   Clear the queue.

**ping** | **check-health** | **quit**
:   Daemon liveness/health and shutdown. `check-health --json` returns the health report.

**config** [**--validate**] [**--reset**]
:   Open the client config in `$VISUAL`/`$EDITOR`, validate it, or reset to defaults.


# OPTIONS

**\--socket**, **-s** *path*
:   Path to the daemon's Unix socket.

**\--cli**, **-c**
:   Run in CLI mode instead of TUI.

**\--json**, **-j**
:   Output as JSON (CLI mode only).

**\--verbose**, **-v**
:   Enable verbose output (global).

**\--version**, **-V**
:   Show version information.

**\--help**, **-h**
:   Show help message.

# ENVIRONMENT

`XDG_RUNTIME_DIR`
:   Used to derive the default socket path.

`XDG_CONFIG_HOME`, `XDG_DATA_HOME`, `XDG_CACHE_HOME`
:   Config, data and cache roots (default `~/.config`, `~/.local/share`,
    `~/.cache`).

`GTM_THEME_MODE`
:   Force OS-theme detection to `dark` / `light`. Honored only when
    `theme_mode = "auto"`.

`GTM_NERD_FONTS`
:   Set to `0` / `false` / `no` to force plain-ASCII glyphs.

`GTM_YT_HOSTS`
:   Register extra YouTube-family search providers.

`GTM_SPOTIFY_PORT`
:   Override the Spotify OAuth callback port (default 8990). There is no
    Last.fm equivalent: its desktop flow has no callback.

`GTK_THEME`, `XDG_STATE_HOME`
:   Probed for the OS theme (a trailing `-dark` means dark) and the Omarchy
    `colors.toml` respectively.

`RUST_LOG`
:   Daemon log verbosity, e.g. `RUST_LOG=gtm=debug`. Overrides `--verbose`.

# FILES

$XDG_RUNTIME_DIR/gtm/gtmd.sock
:   Default daemon IPC socket.

/tmp/gtm-$USER/gtm/gtmd.sock
:   Fallback socket path if $XDG_RUNTIME_DIR is not set.

$TMPDIR/gtm/gtmd.sock
:   Further fallback.

$HOME/.gtm/gtm/gtmd.sock
:   Final fallback.

$XDG_CONFIG_HOME/gtm/radios.toml
:   Custom radio stations used by radio features (managed via TUI/IPC or by editing the file directly). Default: `~/.config/gtm/radios.toml`.

$XDG_CONFIG_HOME/gtm/footer.toml
:   Optional user footer presets. Built-in presets are **Default**, **Minimal**
    and **Full**; the Default footer shows the platform icon (`System`) instead
    of the audio output device or backend — add the `Device` or `Backend`
    module to a preset when that detail is needed. A legacy `middle` key still
    parses so old files load, but its modules are discarded; use `left`/`right`.
    Individual modules cannot be toggled from the TUI.

$XDG_CONFIG_HOME/gtm/themes/*.toml
:   User theme files. All 18 colour fields are required — a file missing any of
    them is skipped silently, so the theme never appears in the picker. A user
    theme whose `name` matches a built-in replaces it.

$XDG_CONFIG_HOME/gtm/config.toml
:   Client preferences, re-read every 120 frames when the file's mtime
    changes. See **config**(1) above for the enum values that silently reset
    the file when mistyped.

# SEE ALSO

**gtmd**(1), **gtmd-ipc**(1)

# AUTHORS

prjctimg <prjctimg@outlook.com>

# BUGS

Report bugs to <https://github.com/craftty-org/gtm/issues> or by email to
<prjctimg@outlook.com>.

# COPYRIGHT

Copyright (c) 2026 prjctimg.

This is free software released under the GPL-3.0 license. See the LICENSE
file for the full license text.