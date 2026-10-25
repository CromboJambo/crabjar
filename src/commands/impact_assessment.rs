//! Impact Assessment Logic
//!
//! Classifies changes as low, medium, or high impact based on multiple factors:
//! - Number of affected users
//! - Criticality of affected services
//! - Type of change (config, data, code)
//! - Historical incident data

use std::collections::HashMap;

/// Impact levels for changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImpactLevel {
    Low,
    Medium,
    High,
}

impl std::fmt::Display for ImpactLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImpactLevel::Low => write!(f, "low"),
            ImpactLevel::Medium => write!(f, "medium"),
            ImpactLevel::High => write!(f, "high"),
        }
    }
}

/// Service criticality levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceCriticality {
    Critical,
    Important,
    Standard,
}

impl std::fmt::Display for ServiceCriticality {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServiceCriticality::Critical => write!(f, "critical"),
            ServiceCriticality::Important => write!(f, "important"),
            ServiceCriticality::Standard => write!(f, "standard"),
        }
    }
}

/// Impact assessment result.
#[derive(Debug)]
pub struct ImpactAssessment {
    pub level: ImpactLevel,
    pub factors: HashMap<String, String>,
}

/// Assess the impact of a change based on multiple factors.
pub fn assess_impact(
    affected_users: usize,
    services: Vec<ServiceCriticality>,
    change_type: &str,
    recent_incidents: bool,
) -> ImpactAssessment {
    let mut base_level = ImpactLevel::Low;

    // Factor 1: Number of affected users
    if affected_users > 100 {
        base_level = ImpactLevel::High;
    } else if affected_users >= 10 {
        base_level = ImpactLevel::Medium;
    }

    // Factor 2: Service criticality
    let max_criticality = services.iter().max_by(|a, b| {
        match (a, b) {
            (ServiceCriticality::Critical, _) => std::cmp::Ordering::Greater,
            (_, ServiceCriticality::Critical) => std::cmp::Ordering::Less,
            (ServiceCriticality::Important, _) => std::cmp::Ordering::Greater,
            (_, ServiceCriticality::Important) => std::cmp::Ordering::Less,
            _ => std::cmp::Ordering::Equal,
        }
    });

    if let Some(criticality) = max_criticality {
        match criticality {
            ServiceCriticality::Critical => {
                if base_level != ImpactLevel::High {
                    base_level = ImpactLevel::Medium;
                }
            }
            ServiceCriticality::Important => {
                if base_level == ImpactLevel::Low {
                    base_level = ImpactLevel::Medium;
                }
            }
            _ => {}
        }
    }

    // Factor 3: Change type
    match change_type {
        "config" => {} // No additional impact
        "data" => {
            if base_level == ImpactLevel::Low {
                base_level = ImpactLevel::Medium;
            }
        }
        "code" => {
            if base_level != ImpactLevel::High {
                base_level = ImpactLevel::Medium;
            }
        }
        _ => {}
    }

    // Factor 4: Recent incidents bump impact up one level
    if recent_incidents && base_level != ImpactLevel::High {
        base_level = match base_level {
            ImpactLevel::Low => ImpactLevel::Medium,
            ImpactLevel::Medium => ImpactLevel::High,
            ImpactLevel::High => ImpactLevel::High,
        };
    }

    let mut factors = HashMap::new();
    factors.insert("affected_users".to_string(), affected_users.to_string());
    factors.insert("change_type".to_string(), change_type.to_string());
    if recent_incidents {
        factors.insert("recent_incidents".to_string(), "true".to_string());
    }

    ImpactAssessment {
        level: base_level,
        factors,
    }
}
