// Deterministic simulation testing primitives.
//
// Gated behind `test-support`/`cfg(test)` - never compiled into release
// binaries. Provides a single-threaded, seeded, tick-driven world for testing
// state machines without real time, sockets, fs, or audio devices.
#![allow(dead_code)]

pub mod mixer;

use std::cmp::Ordering;
use std::collections::BinaryHeap;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct Tick(pub u64);

#[derive(Clone, Copy, Debug)]
pub struct SimEvent {
    pub at: Tick,
    pub seq: u64,
    pub kind: SimEventKind,
}

#[derive(Clone, Copy, Debug)]
pub enum SimEventKind {
    Wake,
    NetworkDeliver { msg_id: u64 },
    Fault { id: u64 },
    Custom { id: u64 },
}

impl Eq for SimEvent {}
impl PartialEq for SimEvent {
    fn eq(&self, other: &Self) -> bool {
        self.at == other.at && self.seq == other.seq
    }
}
impl Ord for SimEvent {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .at
            .cmp(&self.at)
            .then_with(|| other.seq.cmp(&self.seq))
    }
}
impl PartialOrd for SimEvent {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Tiny deterministic RNG (xorshift64*) for reproducible test decisions.
#[derive(Clone, Debug)]
pub struct SimRng(u64);

impl SimRng {
    pub fn new(seed: u64) -> Self {
        Self(if seed == 0 { 0x9e3779b97f4a7c15 } else { seed })
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }
    pub fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / ((1u64 << 53) as f64)
    }
    pub fn gen_range(&mut self, upper: u64) -> u64 {
        if upper == 0 {
            0
        } else {
            self.next_u64() % upper
        }
    }
    pub fn chance(&mut self, p: f64) -> bool {
        self.next_f64() < p
    }
}

#[derive(Debug, Default)]
pub struct Trace {
    entries: Vec<String>,
}

impl Trace {
    pub fn push(&mut self, tick: Tick, msg: impl Into<String>) {
        self.entries.push(format!("[t={}] {}", tick.0, msg.into()));
    }
    pub fn entries(&self) -> &[String] {
        &self.entries
    }
    pub fn dump(&self) -> String {
        self.entries.join("\n")
    }
}

pub struct SimWorld {
    pub seed: u64,
    pub tick: Tick,
    pub rng: SimRng,
    pub trace: Trace,
    seq: u64,
    queue: BinaryHeap<SimEvent>,
}

impl SimWorld {
    pub fn new(seed: u64) -> Self {
        let mut w = Self {
            seed,
            tick: Tick(0),
            rng: SimRng::new(seed),
            trace: Trace::default(),
            seq: 0,
            queue: BinaryHeap::new(),
        };
        w.trace.push(Tick(0), format!("seed={seed}"));
        w
    }

    pub fn schedule(&mut self, at: Tick, kind: SimEventKind) {
        let seq = self.seq;
        self.seq += 1;
        self.queue.push(SimEvent { at, seq, kind });
    }

    pub fn schedule_in(&mut self, ticks: u64, kind: SimEventKind) {
        self.schedule(Tick(self.tick.0.saturating_add(ticks)), kind);
    }

    pub fn advance_tick(&mut self) -> Vec<SimEvent> {
        self.tick = Tick(self.tick.0 + 1);
        let mut out = Vec::new();
        while self.queue.peek().is_some_and(|e| e.at <= self.tick) {
            out.push(self.queue.pop().unwrap());
        }
        out
    }

    pub fn tick_until<F>(&mut self, max_ticks: u64, mut f: F) -> bool
    where
        F: FnMut(&mut Self) -> bool,
    {
        if f(self) {
            return true;
        }
        for _ in 0..max_ticks {
            let _events = self.advance_tick();
            if f(self) {
                return true;
            }
        }
        false
    }

    pub fn log(&mut self, msg: impl Into<String>) {
        let tick = self.tick;
        self.trace.push(tick, msg);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::Mixer;
    use crate::dst::mixer::SimMixer;

    #[test]
    fn sim_world_is_deterministic() {
        fn run(seed: u64) -> Vec<String> {
            let mut sim = SimWorld::new(seed);
            let mut mixer = SimMixer::new();
            mixer.load_active("a", 0.0).unwrap();
            mixer.set_duration(2.0);
            mixer.play().unwrap();
            sim.tick_until(100, |_| {
                mixer.advance(0.1);
                !mixer.is_playing()
            });
            sim.trace.entries().to_vec()
        }
        assert_eq!(run(123), run(123));
        assert_ne!(run(1), run(2));
    }

    #[test]
    fn sim_rng_is_deterministic() {
        let mut a = SimRng::new(99);
        let mut b = SimRng::new(99);
        for _ in 0..100 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }
}
