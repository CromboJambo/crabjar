/// Events emitted by the simulation loop.
#[derive(Debug, Clone)]
pub enum SimEvent {
    Tick {
        tick: u64,
    },
    AgentAction {
        agent_id: String,
        action: String,
        params: Option<serde_json::Value>,
    },
    ResourceUpdate {
        resource: String,
        value: f64,
    },
    StateChanged {
        from: String,
        to: String,
    },
    Error(String),
}

impl SimEvent {
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Tick { .. } => "tick",
            Self::AgentAction { .. } => "agent_action",
            Self::ResourceUpdate { .. } => "resource_update",
            Self::StateChanged { .. } => "state_changed",
            Self::Error(_) => "error",
        }
    }

    pub fn serialize(&self) -> String {
        match self {
            Self::Tick { tick } => serde_json::json!({
                "type": "tick",
                "tick": tick,
            })
            .to_string(),
            Self::AgentAction {
                agent_id,
                action,
                params,
            } => {
                let mut obj = serde_json::json!({
                    "type": "agent_action",
                    "agent_id": agent_id,
                    "action": action,
                });
                if let Some(p) = params {
                    obj["params"] = p.clone();
                }
                obj.to_string()
            }
            Self::ResourceUpdate { resource, value } => serde_json::json!({
                "type": "resource_update",
                "resource": resource,
                "value": value,
            })
            .to_string(),
            Self::StateChanged { from, to } => serde_json::json!({
                "type": "state_changed",
                "from": from,
                "to": to,
            })
            .to_string(),
            Self::Error(msg) => serde_json::json!({
                "type": "error",
                "message": msg,
            })
            .to_string(),
        }
    }

    pub fn deserialize(s: &str) -> Option<Self> {
        let val: serde_json::Value = serde_json::from_str(s).ok()?;
        if let Some(ty) = val.get("type").and_then(|t| t.as_str()) {
            match ty {
                "tick" => {
                    let tick = val.get("tick")?.as_u64()?;
                    return Some(Self::Tick { tick });
                }
                "agent_action" => {
                    let agent_id = val.get("agent_id")?.as_str()?.to_string();
                    let action = val.get("action")?.as_str()?.to_string();
                    let params = val.get("params").cloned();
                    return Some(Self::AgentAction {
                        agent_id,
                        action,
                        params,
                    });
                }
                "resource_update" => {
                    let resource = val.get("resource")?.as_str()?.to_string();
                    let value = val.get("value")?.as_f64()?;
                    return Some(Self::ResourceUpdate { resource, value });
                }
                "state_changed" => {
                    let from = val.get("from")?.as_str()?.to_string();
                    let to = val.get("to")?.as_str()?.to_string();
                    return Some(Self::StateChanged { from, to });
                }
                "error" => {
                    let msg = val.get("message")?.as_str()?.to_string();
                    return Some(Self::Error(msg));
                }
                _ => {}
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialize_tick_event() {
        let ev = SimEvent::Tick { tick: 5 };
        let s = ev.serialize();
        assert!(s.contains("\"tick\":5"));
    }

    #[test]
    fn deserialize_tick_event() {
        let ev = SimEvent::deserialize(r#"{"type":"tick","tick":10}"#);
        match ev {
            Some(SimEvent::Tick { tick }) => assert_eq!(tick, 10),
            _ => panic!("expected Tick event"),
        }
    }

    #[test]
    fn serialize_agent_action() {
        let ev = SimEvent::AgentAction {
            agent_id: "agent-1".to_string(),
            action: "move".to_string(),
            params: None,
        };
        let s = ev.serialize();
        assert!(s.contains("\"agent_id\":\"agent-1\""));
    }

    #[test]
    fn serialize_resource_update() {
        let ev = SimEvent::ResourceUpdate {
            resource: "food".to_string(),
            value: 42.0,
        };
        let s = ev.serialize();
        assert!(s.contains("\"resource\":\"food\""));
        assert!(s.contains("42"));
    }

    #[test]
    fn serialize_state_changed() {
        let ev = SimEvent::StateChanged {
            from: "idle".to_string(),
            to: "active".to_string(),
        };
        let s = ev.serialize();
        assert!(s.contains("\"from\":\"idle\""));
        assert!(s.contains("\"to\":\"active\""));
    }

    #[test]
    fn serialize_error() {
        let ev = SimEvent::Error("boom".to_string());
        let s = ev.serialize();
        assert!(s.contains("\"message\":\"boom\""));
    }

    #[test]
    fn deserialize_resource_update() {
        let ev =
            SimEvent::deserialize(r#"{"type":"resource_update","resource":"water","value":3.14}"#);
        match ev {
            Some(SimEvent::ResourceUpdate { resource, value }) => {
                assert_eq!(resource, "water");
                assert!((value - 3.14).abs() < f64::EPSILON);
            }
            _ => panic!("expected ResourceUpdate event"),
        }
    }

    #[test]
    fn deserialize_state_changed() {
        let ev = SimEvent::deserialize(r#"{"type":"state_changed","from":"a","to":"b"}"#);
        match ev {
            Some(SimEvent::StateChanged { from, to }) => {
                assert_eq!(from, "a");
                assert_eq!(to, "b");
            }
            _ => panic!("expected StateChanged event"),
        }
    }

    #[test]
    fn deserialize_error() {
        let ev = SimEvent::deserialize(r#"{"type":"error","message":"oops"}"#);
        match ev {
            Some(SimEvent::Error(msg)) => assert_eq!(msg, "oops"),
            _ => panic!("expected Error event"),
        }
    }

    #[test]
    fn deserialize_invalid_json() {
        let ev = SimEvent::deserialize("not json");
        assert!(ev.is_none());
    }

    #[test]
    fn deserialize_unknown_type() {
        let ev = SimEvent::deserialize(r#"{"type":"unknown"}"#);
        assert!(ev.is_none());
    }

    #[test]
    fn type_name_tick() {
        let ev = SimEvent::Tick { tick: 1 };
        assert_eq!(ev.type_name(), "tick");
    }

    #[test]
    fn type_name_agent_action() {
        let ev = SimEvent::AgentAction {
            agent_id: "a".to_string(),
            action: "b".to_string(),
            params: None,
        };
        assert_eq!(ev.type_name(), "agent_action");
    }

    #[test]
    fn type_name_resource_update() {
        let ev = SimEvent::ResourceUpdate {
            resource: "r".to_string(),
            value: 1.0,
        };
        assert_eq!(ev.type_name(), "resource_update");
    }

    #[test]
    fn type_name_state_changed() {
        let ev = SimEvent::StateChanged {
            from: "f".to_string(),
            to: "t".to_string(),
        };
        assert_eq!(ev.type_name(), "state_changed");
    }

    #[test]
    fn type_name_error() {
        let ev = SimEvent::Error("e".to_string());
        assert_eq!(ev.type_name(), "error");
    }
}
