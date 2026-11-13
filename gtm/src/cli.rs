// Copyright (c) 2026
// Author: prjctimg <prjctimg@outlook.com>
// CLI subcommand dispatch via daemon IPC
//
// This is free software released under the GPL-3.0 license.

use std::io::Write;
use std::path::PathBuf;

use crate::shared::client::DaemonClient;
use crate::shared::daemon::ensure_daemon_running;
use crate::shared::global::{PlaybackStatus, RepeatMode};
use crate::shared::ipc::HealthStatus;
use crate::shared::resolve_command_socket;
use clap::{Parser, Subcommand};

use crate::app::{Prefs, ensure_prefs_file};
use crate::footer::format_uptime;

#[derive(Parser)]
#[command(
    name = "gtm",
    version = option_env!("CARGO_PKG_VERSION").unwrap_or("0.1.0"),
    about = "gtm music player"
)]
pub struct Args {
    #[arg(long, short, help = "Run in CLI mode instead of TUI")]
    pub cli: bool,

    #[arg(long, short, global = true, help = "Verbose output")]
    pub verbose: bool,

    #[arg(long, short, global = true, help = "Daemon socket path")]
    pub socket: Option<String>,

    #[arg(long, short, global = true, help = "Output as JSON (CLI mode only)")]
    pub json: bool,

    /// Write a shell completion script to stdout and exit
    #[arg(long, value_name = "SHELL", hide = true)]
    pub completions: Option<clap_complete::Shell>,

    #[command(subcommand)]
    pub command: Option<CliCommand>,
}

#[derive(Subcommand)]
pub enum CliCommand {
    /// Play an audio file, URL, or resume playback
    Play {
        #[arg(value_name = "PATH", value_hint = clap::ValueHint::FilePath)]
        path: String,
        #[arg(value_name = "SECONDS")]
        start_pos: Option<f64>,
    },
    /// Play an HTTP(S) stream URL
    Stream {
        #[arg(value_name = "URL")]
        url: String,
    },
    /// Toggle play/pause
    PlayPause,
    /// Pause playback
    Pause,
    /// Stop playback
    Stop,
    /// Skip to next track
    Next,
    /// Skip to previous track
    Prev,
    /// Seek to a position in seconds
    Seek { position_secs: f64 },
    /// Get or set the volume (0-100)
    Volume {
        /// Volume percentage; omit to read the current volume
        volume: Option<u8>,
    },
    /// Toggle shuffle
    Shuffle,
    /// Set repeat mode (off, one, all)
    Repeat {
        #[arg(value_name = "MODE", value_parser = ["off", "one", "all"])]
        mode: String,
    },
    /// Toggle mute
    Mute,
    /// Show current playback status
    Status {
        /// Redraw the status line continuously
        #[arg(long)]
        stream: bool,
    },
    /// Show the current queue
    Queue,
    /// Add audio files or directories to the queue
    QueueAdd {
        #[arg(value_name = "PATH", value_hint = clap::ValueHint::AnyPath, num_args = 1..)]
        paths: Vec<String>,
        #[arg(long, value_name = "INDEX")]
        position: Option<u64>,
    },
    /// Remove a track from the queue by index
    QueueRemove { index: u64 },
    /// Clear the queue
    QueueClear,
    /// Ping the daemon
    Ping,
    /// Check daemon health
    CheckHealth,
    /// Quit the daemon
    Quit,
    /// Manage config file
    Config {
        #[arg(long)]
        reset: bool,
        #[arg(long)]
        validate: bool,
    },
}

pub fn run(socket: Option<String>, json: bool, verbose: bool, cmd: &CliCommand) {
    if let CliCommand::Config { reset, validate } = cmd {
        if *reset {
            let path = ensure_prefs_file();
            let _ = std::fs::remove_file(&path);
            let path = ensure_prefs_file();
            println!("config reset to defaults at {}", path.display());
        } else if *validate {
            let path = ensure_prefs_file();
            match std::fs::read_to_string(&path) {
                Ok(contents) => match toml::from_str::<Prefs>(&contents) {
                    Ok(_) => println!("config is valid"),
                    Err(e) => {
                        eprintln!("config error: {e}");
                        std::process::exit(1);
                    }
                },
                Err(e) => {
                    eprintln!("failed to read config: {e}");
                    std::process::exit(1);
                }
            }
        } else if let Err(e) = open_config() {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
        return;
    }

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result: Result<String, String> = rt.block_on(async {
        let socket_path = socket
            .clone()
            .map(PathBuf::from)
            .unwrap_or_else(resolve_command_socket);

        if !matches!(cmd, CliCommand::Ping | CliCommand::Quit) {
            ensure_daemon_running(&socket_path).await?;
        }

        let client = DaemonClient::connect(&socket_path)
            .await
            .map_err(|e| format!("Failed to connect to daemon at {socket_path:?}: {e}"))?;

        match cmd {
            CliCommand::Play { path, start_pos } => {
                let pos = start_pos.unwrap_or(0.0);
                let path = match std::path::absolute(path) {
                    Ok(p) => p.to_string_lossy().into_owned(),
                    Err(_) => path.clone(),
                };
                if !std::path::Path::new(&path).exists() {
                    Err(format!("no such file: {path}"))
                } else {
                    client.play(&path, pos).await.map(|()| "ok".into()).map_err(|e| e.to_string())
                }
            }
            CliCommand::Stream { url } => client
                .play_stream(url)
                .await
                .map(|()| "ok".into())
                .map_err(|e| e.to_string()),
            CliCommand::PlayPause => client.play_pause().await.map(|()| "ok".into()).map_err(|e| e.to_string()),
            CliCommand::Pause => client.pause().await.map(|()| "ok".into()).map_err(|e| e.to_string()),
            CliCommand::Stop => client.stop().await.map(|()| "ok".into()).map_err(|e| e.to_string()),
            CliCommand::Next => {
                client.next().await.map_err(|e| e.to_string())?;
                if verbose {
                    match client.get_status().await {
                        Ok(state) => Ok(format_now_playing(&state)),
                        Err(_) => Ok("ok".into()),
                    }
                } else {
                    Ok("ok".into())
                }
            }
            CliCommand::Prev => {
                client.prev().await.map_err(|e| e.to_string())?;
                if verbose {
                    match client.get_status().await {
                        Ok(state) => Ok(format_now_playing(&state)),
                        Err(_) => Ok("ok".into()),
                    }
                } else {
                    Ok("ok".into())
                }
            }
            CliCommand::Seek { position_secs } => client.seek(*position_secs).await.map(|()| "ok".into()).map_err(|e| e.to_string()),
            CliCommand::Volume { volume } => match volume {
                Some(v) => {
                    client.set_volume(*v).await.map_err(|e| e.to_string())?;
                    if verbose { Ok(format!("Volume: {v}%")) } else { Ok("ok".into()) }
                }
                None => Ok(format!("{}", client.get_volume().await.map_err(|e| e.to_string())?)),
            },
            CliCommand::Shuffle => client.toggle_shuffle().await.map(|()| "ok".into()).map_err(|e| e.to_string()),
            CliCommand::Repeat { mode } => {
                let mode: RepeatMode = mode.parse().map_err(|e: String| e)?;
                client.cycle_repeat(mode).await.map(|()| "ok".into()).map_err(|e| e.to_string())
            }
            CliCommand::Mute => client.toggle_mute().await.map(|()| "ok".into()).map_err(|e| e.to_string()),
            CliCommand::Status { stream } => {
                if *stream {
                    loop {
                        let state = client.get_status().await.map_err(|e| e.to_string())?;
                        let elapsed = state.time_pos as u64;
                        let dur = state.duration as u64;
                        let track = state.current_track.as_ref().map_or("No track".into(), |t| {
                            let title = t.display_title();
                            if t.artist.is_empty() { title } else { format!("{} - {}", t.artist, title) }
                        });
                        let status_label = match state.status {
                            PlaybackStatus::Playing => "Playing",
                            PlaybackStatus::Paused => "Paused",
                            PlaybackStatus::Stopped => "Stopped",
                        };
                        print!("\x1b[H\x1b[J{status_label}: {track} | {elapsed}s / {dur}s | {}%", state.volume);
                        std::io::stdout().flush().ok();
                        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                    }
                } else {
                    let state = client.get_status().await.map_err(|e| e.to_string())?;
                    if json {
                        serde_json::to_string_pretty(&state).map_err(|e| e.to_string())
                    } else {
                        Ok(format_status(&state))
                    }
                }
            }
            CliCommand::Queue => {
                let res = client.queue().list().await.map_err(|e| e.to_string())?;
                if json { serde_json::to_string_pretty(&res).map_err(|e| e.to_string()) } else { Ok(format!("{res:?}")) }
            }
            CliCommand::QueueAdd { paths, position } => client.queue().add_many(paths.clone(), *position).await.map(|()| "ok".into()).map_err(|e| e.to_string()),
            CliCommand::QueueRemove { index } => client.queue().remove(*index).await.map(|()| "ok".into()).map_err(|e| e.to_string()),
            CliCommand::QueueClear => client.queue().clear().await.map(|()| "ok".into()).map_err(|e| e.to_string()),
            CliCommand::Ping => { client.ping().await.map_err(|e| e.to_string())?; Ok("pong".into()) }
            CliCommand::CheckHealth => {
                let report = client.check_health().await.map_err(|e| e.to_string())?;
                if json {
                    serde_json::to_string_pretty(&report).map_err(|e| e.to_string())
                } else {
                    let mut out = format!(
                        "\x1b[1mgtm Health Report\x1b[0m (v{})\nDaemon uptime: {}\n",
                        report.version,
                        format_uptime(report.daemon_uptime_secs)
                    );
                    for c in &report.components {
                        let icon = match c.status {
                            HealthStatus::Ok => "\x1b[32m✓\x1b[0m",
                            HealthStatus::Degraded => "\x1b[33m⚠\x1b[0m",
                            HealthStatus::Error => "\x1b[31m✗\x1b[0m",
                        };
                        out += &format!("  {icon} \x1b[1m{}\x1b[0m", c.name);
                        if let Some(ref msg) = c.message { out += &format!(": {msg}"); }
                        if let Some(uptime) = c.uptime_secs { out += &format!(" (uptime {:.0}s)", uptime); }
                        out += "\n";
                    }
                    Ok(out)
                }
            }
            CliCommand::Quit => client.quit().await.map(|()| "ok".into()).map_err(|e| e.to_string()),
            CliCommand::Config { .. } => unreachable!(),
        }
    });

    match result {
        Ok(msg) => println!("{msg}"),
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    }
}

fn format_now_playing(state: &crate::shared::global::DaemonState) -> String {
    if let Some(ref t) = state.current_track {
        format!("Now Playing: {}\n{} - {}", t.title, t.artist, t.album)
    } else {
        "Stopped".into()
    }
}

fn format_status(state: &crate::shared::global::DaemonState) -> String {
    let status_str = match state.status {
        PlaybackStatus::Playing => "\x1b[32m▶ Playing\x1b[0m",
        PlaybackStatus::Paused => "\x1b[33m⏸ Paused\x1b[0m",
        PlaybackStatus::Stopped => "\x1b[31m⏹ Stopped\x1b[0m",
    };
    let track_str = state.current_track.as_ref().map(|t| {
        let title = match t.display_title() { s if s.is_empty() => "Unknown".to_string(), s => s };
        if t.artist.is_empty() { title } else { format!("{}: {}", t.artist, title) }
    }).unwrap_or_else(|| "No track".into());
    format!(
        "\x1b[1mPlayback:\x1b[0m  {status_str}\n\
         \x1b[1mTrack:\x1b[0m    {track_str}\n\
         \x1b[1mVolume:\x1b[0m   \x1b[36m{}%\x1b[0m ({})\n\
         \x1b[1mRepeat:\x1b[0m   {:?}\n\
         \x1b[1mShuffle:\x1b[0m  {}\n\
         \x1b[1mQueue:\x1b[0m    {} tracks, cursor {}/{}",
        state.volume,
        if state.mute { "\x1b[33mMuted\x1b[0m" } else { "Unmuted" },
        state.repeat,
        if state.shuffle { "\x1b[32mOn\x1b[0m" } else { "Off" },
        state.queue.len(),
        state.queue_cursor + 1,
        state.queue.len().max(1),
    )
}

fn open_config() -> Result<(), String> {
    let path = ensure_prefs_file();
    let editor = pick_editor().ok_or_else(|| {
        format!("no editor found: set $VISUAL or $EDITOR to open {}", path.display())
    })?;
    let program = editor[0].clone();
    let status = std::process::Command::new(&program)
        .args(&editor[1..])
        .arg(&path)
        .status()
        .map_err(|e| format!("failed to launch editor `{program}`: {e}"))?;
    if status.success() {
        println!("Opened config at {}", path.display());
        Ok(())
    } else {
        Err(format!("editor `{program}` exited with status {status}"))
    }
}

fn pick_editor() -> Option<Vec<String>> {
    for var in ["VISUAL", "EDITOR"] {
        if let Ok(val) = std::env::var(var) {
            let parts: Vec<String> = val.split_whitespace().map(String::from).collect();
            if let Some(program) = parts.first() {
                if command_exists(program) { return Some(parts); }
            }
        }
    }
    for name in ["vim", "nvim", "vi", "nano", "micro", "emacs", "ed"] {
        if command_exists(name) { return Some(vec![name.to_string()]); }
    }
    None
}

fn command_exists(name: &str) -> bool {
    std::env::var_os("PATH")
        .map(|paths| std::env::split_paths(&paths).any(|dir| dir.join(name).is_file()))
        .unwrap_or(false)
}
