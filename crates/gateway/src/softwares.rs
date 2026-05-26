// crates/gateway/src/softwares.rs
// Public software registry — metadata safe for external consumption

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Software {
    pub name: String,
    pub version: String,
    pub description: String,
    pub category: String,
    pub repository: String,
    pub homepage: Option<String>,
    pub license: String,
    pub maintainers: Vec<String>,
    pub source_types: Vec<String>, // ["nix", "linux", "docker", "cargo"]
    pub source_url: String,        // e.g., "https://github.com/voidnxlabs/{name}"
}

#[derive(Clone, Debug)]
pub struct SoftwareRegistry {
    softwares: HashMap<String, Software>,
}

impl SoftwareRegistry {
    pub fn new() -> Self {
        let mut softwares = HashMap::new();

        // Define public software from ~/master/*
        softwares.insert(
            "securellm-bridge".to_string(),
            Software {
                name: "securellm-bridge".to_string(),
                version: "0.1.0".to_string(),
                description: "Universal LLM provider gateway with security, rate-limiting, and multi-provider support".to_string(),
                category: "ai".to_string(),
                repository: "https://github.com/voidnxlabs/securellm-bridge".to_string(),
                homepage: Some("https://voidnxlabs.io".to_string()),
                license: "Apache-2.0".to_string(),
                maintainers: vec!["voidnxlabs <dev@voidnxlabs.io>".to_string()],
                source_types: vec!["nix".to_string(), "cargo".to_string(), "docker".to_string()],
                source_url: "https://github.com/voidnxlabs/securellm-bridge".to_string(),
            },
        );

        softwares.insert(
            "spider-nix".to_string(),
            Software {
                name: "spider-nix".to_string(),
                version: "0.1.0".to_string(),
                description: "Domain reconnaissance and OSINT toolkit for NixOS".to_string(),
                category: "security".to_string(),
                repository: "https://github.com/voidnxlabs/spider-nix".to_string(),
                homepage: Some("https://voidnxlabs.io".to_string()),
                license: "Apache-2.0".to_string(),
                maintainers: vec!["voidnxlabs <dev@voidnxlabs.io>".to_string()],
                source_types: vec!["nix".to_string(), "cargo".to_string()],
                source_url: "https://github.com/voidnxlabs/spider-nix".to_string(),
            },
        );

        softwares.insert(
            "cerebro".to_string(),
            Software {
                name: "cerebro".to_string(),
                version: "0.1.0".to_string(),
                description: "Multi-model LLM orchestration and inference engine".to_string(),
                category: "ai".to_string(),
                repository: "https://github.com/voidnxlabs/cerebro".to_string(),
                homepage: Some("https://voidnxlabs.io".to_string()),
                license: "Apache-2.0".to_string(),
                maintainers: vec!["voidnxlabs <dev@voidnxlabs.io>".to_string()],
                source_types: vec!["nix".to_string(), "docker".to_string()],
                source_url: "https://github.com/voidnxlabs/cerebro".to_string(),
            },
        );

        softwares.insert(
            "phantom".to_string(),
            Software {
                name: "phantom".to_string(),
                version: "0.1.0".to_string(),
                description: "Advanced threat detection and forensics framework".to_string(),
                category: "security".to_string(),
                repository: "https://github.com/voidnxlabs/phantom".to_string(),
                homepage: Some("https://voidnxlabs.io".to_string()),
                license: "Apache-2.0".to_string(),
                maintainers: vec!["voidnxlabs <dev@voidnxlabs.io>".to_string()],
                source_types: vec!["nix".to_string(), "linux".to_string()],
                source_url: "https://github.com/voidnxlabs/phantom".to_string(),
            },
        );

        softwares.insert(
            "spectre".to_string(),
            Software {
                name: "spectre".to_string(),
                version: "0.1.0".to_string(),
                description: "Distributed system analyzer and performance profiler".to_string(),
                category: "devops".to_string(),
                repository: "https://github.com/voidnxlabs/spectre".to_string(),
                homepage: Some("https://voidnxlabs.io".to_string()),
                license: "Apache-2.0".to_string(),
                maintainers: vec!["voidnxlabs <dev@voidnxlabs.io>".to_string()],
                source_types: vec!["nix".to_string(), "docker".to_string()],
                source_url: "https://github.com/voidnxlabs/spectre".to_string(),
            },
        );

        Self { softwares }
    }

    pub fn list(&self) -> Vec<Software> {
        self.softwares.values().cloned().collect()
    }

    pub fn get(&self, name: &str) -> Option<Software> {
        self.softwares.get(name).cloned()
    }

    pub fn by_category(&self, category: &str) -> Vec<Software> {
        self.softwares
            .values()
            .filter(|s| s.category == category)
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_initialization() {
        let reg = SoftwareRegistry::new();
        assert!(!reg.list().is_empty());
        assert!(reg.get("spider-nix").is_some());
    }

    #[test]
    fn test_category_filter() {
        let reg = SoftwareRegistry::new();
        let security = reg.by_category("security");
        assert!(!security.is_empty());
    }
}
