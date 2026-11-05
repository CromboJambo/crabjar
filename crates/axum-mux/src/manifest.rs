use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize, Clone)]
pub struct Manifest {
    /// The address each worker binds its websocket listener to.
    /// Set this to the host's tailscale IP (e.g. "100.x.x.x") —
    /// it deliberately has no default, so forgetting to set it
    /// is a config error rather than a silent bind to 0.0.0.0.
    pub bind_addr: String,
    #[serde(default)]
    pub vms: Vec<Vm>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Vm {
    pub name: String,
    #[allow(dead_code)] // not parsed yet, just documentation for now — see notes
    pub protocol: Protocol,
    /// "127.0.0.1:5930" for TCP (typical qemu -vnc / -spice port),
    /// or "unix:/run/vm-sockets/<name>.sock" for a unix socket target.
    pub target: String,
    /// Port this VM's worker binds its websocket listener to, on the
    /// tailscale interface.
    pub listen_port: u16,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Protocol {
    Spice,
    Vnc,
}

impl Manifest {
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let raw = std::fs::read_to_string(&path)
            .with_context(|| format!("reading manifest at {:?}", path.as_ref()))?;
        toml::from_str(&raw).context("parsing manifest.toml")
    }

    pub fn find(&self, name: &str) -> Result<Vm> {
        self.vms
            .iter()
            .find(|v| v.name == name)
            .cloned()
            .with_context(|| format!("no VM named '{name}' in manifest"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_MANIFEST: &str = r#"
bind_addr = "100.115.42.1"

[[vms]]
name = "build-box"
protocol = "spice"
target = "127.0.0.1:5930"
listen_port = 7001

[[vms]]
name = "scratch-win11"
protocol = "vnc"
target = "unix:/run/vm-sockets/scratch-win11.sock"
listen_port = 7002
"#;

    #[test]
    fn parse_valid_manifest() {
        let manifest: Manifest = toml::from_str(SAMPLE_MANIFEST).unwrap();
        assert_eq!(manifest.bind_addr, "100.115.42.1");
        assert_eq!(manifest.vms.len(), 2);
        assert_eq!(manifest.vms[0].name, "build-box");
        assert_eq!(manifest.vms[0].protocol, Protocol::Spice);
        assert_eq!(manifest.vms[0].target, "127.0.0.1:5930");
        assert_eq!(manifest.vms[0].listen_port, 7001);
        assert_eq!(manifest.vms[1].name, "scratch-win11");
        assert_eq!(manifest.vms[1].protocol, Protocol::Vnc);
        assert_eq!(
            manifest.vms[1].target,
            "unix:/run/vm-sockets/scratch-win11.sock"
        );
        assert_eq!(manifest.vms[1].listen_port, 7002);
    }

    #[test]
    fn find_existing_vm() {
        let manifest: Manifest = toml::from_str(SAMPLE_MANIFEST).unwrap();
        let vm = manifest.find("build-box").unwrap();
        assert_eq!(vm.name, "build-box");
    }

    #[test]
    fn find_missing_vm() {
        let manifest: Manifest = toml::from_str(SAMPLE_MANIFEST).unwrap();
        assert!(manifest.find("nonexistent").is_err());
    }

    #[test]
    fn missing_bind_addr_rejects() {
        let bad = r#"
[[vms]]
name = "test"
protocol = "spice"
target = "127.0.0.1:5930"
listen_port = 7001
"#;
        let result: Result<Manifest, _> = toml::from_str(bad);
        assert!(result.is_err());
    }

    #[test]
    fn empty_vms_list_parses() {
        let minimal = r#"
bind_addr = "100.0.0.1"
"#;
        let manifest: Manifest = toml::from_str(minimal).unwrap();
        assert_eq!(manifest.bind_addr, "100.0.0.1");
        assert!(manifest.vms.is_empty());
    }

    #[test]
    fn protocol_deserialize_via_manifest() {
        // Protocol deserialization is already covered by parse_valid_manifest;
        // this test just confirms the rename_all = "lowercase" works.
        let only_proto = r#"
bind_addr = "100.0.0.1"

[[vms]]
name = "p1"
protocol = "spice"
target = "127.0.0.1:5930"
listen_port = 7001

[[vms]]
name = "p2"
protocol = "vnc"
target = "127.0.0.1:5900"
listen_port = 7002
"#;
        let manifest: Manifest = toml::from_str(only_proto).unwrap();
        assert_eq!(manifest.vms[0].protocol, Protocol::Spice);
        assert_eq!(manifest.vms[1].protocol, Protocol::Vnc);
    }
}
