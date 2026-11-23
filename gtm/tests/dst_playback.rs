use gtm::audio::{AudioEvent, Mixer};
use gtm::dst::mixer::SimMixer;
use gtm::dst::SimWorld;

#[test]
fn dst_playback_advances_deterministically() {
    let mut sim = SimWorld::new(42);
    let mut mixer = SimMixer::new();
    mixer.load_active("track.mp3", 0.0).unwrap();
    mixer.set_duration(5.0);
    mixer.play().unwrap();

    let ok = sim.tick_until(1000, |s| {
        mixer.advance(0.1);
        s.log(format!("pos={:.1}", mixer.current_position()));
        mixer.current_position() >= 3.0
    });
    assert!(ok, "seed={} trace=\n{}", sim.seed, sim.trace.dump());
    assert!((mixer.current_position() - 3.0).abs() < 0.001);
}

#[test]
fn dst_playback_finishes_and_emits_event() {
    let mut sim = SimWorld::new(7);
    let mut mixer = SimMixer::new();
    mixer.load_active("track.mp3", 0.0).unwrap();
    mixer.set_duration(1.0);
    mixer.play().unwrap();

    let finished = sim.tick_until(200, |_s| {
        mixer.advance(0.1);
        !mixer.is_playing() && mixer.current_position() >= 1.0
    });
    assert!(finished, "seed={} trace=\n{}", sim.seed, sim.trace.dump());
    let ev = mixer.poll().unwrap();
    assert!(matches!(ev, Some(AudioEvent::Finished)));
}

#[test]
fn dst_seek_clamps_to_duration() {
    let mut mixer = SimMixer::new();
    mixer.load_active("track.mp3", 0.0).unwrap();
    mixer.set_duration(10.0);
    mixer.seek(20.0).unwrap();
    assert_eq!(mixer.current_position(), 10.0);
    mixer.seek(-5.0).unwrap();
    assert_eq!(mixer.current_position(), 0.0);
}

#[test]
fn dst_crossfade_completes_deterministically() {
    let mut sim = SimWorld::new(99);
    let mut mixer = SimMixer::new();
    mixer.load_active("a.mp3", 0.0).unwrap();
    mixer.set_duration(10.0);
    mixer.play().unwrap();
    mixer.load_standby("b.mp3").unwrap();
    mixer.set_standby_duration(8.0);
    mixer.start_crossfade(1.0);
    assert!(mixer.is_crossfading());

    let done = sim.tick_until(50, |_s| {
        mixer.advance(0.2);
        !mixer.is_crossfading()
    });
    assert!(done, "seed={} trace=\n{}", sim.seed, sim.trace.dump());
    assert!(!mixer.standby_is_loaded());
}
