use super::*;
use std::cell::Cell;

fn bridge(active: bool) -> EcBridge {
    let bridge = EcBridge::dummy();
    bridge.cache.lock().fan_boost = active;
    bridge
}

#[test]
fn regression_failed_boost_does_not_update_state_or_persist_and_keeps_echo_guard() {
    for initial in [false, true] {
        let bridge = bridge(initial);
        let result = bridge.apply_fan_boost(
            !initial,
            |target| {
                assert_eq!(target, !initial);
                assert_eq!(bridge.is_fan_boost(), initial);
                assert!(bridge.is_in_fan_boost_cooldown());
                bridge.cache.lock().fan_boost_cooldown_until =
                    Instant::now() - Duration::from_secs(1);
                false
            },
            |_| panic!("A failed hardware write must not persist or notify success"),
        );
        assert!(result.is_err());
        assert_eq!(bridge.is_fan_boost(), initial);
        assert!(bridge.is_in_fan_boost_cooldown());
    }
}

#[test]
fn regression_boost_commits_state_only_after_acknowledgement() {
    let bridge = bridge(false);
    let applied = Cell::new(false);
    bridge
        .apply_fan_boost(
            true,
            |_| {
                assert!(!bridge.is_fan_boost());
                assert!(bridge.is_in_fan_boost_cooldown());
                bridge.cache.lock().fan_boost_cooldown_until =
                    Instant::now() - Duration::from_secs(1);
                applied.set(true);
                true
            },
            |target| {
                assert!(applied.get());
                assert!(target && bridge.is_fan_boost());
                assert!(bridge.is_in_fan_boost_cooldown());
                Ok(())
            },
        )
        .unwrap();
}

#[test]
fn regression_persistence_failure_does_not_hide_an_acknowledged_control_change() {
    let bridge = bridge(false);
    let result = bridge.apply_fan_boost(
        true,
        |_| true,
        |_| Err("settings could not be saved".into()),
    );
    assert_eq!(result.unwrap_err(), "settings could not be saved");
    assert!(bridge.is_fan_boost());
}
