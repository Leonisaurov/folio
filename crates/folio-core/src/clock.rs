use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use jiff::{Timestamp, Zoned};

/// Reloj inyectable; los tests nunca dependen de la hora real.
pub trait Clock: Send + Sync {
    fn now(&self) -> Timestamp;

    fn now_rfc3339(&self) -> String {
        self.now().to_string()
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Timestamp {
        Timestamp::now()
    }
}

/// Reloj fijo que avanza manualmente en los tests.
#[derive(Debug, Clone)]
pub struct FixedClock {
    millis: Arc<AtomicI64>,
}

impl FixedClock {
    pub fn new(ts: Timestamp) -> Self {
        Self {
            millis: Arc::new(AtomicI64::new(ts.as_millisecond())),
        }
    }

    pub fn at(rfc3339: &str) -> Self {
        let ts: Timestamp = rfc3339.parse().expect("timestamp RFC 3339 válido");
        Self::new(ts)
    }

    pub fn advance_ms(&self, ms: i64) {
        self.millis.fetch_add(ms, Ordering::SeqCst);
    }

    pub fn zoned(&self, tz: &jiff::tz::TimeZone) -> Zoned {
        self.now().to_zoned(tz.clone())
    }
}

impl Clock for FixedClock {
    fn now(&self) -> Timestamp {
        Timestamp::from_millisecond(self.millis.load(Ordering::SeqCst)).expect("milisegundo válido")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_clock_advances() {
        let clock = FixedClock::at("2026-10-29T09:00:00Z");
        let before = clock.now();
        clock.advance_ms(1500);
        assert_eq!(clock.now().as_millisecond() - before.as_millisecond(), 1500);
    }
}
