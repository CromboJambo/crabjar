use crabjar_guard::domain_allowlist::{DomainAllowlist, DomainEntry, DomainTrustLevel};

#[test]
fn test_exact_domain_match() {
    let allowlist = DomainAllowlist::new();
    assert!(allowlist.check("github.com").is_ok());
    assert!(allowlist.check("crates.io").is_ok());
}

#[test]
fn test_wildcard_domain_match() {
    let allowlist = DomainAllowlist::new();
    assert!(allowlist.check("api.github.com").is_ok());
    assert!(allowlist.check("raw.githubusercontent.com").is_ok());
    assert!(allowlist.check("avatars.githubusercontent.com").is_ok());
}

#[test]
fn test_wildcard_does_not_match_prefix() {
    let allowlist = DomainAllowlist::new();
    // "notgithub.com" is not in the default allowlist
    assert!(allowlist.check("notgithub.com").is_err());
}

#[test]
fn test_domain_not_in_allowlist() {
    let allowlist = DomainAllowlist::new();
    assert!(allowlist.check("evil.com").is_err());
    assert!(allowlist.check("malicious.org").is_err());
}

#[test]
fn test_default_entries_exist() {
    let allowlist = DomainAllowlist::new();
    assert!(!allowlist.is_empty());
    assert!(allowlist.is_allowed("github.com"));
    assert!(allowlist.is_allowed("crates.io"));
    assert!(allowlist.is_allowed("localhost"));
}

#[test]
fn test_custom_entries() {
    let entries = vec![
        DomainEntry::new("custom.example.com", DomainTrustLevel::Trusted, "test"),
        DomainEntry::new("*.custom.example.com", DomainTrustLevel::Monitored, "test"),
    ];
    let allowlist = DomainAllowlist::with_entries(entries);

    assert!(allowlist.check("custom.example.com").is_ok());
    assert!(allowlist.check("api.custom.example.com").is_ok());
    assert!(allowlist.check("evil.com").is_err());
}

#[test]
fn test_trust_layer_check() {
    let entries = vec![
        DomainEntry::new("trusted.example.com", DomainTrustLevel::Trusted, "test"),
        DomainEntry::new("monitored.example.com", DomainTrustLevel::Monitored, "test"),
        DomainEntry::new(
            "restricted.example.com",
            DomainTrustLevel::Restricted,
            "test",
        ),
    ];
    let allowlist = DomainAllowlist::with_entries(entries);

    // Layer 3 (high) can access all
    assert!(
        allowlist
            .check_for_trust_layer("trusted.example.com", 3)
            .is_ok()
    );
    assert!(
        allowlist
            .check_for_trust_layer("monitored.example.com", 3)
            .is_ok()
    );
    assert!(
        allowlist
            .check_for_trust_layer("restricted.example.com", 3)
            .is_ok()
    );

    // Layer 2 (medium) can access trusted + monitored
    assert!(
        allowlist
            .check_for_trust_layer("trusted.example.com", 2)
            .is_ok()
    );
    assert!(
        allowlist
            .check_for_trust_layer("monitored.example.com", 2)
            .is_ok()
    );
    assert!(
        allowlist
            .check_for_trust_layer("restricted.example.com", 2)
            .is_err()
    );

    // Layer 1 (low) can access trusted only
    assert!(
        allowlist
            .check_for_trust_layer("trusted.example.com", 1)
            .is_ok()
    );
    assert!(
        allowlist
            .check_for_trust_layer("monitored.example.com", 1)
            .is_err()
    );
    assert!(
        allowlist
            .check_for_trust_layer("restricted.example.com", 1)
            .is_err()
    );
}

#[test]
fn test_add_and_remove_entries() {
    let mut allowlist = DomainAllowlist::new();
    assert!(!allowlist.is_allowed("new.example.com"));

    allowlist.add_entry(DomainEntry::new(
        "new.example.com",
        DomainTrustLevel::Trusted,
        "test",
    ));
    assert!(allowlist.is_allowed("new.example.com"));

    allowlist.remove_entry("new.example.com");
    assert!(!allowlist.is_allowed("new.example.com"));
}

#[test]
fn test_domain_trust_level_display() {
    assert_eq!(format!("{}", DomainTrustLevel::Trusted), "trusted");
    assert_eq!(format!("{}", DomainTrustLevel::Monitored), "monitored");
    assert_eq!(format!("{}", DomainTrustLevel::Restricted), "restricted");
}

#[test]
fn test_domain_check_error_display() {
    use crabjar_guard::domain_allowlist::DomainCheckError;

    let err = DomainCheckError::NotAllowed("evil.com".to_string());
    assert!(format!("{}", err).contains("evil.com"));

    let err = DomainCheckError::InsufficientTrustLayer {
        domain: "restricted.com".to_string(),
        required_layer: 3,
        current_layer: 2,
    };
    let msg = format!("{}", err);
    assert!(msg.contains("restricted.com"));
    assert!(msg.contains("3"));
    assert!(msg.contains("2"));
}
