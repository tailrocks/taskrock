//! Tests for routing fields.

use super::{OWNER, ROUTING, Routing, draft_key};

#[test]
fn set_then_get() {
    let mut routing = Routing::default();
    assert_eq!(routing.get("Agent"), None);
    routing.set("Agent", OWNER);
    assert_eq!(routing.get("Agent"), Some(OWNER));
}

#[test]
fn owner_means_agent_is_owner() {
    let mut routing = Routing::default();
    assert!(!routing.is_owner());
    routing.set("Agent", OWNER);
    assert!(routing.is_owner());
    routing.set("Agent", "Muse");
    assert!(!routing.is_owner());
}

#[test]
fn empty_routing_misses_everything() {
    assert_eq!(Routing::default().missing(), ROUTING);
}

#[test]
fn full_routing_misses_nothing() {
    let mut routing = Routing::default();
    for field in ROUTING {
        routing.set(field, "x");
    }
    assert_eq!(routing.missing(), [] as [&str; 0]);
}

#[test]
fn draft_keys() {
    let keys: Vec<String> = ROUTING.iter().map(|field| draft_key(field)).collect();
    assert_eq!(
        keys,
        [
            "agent",
            "runtime",
            "model",
            "effort",
            "review_agent",
            "review_runtime",
            "review_model",
            "review_effort",
        ]
    );
}
