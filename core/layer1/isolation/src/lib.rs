// SPDX-License-Identifier: Apache-2.0
use std::collections::BTreeMap;

const MAX_CAPABILITY_NAME_LEN: usize = 96;
const MAX_CAPABILITIES: usize = 256;

fn sanitize_capability_name(input: &str) -> String {
    input
        .chars()
        .filter(|c| {
            !matches!(
                *c,
                '\u{200B}' | '\u{200C}' | '\u{200D}' | '\u{2060}' | '\u{FEFF}'
            )
        })
        .filter(|c| !c.is_control())
        .collect::<String>()
        .trim()
        .to_lowercase()
        .chars()
        .take(MAX_CAPABILITY_NAME_LEN)
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityHandle {
    pub name: String,
    pub granted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sandbox {
    pub capabilities: Vec<CapabilityHandle>,
}

impl Sandbox {
    pub fn new(capabilities: Vec<CapabilityHandle>) -> Self {
        let mut by_name = BTreeMap::new();
        for cap in capabilities {
            let normalized = sanitize_capability_name(cap.name.as_str());
            if normalized.is_empty() {
                continue;
            }
            let granted = cap.granted || by_name.get(&normalized).copied().unwrap_or(false);
            by_name.insert(normalized, granted);
            if by_name.len() >= MAX_CAPABILITIES {
                break;
            }
        }
        Self {
            capabilities: by_name
                .into_iter()
                .map(|(name, granted)| CapabilityHandle { name, granted })
                .collect(),
        }
    }

    fn capability_state(&self, capability_name: &str) -> Option<bool> {
        let requested = sanitize_capability_name(capability_name);
        if requested.is_empty() {
            return None;
        }
        self.capabilities
            .iter()
            .find(|cap| cap.name == requested)
            .map(|cap| cap.granted)
    }

    pub fn can_execute(&self, capability_name: &str) -> bool {
        self.capability_state(capability_name).unwrap_or(false)
    }

    pub fn run_stub(&self, capability_name: &str) -> Result<(), &'static str> {
        match self.capability_state(capability_name) {
            None => Err("capability_invalid"),
            Some(true) => Ok(()),
            Some(false) => Err("capability_denied"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{CapabilityHandle, Sandbox};

    #[test]
    fn sandbox_stub_denies_when_capability_missing() {
        let sandbox = Sandbox::new(vec![CapabilityHandle {
            name: "net.read".to_string(),
            granted: true,
        }]);

        assert!(sandbox.run_stub("fs.write").is_err());
        assert!(sandbox.run_stub("net.read").is_ok());
    }

    #[test]
    fn capability_names_are_normalized_and_deduped() {
        let sandbox = Sandbox::new(vec![
            CapabilityHandle {
                name: " NET.READ ".to_string(),
                granted: false,
            },
            CapabilityHandle {
                name: "net.read".to_string(),
                granted: true,
            },
        ]);
        assert!(sandbox.run_stub(" \u{200B}Net.Read ").is_ok());
        assert_eq!(sandbox.run_stub("\u{200C}"), Err("capability_invalid"));
    }
}
