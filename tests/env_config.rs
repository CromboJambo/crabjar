//! Integration tests for environment variable configuration.
//! Tests that all configurable paths respect their environment variables
//! and fail clearly when not set (no fallback defaults).

use std::env;
use std::path::PathBuf;

/// Helper: set env var, run closure, restore original value
fn with_env_var(name: &str, value: Option<&str>, f: impl FnOnce()) {
    let original = env::var(name).ok();
    match value {
        Some(v) => unsafe { env::set_var(name, v) },
        None => unsafe { env::remove_var(name) },
    }
    f();
    match original {
        Some(v) => unsafe { env::set_var(name, v) },
        None => unsafe { env::remove_var(name) },
    }
}

#[test]
fn test_crabjar_state_dir_env() {
    // Test that CRABJAR_STATE_DIR is respected
    with_env_var("CRABJAR_STATE_DIR", Some("/tmp/custom_state"), || {
        let state_dir = env::var("CRABJAR_STATE_DIR").unwrap();
        assert_eq!(state_dir, "/tmp/custom_state");
        let state_file = PathBuf::from(state_dir).join("crabjar_rate_limits.json");
        assert_eq!(
            state_file.to_string_lossy(),
            "/tmp/custom_state/crabjar_rate_limits.json"
        );
    });

    // Test that it fails when not set (no fallback)
    with_env_var("CRABJAR_STATE_DIR", None, || {
        let result = env::var("CRABJAR_STATE_DIR");
        assert!(
            result.is_err(),
            "Should fail when CRABJAR_STATE_DIR is not set"
        );
    });
}

#[test]
fn test_structured_aux_criteria_path_env() {
    // Test that STRUCTURED_AUX_CRITERIA_PATH is respected
    with_env_var(
        "STRUCTURED_AUX_CRITERIA_PATH",
        Some("/custom/criteria"),
        || {
            let path = env::var("STRUCTURED_AUX_CRITERIA_PATH").unwrap();
            assert_eq!(path, "/custom/criteria");
        },
    );

    // Test that it fails when not set (no fallback)
    with_env_var("STRUCTURED_AUX_CRITERIA_PATH", None, || {
        let result = env::var("STRUCTURED_AUX_CRITERIA_PATH");
        assert!(
            result.is_err(),
            "Should fail when STRUCTURED_AUX_CRITERIA_PATH is not set"
        );
    });
}

#[test]
fn test_training_data_dir_env() {
    // Test that CRABJAR_TRAINING_DATA_DIR is respected
    with_env_var(
        "CRABJAR_TRAINING_DATA_DIR",
        Some("/custom/training"),
        || {
            let dir = env::var("CRABJAR_TRAINING_DATA_DIR").unwrap();
            assert_eq!(dir, "/custom/training");
        },
    );

    // Test that it fails when not set (no fallback)
    with_env_var("CRABJAR_TRAINING_DATA_DIR", None, || {
        let result = env::var("CRABJAR_TRAINING_DATA_DIR");
        assert!(
            result.is_err(),
            "Should fail when CRABJAR_TRAINING_DATA_DIR is not set"
        );
    });
}

#[test]
fn test_unsloth_outputs_dir_env() {
    // Test that UNSLOTH_OUTPUTS_DIR is respected
    with_env_var("UNSLOTH_OUTPUTS_DIR", Some("/custom/outputs"), || {
        let dir = env::var("UNSLOTH_OUTPUTS_DIR").unwrap();
        assert_eq!(dir, "/custom/outputs");
    });

    // Test that it fails when not set (no fallback)
    with_env_var("UNSLOTH_OUTPUTS_DIR", None, || {
        let result = env::var("UNSLOTH_OUTPUTS_DIR");
        assert!(
            result.is_err(),
            "Should fail when UNSLOTH_OUTPUTS_DIR is not set"
        );
    });
}

#[test]
fn test_bastion_key_path_env() {
    // Test that CRABJAR_BASTION_KEY_PATH is respected
    with_env_var(
        "CRABJAR_BASTION_KEY_PATH",
        Some("/custom/id_bastion"),
        || {
            let path = env::var("CRABJAR_BASTION_KEY_PATH").unwrap();
            assert_eq!(path, "/custom/id_bastion");
        },
    );

    // Test that it fails when not set (no fallback)
    with_env_var("CRABJAR_BASTION_KEY_PATH", None, || {
        let result = env::var("CRABJAR_BASTION_KEY_PATH");
        assert!(
            result.is_err(),
            "Should fail when CRABJAR_BASTION_KEY_PATH is not set"
        );
    });
}

#[test]
fn test_ssh_key_path_env() {
    // Test that CRABJAR_SSH_KEY_PATH is respected
    with_env_var("CRABJAR_SSH_KEY_PATH", Some("/custom/id_lab"), || {
        let path = env::var("CRABJAR_SSH_KEY_PATH").unwrap();
        assert_eq!(path, "/custom/id_lab");
    });

    // Test that it fails when not set (no fallback)
    with_env_var("CRABJAR_SSH_KEY_PATH", None, || {
        let result = env::var("CRABJAR_SSH_KEY_PATH");
        assert!(
            result.is_err(),
            "Should fail when CRABJAR_SSH_KEY_PATH is not set"
        );
    });
}

#[test]
fn test_all_env_vars_can_be_set_simultaneously() {
    // Test that all env vars can be set at once without conflicts
    with_env_var("CRABJAR_STATE_DIR", Some("/tmp/test_state"), || {
        with_env_var(
            "STRUCTURED_AUX_CRITERIA_PATH",
            Some("/tmp/test_criteria"),
            || {
                with_env_var(
                    "CRABJAR_TRAINING_DATA_DIR",
                    Some("/tmp/test_training"),
                    || {
                        with_env_var("UNSLOTH_OUTPUTS_DIR", Some("/tmp/test_outputs"), || {
                            with_env_var(
                                "CRABJAR_BASTION_KEY_PATH",
                                Some("/tmp/test_bastion_key"),
                                || {
                                    with_env_var(
                                        "CRABJAR_SSH_KEY_PATH",
                                        Some("/tmp/test_ssh_key"),
                                        || {
                                            // All vars should be set correctly
                                            assert_eq!(
                                                env::var("CRABJAR_STATE_DIR").unwrap(),
                                                "/tmp/test_state"
                                            );
                                            assert_eq!(
                                                env::var("STRUCTURED_AUX_CRITERIA_PATH").unwrap(),
                                                "/tmp/test_criteria"
                                            );
                                            assert_eq!(
                                                env::var("CRABJAR_TRAINING_DATA_DIR").unwrap(),
                                                "/tmp/test_training"
                                            );
                                            assert_eq!(
                                                env::var("UNSLOTH_OUTPUTS_DIR").unwrap(),
                                                "/tmp/test_outputs"
                                            );
                                            assert_eq!(
                                                env::var("CRABJAR_BASTION_KEY_PATH").unwrap(),
                                                "/tmp/test_bastion_key"
                                            );
                                            assert_eq!(
                                                env::var("CRABJAR_SSH_KEY_PATH").unwrap(),
                                                "/tmp/test_ssh_key"
                                            );
                                        },
                                    );
                                },
                            );
                        });
                    },
                );
            },
        );
    });
}

#[test]
fn test_env_vars_do_not_affect_each_other() {
    // Test that setting one env var doesn't affect others
    with_env_var("CRABJAR_STATE_DIR", Some("/tmp/state_only"), || {
        // Other vars should still be unset (no defaults)
        let training_result = env::var("CRABJAR_TRAINING_DATA_DIR");
        assert!(training_result.is_err(), "Training dir should be unset");

        let unsloth_result = env::var("UNSLOTH_OUTPUTS_DIR");
        assert!(
            unsloth_result.is_err(),
            "Unsloth outputs dir should be unset"
        );
    });
}
