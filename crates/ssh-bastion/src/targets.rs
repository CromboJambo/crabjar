// CrabJar SSH Bastion - Target Router
// Maps authenticated users to allowed target machines

use std::collections::HashMap;
use tracing::info;

#[derive(Debug, Clone)]
pub struct Machine {
    pub name: String,
    pub ip: String,
    pub ssh_user: String,
}

#[derive(Debug)]
pub struct TargetRouter {
    machines: Vec<Machine>,
    access_rules: HashMap<String, Vec<String>>, // user -> allowed machine names
}

impl TargetRouter {
    pub fn new(machines: Vec<Machine>) -> Self {
        info!("Target router initialized with {} machines", machines.len());
        Self {
            machines,
            access_rules: HashMap::new(),
        }
    }

    pub fn get_targets_for_user(&self, user: &str) -> Vec<&Machine> {
        match self.access_rules.get(user) {
            Some(allowed) => {
                info!("User {} allowed on: {:?}", user, allowed);
                self.machines
                    .iter()
                    .filter(|m| allowed.contains(&m.name))
                    .collect()
            }
            None => {
                info!("No access rules for user {}, granting all", user);
                self.machines.iter().collect()
            }
        }
    }

    pub fn get_machine_by_name(&self, name: &str) -> Option<&Machine> {
        self.machines.iter().find(|m| m.name == name)
    }
}
