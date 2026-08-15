use super::models::Rule;

/// Evaluates Mojang library rules for the current platform (Linux).
pub fn is_rule_allowed(rules: &Option<Vec<Rule>>) -> bool {
    let Some(rules) = rules else {
        return true;
    };

    let mut allowed = false;
    for rule in rules {
        if rule.action == "allow" {
            if let Some(os) = &rule.os {
                if is_current_os(&os.name) {
                    allowed = true;
                }
            } else {
                allowed = true;
            }
        } else if rule.action == "disallow" {
            if let Some(os) = &rule.os {
                if is_current_os(&os.name) {
                    allowed = false;
                }
            } else {
                allowed = false;
            }
        }
    }
    allowed
}

/// Checks if a given OS identifier matches Linux.
pub fn is_current_os(os_name: &str) -> bool {
    os_name == "linux" || os_name == "linux-x86_64" || os_name == "linux-arm64"
}

/// Evaluates raw JSON argument rules for JVM/game arguments.
pub fn is_json_rule_allowed(rules: &[serde_json::Value]) -> bool {
    let mut allowed = false;
    for rule in rules {
        let action = rule.get("action").and_then(|a| a.as_str()).unwrap_or("allow");
        let os_match = if let Some(os) = rule.get("os") {
            if let Some(name) = os.get("name").and_then(|n| n.as_str()) {
                is_current_os(name)
            } else {
                true
            }
        } else {
            true
        };

        if os_match {
            allowed = action == "allow";
        }
    }
    allowed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::core::mojang::models::Os;

    #[test]
    fn test_rules_none_allowed() {
        assert!(is_rule_allowed(&None));
    }

    #[test]
    fn test_rules_allow_linux() {
        let rules = Some(vec![Rule {
            action: "allow".to_string(),
            os: Some(Os {
                name: "linux".to_string(),
            }),
        }]);
        assert!(is_rule_allowed(&rules));
    }

    #[test]
    fn test_rules_allow_osx_only() {
        let rules = Some(vec![Rule {
            action: "allow".to_string(),
            os: Some(Os {
                name: "osx".to_string(),
            }),
        }]);
        assert!(!is_rule_allowed(&rules));
    }

    #[test]
    fn test_rules_disallow_linux() {
        let rules = Some(vec![
            Rule {
                action: "allow".to_string(),
                os: None,
            },
            Rule {
                action: "disallow".to_string(),
                os: Some(Os {
                    name: "linux".to_string(),
                }),
            },
        ]);
        assert!(!is_rule_allowed(&rules));
    }
}
