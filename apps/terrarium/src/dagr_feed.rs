//! DAGR producer for CrabJar isometric game state.
//!
//! Converts game state (tiles, entities, events) into DAGR habitat events
//! that can be consumed by the habitat dashboard and agent workflows.

use serde::{Deserialize, Serialize};
use std::time::SystemTime;

// ============================================================================
// DAGR Event Types
// ============================================================================

/// A single DAGR habitat event representing a game state change.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameEvent {
    /// Unique event ID (UUID v4)
    pub id: String,
    
    /// Event type (world update, entity move, build action, etc.)
    #[serde(rename = "type")]
    pub event_type: EventType,
    
    /// Timestamp when event occurred
    pub timestamp: u64,
    
    /// Game tick when event occurred
    pub tick: u64,
    
    /// Source of the event (agent, simulation, user)
    pub source: EventSource,
    
    /// Event payload (varies by type)
    pub payload: EventPayload,
}

/// Event type classification.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "subtype", content = "data")]
pub enum EventType {
    /// World state update (tile changes, terrain modification)
    WorldUpdate {
        tile_x: i32,
        tile_y: i32,
        new_tile_type: TileType,
        old_tile_type: Option<TileType>,
    },
    
    /// Entity movement or position change
    EntityMove {
        entity_id: String,
        from_x: f32,
        from_y: f32,
        to_x: f32,
        to_y: f32,
        direction: Option<Direction>,
    },
    
    /// Entity interaction with world or other entities
    EntityInteract {
        entity_id: String,
        target_id: String,
        action: String,
        result: InteractionResult,
    },
    
    /// Build/construction action
    BuildAction {
        entity_id: String,
        tile_x: i32,
        tile_y: i32,
        structure_type: StructureType,
        height: i32,
    },
    
    /// Agent command that triggered game action
    AgentCommand {
        agent_id: String,
        command: String,
        parameters: serde_json::Value,
        success: bool,
    },
}

/// Source of the event.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventSource {
    /// Agent-driven action (from agent work)
    Agent,
    
    /// Simulation tick (autonomous game logic)
    Simulation,
    
    /// User input (if keyboard controls enabled)
    User,
}

/// Interaction result type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InteractionResult {
    Success,
    Failure(String), // Error message if failed
    Partial(String), // Partial success with details
}

// ============================================================================
// Game World Types (for serialization)
// ============================================================================

/// Tile type in the game world.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TileType {
    Grass,
    Water,
    Sand,
    Stone,
    Wood,
}

/// Direction enum for movement events.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    North,
    South,
    East,
    West,
    Northeast,
    Northwest,
    Southeast,
    Southwest,
}

/// Structure type for build actions.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StructureType {
    Wall,
    Tower,
    Bridge,
    Gate,
    ResourceNode,
}

// ============================================================================
// DAGR Producer Trait
// ============================================================================

/// Producer of DAGR events from game state.
pub trait DagrGameProducer {
    /// Generate a single event from current game state.
    fn produce_event(&self) -> GameEvent;
    
    /// Generate multiple events (batch mode).
    fn produce_batch(&self, count: usize) -> Vec<GameEvent> {
        (0..count).map(|_| self.produce_event()).collect()
    }
}

// ============================================================================
// Default Implementation for GameWorld
// ============================================================================

/// Convert a game world state into DAGR events.
pub struct GameWorldDagrProducer {
    pub tick: u64,
    pub entity_count: usize,
}

impl DagrGameProducer for GameWorldDagrProducer {
    fn produce_event(&self) -> GameEvent {
        // Generate a sample "simulation tick" event
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        GameEvent {
            id: uuid::Uuid::new_v4().to_string(),
            event_type: EventType::WorldUpdate {
                tile_x: 0,
                tile_y: 0,
                new_tile_type: TileType::Grass,
                old_tile_type: None,
            },
            timestamp,
            tick: self.tick,
            source: EventSource::Simulation,
            payload: EventPayload::WorldUpdate {
                tile_x: 0,
                tile_y: 0,
                new_tile_type: TileType::Grass,
                old_tile_type: None,
            },
        }
    }
}

// ============================================================================
// Event Payload (for serialization)
// ============================================================================

/// Serialized event payload matching the event type.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum EventPayload {
    WorldUpdate {
        tile_x: i32,
        tile_y: i32,
        new_tile_type: TileType,
        old_tile_type: Option<TileType>,
    },
    EntityMove {
        entity_id: String,
        from_x: f32,
        from_y: f32,
        to_x: f32,
        to_y: f32,
        direction: Option<Direction>,
    },
    EntityInteract {
        entity_id: String,
        target_id: String,
        action: String,
        result: InteractionResult,
    },
    BuildAction {
        entity_id: String,
        tile_x: i32,
        tile_y: i32,
        structure_type: StructureType,
        height: i32,
    },
    AgentCommand {
        agent_id: String,
        command: String,
        parameters: serde_json::Value,
        success: bool,
    },
}

// ============================================================================
// Event Construction (DRY helpers)
// ============================================================================

/// Derive the payload from the event type so callers never write the same
/// fields twice. The `type` and `payload` of a well-formed event are always in
/// sync — this makes that invariant structural rather than conventional.
impl EventType {
    pub fn to_payload(&self) -> EventPayload {
        match self {
            EventType::WorldUpdate {
                tile_x,
                tile_y,
                new_tile_type,
                old_tile_type,
            } => EventPayload::WorldUpdate {
                tile_x: *tile_x,
                tile_y: *tile_y,
                new_tile_type: *new_tile_type,
                old_tile_type: *old_tile_type,
            },
            EventType::EntityMove {
                entity_id,
                from_x,
                from_y,
                to_x,
                to_y,
                direction,
            } => EventPayload::EntityMove {
                entity_id: entity_id.clone(),
                from_x: *from_x,
                from_y: *from_y,
                to_x: *to_x,
                to_y: *to_y,
                direction: direction.clone(),
            },
            EventType::EntityInteract {
                entity_id,
                target_id,
                action,
                result,
            } => EventPayload::EntityInteract {
                entity_id: entity_id.clone(),
                target_id: target_id.clone(),
                action: action.clone(),
                result: result.clone(),
            },
            EventType::BuildAction {
                entity_id,
                tile_x,
                tile_y,
                structure_type,
                height,
            } => EventPayload::BuildAction {
                entity_id: entity_id.clone(),
                tile_x: *tile_x,
                tile_y: *tile_y,
                structure_type: structure_type.clone(),
                height: *height,
            },
            EventType::AgentCommand {
                agent_id,
                command,
                parameters,
                success,
            } => EventPayload::AgentCommand {
                agent_id: agent_id.clone(),
                command: command.clone(),
                parameters: parameters.clone(),
                success: *success,
            },
        }
    }
}

/// Build a full event from type + provenance. Payload is derived from the type.
pub fn build_event(
    event_type: EventType,
    source: EventSource,
    tick: u64,
) -> GameEvent {
    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    GameEvent {
        id: uuid::Uuid::new_v4().to_string(),
        payload: event_type.to_payload(),
        event_type,
        timestamp,
        tick,
        source,
    }
}

/// Emit an event to the DAGR feed on stderr (stdout is reserved for JSON-RPC).
pub fn emit_event(event: &GameEvent) {
    eprintln!(
        "DAGR EVENT: {}",
        serde_json::to_string(event).unwrap_or_default()
    );
}

// ============================================================================
// DAGR Integration Helpers
// ============================================================================

/// Convert game events to JSON for DAGR ingestion.
pub fn events_to_json(events: &[GameEvent]) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(events)
}

/// Convert single event to JSON.
pub fn event_to_json(event: &GameEvent) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(event)
}

/// Parse DAGR habitat event back into GameEvent.
pub fn parse_dagr_event(json: &str) -> Result<GameEvent, serde_json::Error> {
    serde_json::from_str(json)
}

// ============================================================================
// Agent Command Parser (bridge from agent → game)
// ============================================================================

/// Agent command that can be executed in the game world.
#[derive(Debug, Clone, Deserialize)]
pub struct AgentCommand {
    pub agent_id: String,
    pub action: GameAction,
}

/// Game action types that agents can trigger.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "action", content = "params")]
#[serde(rename_all = "snake_case")]
pub enum GameAction {
    /// Move an entity in a direction for N ticks
    Move {
        entity_id: String,
        direction: Direction,
        ticks: u64,
    },
    
    /// Build a structure at coordinates
    Build {
        entity_id: String,
        tile_x: i32,
        tile_y: i32,
        structure_type: StructureType,
        height: i32,
    },
    
    /// Interact with a target
    Interact {
        entity_id: String,
        target_id: String,
        action: String,
    },
}

/// Parse agent command from JSON-RPC request.
pub fn parse_agent_command(json: &str) -> Result<AgentCommand, serde_json::Error> {
    serde_json::from_str(json)
}

// ============================================================================
// State-Document Query Integration (optional)
// ============================================================================

/// Query recent game events from state-docs index.
/// 
/// This would integrate with crabjar's memory/state-docs system to persist
/// and query game history.
pub struct GameEventQuerier {
    // Would hold reference to GuardDb or similar
}

impl GameEventQuerier {
    /// Query events by tick range.
    pub fn query_by_tick_range(&self, _start: u64, _end: u64) -> Vec<GameEvent> {
        // Implementation would query state-docs index
        vec![]
    }
    
    /// Query events by entity ID.
    pub fn query_by_entity(&self, _entity_id: &str) -> Vec<GameEvent> {
        // Implementation would query state-docs index
        vec![]
    }
}
