// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer1/memory_runtime (authoritative)

pub const CHECK_ID: &str = "layer1_memory_runtime_contract";
pub mod lensmap_annotations;
pub mod recall_policy;
pub mod token_telemetry;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecallCommand {
    QueryIndex,
    GetNode,
    BuildIndex,
    VerifyEnvelope,
    Probe,
}

fn strip_invisible_unicode(raw: &str) -> String {
    raw.chars()
        .filter(|ch| {
            !matches!(
                *ch,
                '\u{200B}'
                    | '\u{200C}'
                    | '\u{200D}'
                    | '\u{200E}'
                    | '\u{200F}'
                    | '\u{202A}'
                    | '\u{202B}'
                    | '\u{202C}'
                    | '\u{202D}'
                    | '\u{202E}'
                    | '\u{2060}'
                    | '\u{FEFF}'
            )
        })
        .collect::<String>()
}

fn sanitize_command_token(raw: &str) -> String {
    strip_invisible_unicode(raw)
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch.to_ascii_lowercase()
            } else if ch.is_whitespace() {
                '-'
            } else {
                '\0'
            }
        })
        .filter(|ch| *ch != '\0')
        .collect::<String>()
        .trim_matches('-')
        .chars()
        .take(64)
        .collect::<String>()
}

pub fn normalize_recall_command_token(cmd: &str) -> String {
    sanitize_command_token(cmd)
}

pub fn map_memory_recall_command(cmd: &str) -> RecallCommand {
    match normalize_recall_command_token(cmd).as_str() {
        "get" | "get-node" => RecallCommand::GetNode,
        "build-index" | "build" | "index-build" => RecallCommand::BuildIndex,
        "verify-envelope" | "verify" => RecallCommand::VerifyEnvelope,
        "probe" => RecallCommand::Probe,
        _ => RecallCommand::QueryIndex,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lensmap_annotations::parse_lensmap_annotation;
    use crate::recall_policy::{
        enforce_index_first, enforce_recall_budget, FailClosedMode, RecallBudgetInput,
    };
    use crate::token_telemetry::{evaluate_burn_slo, RetrievalMode, TokenTelemetryEvent};

    #[test]
    fn default_maps_to_query_index() {
        assert_eq!(
            map_memory_recall_command("query"),
            RecallCommand::QueryIndex
        );
        assert_eq!(
            map_memory_recall_command("status"),
            RecallCommand::QueryIndex
        );
    }

    #[test]
    fn explicit_get_maps_correctly() {
        assert_eq!(map_memory_recall_command("get"), RecallCommand::GetNode);
        assert_eq!(
            map_memory_recall_command("get-node"),
            RecallCommand::GetNode
        );
    }

    #[test]
    fn strips_invisible_unicode_from_command_token() {
        assert_eq!(
            normalize_recall_command_token("ver\u{200B}ify-envelope"),
            "verify-envelope"
        );
        assert_eq!(
            map_memory_recall_command("ver\u{200B}ify-envelope"),
            RecallCommand::VerifyEnvelope
        );
    }

    #[test]
    fn unknown_or_control_token_falls_back_to_query_index() {
        assert_eq!(
            map_memory_recall_command(" \u{0000}\u{0008} "),
            RecallCommand::QueryIndex
        );
    }

    #[test]
    fn lensmap_annotation_parser_available() {
        let out = parse_lensmap_annotation("@lensmap tags=memory nodes=recall jot=budget");
        assert!(out.ok);
    }

    #[test]
    fn token_telemetry_slo_available() {
        let event = TokenTelemetryEvent {
            startup_tokens: 20,
            hydration_tokens: 20,
            retrieval_tokens: 40,
            response_tokens: 40,
            mode: RetrievalMode::NodeRead,
        };
        assert!(evaluate_burn_slo(&event, 200).ok);
    }

    #[test]
    fn recall_budget_policy_available() {
        let out = enforce_recall_budget(&RecallBudgetInput {
            requested_top: 7,
            requested_max_files: 1,
            requested_expand_lines: 0,
            mode: FailClosedMode::Reject,
            max_top: 50,
            max_files: 20,
            max_expand_lines: 300,
        });
        assert!(out.ok);
        assert!(enforce_index_first(&["sqlite:index".to_string()], 1).ok);
    }
}
