mod support;

use palpites_back::application::shared::Clock;
use support::FixedClock;

#[test]
fn fixed_clock_returns_its_configured_instant() {
    let clock = FixedClock::new("2026-06-18 12:00:00");

    assert_eq!(clock.now().as_str(), "2026-06-18 12:00:00");
    assert_eq!(clock.now(), clock.now());
}
