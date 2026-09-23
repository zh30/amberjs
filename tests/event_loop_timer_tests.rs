use amberjs::event_loop::{AsyncTimerManager, TimerScheduleError};
use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
async fn shorter_deadline_fires_before_longer_deadline() {
    let manager = AsyncTimerManager::new();

    manager
        .schedule_timeout(Duration::from_millis(50), 10, || {})
        .unwrap();
    manager
        .schedule_timeout(Duration::from_millis(10), 20, || {})
        .unwrap();

    sleep(Duration::from_millis(80)).await;

    assert_eq!(manager.poll_fired_timers(), vec![20, 10]);
}

#[tokio::test]
async fn same_delay_timers_fire_in_insertion_order() {
    let manager = AsyncTimerManager::new();

    manager
        .schedule_timeout(Duration::from_millis(10), 200, || {})
        .unwrap();
    manager
        .schedule_timeout(Duration::from_millis(10), 100, || {})
        .unwrap();

    sleep(Duration::from_millis(40)).await;

    assert_eq!(manager.poll_fired_timers(), vec![200, 100]);
}

#[test]
fn scheduling_reports_queue_full() {
    let manager = AsyncTimerManager::new_with_command_queue_size(0);

    let err = manager
        .try_schedule_timeout(Duration::from_millis(10), 300, || {})
        .unwrap_err();

    assert_eq!(err, TimerScheduleError::QueueFull);
}

#[test]
fn schedule_timeout_reports_queue_full() {
    let manager = AsyncTimerManager::new_with_command_queue_size(0);

    let err = manager
        .schedule_timeout(Duration::from_millis(10), 301, || {})
        .unwrap_err();

    assert_eq!(err, TimerScheduleError::QueueFull);
}

#[tokio::test]
async fn scheduling_reports_channel_closed_after_shutdown() {
    let manager = AsyncTimerManager::new();

    manager.shutdown_with_ack().await.unwrap();
    let err = manager
        .try_schedule_timeout(Duration::from_millis(10), 400, || {})
        .unwrap_err();

    assert_eq!(err, TimerScheduleError::ChannelClosed);
}

#[tokio::test]
async fn schedule_interval_reports_channel_closed_after_shutdown() {
    let manager = AsyncTimerManager::new();

    manager.shutdown_with_ack().await.unwrap();
    let err = manager
        .schedule_interval(Duration::from_millis(10), 0, 401, || {})
        .unwrap_err();

    assert_eq!(err, TimerScheduleError::ChannelClosed);
}

#[tokio::test]
async fn cancel_with_ack_prevents_pending_timer_from_firing() {
    let manager = AsyncTimerManager::new();

    manager
        .try_schedule_timeout(Duration::from_millis(50), 500, || {})
        .unwrap();

    assert!(manager.cancel_with_ack(500).await.unwrap());

    sleep(Duration::from_millis(80)).await;
    assert_eq!(manager.poll_fired_timers(), Vec::<u64>::new());
}
