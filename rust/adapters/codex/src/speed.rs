use std::fs;

use crate::{CodexServiceTier, cli::CodexSpeed};

use super::paths;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CodexSpeedPolicy {
    Auto(CodexServiceTier),
    Forced(CodexServiceTier),
}

impl From<CodexSpeed> for CodexSpeedPolicy {
    fn from(speed: CodexSpeed) -> Self {
        match speed {
            CodexSpeed::Auto => Self::Auto(CodexServiceTier::Standard),
            CodexSpeed::Standard => Self::Forced(CodexServiceTier::Standard),
            CodexSpeed::Fast => Self::Forced(CodexServiceTier::Fast),
            CodexSpeed::Flex => Self::Forced(CodexServiceTier::Flex),
        }
    }
}

pub fn resolve_codex_speed(requested: CodexSpeed) -> CodexSpeedPolicy {
    match requested {
        CodexSpeed::Auto => CodexSpeedPolicy::Auto(detect_codex_service_tier()),
        CodexSpeed::Standard => CodexSpeedPolicy::Forced(CodexServiceTier::Standard),
        CodexSpeed::Fast => CodexSpeedPolicy::Forced(CodexServiceTier::Fast),
        CodexSpeed::Flex => CodexSpeedPolicy::Forced(CodexServiceTier::Flex),
    }
}

fn detect_codex_service_tier() -> CodexServiceTier {
    let configured_tiers: Vec<_> = codex_home_paths()
        .iter()
        .filter_map(|path| {
            fs::read_to_string(path.join("config.toml"))
                .ok()
                .and_then(|content| codex_config_service_tier(&content))
        })
        .collect();
    if configured_tiers.contains(&CodexServiceTier::Fast) {
        CodexServiceTier::Fast
    } else if configured_tiers.contains(&CodexServiceTier::Flex) {
        CodexServiceTier::Flex
    } else {
        CodexServiceTier::Standard
    }
}

fn codex_home_paths() -> Vec<std::path::PathBuf> {
    paths::codex_home_paths().unwrap_or_default()
}

fn codex_config_service_tier(content: &str) -> Option<CodexServiceTier> {
    // ponytail: scans inactive profiles; resolve active profiles if legacy tier accuracy matters.
    content
        .lines()
        .filter_map(|line| {
            let setting = line.split('#').next().unwrap_or_default().trim();
            let (key, value) = setting.split_once('=')?;
            if key.trim() != "service_tier" {
                return None;
            }
            let value = value.trim().trim_matches(['"', '\'']);
            match value {
                "default" | "standard" => Some(CodexServiceTier::Standard),
                "fast" | "priority" => Some(CodexServiceTier::Fast),
                "flex" => Some(CodexServiceTier::Flex),
                _ => None,
            }
        })
        .fold(None, |current, incoming| match (current, incoming) {
            (Some(CodexServiceTier::Fast), _) | (_, CodexServiceTier::Fast) => {
                Some(CodexServiceTier::Fast)
            }
            (Some(CodexServiceTier::Flex), _) | (_, CodexServiceTier::Flex) => {
                Some(CodexServiceTier::Flex)
            }
            _ => Some(CodexServiceTier::Standard),
        })
}

#[cfg(test)]
mod tests {
    use crate::CodexServiceTier;

    use super::codex_config_service_tier;

    #[test]
    fn detects_explicit_fast_service_tier_values() {
        assert_eq!(
            codex_config_service_tier(r#"service_tier = "fast""#,),
            Some(CodexServiceTier::Fast)
        );
        assert_eq!(
            codex_config_service_tier(r#"service_tier = 'priority' # use higher tier"#,),
            Some(CodexServiceTier::Fast)
        );
        assert_eq!(
            codex_config_service_tier(r#"service_tier = "flex""#,),
            Some(CodexServiceTier::Flex)
        );
    }

    #[test]
    fn ignores_unrelated_or_substring_service_tier_values() {
        assert_eq!(
            codex_config_service_tier(r#"service_tier_override = "fast""#,),
            None
        );
        assert_eq!(
            codex_config_service_tier(r#"service_tier = "breakfast""#,),
            None
        );
        assert_eq!(
            codex_config_service_tier(r#"service_tier = "standard""#,),
            Some(CodexServiceTier::Standard)
        );
    }

    #[test]
    fn prefers_higher_explicit_tier_when_config_contains_multiple_settings() {
        assert_eq!(
            codex_config_service_tier(
                r#"
                    service_tier = "standard"
                    [profiles.flex]
                    service_tier = "flex"
                "#,
            ),
            Some(CodexServiceTier::Flex)
        );
        assert_eq!(
            codex_config_service_tier(
                r#"
                    service_tier = "flex"
                    [profiles.fast]
                    service_tier = "priority"
                "#,
            ),
            Some(CodexServiceTier::Fast)
        );
    }
}
