//! Agent command handler for the CrabJar isometric habitat.
//!
//! Executes agent commands (move / build / interact) against a shared
//! `GameWorld` and reports both the human-readable result and the DAGR events
//! that describe the side effects. The handler takes `&mut GameWorld`, mutates
//! it, and returns *data* — emission to the feed is the caller's job. That keeps
//! the glass clean: decisions flow out as data, the handler never writes to a
//! stream or names a concrete consumer.

use crate::dagr_feed::{
    AgentCommand, Direction, EventType, GameAction, InteractionResult, StructureType,
};
use crate::render_isometric::{GameWorld, IsometricTile, TileType};
use serde::Serialize;

// ============================================================================
// Outcome (result + side-effect events)
// ============================================================================

/// Outcome of executing an agent command: the result plus the DAGR events that
/// describe what actually changed in the world.
#[derive(Debug, Clone)]
pub struct CommandOutcome {
    pub result: CommandResult,
    /// Side-effect events to emit to the DAGR feed (in order).
    pub events: Vec<EventType>,
}

impl CommandOutcome {
    fn ok(message: impl Into<String>) -> Self {
        Self {
            result: CommandResult::success(message),
            events: Vec::new(),
        }
    }

    fn err(message: impl Into<String>) -> Self {
        Self {
            result: CommandResult::failure(message),
            events: Vec::new(),
        }
    }

    fn with_event(mut self, event: EventType) -> Self {
        self.events.push(event);
        self
    }
}

// ============================================================================
// Execution
// ============================================================================

/// Execute an agent command against the world.
pub fn execute_command(world: &mut GameWorld, command: AgentCommand) -> CommandOutcome {
    match command.action {
        GameAction::Move {
            entity_id,
            direction,
            ticks,
        } => execute_move(world, &entity_id, direction, ticks),
        GameAction::Build {
            entity_id,
            tile_x,
            tile_y,
            structure_type,
            height,
        } => execute_build(
            world,
            &entity_id,
            tile_x,
            tile_y,
            structure_type,
            height,
        ),
        GameAction::Interact {
            entity_id,
            target_id,
            action,
        } => execute_interact(world, &entity_id, &target_id, &action),
    }
}

/// Parse and execute a command from raw JSON.
pub fn execute_from_json(world: &mut GameWorld, json: &str) -> CommandOutcome {
    match serde_json::from_str::<AgentCommand>(json) {
        Ok(command) => execute_command(world, command),
        Err(e) => CommandOutcome {
            result: CommandResult::parse_error(e.to_string()),
            events: Vec::new(),
        },
    }
}

/// Move an entity in a direction for N ticks.
fn execute_move(
    world: &mut GameWorld,
    entity_id: &str,
    direction: Direction,
    ticks: u64,
) -> CommandOutcome {
    let (dx, dy) = match direction {
        Direction::North => (0.0, -1.0),
        Direction::South => (0.0, 1.0),
        Direction::East => (1.0, 0.0),
        Direction::West => (-1.0, 0.0),
        Direction::Northeast => (0.707, -0.707),
        Direction::Northwest => (-0.707, -0.707),
        Direction::Southeast => (0.707, 0.707),
        Direction::Southwest => (-0.707, 0.707),
    };

    for entity in &mut world.entities {
        if entity.id != entity_id {
            continue;
        }
        let (from_x, from_y) = (entity.x, entity.y);
        for _ in 0..ticks {
            entity.x += dx;
            entity.y += dy;
            // Keep the entity on the habitat grid: clamp and flip velocity.
            if entity.x < 0.0 || entity.x > world.width as f32 {
                entity.vx = -entity.vx;
            }
            if entity.y < 0.0 || entity.y > world.height as f32 {
                entity.vy = -entity.vy;
            }
            entity.x = entity.x.clamp(0.0, world.width as f32);
            entity.y = entity.y.clamp(0.0, world.height as f32);
        }

        return CommandOutcome::ok(format!(
            "Moved {} {} for {} ticks",
            entity_id,
            direction_name(&direction),
            ticks
        ))
        .with_event(EventType::EntityMove {
            entity_id: entity.id.clone(),
            from_x,
            from_y,
            to_x: entity.x,
            to_y: entity.y,
            direction: Some(direction),
        });
    }

    CommandOutcome::err(format!("Entity '{}' not found", entity_id))
}

/// Build a structure at tile coordinates.
fn execute_build(
    world: &mut GameWorld,
    entity_id: &str,
    tile_x: i32,
    tile_y: i32,
    structure_type: StructureType,
    height: i32,
) -> CommandOutcome {
    let new_type = match structure_type {
        StructureType::Wall | StructureType::Tower => TileType::Stone,
        StructureType::Bridge | StructureType::Gate => TileType::Wood,
        StructureType::ResourceNode => TileType::Grass,
    };

    if let Some(tile) = world.tiles.iter_mut().find(|t| t.x == tile_x && t.y == tile_y) {
        tile.tile_type = new_type;
        tile.z = height;
    } else {
        world.tiles.push(IsometricTile {
            x: tile_x,
            y: tile_y,
            z: height,
            tile_type: new_type,
        });
    }

    CommandOutcome::ok(format!(
        "Built {} at ({}, {}) with height {}",
        structure_type_name(&structure_type),
        tile_x,
        tile_y,
        height
    ))
    .with_event(EventType::BuildAction {
        entity_id: entity_id.to_string(),
        tile_x,
        tile_y,
        structure_type,
        height,
    })
}

/// Perform an interaction between two entities.
fn execute_interact(
    world: &mut GameWorld,
    entity_id: &str,
    target_id: &str,
    action: &str,
) -> CommandOutcome {
    let entity = world.entities.iter().find(|e| e.id == entity_id);
    let target = world.entities.iter().find(|e| e.id == target_id);

    match (entity, target) {
        (Some(e), Some(t)) => {
            let dx = e.x - t.x;
            let dy = e.y - t.y;
            let distance = (dx * dx + dy * dy).sqrt();
            if distance < 5.0 {
                CommandOutcome::ok(format!(
                    "{} {} -> {} ({:.1} units away)",
                    entity_id, action, target_id, distance
                ))
                .with_event(EventType::EntityInteract {
                    entity_id: e.id.clone(),
                    target_id: t.id.clone(),
                    action: action.to_string(),
                    result: InteractionResult::Success,
                })
            } else {
                CommandOutcome::err(format!(
                    "{} and {} too far apart (distance: {:.1})",
                    entity_id, target_id, distance
                ))
                .with_event(EventType::EntityInteract {
                    entity_id: e.id.clone(),
                    target_id: t.id.clone(),
                    action: action.to_string(),
                    result: InteractionResult::Failure("too far apart".to_string()),
                })
            }
        }
        (None, _) => CommandOutcome::err(format!("Entity '{}' not found", entity_id)),
        (_, None) => CommandOutcome::err(format!("Target '{}' not found", target_id)),
    }
}

// ============================================================================
// Result Types
// ============================================================================

/// Result of command execution.
#[derive(Debug, Clone, Serialize)]
pub struct CommandResult {
    pub success: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<CommandDetails>,
}

impl CommandResult {
    pub fn success(message: impl Into<String>) -> Self {
        Self {
            success: true,
            message: message.into(),
            details: None,
        }
    }

    pub fn failure(message: impl Into<String>) -> Self {
        Self {
            success: false,
            message: message.into(),
            details: None,
        }
    }

    pub fn parse_error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            message: format!("Parse error: {}", message.into()),
            details: None,
        }
    }

    pub fn with_details(mut self, details: CommandDetails) -> Self {
        self.details = Some(details);
        self
    }
}

/// Optional details for successful commands.
#[derive(Debug, Clone, Serialize)]
pub struct CommandDetails {
    pub entity_id: Option<String>,
    pub tile_x: Option<i32>,
    pub tile_y: Option<i32>,
    pub new_position: Option<(f32, f32)>,
}

// ============================================================================
// Helper Functions
// ============================================================================

fn direction_name(d: &Direction) -> &'static str {
    match d {
        Direction::North => "north",
        Direction::South => "south",
        Direction::East => "east",
        Direction::West => "west",
        Direction::Northeast => "northeast",
        Direction::Northwest => "northwest",
        Direction::Southeast => "southeast",
        Direction::Southwest => "southwest",
    }
}

fn structure_type_name(s: &StructureType) -> &'static str {
    match s {
        StructureType::Wall => "wall",
        StructureType::Tower => "tower",
        StructureType::Bridge => "bridge",
        StructureType::Gate => "gate",
        StructureType::ResourceNode => "resource_node",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render_isometric::generate_world;

    #[test]
    fn move_known_entity_reports_success_and_event() {
        let mut world = generate_world(10, 8);
        let cmd = AgentCommand {
            agent_id: "a".to_string(),
            action: GameAction::Move {
                entity_id: world.entities[0].id.clone(),
                direction: Direction::East,
                ticks: 2,
            },
        };
        let outcome = execute_command(&mut world, cmd);
        assert!(outcome.result.success);
        assert_eq!(outcome.events.len(), 1);
        match &outcome.events[0] {
            EventType::EntityMove { to_x, .. } => assert!(*to_x > 0.0),
            _ => panic!("expected EntityMove"),
        }
    }

    #[test]
    fn move_unknown_entity_reports_failure() {
        let mut world = generate_world(10, 8);
        let cmd = AgentCommand {
            agent_id: "a".to_string(),
            action: GameAction::Move {
                entity_id: "ghost".to_string(),
                direction: Direction::East,
                ticks: 1,
            },
        };
        let outcome = execute_command(&mut world, cmd);
        assert!(!outcome.result.success);
        assert!(outcome.events.is_empty());
    }

    #[test]
    fn build_creates_tile_and_emits_event() {
        let mut world = generate_world(10, 8);
        let before = world.tiles.len();
        // Build at an out-of-grid coordinate so it appends a brand-new tile
        // (in-grid coordinates mutate the existing cell in place).
        let cmd = AgentCommand {
            agent_id: "builder".to_string(),
            action: GameAction::Build {
                entity_id: "crab_001".to_string(),
                tile_x: 20,
                tile_y: 20,
                structure_type: StructureType::Tower,
                height: 3,
            },
        };
        let outcome = execute_command(&mut world, cmd);
        assert!(outcome.result.success);
        assert_eq!(world.tiles.len(), before + 1);
        matches!(&outcome.events[0], EventType::BuildAction { .. });
    }
}
