//! Integration tests for the event bus subsystem — subscriber fanout and capacity.

use crabjar_host_core::{EventBus, EventType};
use std::sync::Arc;

#[tokio::test]
async fn test_event_bus_multiple_subscribers() {
    let bus = Arc::new(EventBus::new(64));

    // Create two subscribers
    let mut sub1 = bus.subscriber();
    let mut sub2 = bus.subscriber();

    // Publish an event
    bus.publish_typed(EventType::Tick { interval_ms: 100 }, "test")
        .unwrap();

    // Both subscribers should receive it
    let e1 = tokio::time::timeout(tokio::time::Duration::from_millis(100), sub1.recv())
        .await
        .unwrap()
        .unwrap();
    let e2 = tokio::time::timeout(tokio::time::Duration::from_millis(100), sub2.recv())
        .await
        .unwrap()
        .unwrap();

    assert_eq!(e1.kind, EventType::Tick { interval_ms: 100 });
    assert_eq!(e2.kind, EventType::Tick { interval_ms: 100 });
}

#[tokio::test]
async fn test_event_bus_capacity_overflow() {
    // Small capacity to force overflow
    let bus = EventBus::new(4);

    // Create a subscriber to receive events
    let mut sub = bus.subscriber();

    // Publish more events than capacity
    let mut success_count = 0;
    for i in 0..10 {
        let result = bus.publish_typed(EventType::Tick { interval_ms: i }, "test");
        if result.is_ok() {
            success_count += 1;
        }
    }

    // At least some should succeed (capacity allows at least 4)
    assert!(
        success_count >= 4,
        "Expected at least 4 successful publishes, got {}",
        success_count
    );

    // Drain the subscriber
    while tokio::time::timeout(tokio::time::Duration::from_millis(10), sub.recv())
        .await
        .is_ok()
    {}
}
