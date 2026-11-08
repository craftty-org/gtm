//! Connectivity state (primitive provided by `tuikit`).
//!
//! GTM keeps this module path stable for daemon/TUI callers.

pub use tuikit::http::connectivity::{
    Connectivity, ConnectivityReading, FRESHNESS, OFFLINE_STREAK, ONLINE_STREAK, Verdict,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requires_streaks() {
        let mut v = Verdict::settled(Connectivity::Unknown);
        assert_eq!(v.observe(true), None);
        assert_eq!(v.observe(true), Some(Connectivity::Online));
        assert_eq!(v.observe(false), None);
        assert_eq!(v.observe(false), Some(Connectivity::Offline));
    }

    #[test]
    fn ages_to_stale() {
        let now = 1_000_000i64;
        let r = ConnectivityReading {
            state: Connectivity::Online,
            checked_at: now,
        };
        assert_eq!(r.now(now), Connectivity::Online);
        assert_eq!(r.effective_state(now), Connectivity::Online);
    }
}
