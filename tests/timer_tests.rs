// v0.3.244: Timer API tests
// Tests for setTimeout, setInterval, setImmediate and their clear counterparts

#[cfg(test)]
mod timer_tests {
    use amberjs::nodejs_core::timers::{
        get_next_timer_id, get_timer_epoch, TimerMetadata, TimerType, TIMER_METADATA,
    };
    use serial_test::serial;

    #[test]
    fn test_timer_id_generation() {
        let id1 = get_next_timer_id();
        let id2 = get_next_timer_id();
        let id3 = get_next_timer_id();

        assert!(id1 > 0);
        assert!(id2 > id1);
        assert!(id3 > id2);
    }

    #[test]
    #[serial]
    fn test_timer_metadata_storage() {
        // Test storing timer metadata
        let timer_id = get_next_timer_id();

        {
            let mut metadata = TIMER_METADATA.lock().unwrap();
            metadata.insert(
                timer_id,
                TimerMetadata {
                    timer_type: TimerType::Timeout,
                    delay: 1000,
                    is_unrefed: false,
                    epoch: get_timer_epoch(),
                },
            );
        }

        // Verify storage
        {
            let metadata = TIMER_METADATA.lock().unwrap();
            assert!(metadata.contains_key(&timer_id));

            let timer_meta = metadata.get(&timer_id).unwrap();
            assert_eq!(timer_meta.timer_type, TimerType::Timeout);
            assert_eq!(timer_meta.delay, 1000);
            assert!(!timer_meta.is_unrefed);
        }
    }

    #[test]
    fn test_timer_type_variants() {
        assert_eq!(TimerType::Timeout, TimerType::Timeout);
        assert_eq!(TimerType::Interval, TimerType::Interval);
        assert_eq!(TimerType::Immediate, TimerType::Immediate);
    }

    #[test]
    #[serial]
    fn test_interval_metadata() {
        let timer_id = get_next_timer_id();

        {
            let mut metadata = TIMER_METADATA.lock().unwrap();
            metadata.insert(
                timer_id,
                TimerMetadata {
                    timer_type: TimerType::Interval,
                    delay: 500,
                    is_unrefed: false,
                    epoch: get_timer_epoch(),
                },
            );
        }

        {
            let metadata = TIMER_METADATA.lock().unwrap();
            let timer_meta = metadata.get(&timer_id).unwrap();
            assert_eq!(timer_meta.timer_type, TimerType::Interval);
            assert_eq!(timer_meta.delay, 500);
        }
    }

    #[test]
    #[serial]
    fn test_immediate_metadata() {
        let timer_id = get_next_timer_id();

        {
            let mut metadata = TIMER_METADATA.lock().unwrap();
            metadata.insert(
                timer_id,
                TimerMetadata {
                    timer_type: TimerType::Immediate,
                    delay: 0,
                    is_unrefed: false,
                    epoch: get_timer_epoch(),
                },
            );
        }

        {
            let metadata = TIMER_METADATA.lock().unwrap();
            let timer_meta = metadata.get(&timer_id).unwrap();
            assert_eq!(timer_meta.timer_type, TimerType::Immediate);
            assert_eq!(timer_meta.delay, 0);
        }
    }

    #[test]
    #[serial]
    fn test_clear_timer() {
        let timer_id = get_next_timer_id();

        // Add timer
        {
            let mut metadata = TIMER_METADATA.lock().unwrap();
            metadata.insert(
                timer_id,
                TimerMetadata {
                    timer_type: TimerType::Timeout,
                    delay: 1000,
                    is_unrefed: false,
                    epoch: get_timer_epoch(),
                },
            );
        }

        // Verify it exists
        {
            let metadata = TIMER_METADATA.lock().unwrap();
            assert!(metadata.contains_key(&timer_id));
        }

        // Clear timer
        {
            let mut metadata = TIMER_METADATA.lock().unwrap();
            metadata.remove(&timer_id);
        }

        // Verify it's gone
        {
            let metadata = TIMER_METADATA.lock().unwrap();
            assert!(!metadata.contains_key(&timer_id));
        }
    }

    #[test]
    #[serial]
    fn test_clear_all_timers() {
        // Add multiple timers
        for i in 1..=5 {
            let timer_id = get_next_timer_id();
            let mut metadata = TIMER_METADATA.lock().unwrap();
            metadata.insert(
                timer_id,
                TimerMetadata {
                    timer_type: if i % 2 == 0 {
                        TimerType::Interval
                    } else {
                        TimerType::Timeout
                    },
                    delay: i * 100,
                    is_unrefed: false,
                    epoch: get_timer_epoch(),
                },
            );
        }

        // Verify timers exist
        {
            let metadata = TIMER_METADATA.lock().unwrap();
            assert!(metadata.len() >= 5);
        }

        // Clear all timers
        {
            let mut metadata = TIMER_METADATA.lock().unwrap();
            metadata.clear();
        }

        // Verify all timers are gone
        {
            let metadata = TIMER_METADATA.lock().unwrap();
            assert!(metadata.is_empty());
        }
    }

    #[test]
    #[serial]
    fn test_unref_state() {
        let timer_id = get_next_timer_id();

        {
            let mut metadata = TIMER_METADATA.lock().unwrap();
            metadata.insert(
                timer_id,
                TimerMetadata {
                    timer_type: TimerType::Timeout,
                    delay: 1000,
                    is_unrefed: false,
                    epoch: get_timer_epoch(),
                },
            );
        }

        // Update unref state
        {
            let mut metadata = TIMER_METADATA.lock().unwrap();
            if let Some(timer_meta) = metadata.get_mut(&timer_id) {
                timer_meta.is_unrefed = true;
            }
        }

        // Verify unref state
        {
            let metadata = TIMER_METADATA.lock().unwrap();
            let timer_meta = metadata.get(&timer_id).unwrap();
            assert!(timer_meta.is_unrefed);
        }
    }

    #[test]
    #[serial]
    fn test_zero_delay_timeout() {
        // setTimeout with delay 0 should execute immediately
        let timer_id = get_next_timer_id();

        {
            let mut metadata = TIMER_METADATA.lock().unwrap();
            metadata.insert(
                timer_id,
                TimerMetadata {
                    timer_type: TimerType::Timeout,
                    delay: 0,
                    is_unrefed: false,
                    epoch: get_timer_epoch(),
                },
            );
        }

        {
            let metadata = TIMER_METADATA.lock().unwrap();
            let timer_meta = metadata.get(&timer_id).unwrap();
            assert_eq!(timer_meta.delay, 0);
        }
    }

    #[test]
    #[serial]
    fn test_various_delays() {
        // Test various delay values
        let delays = [0, 1, 10, 100, 1000, 5000];

        for (i, delay) in delays.iter().enumerate() {
            let timer_id = get_next_timer_id();
            let mut metadata = TIMER_METADATA.lock().unwrap();
            metadata.insert(
                timer_id,
                TimerMetadata {
                    timer_type: TimerType::Timeout,
                    delay: *delay,
                    is_unrefed: false,
                    epoch: get_timer_epoch(),
                },
            );

            // Verify
            let timer_meta = metadata.get(&timer_id).unwrap();
            assert_eq!(timer_meta.delay, *delay, "Delay mismatch at index {}", i);
        }
    }
}
