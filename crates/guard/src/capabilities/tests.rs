//! Unit tests for the capabilities module (Positive Boundary Model).

use crate::capabilities::enforcer::CapabilityError;
use crate::capabilities::{
    AccessMode, ActionGrant, CapabilityEnforcer, CapabilityScope, CapabilitySet, DataGrant,
    DataSourceId, GrantOrigin, InMemoryCapabilityEnforcer, NetworkGrant, ParameterConstraints,
    Precondition, RateLimit, SideEffect, ToolGrant, ToolId, WorkerId,
};

fn test_enforcer() -> InMemoryCapabilityEnforcer {
    InMemoryCapabilityEnforcer::new()
}

#[test]
fn test_worker_id_display() {
    let id = WorkerId::new("worker-1");
    assert_eq!(format!("{}", id), "worker-1");
    assert_eq!(id.as_str(), "worker-1");
}

#[test]
fn test_tool_id_display() {
    let id = ToolId::new("read_file");
    assert_eq!(format!("{}", id), "read_file");
}

#[test]
fn test_access_mode_ordering() {
    assert!(AccessMode::ReadOnly < AccessMode::ReadWrite);
    assert!(AccessMode::ReadWrite < AccessMode::Execute);
    assert!(AccessMode::ReadOnly <= AccessMode::ReadOnly);
}

#[test]
fn test_access_mode_display() {
    assert_eq!(format!("{}", AccessMode::ReadOnly), "read-only");
    assert_eq!(format!("{}", AccessMode::ReadWrite), "read-write");
    assert_eq!(format!("{}", AccessMode::Execute), "execute");
}

#[test]
fn test_capability_set_new_is_empty() {
    let set = CapabilitySet::new("worker-1");
    assert!(set.tools.is_empty());
    assert!(set.data_sources.is_empty());
    assert!(set.actions.is_empty());
    assert!(set.network.is_none());
    assert!(!set.is_expired());
}

#[test]
fn test_capability_set_with_tool() {
    let origin = GrantOrigin::new("supervisor", "task-1");
    let set = CapabilitySet::new("worker-1").with_tool(ToolGrant::new("read_file", origin));
    assert!(set.has_tool(&ToolId::new("read_file")));
    assert!(!set.has_tool(&ToolId::new("write_file")));
}

#[test]
fn test_capability_set_with_data_source() {
    let origin = GrantOrigin::new("supervisor", "task-1");
    let scope = CapabilityScope::new().with_paths(["/data/*"]);
    let set = CapabilitySet::new("worker-1").with_data_source(DataGrant::new(
        "db-main",
        AccessMode::ReadOnly,
        scope,
        GrantOrigin::new("supervisor", "task-1"),
    ));
    assert!(set.has_data_source_with_mode(&DataSourceId::new("db-main"), &AccessMode::ReadOnly));
    assert!(!set.has_data_source_with_mode(&DataSourceId::new("db-main"), &AccessMode::ReadWrite));
}

#[test]
fn test_capability_set_with_action() {
    let origin = GrantOrigin::new("supervisor", "task-1");
    let set = CapabilitySet::new("worker-1").with_action(
        ActionGrant::new("deploy", origin.clone()).with_precondition(Precondition {
            description: "Build must pass".to_string(),
            condition: "build_status == 'passed'".to_string(),
        }),
    );
    assert!(set.has_action("deploy"));
    let grant = set.get_action("deploy").unwrap();
    assert_eq!(grant.preconditions.len(), 1);
    assert_eq!(grant.preconditions[0].description, "Build must pass");
}

#[test]
fn test_capability_set_expiration() {
    let set = CapabilitySet::new("worker-1").expires_in(1); // 1 second
    // Should not be expired immediately
    assert!(!set.is_expired());
    // Wait for expiration
    std::thread::sleep(std::time::Duration::from_secs(2));
    assert!(set.is_expired());
}

#[test]
fn test_capability_set_intersect_tools() {
    let origin = GrantOrigin::new("supervisor", "task-1");
    let set_a = CapabilitySet::new("worker-1")
        .with_tool(ToolGrant::new("read_file", origin.clone()))
        .with_tool(ToolGrant::new("write_file", origin.clone()));
    let set_b = CapabilitySet::new("worker-1")
        .with_tool(ToolGrant::new(
            "read_file",
            GrantOrigin::new("supervisor", "task-2"),
        ))
        .with_tool(ToolGrant::new(
            "list_dir",
            GrantOrigin::new("supervisor", "task-2"),
        ));

    let result = set_a.intersect(&set_b);
    assert!(result.has_tool(&ToolId::new("read_file")));
    assert!(!result.has_tool(&ToolId::new("write_file")));
    assert!(!result.has_tool(&ToolId::new("list_dir")));
}

#[test]
fn test_capability_set_intersect_data_sources() {
    let origin = GrantOrigin::new("supervisor", "task-1");
    let scope_a = CapabilityScope::new()
        .with_paths(["/data/*"])
        .with_tables(["users*"]);
    let scope_b = CapabilityScope::new()
        .with_paths(["/data/*"])
        .with_tables(["orders*"]);

    let set_a = CapabilitySet::new("worker-1").with_data_source(DataGrant::new(
        "db-main",
        AccessMode::ReadWrite,
        scope_a,
        origin.clone(),
    ));
    let set_b = CapabilitySet::new("worker-1").with_data_source(DataGrant::new(
        "db-main",
        AccessMode::ReadOnly,
        scope_b,
        GrantOrigin::new("supervisor", "task-2"),
    ));

    let result = set_a.intersect(&set_b);
    // Most restrictive mode wins
    assert!(result.has_data_source_with_mode(&DataSourceId::new("db-main"), &AccessMode::ReadOnly));
    assert!(
        !result.has_data_source_with_mode(&DataSourceId::new("db-main"), &AccessMode::ReadWrite)
    );
    // Scope intersection keeps common patterns
    let grant = result
        .data_sources
        .iter()
        .find(|d| d.source_id == DataSourceId::new("db-main"))
        .unwrap();
    assert!(grant.scope.path_in_scope("/data/file.txt"));
}

#[test]
fn test_capability_set_intersect_actions() {
    let origin = GrantOrigin::new("supervisor", "task-1");
    let set_a = CapabilitySet::new("worker-1")
        .with_action(ActionGrant::new("deploy", origin.clone()))
        .with_action(ActionGrant::new("rollback", origin.clone()));
    let set_b = CapabilitySet::new("worker-1").with_action(ActionGrant::new(
        "deploy",
        GrantOrigin::new("supervisor", "task-2"),
    ));

    let result = set_a.intersect(&set_b);
    assert!(result.has_action("deploy"));
    assert!(!result.has_action("rollback"));
}

#[test]
fn test_enforcer_grant_and_resolve() {
    let enforcer = test_enforcer();
    let origin = GrantOrigin::new("supervisor", "task-1");
    let set = CapabilitySet::new("worker-1").with_tool(ToolGrant::new("read_file", origin));

    enforcer.grant(set.clone()).unwrap();
    let resolved = enforcer.resolve(&WorkerId::new("worker-1")).unwrap();
    assert_eq!(resolved.worker_id, set.worker_id);
    assert!(resolved.has_tool(&ToolId::new("read_file")));
}

#[test]
fn test_enforcer_worker_not_found() {
    let enforcer = test_enforcer();
    let result = enforcer.resolve(&WorkerId::new("nonexistent"));
    assert!(matches!(result, Err(CapabilityError::WorkerNotFound(_))));
}

#[test]
fn test_enforcer_check_tool() {
    let enforcer = test_enforcer();
    let origin = GrantOrigin::new("supervisor", "task-1");
    let set = CapabilitySet::new("worker-1").with_tool(ToolGrant::new("read_file", origin));
    enforcer.grant(set).unwrap();

    // Granted tool
    let result = enforcer.check_tool(&WorkerId::new("worker-1"), &ToolId::new("read_file"));
    assert!(result.is_ok());
    assert!(result.unwrap().is_some());

    // Not granted tool
    let result = enforcer.check_tool(&WorkerId::new("worker-1"), &ToolId::new("write_file"));
    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
}

#[test]
fn test_enforcer_check_action() {
    let enforcer = test_enforcer();
    let origin = GrantOrigin::new("supervisor", "task-1");
    let set = CapabilitySet::new("worker-1").with_action(
        ActionGrant::new("deploy", origin.clone()).with_side_effect(SideEffect {
            description: "Deploys to production".to_string(),
            resource: "prod-cluster".to_string(),
        }),
    );
    enforcer.grant(set).unwrap();

    let result = enforcer.check_action(&WorkerId::new("worker-1"), "deploy");
    assert!(result.is_ok());
    let grant = result.unwrap().unwrap();
    assert_eq!(grant.side_effects.len(), 1);
}

#[test]
fn test_enforcer_check_data_access() {
    let enforcer = test_enforcer();
    let origin = GrantOrigin::new("supervisor", "task-1");
    let scope = CapabilityScope::new().with_paths(["/data/*"]);
    let set = CapabilitySet::new("worker-1").with_data_source(DataGrant::new(
        "db-main",
        AccessMode::ReadOnly,
        scope,
        GrantOrigin::new("supervisor", "task-1"),
    ));
    enforcer.grant(set).unwrap();

    // Read access granted
    let result = enforcer.check_data(
        &WorkerId::new("worker-1"),
        &DataSourceId::new("db-main"),
        &AccessMode::ReadOnly,
    );
    assert!(result.is_ok());
    assert!(result.unwrap().is_some());

    // Write access denied
    let result = enforcer.check_data(
        &WorkerId::new("worker-1"),
        &DataSourceId::new("db-main"),
        &AccessMode::ReadWrite,
    );
    assert!(matches!(result, Err(CapabilityError::AccessDenied { .. })));
}

#[test]
fn test_enforcer_visible_tools() {
    let enforcer = test_enforcer();
    let origin = GrantOrigin::new("supervisor", "task-1");
    let set = CapabilitySet::new("worker-1")
        .with_tool(ToolGrant::new("read_file", origin.clone()))
        .with_tool(ToolGrant::new("list_dir", origin));
    enforcer.grant(set).unwrap();

    let tools = enforcer.visible_tools(&WorkerId::new("worker-1")).unwrap();
    assert_eq!(tools.len(), 2);
    assert!(tools.iter().any(|t| t.tool_id == ToolId::new("read_file")));
    assert!(tools.iter().any(|t| t.tool_id == ToolId::new("list_dir")));
}

#[test]
fn test_enforcer_visible_sources() {
    let enforcer = test_enforcer();
    let origin = GrantOrigin::new("supervisor", "task-1");
    let scope = CapabilityScope::new().with_paths(["/data/*"]);
    let set = CapabilitySet::new("worker-1").with_data_source(DataGrant::new(
        "db-main",
        AccessMode::ReadOnly,
        scope,
        GrantOrigin::new("supervisor", "task-1"),
    ));
    enforcer.grant(set).unwrap();

    let sources = enforcer
        .visible_sources(&WorkerId::new("worker-1"))
        .unwrap();
    assert_eq!(sources.len(), 1);
    assert_eq!(sources[0].source_id, DataSourceId::new("db-main"));
    assert_eq!(sources[0].mode, AccessMode::ReadOnly);
}

#[test]
fn test_enforcer_revoke() {
    let enforcer = test_enforcer();
    let origin = GrantOrigin::new("supervisor", "task-1");
    let set = CapabilitySet::new("worker-1").with_tool(ToolGrant::new("read_file", origin));
    enforcer.grant(set).unwrap();

    // Revoke
    enforcer.revoke(&WorkerId::new("worker-1")).unwrap();

    // Should not be found anymore
    let result = enforcer.resolve(&WorkerId::new("worker-1"));
    assert!(matches!(result, Err(CapabilityError::WorkerNotFound(_))));
}

#[test]
fn test_enforcer_expired_capability() {
    let enforcer = test_enforcer();
    let origin = GrantOrigin::new("supervisor", "task-1");
    let set = CapabilitySet::new("worker-1")
        .with_tool(ToolGrant::new("read_file", origin))
        .expires_in(1); // 1 second
    enforcer.grant(set).unwrap();

    // Wait for expiration
    std::thread::sleep(std::time::Duration::from_secs(2));

    let result = enforcer.resolve(&WorkerId::new("worker-1"));
    assert!(matches!(result, Err(CapabilityError::Expired { .. })));
}

#[test]
fn test_observer_capabilities() {
    let caps = InMemoryCapabilityEnforcer::observer_capabilities("observer-1");
    assert!(caps.has_tool(&ToolId::new("record_observation")));
    assert!(caps.has_tool(&ToolId::new("read_waypoints")));
    assert!(caps.has_tool(&ToolId::new("read_lineage")));
    // No write capabilities
    assert!(!caps.has_tool(&ToolId::new("write_file")));
}

#[test]
fn test_path_pattern_matching() {
    use crate::capabilities::PathPattern;
    let pattern = PathPattern::new("/data/*");
    assert!(pattern.matches("/data/file.txt"));
    assert!(pattern.matches("/data/subdir/file.txt"));
    assert!(!pattern.matches("/other/file.txt"));
}

#[test]
fn test_tool_grant_with_rate_limit() {
    let origin = GrantOrigin::new("supervisor", "task-1");
    let grant = ToolGrant::new("search", origin)
        .with_rate_limit(RateLimit {
            max_calls: 10,
            window_seconds: 60,
        })
        .with_parameters(ParameterConstraints {
            allowed_params: vec!["query".to_string()],
            required_params: vec!["query".to_string()],
        });
    assert!(grant.rate_limit.is_some());
    assert_eq!(grant.rate_limit.as_ref().unwrap().max_calls, 10);
    assert_eq!(grant.parameters.allowed_params.len(), 1);
}

#[test]
fn test_network_grant() {
    let origin = GrantOrigin::new("supervisor", "task-1");
    let grant = NetworkGrant::new(["api.example.com".to_string()], origin).with_ports([443, 8080]);
    assert!(grant.host_allowed("api.example.com"));
    assert!(!grant.host_allowed("evil.com"));
    assert!(grant.port_allowed(443));
    assert!(grant.port_allowed(8080));
    assert!(!grant.port_allowed(22));
}
