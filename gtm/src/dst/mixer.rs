use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

use rodio::Source;

use crate::audio::backend::{AudioEvent, AudioResult};
use crate::audio::mixer::Mixer;
use crate::audio::symphonia::StreamingReopen;
use crate::shared::MAX_VOLUME;
use crate::shared::global::{EqPreset, ReverbConfig};

/// Deterministic mixer for DST. No audio device, no threads, no real time.
/// Call `advance(dt_secs)` to move playback forward.
pub struct SimMixer {
    playing: AtomicBool,
    volume: AtomicU8,
    position: Mutex<f64>,
    duration: Mutex<f64>,
    crossfading: Mutex<bool>,
    crossfade_remaining: Mutex<f64>,
    standby_loaded: Mutex<bool>,
    standby_duration: Mutex<f64>,
    finished_pending: Mutex<bool>,
    events: Mutex<Vec<AudioEvent>>,
    mono: AtomicBool,
    visualizer_enabled: AtomicBool,
}

impl Default for SimMixer {
    fn default() -> Self {
        Self::new()
    }
}

impl SimMixer {
    pub fn new() -> Self {
        Self {
            playing: AtomicBool::new(false),
            volume: AtomicU8::new(MAX_VOLUME),
            position: Mutex::new(0.0),
            duration: Mutex::new(0.0),
            crossfading: Mutex::new(false),
            crossfade_remaining: Mutex::new(0.0),
            standby_loaded: Mutex::new(false),
            standby_duration: Mutex::new(0.0),
            finished_pending: Mutex::new(false),
            events: Mutex::new(Vec::new()),
            mono: AtomicBool::new(false),
            visualizer_enabled: AtomicBool::new(false),
        }
    }

    /// Deterministically advance playback by `dt` seconds. Emits events as needed.
    pub fn advance(&self, dt: f64) {
        if !self.playing.load(Ordering::SeqCst) {
            return;
        }
        let mut pos = self.position.lock().unwrap();
        let dur = *self.duration.lock().unwrap();
        *pos += dt.max(0.0);
        let mut cf = self.crossfading.lock().unwrap();
        if *cf {
            let mut rem = self.crossfade_remaining.lock().unwrap();
            *rem -= dt.max(0.0);
            if *rem <= 0.0 {
                *cf = false;
                *rem = 0.0;
                let standby_dur = *self.standby_duration.lock().unwrap();
                *self.duration.lock().unwrap() = standby_dur;
                *pos = 0.0;
                *self.standby_loaded.lock().unwrap() = false;
            }
        }
        if dur > 0.0 && *pos >= dur && !*cf {
            *pos = dur;
            self.playing.store(false, Ordering::SeqCst);
            *self.finished_pending.lock().unwrap() = true;
            self.events.lock().unwrap().push(AudioEvent::Finished);
        }
    }

    pub fn push_event(&self, ev: AudioEvent) {
        self.events.lock().unwrap().push(ev);
    }

    pub fn set_duration(&self, dur: f64) {
        *self.duration.lock().unwrap() = dur;
    }

    pub fn set_standby_duration(&self, dur: f64) {
        *self.standby_duration.lock().unwrap() = dur;
    }
}

impl Mixer for SimMixer {
    fn load_active(&mut self, _path: &str, start_pos: f64) -> AudioResult<()> {
        *self.position.lock().unwrap() = start_pos.max(0.0);
        *self.duration.lock().unwrap() = 0.0;
        *self.crossfading.lock().unwrap() = false;
        *self.crossfade_remaining.lock().unwrap() = 0.0;
        *self.finished_pending.lock().unwrap() = false;
        self.playing.store(false, Ordering::SeqCst);
        Ok(())
    }

    fn load_active_decoded(
        &mut self,
        _source: Box<dyn Source<Item = f32> + Send>,
        start_pos: f64,
    ) -> AudioResult<()> {
        self.load_active("", start_pos)
    }

    fn load_active_stream(
        &mut self,
        _source: Box<dyn Source<Item = f32> + Send>,
        start_pos: f64,
        duration_secs: f64,
    ) -> AudioResult<()> {
        self.load_active("", start_pos)?;
        *self.duration.lock().unwrap() = duration_secs.max(0.0);
        Ok(())
    }

    fn load_active_reader(
        &mut self,
        _reader: Box<dyn std::io::Read + Send>,
        _reopen: Option<Box<dyn StreamingReopen>>,
        start_pos: f64,
    ) -> AudioResult<()> {
        self.load_active("", start_pos)
    }

    fn dropped_samples(&self) -> u64 {
        0
    }

    fn load_standby(&mut self, _path: &str) -> AudioResult<()> {
        *self.standby_loaded.lock().unwrap() = true;
        *self.standby_duration.lock().unwrap() = 0.0;
        Ok(())
    }

    fn load_standby_decoded(
        &mut self,
        _source: Box<dyn Source<Item = f32> + Send>,
    ) -> AudioResult<()> {
        self.load_standby("")
    }

    fn load_standby_reader(
        &mut self,
        _reader: Box<dyn std::io::Read + Send>,
        _reopen: Option<Box<dyn StreamingReopen>>,
    ) -> AudioResult<()> {
        self.load_standby("")
    }

    fn standby_is_loaded(&self) -> bool {
        *self.standby_loaded.lock().unwrap()
    }

    fn play(&mut self) -> AudioResult<()> {
        self.playing.store(true, Ordering::SeqCst);
        Ok(())
    }
    fn pause(&mut self) -> AudioResult<()> {
        self.playing.store(false, Ordering::SeqCst);
        Ok(())
    }
    fn stop(&mut self) -> AudioResult<()> {
        self.playing.store(false, Ordering::SeqCst);
        *self.position.lock().unwrap() = 0.0;
        *self.duration.lock().unwrap() = 0.0;
        *self.crossfading.lock().unwrap() = false;
        *self.standby_loaded.lock().unwrap() = false;
        Ok(())
    }
    fn seek(&mut self, position_secs: f64) -> AudioResult<()> {
        let dur = *self.duration.lock().unwrap();
        let mut pos = self.position.lock().unwrap();
        *pos = if dur > 0.0 {
            position_secs.clamp(0.0, dur)
        } else {
            position_secs.max(0.0)
        };
        Ok(())
    }
    fn set_volume(&mut self, volume: u8) -> AudioResult<()> {
        self.volume.store(volume.min(MAX_VOLUME), Ordering::SeqCst);
        Ok(())
    }
    fn volume(&self) -> u8 {
        self.volume.load(Ordering::SeqCst)
    }
    fn is_playing(&self) -> bool {
        self.playing.load(Ordering::SeqCst)
    }
    fn current_position(&self) -> f64 {
        *self.position.lock().unwrap()
    }
    fn duration(&self) -> f64 {
        *self.duration.lock().unwrap()
    }
    fn active_remaining(&self) -> f64 {
        let d = *self.duration.lock().unwrap();
        let p = *self.position.lock().unwrap();
        (d - p).max(0.0)
    }
    fn start_crossfade(&mut self, duration_secs: f64) {
        if *self.standby_loaded.lock().unwrap() {
            *self.crossfading.lock().unwrap() = true;
            *self.crossfade_remaining.lock().unwrap() = duration_secs.max(0.0);
        }
    }
    fn is_crossfading(&self) -> bool {
        *self.crossfading.lock().unwrap()
    }
    fn force_complete_crossfade(&mut self) {
        if *self.crossfading.lock().unwrap() {
            *self.crossfading.lock().unwrap() = false;
            *self.crossfade_remaining.lock().unwrap() = 0.0;
            *self.duration.lock().unwrap() = *self.standby_duration.lock().unwrap();
            *self.position.lock().unwrap() = 0.0;
            *self.standby_loaded.lock().unwrap() = false;
        }
    }
    fn cancel_crossfade(&mut self) {
        *self.crossfading.lock().unwrap() = false;
        *self.crossfade_remaining.lock().unwrap() = 0.0;
        *self.standby_loaded.lock().unwrap() = false;
    }
    fn drop_active(&mut self) {
        self.playing.store(false, Ordering::SeqCst);
        *self.position.lock().unwrap() = 0.0;
        *self.duration.lock().unwrap() = 0.0;
    }
    fn poll(&mut self) -> AudioResult<Option<AudioEvent>> {
        let mut ev = self.events.lock().unwrap();
        if ev.is_empty() {
            Ok(None)
        } else {
            Ok(Some(ev.remove(0)))
        }
    }
    fn current_peak_level(&self) -> f32 {
        0.0
    }
    fn current_spectrum(&self) -> Vec<f32> {
        Vec::new()
    }
    fn set_eq_preset(&self, _preset: &EqPreset) {}
    fn set_eq_enabled(&self, _enabled: bool) {}
    fn set_pre_gain(&self, _db: f32) {}
    fn set_reverb(&self, _config: &ReverbConfig) {}
    fn set_mono(&self, enabled: bool) {
        self.mono.store(enabled, Ordering::SeqCst);
    }
    fn mono(&self) -> bool {
        self.mono.load(Ordering::SeqCst)
    }
    fn set_visualizer_enabled(&self, enabled: bool) {
        self.visualizer_enabled.store(enabled, Ordering::SeqCst);
    }
}
