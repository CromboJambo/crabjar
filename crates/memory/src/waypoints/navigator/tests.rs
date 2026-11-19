use super::*;
use crate::waypoints::schema as wp_schema;
use tempfile::tempdir;

fn test_navigator() -> SqliteWaypointNavigator {
    let dir = tempdir().unwrap();
    let conn = Connection::open(dir.path().join("test.db")).unwrap();
    wp_schema::migrate(&conn).unwrap();
    SqliteWaypointNavigator::new(conn)
}

fn test_env() -> EnvironmentId {
    EnvironmentId("test-env".to_string())
}

#[test]
fn create_and_get_waypoint() {
    let nav = test_navigator();
    let wp = Waypoint::new(
        test_env(),
        WaypointCategory::Orientation,
        "Test orientation",
    );
    let created = nav.create(&wp).unwrap();
    assert_eq!(created.id, wp.id);
    assert_eq!(created.description, "Test orientation");

    let fetched = nav.get(&created.id).unwrap();
    assert_eq!(fetched.id, created.id);
}

#[test]
fn set_and_get_root() {
    let nav = test_navigator();
    let env = test_env();
    let root_wp = Waypoint::new(env.clone(), WaypointCategory::Orientation, "Root");
    let created = nav.create(&root_wp).unwrap();
    nav.set_root(&env, &created.id).unwrap();

    let root = nav.root(&env).unwrap();
    assert_eq!(root.id, created.id);
}

#[test]
fn children_query() {
    let nav = test_navigator();
    let env = test_env();
    let parent = Waypoint::new(env.clone(), WaypointCategory::Orientation, "Parent");
    let parent = nav.create(&parent).unwrap();

    let child1 = Waypoint::new(env.clone(), WaypointCategory::Structural, "Child 1")
        .with_parent(parent.id.clone());
    let child2 = Waypoint::new(env.clone(), WaypointCategory::Task, "Child 2")
        .with_parent(parent.id.clone());

    nav.create(&child1).unwrap();
    nav.create(&child2).unwrap();

    let children = nav.children(&parent.id).unwrap();
    assert_eq!(children.len(), 2);
}

#[test]
fn traverse_follows_destinations() {
    let nav = test_navigator();
    let env = test_env();

    let root = Waypoint::new(env.clone(), WaypointCategory::Orientation, "Root");
    let root = nav.create(&root).unwrap();

    let task = Waypoint::new(env.clone(), WaypointCategory::Task, "Target Task");
    let task = nav.create(&task).unwrap();

    // Link root to task via destinations
    let mut root_with_dest = root.clone();
    root_with_dest.destinations = vec![task.id.clone()];
    nav.update(&root_with_dest).unwrap();

    let path = nav.traverse("test-worker", &root.id, 3).unwrap();
    assert_eq!(path.total_hops, 1);
    assert!(path.reached_target);
    assert_eq!(path.waypoints.len(), 2);
}

#[test]
fn find_by_category_in_subtree() {
    let nav = test_navigator();
    let env = test_env();

    let root = Waypoint::new(env.clone(), WaypointCategory::Orientation, "Root");
    let root = nav.create(&root).unwrap();

    let structural = Waypoint::new(env.clone(), WaypointCategory::Structural, "Structural info")
        .with_parent(root.id.clone());
    let structural = nav.create(&structural).unwrap();

    let task = Waypoint::new(env.clone(), WaypointCategory::Task, "A task")
        .with_parent(structural.id.clone());
    nav.create(&task).unwrap();

    let results = nav
        .find_by_category(&root.id, WaypointCategory::Task)
        .unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].category, WaypointCategory::Task);
}

#[test]
fn lineage_follows_predecessors() {
    let nav = test_navigator();
    let env = test_env();

    let source = Waypoint::new(env.clone(), WaypointCategory::Orientation, "Source");
    let source = nav.create(&source).unwrap();

    let compressed = Waypoint::new(env.clone(), WaypointCategory::Orientation, "Compressed")
        .with_predecessor(source.id.clone());
    let compressed = nav.create(&compressed).unwrap();

    let lineage = nav.lineage(&compressed.id).unwrap();
    assert_eq!(lineage.len(), 2);
    assert_eq!(lineage[0].id, source.id); // Source first (oldest)
    assert_eq!(lineage[1].id, compressed.id);
}

#[test]
fn freshness_check_works() {
    let nav = test_navigator();
    let env = test_env();
    let wp = Waypoint::new(env, WaypointCategory::Operational, "Current state");
    let created = nav.create(&wp).unwrap();

    // Just created — should be fresh with 24h threshold
    let check = nav.check_freshness(&created.id, 24).unwrap();
    assert!(check.is_fresh);

    // With 0h threshold — immediately stale
    let check = nav.check_freshness(&created.id, 0).unwrap();
    assert!(!check.is_fresh);
}

#[test]
fn refresh_updates_freshness() {
    let nav = test_navigator();
    let env = test_env();
    let wp = Waypoint::new(env, WaypointCategory::Operational, "State");
    let created = nav.create(&wp).unwrap();

    let refreshed = nav.refresh(&created.id).unwrap();
    assert!(refreshed.freshness >= created.freshness);
}

#[test]
fn delete_subtree_removes_descendants() {
    let nav = test_navigator();
    let env = test_env();

    let parent = Waypoint::new(env.clone(), WaypointCategory::Orientation, "Parent");
    let parent = nav.create(&parent).unwrap();

    let child =
        Waypoint::new(env.clone(), WaypointCategory::Task, "Child").with_parent(parent.id.clone());
    nav.create(&child).unwrap();

    let deleted = nav.delete_subtree(&parent.id).unwrap();
    assert_eq!(deleted, 2);

    assert!(nav.get(&parent.id).is_err());
    assert!(nav.get(&child.id).is_err());
}

#[test]
fn traversal_is_logged() {
    let nav = test_navigator();
    let env = test_env();
    let root = Waypoint::new(env, WaypointCategory::Orientation, "Root");
    let root = nav.create(&root).unwrap();

    nav.traverse("worker-1", &root.id, 2).unwrap();

    // Check traversal was logged
    let count: i64 = nav
        .conn
        .lock()
        .unwrap()
        .query_row(
            "SELECT COUNT(*) FROM waypoint_traversals WHERE worker_id = 'worker-1'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 1);
}

#[test]
fn get_nonexistent_returns_not_found() {
    let nav = test_navigator();
    let missing = WaypointId("wp-nonexistent".to_string());
    assert!(matches!(nav.get(&missing), Err(Error::NotFound(_))));
}

#[test]
fn root_for_unknown_env_returns_not_found() {
    let nav = test_navigator();
    let unknown = EnvironmentId("unknown-env".to_string());
    assert!(matches!(nav.root(&unknown), Err(Error::NotFound(_))));
}

// Helper for tests — sets predecessor on a waypoint
trait WaypointBuilderExt {
    fn with_predecessor(self, pred: WaypointId) -> Waypoint;
}

impl WaypointBuilderExt for Waypoint {
    fn with_predecessor(mut self, pred: WaypointId) -> Self {
        self.predecessor = Some(pred);
        self
    }
}
