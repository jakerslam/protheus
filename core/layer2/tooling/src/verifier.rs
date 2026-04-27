use crate::deterministic_hash;
use crate::schemas::{Claim, ClaimBundle, ClaimStatus, ConfidenceVector, EvidenceCard};
use std::collections::{HashMap, HashSet};

#[derive(Default)]
pub struct StructuredVerifier;

fn has_negative_cue(text: &str) -> bool {
    const NEGATIVE_CUES: &[&str] = &["not", "no", "failed", "fails", "denied", "missing"];
    let tokens = text
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|row| !row.is_empty())
        .map(|row| row.to_ascii_lowercase())
        .collect::<Vec<_>>();
    tokens
        .iter()
        .any(|token| NEGATIVE_CUES.iter().any(|cue| token == cue))
}

fn contradiction_topic_key(text: &str) -> String {
    const STOP: &[&str] = &[
        "the", "a", "an", "is", "are", "was", "were", "to", "in", "for", "on", "and", "or", "of",
        "by", "with", "it", "this", "that", "not", "no", "failed", "fails", "denied", "missing",
    ];
    text.split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|row| !row.is_empty())
        .map(|row| row.to_ascii_lowercase())
        .filter(|row| !STOP.iter().any(|stop| stop == row))
        .take(10)
        .collect::<Vec<_>>()
        .join(" ")
}

fn source_domain_key(source_ref: &str) -> String {
    let cleaned = source_ref.trim();
    if cleaned.is_empty() {
        return "unknown".to_string();
    }
    if cleaned.contains("://") {
        return cleaned
            .split("://")
            .next()
            .unwrap_or("unknown")
            .to_ascii_lowercase();
    }
    if cleaned.starts_with("core/")
        || cleaned.starts_with("surface/")
        || cleaned.starts_with("client/")
    {
        return "workspace".to_string();
    }
    "local".to_string()
}

fn source_language_key(source_ref: &str) -> Option<String> {
    let cleaned = source_ref.trim().to_ascii_lowercase();
    if cleaned.is_empty() {
        return None;
    }
    let marker = "tree-sitter/queries/";
    if let Some(idx) = cleaned.find(marker) {
        let tail = &cleaned[(idx + marker.len())..];
        let lang = tail
            .split(['/', '.'])
            .next()
            .map(str::trim)
            .unwrap_or_default();
        if !lang.is_empty() {
            return Some(lang.to_string());
        }
    }
    if let Some(lang) = [
        "c-sharp",
        "cpp",
        "java",
        "javascript",
        "kotlin",
        "python",
        "ruby",
        "php",
        "go",
        "rust",
        "swift",
        "typescript",
        "c",
    ]
    .iter()
    .find(|lang| cleaned.contains(&format!("/queries/{lang}.")))
    {
        return Some((*lang).to_string());
    }
    None
}

fn source_surface_key(source_ref: &str) -> Option<String> {
    let cleaned = source_ref.trim().to_ascii_lowercase();
    if cleaned.is_empty() {
        return None;
    }
    let marker = "webview-ui/src/components/mcp/";
    if let Some(idx) = cleaned.find(marker) {
        let tail = &cleaned[(idx + marker.len())..];
        let mut parts = tail.split('/');
        let primary = parts.next().unwrap_or_default().trim();
        let secondary = parts.next().unwrap_or_default().trim();
        let tertiary = parts.next().unwrap_or_default().trim();
        let surface = if secondary.is_empty() {
            primary.to_string()
        } else if primary == "configuration" && !tertiary.is_empty() {
            format!("{primary}/{secondary}/{tertiary}")
        } else {
            format!("{primary}/{secondary}")
        };
        if !surface.trim().is_empty() {
            return Some(surface);
        }
    }
    if cleaned.contains("src/shared/proto-conversions/cline-message") {
        return Some("shared/cline-message".to_string());
    }
    let shared_marker = "src/shared/";
    if let Some(idx) = cleaned.find(shared_marker) {
        let tail = &cleaned[(idx + shared_marker.len())..];
        if !tail.is_empty() && !tail.contains('/') {
            let shared_name = tail.split('.').next().map(str::trim).unwrap_or_default();
            if !shared_name.is_empty() {
                return Some(format!("shared/contracts/{shared_name}"));
            }
        }
        if let Some(cline_tail) = tail.strip_prefix("cline/") {
            let cline_name = cline_tail
                .split(['/', '.'])
                .next()
                .map(str::trim)
                .unwrap_or_default();
            if !cline_name.is_empty() {
                return Some(format!("shared/cline/{cline_name}"));
            }
        }
        if let Some(client_tail) = tail.strip_prefix("clients/") {
            let client_name = client_tail
                .split(['/', '.'])
                .next()
                .map(str::trim)
                .unwrap_or_default();
            if !client_name.is_empty() {
                return Some(format!("shared/clients/{client_name}"));
            }
        }
        if let Some(internal_tail) = tail.strip_prefix("internal/") {
            let internal_name = internal_tail
                .split(['/', '.'])
                .next()
                .map(str::trim)
                .unwrap_or_default();
            if !internal_name.is_empty() {
                return Some(format!("shared/internal/{internal_name}"));
            }
        }
        if let Some(messages_tail) = tail.strip_prefix("messages/") {
            let message_name = messages_tail
                .split(['/', '.'])
                .next()
                .map(str::trim)
                .unwrap_or_default();
            if !message_name.is_empty() {
                return Some(format!("shared/messages/{message_name}"));
            }
        }
        if let Some(service_tail) = tail.strip_prefix("services/") {
            let service_name = service_tail
                .split(['/', '.'])
                .next()
                .map(str::trim)
                .unwrap_or_default();
            if !service_name.is_empty() {
                return Some(format!("shared/services/{service_name}"));
            }
        }
        if let Some(multi_tail) = tail.strip_prefix("multi-root/") {
            let multi_name = multi_tail
                .split(['/', '.'])
                .next()
                .map(str::trim)
                .unwrap_or_default();
            if !multi_name.is_empty() {
                return Some(format!("shared/multi_root/{multi_name}"));
            }
        }
    }
    let services_temp_marker = "src/services/temp/";
    if let Some(idx) = cleaned.find(services_temp_marker) {
        let tail = &cleaned[(idx + services_temp_marker.len())..];
        let service_name = tail
            .split(['/', '.'])
            .next()
            .map(str::trim)
            .unwrap_or_default();
        if !service_name.is_empty() {
            return Some(format!("services/temp/{service_name}"));
        }
    }
    let services_test_marker = "src/services/test/";
    if let Some(idx) = cleaned.find(services_test_marker) {
        let tail = &cleaned[(idx + services_test_marker.len())..];
        let service_name = tail
            .split(['/', '.'])
            .next()
            .map(str::trim)
            .unwrap_or_default();
        if !service_name.is_empty() {
            return Some(format!("services/test/{service_name}"));
        }
    }
    let services_uri_marker = "src/services/uri/";
    if let Some(idx) = cleaned.find(services_uri_marker) {
        let tail = &cleaned[(idx + services_uri_marker.len())..];
        let service_name = tail
            .split(['/', '.'])
            .next()
            .map(str::trim)
            .unwrap_or_default();
        if !service_name.is_empty() {
            return Some(format!("services/uri/{service_name}"));
        }
    }
    let config_marker = "webview-ui/src/config/";
    if let Some(idx) = cleaned.find(config_marker) {
        let tail = &cleaned[(idx + config_marker.len())..];
        let config_name = tail
            .split(['/', '.'])
            .next()
            .map(str::trim)
            .unwrap_or_default();
        if !config_name.is_empty() {
            return Some(format!("config/{config_name}"));
        }
    }
    let hooks_marker = "webview-ui/src/hooks/";
    if let Some(idx) = cleaned.find(hooks_marker) {
        let tail = &cleaned[(idx + hooks_marker.len())..];
        let hook_name = tail
            .split(['/', '.'])
            .next()
            .map(str::trim)
            .unwrap_or_default();
        if !hook_name.is_empty() {
            return Some(format!("hooks/{hook_name}"));
        }
    }
    let services_marker = "webview-ui/src/services/";
    if let Some(idx) = cleaned.find(services_marker) {
        let tail = &cleaned[(idx + services_marker.len())..];
        let service_name = tail
            .split(['/', '.'])
            .next()
            .map(str::trim)
            .unwrap_or_default();
        if !service_name.is_empty() {
            return Some(format!("services/{service_name}"));
        }
    }
    let lib_marker = "webview-ui/src/lib/";
    if let Some(idx) = cleaned.find(lib_marker) {
        let tail = &cleaned[(idx + lib_marker.len())..];
        let lib_name = tail
            .split(['/', '.'])
            .next()
            .map(str::trim)
            .unwrap_or_default();
        if !lib_name.is_empty() {
            return Some(format!("lib/{lib_name}"));
        }
    }
    let utils_marker = "webview-ui/src/utils/";
    if let Some(idx) = cleaned.find(utils_marker) {
        let tail = &cleaned[(idx + utils_marker.len())..];
        let util_name = tail
            .split(['/', '.'])
            .next()
            .map(str::trim)
            .unwrap_or_default();
        if !util_name.is_empty() {
            return Some(format!("utils/{util_name}"));
        }
    }
    let chat_hooks_marker = "webview-ui/src/components/chat/chat-view/hooks/";
    if let Some(idx) = cleaned.find(chat_hooks_marker) {
        let tail = &cleaned[(idx + chat_hooks_marker.len())..];
        let hook_name = tail
            .split(['/', '.'])
            .next()
            .map(str::trim)
            .unwrap_or_default();
        if !hook_name.is_empty() {
            return Some(format!("chat/hooks/{hook_name}"));
        }
    }
    let chat_utils_marker = "webview-ui/src/components/chat/chat-view/utils/";
    if let Some(idx) = cleaned.find(chat_utils_marker) {
        let tail = &cleaned[(idx + chat_utils_marker.len())..];
        let util_name = tail
            .split(['/', '.'])
            .next()
            .map(str::trim)
            .unwrap_or_default();
        if !util_name.is_empty() {
            return Some(format!("chat/utils/{util_name}"));
        }
    }
    let chat_layout_marker = "webview-ui/src/components/chat/chat-view/components/layout/";
    if let Some(idx) = cleaned.find(chat_layout_marker) {
        let tail = &cleaned[(idx + chat_layout_marker.len())..];
        let layout_name = tail
            .split(['/', '.'])
            .next()
            .map(str::trim)
            .unwrap_or_default();
        if !layout_name.is_empty() {
            return Some(format!("chat/layout/{layout_name}"));
        }
    }
    let chat_messages_marker = "webview-ui/src/components/chat/chat-view/components/messages/";
    if let Some(idx) = cleaned.find(chat_messages_marker) {
        let tail = &cleaned[(idx + chat_messages_marker.len())..];
        let message_name = tail
            .split(['/', '.'])
            .next()
            .map(str::trim)
            .unwrap_or_default();
        if !message_name.is_empty() {
            return Some(format!("chat/messages/{message_name}"));
        }
    }
    let chat_shared_marker = "webview-ui/src/components/chat/chat-view/shared/";
    if let Some(idx) = cleaned.find(chat_shared_marker) {
        let tail = &cleaned[(idx + chat_shared_marker.len())..];
        let shared_name = tail
            .split(['/', '.'])
            .next()
            .map(str::trim)
            .unwrap_or_default();
        if !shared_name.is_empty() {
            return Some(format!("chat/shared/{shared_name}"));
        }
    }
    let chat_types_marker = "webview-ui/src/components/chat/chat-view/types/";
    if let Some(idx) = cleaned.find(chat_types_marker) {
        let tail = &cleaned[(idx + chat_types_marker.len())..];
        let type_name = tail
            .split(['/', '.'])
            .next()
            .map(str::trim)
            .unwrap_or_default();
        if !type_name.is_empty() {
            return Some(format!("chat/types/{type_name}"));
        }
    }
    let chat_view_marker = "webview-ui/src/components/chat/chat-view/";
    if let Some(idx) = cleaned.find(chat_view_marker) {
        let tail = &cleaned[(idx + chat_view_marker.len())..];
        let view_name = tail
            .split(['/', '.'])
            .next()
            .map(str::trim)
            .unwrap_or_default();
        if !view_name.is_empty() {
            return Some(format!("chat/view/{view_name}"));
        }
    }
    let chat_root_marker = "webview-ui/src/components/chat/";
    if let Some(idx) = cleaned.find(chat_root_marker) {
        let tail = &cleaned[(idx + chat_root_marker.len())..];
        if !tail.contains('/') {
            let root_name = tail
                .split(['/', '.'])
                .next()
                .map(str::trim)
                .unwrap_or_default();
            if !root_name.is_empty() {
                return Some(format!("chat/root/{root_name}"));
            }
        }
    }
    let chat_components_marker = "webview-ui/src/components/chat/";
    if let Some(idx) = cleaned.find(chat_components_marker) {
        let tail = &cleaned[(idx + chat_components_marker.len())..];
        if !tail.starts_with("chat-view/")
            && !tail.starts_with("task-header/")
            && !tail.starts_with("auto-approve-menu/")
        {
            let component_name = tail
                .split(['/', '.'])
                .next()
                .map(str::trim)
                .unwrap_or_default();
            if !component_name.is_empty() {
                return Some(format!("chat/components/{component_name}"));
            }
        }
    }
    let chat_task_header_marker = "webview-ui/src/components/chat/task-header/";
    if let Some(idx) = cleaned.find(chat_task_header_marker) {
        let tail = &cleaned[(idx + chat_task_header_marker.len())..];
        if let Some(button_tail) = tail.strip_prefix("buttons/") {
            let button_name = button_tail
                .split(['/', '.'])
                .next()
                .map(str::trim)
                .unwrap_or_default();
            if !button_name.is_empty() {
                return Some(format!("chat/task_header_buttons/{button_name}"));
            }
        }
        let task_header_name = tail
            .split(['/', '.'])
            .next()
            .map(str::trim)
            .unwrap_or_default();
        if !task_header_name.is_empty() {
            return Some(format!("chat/task_header/{task_header_name}"));
        }
    }
    let chat_auto_approve_marker = "webview-ui/src/components/chat/auto-approve-menu/";
    if let Some(idx) = cleaned.find(chat_auto_approve_marker) {
        let tail = &cleaned[(idx + chat_auto_approve_marker.len())..];
        let auto_approve_name = tail
            .split(['/', '.'])
            .next()
            .map(str::trim)
            .unwrap_or_default();
        if !auto_approve_name.is_empty() {
            return Some(format!("chat/auto_approve/{auto_approve_name}"));
        }
    }
    let cline_rules_marker = "webview-ui/src/components/cline-rules/";
    if let Some(idx) = cleaned.find(cline_rules_marker) {
        let tail = &cleaned[(idx + cline_rules_marker.len())..];
        let rule_name = tail
            .split(['/', '.'])
            .next()
            .map(str::trim)
            .unwrap_or_default();
        if !rule_name.is_empty() {
            return Some(format!("cline_rules/{rule_name}"));
        }
    }
    let common_components_marker = "webview-ui/src/components/common/";
    if let Some(idx) = cleaned.find(common_components_marker) {
        let tail = &cleaned[(idx + common_components_marker.len())..];
        let component_name = tail
            .split(['/', '.'])
            .next()
            .map(str::trim)
            .unwrap_or_default();
        if !component_name.is_empty() {
            return Some(format!("common/components/{component_name}"));
        }
    }
    let history_marker = "webview-ui/src/components/history/";
    if let Some(idx) = cleaned.find(history_marker) {
        let tail = &cleaned[(idx + history_marker.len())..];
        let history_name = tail
            .split(['/', '.'])
            .next()
            .map(str::trim)
            .unwrap_or_default();
        if !history_name.is_empty() {
            return Some(format!("history/{history_name}"));
        }
    }
    let menu_marker = "webview-ui/src/components/menu/";
    if let Some(idx) = cleaned.find(menu_marker) {
        let tail = &cleaned[(idx + menu_marker.len())..];
        let menu_name = tail
            .split(['/', '.'])
            .next()
            .map(str::trim)
            .unwrap_or_default();
        if !menu_name.is_empty() {
            return Some(format!("menu/{menu_name}"));
        }
    }
    let onboarding_marker = "webview-ui/src/components/onboarding/";
    if let Some(idx) = cleaned.find(onboarding_marker) {
        let tail = &cleaned[(idx + onboarding_marker.len())..];
        let onboarding_name = tail
            .split(['/', '.'])
            .next()
            .map(str::trim)
            .unwrap_or_default();
        if !onboarding_name.is_empty() {
            return Some(format!("onboarding/{onboarding_name}"));
        }
    }
    let browser_marker = "webview-ui/src/components/browser/";
    if let Some(idx) = cleaned.find(browser_marker) {
        let tail = &cleaned[(idx + browser_marker.len())..];
        let browser_name = tail
            .split(['/', '.'])
            .next()
            .map(str::trim)
            .unwrap_or_default();
        if !browser_name.is_empty() {
            return Some(format!("browser/{browser_name}"));
        }
    }
    let settings_marker = "webview-ui/src/components/settings/";
    if let Some(idx) = cleaned.find(settings_marker) {
        let tail = &cleaned[(idx + settings_marker.len())..];
        if let Some(test_tail) = tail.strip_prefix("__tests__/") {
            let test_name = test_tail
                .split(['/', '.'])
                .next()
                .map(str::trim)
                .unwrap_or_default();
            if !test_name.is_empty() {
                return Some(format!("settings/tests/{test_name}"));
            }
        }
        if let Some(common_tail) = tail.strip_prefix("common/") {
            let common_name = common_tail
                .split(['/', '.'])
                .next()
                .map(str::trim)
                .unwrap_or_default();
            if !common_name.is_empty() {
                return Some(format!("settings/common/{common_name}"));
            }
        }
        if let Some(section_tail) = tail.strip_prefix("sections/") {
            let section_name = section_tail
                .split(['/', '.'])
                .next()
                .map(str::trim)
                .unwrap_or_default();
            if !section_name.is_empty() {
                return Some(format!("settings/sections/{section_name}"));
            }
        }
        if let Some(utils_tail) = tail.strip_prefix("utils/") {
            let utils_name = utils_tail
                .split(['/', '.'])
                .next()
                .map(str::trim)
                .unwrap_or_default();
            if !utils_name.is_empty() {
                return Some(format!("settings/utils/{utils_name}"));
            }
        }
        let name = tail
            .split(['/', '.'])
            .next()
            .map(str::trim)
            .unwrap_or_default();
        if !name.is_empty() {
            return Some(format!("settings/components/{name}"));
        }
    }
    if cleaned.contains("webview-ui/src/components/settings/utils/providerutils") {
        return Some("settings/provider_utils".to_string());
    }
    if cleaned.contains("webview-ui/src/components/ui/tooltip") {
        return Some("ui/tooltip".to_string());
    }
    if cleaned.ends_with("webview-ui/src/app.tsx") {
        return Some("root/providers/providers".to_string());
    }
    if cleaned.ends_with("webview-ui/src/main.tsx") {
        return Some("root/providers/providers".to_string());
    }
    if cleaned.ends_with("webview-ui/src/config/storybookdecorator.tsx") {
        return Some("common/ui/storybook_decorator".to_string());
    }
    if cleaned.ends_with("webview-ui/src/components/ui/hooks/useopenrouterkeyinfo.ts") {
        return Some("hooks/useopenrouterkeyinfo".to_string());
    }
    if cleaned.ends_with("webview-ui/src/utils/bannerutils.tsx") {
        return Some("common/content/banner_utils".to_string());
    }
    if cleaned.ends_with("webview-ui/src/utils/vscstyles.ts") {
        return Some("common/ui/vsc_styles".to_string());
    }
    if cleaned.ends_with("webview-ui/src/components/chat/typewritertext.tsx") {
        return Some("chat/components/typewriter_text".to_string());
    }
    if cleaned.ends_with("webview-ui/src/components/chat/usermessage.tsx") {
        return Some("chat/components/user_message".to_string());
    }
    if cleaned.ends_with("webview-ui/src/components/chat/__tests__/errorblocktitle.spec.tsx") {
        return Some("chat/error/error_block_title".to_string());
    }
    if cleaned.ends_with("webview-ui/src/components/chat/__tests__/usermessage.ime.test.tsx") {
        return Some("chat/interaction/user_message_ime".to_string());
    }
    let ui_marker = "webview-ui/src/components/ui/";
    if let Some(idx) = cleaned.find(ui_marker) {
        let tail = &cleaned[(idx + ui_marker.len())..];
        if !tail.starts_with("hooks/") {
            let ui_name = tail
                .split(['/', '.'])
                .next()
                .map(str::trim)
                .unwrap_or_default();
            if !ui_name.is_empty() {
                return Some(format!("common/components/{ui_name}"));
            }
        }
    }
    let account_marker = "webview-ui/src/components/account/";
    if let Some(idx) = cleaned.find(account_marker) {
        let tail = &cleaned[(idx + account_marker.len())..];
        let account_name = tail
            .split(['/', '.'])
            .next()
            .map(str::trim)
            .unwrap_or_default();
        if !account_name.is_empty() {
            return Some(format!("settings/components/account_{account_name}"));
        }
    }
    let worktrees_marker = "webview-ui/src/components/worktrees/";
    if let Some(idx) = cleaned.find(worktrees_marker) {
        let tail = &cleaned[(idx + worktrees_marker.len())..];
        let worktree_name = tail
            .split(['/', '.'])
            .next()
            .map(str::trim)
            .unwrap_or_default();
        if !worktree_name.is_empty() {
            return Some(format!("settings/components/worktrees_{worktree_name}"));
        }
    }
    let welcome_marker = "webview-ui/src/components/welcome/";
    if let Some(idx) = cleaned.find(welcome_marker) {
        let tail = &cleaned[(idx + welcome_marker.len())..];
        let welcome_name = tail
            .split(['/', '.'])
            .next()
            .map(str::trim)
            .unwrap_or_default();
        if !welcome_name.is_empty() {
            return Some(format!("onboarding/welcome_{welcome_name}"));
        }
    }
    if cleaned.ends_with("webview-ui/src/vite-env.d.ts") {
        return Some("config/vite_env".to_string());
    }
    let coverage_check_marker = ".github/scripts/coverage_check/";
    if let Some(idx) = cleaned.find(coverage_check_marker) {
        let tail = &cleaned[(idx + coverage_check_marker.len())..];
        let script_name = tail
            .split(['/', '.'])
            .next()
            .map(str::trim)
            .unwrap_or_default();
        if !script_name.is_empty() {
            return Some(format!("tooling/coverage_check/{script_name}"));
        }
    }
    let coverage_tests_marker = ".github/scripts/tests/";
    if let Some(idx) = cleaned.find(coverage_tests_marker) {
        let tail = &cleaned[(idx + coverage_tests_marker.len())..];
        let test_name = tail
            .split(['/', '.'])
            .next()
            .map(str::trim)
            .unwrap_or_default();
        if !test_name.is_empty() {
            return Some(format!("tooling/coverage_check/tests/{test_name}"));
        }
    }
    let workflow_marker = ".github/workflows/";
    if let Some(idx) = cleaned.find(workflow_marker) {
        let tail = &cleaned[(idx + workflow_marker.len())..];
        let workflow_name = tail
            .split(['/', '.'])
            .next()
            .map(str::trim)
            .unwrap_or_default();
        if !workflow_name.is_empty() {
            return Some(format!("tooling/workflows/{workflow_name}"));
        }
    }
    let clinerules_workflow_marker = ".clinerules/workflows/";
    if let Some(idx) = cleaned.find(clinerules_workflow_marker) {
        let tail = &cleaned[(idx + clinerules_workflow_marker.len())..];
        let workflow_name = tail
            .split(['/', '.'])
            .next()
            .map(str::trim)
            .unwrap_or_default();
        if !workflow_name.is_empty() {
            return Some(format!("tooling/clinerules/workflows/{workflow_name}"));
        }
    }
    let clinerules_marker = ".clinerules/";
    if let Some(idx) = cleaned.find(clinerules_marker) {
        let tail = &cleaned[(idx + clinerules_marker.len())..];
        if !tail.starts_with("workflows/") {
            if tail.starts_with("hooks/") && tail.contains("/readme.") {
                return Some("tooling/clinerules/docs/hooks_readme".to_string());
            }
            let doc_name = tail
                .split(['/', '.'])
                .next()
                .map(str::trim)
                .unwrap_or_default();
            if !doc_name.is_empty() {
                return Some(format!("tooling/clinerules/docs/{doc_name}"));
            }
        }
    }
    let agent_skill_marker = ".agents/skills/";
    if let Some(idx) = cleaned.find(agent_skill_marker) {
        let tail = &cleaned[(idx + agent_skill_marker.len())..];
        let skill_name = tail
            .split(['/', '.'])
            .next()
            .map(str::trim)
            .unwrap_or_default();
        if !skill_name.is_empty() {
            return Some(format!("tooling/agent_skills/{skill_name}"));
        }
    }
    let storybook_marker = "webview-ui/.storybook/";
    if let Some(idx) = cleaned.find(storybook_marker) {
        let tail = &cleaned[(idx + storybook_marker.len())..];
        let storybook_name = tail
            .split(['/', '.'])
            .next()
            .map(str::trim)
            .unwrap_or_default();
        if !storybook_name.is_empty() {
            return Some(format!("tooling/storybook/{storybook_name}"));
        }
    }
    if cleaned.ends_with("webview-ui/.gitignore") {
        return Some("tooling/storybook/gitignore".to_string());
    }
    if cleaned.ends_with("changelog.md") {
        return Some("tooling/docs/changelog".to_string());
    }
    if cleaned.ends_with("cli/src/vscode-shim.ts") {
        return Some("tooling/cli/shims/vscode".to_string());
    }
    if cleaned.ends_with("src/core/api/adapters/index.ts") {
        return Some("tooling/contracts/core_api_adapters_index".to_string());
    }
    if cleaned.ends_with("src/exports/cline.d.ts") {
        return Some("tooling/contracts/exports_cline_types".to_string());
    }
    if cleaned.ends_with("src/exports/index.ts") {
        return Some("tooling/contracts/exports_index".to_string());
    }
    if cleaned.ends_with("src/types/picomatch.d.ts") {
        return Some("tooling/contracts/types_picomatch".to_string());
    }
    if cleaned.ends_with("src/hooks/hook_check.rs") {
        return Some("tooling/rtk/hooks_hook_check".to_string());
    }
    if cleaned.ends_with("src/hooks/verify_cmd.rs") {
        return Some("tooling/rtk/hooks_verify_cmd".to_string());
    }
    let rtk_hooks_marker = "src/hooks/";
    if let Some(idx) = cleaned.find(rtk_hooks_marker) {
        let tail = &cleaned[(idx + rtk_hooks_marker.len())..];
        let hook_name = tail
            .split(['/', '.'])
            .next()
            .map(str::trim)
            .unwrap_or_default()
            .to_ascii_lowercase()
            .replace('-', "_");
        if !hook_name.is_empty() {
            return Some(format!("tooling/rtk/hooks_{hook_name}"));
        }
    }
    let rtk_learn_marker = "src/learn/";
    if let Some(idx) = cleaned.find(rtk_learn_marker) {
        let tail = &cleaned[(idx + rtk_learn_marker.len())..];
        let learn_name = tail
            .split(['/', '.'])
            .next()
            .map(str::trim)
            .unwrap_or_default()
            .to_ascii_lowercase()
            .replace('-', "_");
        if !learn_name.is_empty() {
            return Some(format!("tooling/rtk/learn_{learn_name}"));
        }
    }
    if cleaned.ends_with("src/parser/formatter.rs") {
        return Some("tooling/rtk/parser_formatter".to_string());
    }
    let rtk_parser_marker = "src/parser/";
    if let Some(idx) = cleaned.find(rtk_parser_marker) {
        let tail = &cleaned[(idx + rtk_parser_marker.len())..];
        let parser_name = tail
            .split(['/', '.'])
            .next()
            .map(str::trim)
            .unwrap_or_default()
            .to_ascii_lowercase()
            .replace('-', "_");
        if !parser_name.is_empty() {
            return Some(format!("tooling/rtk/parser_{parser_name}"));
        }
    }
    if cleaned.ends_with("src/main.rs") {
        return Some("tooling/rtk/root_main".to_string());
    }
    if cleaned.ends_with("src/cmds/mod.rs") {
        return Some("tooling/rtk/cmds/root_mod".to_string());
    }
    let rtk_cloud_cmd_marker = "src/cmds/cloud/";
    if let Some(idx) = cleaned.find(rtk_cloud_cmd_marker) {
        let tail = &cleaned[(idx + rtk_cloud_cmd_marker.len())..];
        let cmd_name = tail
            .split(['/', '.'])
            .next()
            .map(str::trim)
            .unwrap_or_default();
        if !cmd_name.is_empty() {
            return Some(format!("tooling/rtk/cmds/cloud_{cmd_name}"));
        }
    }
    let rtk_dotnet_cmd_marker = "src/cmds/dotnet/";
    if let Some(idx) = cleaned.find(rtk_dotnet_cmd_marker) {
        let tail = &cleaned[(idx + rtk_dotnet_cmd_marker.len())..];
        let cmd_name = tail
            .split(['/', '.'])
            .next()
            .map(str::trim)
            .unwrap_or_default();
        if !cmd_name.is_empty() {
            return Some(format!("tooling/rtk/cmds/dotnet_{cmd_name}"));
        }
    }
    let rtk_git_cmd_marker = "src/cmds/git/";
    if let Some(idx) = cleaned.find(rtk_git_cmd_marker) {
        let tail = &cleaned[(idx + rtk_git_cmd_marker.len())..];
        let cmd_name = tail
            .split(['/', '.'])
            .next()
            .map(str::trim)
            .unwrap_or_default();
        if !cmd_name.is_empty() {
            return Some(format!("tooling/rtk/cmds/git_{cmd_name}"));
        }
    }
    let rtk_go_cmd_marker = "src/cmds/go/";
    if let Some(idx) = cleaned.find(rtk_go_cmd_marker) {
        let tail = &cleaned[(idx + rtk_go_cmd_marker.len())..];
        let cmd_name = tail
            .split(['/', '.'])
            .next()
            .map(str::trim)
            .unwrap_or_default();
        if !cmd_name.is_empty() {
            return Some(format!("tooling/rtk/cmds/go_{cmd_name}"));
        }
    }
    let rtk_js_cmd_marker = "src/cmds/js/";
    if let Some(idx) = cleaned.find(rtk_js_cmd_marker) {
        let tail = &cleaned[(idx + rtk_js_cmd_marker.len())..];
        let cmd_name = tail
            .split(['/', '.'])
            .next()
            .map(str::trim)
            .unwrap_or_default();
        if !cmd_name.is_empty() {
            return Some(format!("tooling/rtk/cmds/js_{cmd_name}"));
        }
    }
    let rtk_python_cmd_marker = "src/cmds/python/";
    if let Some(idx) = cleaned.find(rtk_python_cmd_marker) {
        let tail = &cleaned[(idx + rtk_python_cmd_marker.len())..];
        let cmd_name = tail
            .split(['/', '.'])
            .next()
            .map(str::trim)
            .unwrap_or_default();
        if !cmd_name.is_empty() {
            return Some(format!("tooling/rtk/cmds/python_{cmd_name}"));
        }
    }
    let rtk_ruby_cmd_marker = "src/cmds/ruby/";
    if let Some(idx) = cleaned.find(rtk_ruby_cmd_marker) {
        let tail = &cleaned[(idx + rtk_ruby_cmd_marker.len())..];
        let cmd_name = tail
            .split(['/', '.'])
            .next()
            .map(str::trim)
            .unwrap_or_default();
        if !cmd_name.is_empty() {
            return Some(format!("tooling/rtk/cmds/ruby_{cmd_name}"));
        }
    }
    let rtk_analytics_marker = "src/analytics/";
    if let Some(idx) = cleaned.find(rtk_analytics_marker) {
        let tail = &cleaned[(idx + rtk_analytics_marker.len())..];
        let surface_name = tail
            .split(['/', '.'])
            .next()
            .map(str::trim)
            .unwrap_or_default()
            .to_ascii_lowercase()
            .replace('-', "_");
        if !surface_name.is_empty() {
            return Some(format!("tooling/rtk/analytics_{surface_name}"));
        }
    }
    if cleaned.ends_with("src/cmds/README.md") {
        return Some("tooling/rtk/docs_cmds_readme".to_string());
    }
    if cleaned.ends_with("src/cmds/cloud/README.md") {
        return Some("tooling/rtk/docs_cmds_cloud_readme".to_string());
    }
    if cleaned.ends_with("src/cmds/dotnet/README.md") {
        return Some("tooling/rtk/docs_cmds_dotnet_readme".to_string());
    }
    if cleaned.ends_with("src/cmds/git/README.md") {
        return Some("tooling/rtk/docs_cmds_git_readme".to_string());
    }
    if cleaned.ends_with("src/cmds/go/README.md") {
        return Some("tooling/rtk/docs_cmds_go_readme".to_string());
    }
    if cleaned.ends_with("src/cmds/js/README.md") {
        return Some("tooling/rtk/docs_cmds_js_readme".to_string());
    }
    if cleaned.ends_with("src/cmds/rust/README.md") {
        return Some("tooling/rtk/docs_cmds_rust_readme".to_string());
    }
    if cleaned.ends_with("src/cmds/system/README.md") {
        return Some("tooling/rtk/docs_cmds_system_readme".to_string());
    }
    if cleaned.ends_with("src/core/README.md") {
        return Some("tooling/rtk/docs_core_readme".to_string());
    }
    if cleaned.ends_with("src/discover/README.md") {
        return Some("tooling/rtk/docs_discover_readme".to_string());
    }
    if cleaned.ends_with("src/filters/README.md") {
        return Some("tooling/rtk/docs_filters_readme".to_string());
    }
    let rtk_claude_agents_marker = ".claude/agents/";
    if let Some(idx) = cleaned.find(rtk_claude_agents_marker) {
        let tail = &cleaned[(idx + rtk_claude_agents_marker.len())..];
        let agent_name = tail
            .split('.')
            .next()
            .map(str::trim)
            .unwrap_or_default()
            .to_ascii_lowercase()
            .replace(['-', '/'], "_");
        if !agent_name.is_empty() {
            return Some(format!("tooling/rtk/claude_agents_{agent_name}"));
        }
    }
    let rtk_claude_commands_marker = ".claude/commands/";
    if let Some(idx) = cleaned.find(rtk_claude_commands_marker) {
        let tail = &cleaned[(idx + rtk_claude_commands_marker.len())..];
        let command_name = tail
            .split('.')
            .next()
            .map(str::trim)
            .unwrap_or_default()
            .to_ascii_lowercase()
            .replace(['-', '/'], "_");
        if !command_name.is_empty() {
            return Some(format!("tooling/rtk/claude_commands_{command_name}"));
        }
    }
    let rtk_claude_hooks_marker = ".claude/hooks/";
    if let Some(idx) = cleaned.find(rtk_claude_hooks_marker) {
        let tail = &cleaned[(idx + rtk_claude_hooks_marker.len())..];
        let hook_name = tail
            .split('.')
            .next()
            .map(str::trim)
            .unwrap_or_default()
            .to_ascii_lowercase()
            .replace(['-', '/'], "_");
        if !hook_name.is_empty() {
            return Some(format!("tooling/rtk/claude_hooks_{hook_name}"));
        }
    }
    let rtk_claude_rules_marker = ".claude/rules/";
    if let Some(idx) = cleaned.find(rtk_claude_rules_marker) {
        let tail = &cleaned[(idx + rtk_claude_rules_marker.len())..];
        let rule_name = tail
            .split('.')
            .next()
            .map(str::trim)
            .unwrap_or_default()
            .to_ascii_lowercase()
            .replace(['-', '/'], "_");
        if !rule_name.is_empty() {
            return Some(format!("tooling/rtk/claude_rules_{rule_name}"));
        }
    }
    let rtk_claude_skills_marker = ".claude/skills/";
    if let Some(idx) = cleaned.find(rtk_claude_skills_marker) {
        let tail = &cleaned[(idx + rtk_claude_skills_marker.len())..];
        let skill_name = tail
            .split('.')
            .next()
            .map(str::trim)
            .unwrap_or_default()
            .to_ascii_lowercase()
            .replace(['-', '/'], "_");
        if !skill_name.is_empty() {
            return Some(format!("tooling/rtk/claude_skills_{skill_name}"));
        }
    }
    if cleaned.ends_with(".github/PULL_REQUEST_TEMPLATE.md") {
        return Some("tooling/rtk/github_pr_template".to_string());
    }
    if cleaned.ends_with(".github/copilot-instructions.md") {
        return Some("tooling/rtk/github_copilot_instructions".to_string());
    }
    let rtk_github_hooks_marker = ".github/hooks/";
    if let Some(idx) = cleaned.find(rtk_github_hooks_marker) {
        let tail = &cleaned[(idx + rtk_github_hooks_marker.len())..];
        let hook_name = tail
            .split('.')
            .next()
            .map(str::trim)
            .unwrap_or_default()
            .to_ascii_lowercase()
            .replace(['-', '/'], "_");
        if !hook_name.is_empty() {
            return Some(format!("tooling/rtk/github_hooks_{hook_name}"));
        }
    }
    let rtk_github_workflows_marker = ".github/workflows/";
    if let Some(idx) = cleaned.find(rtk_github_workflows_marker) {
        let tail = &cleaned[(idx + rtk_github_workflows_marker.len())..];
        let workflow_name = tail
            .split('.')
            .next()
            .map(str::trim)
            .unwrap_or_default()
            .to_ascii_lowercase()
            .replace(['-', '/'], "_");
        if !workflow_name.is_empty() {
            return Some(format!("tooling/rtk/github_workflows_{workflow_name}"));
        }
    }
    if cleaned.ends_with(".gitignore") {
        return Some("tooling/rtk/root_gitignore".to_string());
    }
    if cleaned.ends_with(".release-please-manifest.json") {
        return Some("tooling/rtk/root_release_please_manifest".to_string());
    }
    if cleaned.ends_with("ARCHITECTURE.md") {
        return Some("tooling/rtk/docs_architecture".to_string());
    }
    if cleaned.ends_with("CHANGELOG.md") {
        return Some("tooling/rtk/docs_changelog_root".to_string());
    }
    if cleaned.ends_with("CLAUDE.md") {
        return Some("tooling/rtk/docs_claude".to_string());
    }
    if cleaned.ends_with("CONTRIBUTING.md") {
        return Some("tooling/rtk/docs_contributing".to_string());
    }
    if cleaned.ends_with("INSTALL.md") {
        return Some("tooling/rtk/docs_install".to_string());
    }
    if cleaned.ends_with("README.md") {
        return Some("tooling/rtk/docs_readme".to_string());
    }
    if cleaned.ends_with("README_es.md") {
        return Some("tooling/rtk/docs_readme_es".to_string());
    }
    if cleaned.ends_with("README_fr.md") {
        return Some("tooling/rtk/docs_readme_fr".to_string());
    }
    if cleaned.ends_with("README_ja.md") {
        return Some("tooling/rtk/docs_readme_ja".to_string());
    }
    if cleaned.ends_with("README_ko.md") {
        return Some("tooling/rtk/docs_readme_ko".to_string());
    }
    if cleaned.ends_with("README_zh.md") {
        return Some("tooling/rtk/docs_readme_zh".to_string());
    }
    if cleaned.ends_with("ROADMAP.md") {
        return Some("tooling/rtk/docs_roadmap".to_string());
    }
    if cleaned.ends_with("SECURITY.md") {
        return Some("tooling/rtk/docs_security".to_string());
    }
    if cleaned == "build.rs" || cleaned.ends_with("/build.rs") {
        return Some("tooling/rtk/root_build_rs".to_string());
    }
    if cleaned.ends_with("docs/AUDIT_GUIDE.md") {
        return Some("tooling/rtk/docs_audit_guide".to_string());
    }
    if cleaned.ends_with("docs/FEATURES.md") {
        return Some("tooling/rtk/docs_features".to_string());
    }
    if cleaned.ends_with("docs/TECHNICAL.md") {
        return Some("tooling/rtk/docs_technical".to_string());
    }
    if cleaned.ends_with("docs/TROUBLESHOOTING.md") {
        return Some("tooling/rtk/docs_troubleshooting".to_string());
    }
    if cleaned.ends_with("docs/filter-workflow.md") {
        return Some("tooling/rtk/docs_filter_workflow".to_string());
    }
    if cleaned.ends_with("docs/tracking.md") {
        return Some("tooling/rtk/docs_tracking".to_string());
    }
    if cleaned.ends_with("cargo.lock") {
        return Some("tooling/rtk/root_cargo_lock".to_string());
    }
    if cleaned.ends_with("cargo.toml") {
        return Some("tooling/rtk/root_cargo_toml".to_string());
    }
    if cleaned == "license" || cleaned.ends_with("/license") {
        return Some("tooling/rtk/root_license".to_string());
    }
    if cleaned.ends_with("formula/rtk.rb") {
        return Some("tooling/rtk/formula_rtk".to_string());
    }
    if cleaned.ends_with("docs/images/gain-dashboard.jpg") {
        return Some("tooling/rtk/docs_images_gain_dashboard".to_string());
    }
    if cleaned.ends_with("docs/maintainers/maintainers_apply.md") {
        return Some("tooling/rtk/docs_maintainers_apply".to_string());
    }
    if cleaned.ends_with("cli/esbuild.mts") {
        return Some("tooling/rtk/cline_esbuild".to_string());
    }
    if cleaned.ends_with("cli/package.json") {
        return Some("tooling/rtk/cline_package".to_string());
    }
    if cleaned.ends_with("cli/scripts/update-brew-formula.mts") {
        return Some("tooling/rtk/cline_update_brew_formula".to_string());
    }
    if cleaned.ends_with("cli/tsconfig.json") {
        return Some("tooling/rtk/cline_tsconfig".to_string());
    }
    if cleaned.ends_with("cli/tsconfig.lib.json") {
        return Some("tooling/rtk/cline_tsconfig_lib".to_string());
    }
    if cleaned.ends_with("cli/vitest.config.ts") {
        return Some("tooling/rtk/cline_vitest_config".to_string());
    }
    if cleaned.ends_with("cli/workspacestate.json") {
        return Some("tooling/rtk/cline_workspace_state".to_string());
    }
    if cleaned.ends_with("cli/src/stub-devtools.js") {
        return Some("tooling/rtk/cline_stub_devtools".to_string());
    }
    if cleaned.ends_with("cli/src/utils/mcp.test.ts") {
        return Some("tooling/rtk/cline_utils_mcp_test".to_string());
    }
    if cleaned.ends_with("cli/src/utils/slash-commands.test.ts") {
        return Some("tooling/rtk/cline_utils_slash_commands_test".to_string());
    }
    if cleaned.ends_with("cli/src/index.test.ts") {
        return Some("tooling/rtk/cline_index_test".to_string());
    }
    if cleaned.ends_with("cli/src/lib-import.test.ts") {
        return Some("tooling/rtk/cline_lib_import_test".to_string());
    }
    if cleaned.ends_with("cli/src/utils/__tests__/diffcomputer.test.ts") {
        return Some("tooling/rtk/cline_utils_diff_computer_test".to_string());
    }
    if cleaned.ends_with("cli/src/utils/display.test.ts") {
        return Some("tooling/rtk/cline_utils_display_test".to_string());
    }
    if cleaned.ends_with("cli/src/utils/kanban.test.ts") {
        return Some("tooling/rtk/cline_utils_kanban_test".to_string());
    }
    if cleaned.ends_with("cli/src/utils/mode-selection.test.ts") {
        return Some("tooling/rtk/cline_utils_mode_selection_test".to_string());
    }
    if cleaned.ends_with("cli/src/utils/parser.test.ts") {
        return Some("tooling/rtk/cline_utils_parser_test".to_string());
    }
    if cleaned.ends_with("cli/src/utils/piped.test.ts") {
        return Some("tooling/rtk/cline_utils_piped_test".to_string());
    }
    if cleaned.ends_with("cli/src/utils/plain-text-task.test.ts") {
        return Some("tooling/rtk/cline_utils_plain_text_task_test".to_string());
    }
    if cleaned.ends_with("cli/src/utils/task-history.test.ts") {
        return Some("tooling/rtk/cline_utils_task_history_test".to_string());
    }
    if cleaned.ends_with("docs/provider-config/openai-codex.mdx") {
        return Some("tooling/rtk/docs_provider_openai_codex".to_string());
    }
    if cleaned.ends_with(
        "src/core/prompts/system-prompt/__tests__/__snapshots__/openai_gpt_5_codex-basic.snap",
    ) {
        return Some("tooling/rtk/prompts_openai_gpt_5_codex_basic_snap".to_string());
    }
    if cleaned.ends_with(
        "src/core/prompts/system-prompt/__tests__/__snapshots__/openai_gpt_5_codex-no-browser.snap",
    ) {
        return Some("tooling/rtk/prompts_openai_gpt_5_codex_no_browser_snap".to_string());
    }
    if cleaned.ends_with(
        "src/core/prompts/system-prompt/__tests__/__snapshots__/openai_gpt_5_codex-no-focus-chain.snap",
    ) {
        return Some("tooling/rtk/prompts_openai_gpt_5_codex_no_focus_chain_snap".to_string());
    }
    if cleaned.ends_with(
        "src/core/prompts/system-prompt/__tests__/__snapshots__/openai_gpt_5_codex-no-mcp.snap",
    ) {
        return Some("tooling/rtk/prompts_openai_gpt_5_codex_no_mcp_snap".to_string());
    }
    if cleaned == ".codex/environments/environment.toml"
        || cleaned.ends_with(".codex/environments/environment.toml")
    {
        return Some("tooling/rtk/codex_environment_toml".to_string());
    }
    let workspace_hooks_marker = "hooks/";
    if let Some(idx) = cleaned.find(workspace_hooks_marker) {
        let tail = &cleaned[(idx + workspace_hooks_marker.len())..];
        let hook_name = tail
            .split('.')
            .next()
            .map(str::trim)
            .unwrap_or_default()
            .to_ascii_lowercase()
            .replace(['-', '/'], "_");
        if !hook_name.is_empty() {
            return Some(format!("tooling/rtk/workspace_hooks_{hook_name}"));
        }
    }
    let openclaw_marker = "openclaw/";
    if let Some(idx) = cleaned.find(openclaw_marker) {
        let tail = &cleaned[(idx + openclaw_marker.len())..];
        let mut openclaw_name = tail.trim().to_ascii_lowercase();
        for suffix in [
            ".md", ".ts", ".json", ".sh", ".txt", ".rb", ".toml", ".lock",
        ] {
            if let Some(stripped) = openclaw_name.strip_suffix(suffix) {
                openclaw_name = stripped.to_string();
                break;
            }
        }
        let openclaw_name = openclaw_name.replace(['-', '/', '.'], "_");
        if !openclaw_name.is_empty() {
            return Some(format!("tooling/rtk/openclaw_{openclaw_name}"));
        }
    }
    let scripts_marker = "scripts/";
    if let Some(idx) = cleaned.find(scripts_marker) {
        let tail = &cleaned[(idx + scripts_marker.len())..];
        let script_name = tail
            .split('.')
            .next()
            .map(str::trim)
            .unwrap_or_default()
            .to_ascii_lowercase()
            .replace(['-', '/'], "_");
        if !script_name.is_empty() {
            return Some(format!("tooling/rtk/scripts_{script_name}"));
        }
    }
    let fixtures_marker = "tests/fixtures/";
    if let Some(idx) = cleaned.find(fixtures_marker) {
        let tail = &cleaned[(idx + fixtures_marker.len())..];
        let mut fixture_name = tail.trim().to_ascii_lowercase();
        for suffix in [".txt", ".json", ".md", ".sh"] {
            if let Some(stripped) = fixture_name.strip_suffix(suffix) {
                fixture_name = stripped.to_string();
                break;
            }
        }
        let fixture_name = fixture_name.replace(['-', '/', '.'], "_");
        if !fixture_name.is_empty() {
            return Some(format!("tooling/rtk/tests_fixtures_{fixture_name}"));
        }
    }
    if cleaned.ends_with("install.sh") {
        return Some("tooling/rtk/root_install_sh".to_string());
    }
    if cleaned.ends_with("release-please-config.json") {
        return Some("tooling/rtk/root_release_please_config".to_string());
    }
    let rtk_system_cmd_marker = "src/cmds/system/";
    if let Some(idx) = cleaned.find(rtk_system_cmd_marker) {
        let tail = &cleaned[(idx + rtk_system_cmd_marker.len())..];
        let cmd_name = tail
            .split(['/', '.'])
            .next()
            .map(str::trim)
            .unwrap_or_default();
        if !cmd_name.is_empty() {
            return Some(format!("tooling/rtk/cmds/system_{cmd_name}"));
        }
    }
    let rtk_rust_cmd_marker = "src/cmds/rust/";
    if let Some(idx) = cleaned.find(rtk_rust_cmd_marker) {
        let tail = &cleaned[(idx + rtk_rust_cmd_marker.len())..];
        let cmd_name = tail
            .split(['/', '.'])
            .next()
            .map(str::trim)
            .unwrap_or_default();
        if !cmd_name.is_empty() {
            return Some(format!("tooling/rtk/cmds/rust_{cmd_name}"));
        }
    }
    let rtk_core_marker = "src/core/";
    if let Some(idx) = cleaned.find(rtk_core_marker) {
        let tail = &cleaned[(idx + rtk_core_marker.len())..];
        let core_name = tail
            .split(['/', '.'])
            .next()
            .map(str::trim)
            .unwrap_or_default();
        if !core_name.is_empty() {
            return Some(format!("tooling/rtk/core_{core_name}"));
        }
    }
    let rtk_filter_marker = "src/filters/";
    if let Some(idx) = cleaned.find(rtk_filter_marker) {
        let tail = &cleaned[(idx + rtk_filter_marker.len())..];
        let filter_name = tail
            .split(['/', '.'])
            .next()
            .map(str::trim)
            .unwrap_or_default()
            .replace('-', "_");
        if !filter_name.is_empty() {
            return Some(format!("tooling/rtk/filters_{filter_name}"));
        }
    }
    if cleaned.ends_with("src/core/telemetry.rs") {
        return Some("tooling/rtk/core_telemetry".to_string());
    }
    let cli_marker = "cli/src/";
    if let Some(idx) = cleaned.find(cli_marker) {
        let tail = &cleaned[(idx + cli_marker.len())..];
        if let Some(acp_tail) = tail.strip_prefix("acp/") {
            let acp_name = acp_tail
                .split(['/', '.'])
                .next()
                .map(str::trim)
                .unwrap_or_default();
            if !acp_name.is_empty() {
                return Some(format!("tooling/cli/acp/{acp_name}"));
            }
        }
        if let Some(agent_tail) = tail.strip_prefix("agent/") {
            let agent_name = agent_tail
                .split(['/', '.'])
                .next()
                .map(str::trim)
                .unwrap_or_default();
            if !agent_name.is_empty() {
                return Some(format!("tooling/cli/agent/{agent_name}"));
            }
        }
        if let Some(context_tail) = tail.strip_prefix("context/") {
            let context_name = context_tail
                .split(['/', '.'])
                .next()
                .map(str::trim)
                .unwrap_or_default();
            if !context_name.is_empty() {
                return Some(format!("tooling/cli/context/{context_name}"));
            }
        }
        if let Some(hooks_tail) = tail.strip_prefix("hooks/") {
            let hook_name = hooks_tail
                .split(['/', '.'])
                .next()
                .map(str::trim)
                .unwrap_or_default();
            if !hook_name.is_empty() {
                return Some(format!("tooling/cli/hooks/{hook_name}"));
            }
        }
        if let Some(component_tail) = tail.strip_prefix("components/") {
            let component_name = component_tail
                .split(['/', '.'])
                .next()
                .map(str::trim)
                .unwrap_or_default();
            if !component_name.is_empty() {
                return Some(format!("tooling/cli/components/{component_name}"));
            }
        }
        if let Some(controller_tail) = tail.strip_prefix("controllers/") {
            let controller_name = controller_tail
                .split(['/', '.'])
                .next()
                .map(str::trim)
                .unwrap_or_default();
            if !controller_name.is_empty() {
                return Some(format!("tooling/cli/controllers/{controller_name}"));
            }
        }
        if let Some(utils_tail) = tail.strip_prefix("utils/") {
            let util_name = utils_tail
                .split(['/', '.'])
                .next()
                .map(str::trim)
                .unwrap_or_default();
            if !util_name.is_empty() {
                return Some(format!("tooling/cli/utils/{util_name}"));
            }
        }
        if let Some(constants_tail) = tail.strip_prefix("constants/") {
            let constant_name = constants_tail
                .split(['/', '.'])
                .next()
                .map(str::trim)
                .unwrap_or_default();
            if !constant_name.is_empty() {
                return Some(format!("tooling/cli/constants/{constant_name}"));
            }
        }
        if !tail.contains('/') {
            let root_name = tail
                .split(['/', '.'])
                .next()
                .map(str::trim)
                .unwrap_or_default();
            if !root_name.is_empty() {
                return Some(format!("tooling/cli/root/{root_name}"));
            }
        }
    }
    if cleaned.contains("webview-ui/src/utils/__tests__/hooks.spec") {
        return Some("utils/hooks_spec".to_string());
    }
    let hooks_marker = "webview-ui/src/hooks/";
    if let Some(idx) = cleaned.find(hooks_marker) {
        let tail = &cleaned[(idx + hooks_marker.len())..];
        let hook_name = tail
            .split(['/', '.'])
            .next()
            .map(str::trim)
            .unwrap_or_default();
        if !hook_name.is_empty() {
            return Some(format!("hooks/{hook_name}"));
        }
    }
    if cleaned.ends_with("webview-ui/src/providers.tsx") {
        return Some("root/providers/providers".to_string());
    }
    if cleaned.ends_with("webview-ui/src/customposthogprovider.tsx") {
        return Some("root/providers/custom_posthog".to_string());
    }
    let context_marker = "webview-ui/src/context/";
    if let Some(idx) = cleaned.find(context_marker) {
        let tail = &cleaned[(idx + context_marker.len())..];
        let context_name = tail
            .split(['/', '.'])
            .next()
            .map(str::trim)
            .unwrap_or_default();
        if !context_name.is_empty() {
            return Some(format!("context/{context_name}"));
        }
    }
    let provider_marker = "webview-ui/src/components/settings/providers/";
    if let Some(idx) = cleaned.find(provider_marker) {
        let tail = &cleaned[(idx + provider_marker.len())..];
        let provider_raw = tail
            .split(['/', '.'])
            .next()
            .map(str::trim)
            .unwrap_or_default();
        let mut provider = provider_raw.to_string();
        if provider.ends_with("provider") {
            provider = provider.trim_end_matches("provider").to_string();
        }
        provider = match provider.as_str() {
            "openaicompatible" => "openai_compatible".to_string(),
            "openainative" => "openai_native".to_string(),
            "openaicodex" => "openai_codex".to_string(),
            "openrouter" => "openrouter".to_string(),
            "ocamodelpicker" => "oca_model_picker".to_string(),
            "nousresearch" => "nous_research".to_string(),
            "qwencode" => "qwen_code".to_string(),
            "qwen" => "qwen".to_string(),
            "requesty" => "requesty".to_string(),
            "sambanova" => "sambanova".to_string(),
            "sapaicore" => "sap_ai_core".to_string(),
            "vscodelm" => "vscode_lm".to_string(),
            "vercelaigateway" => "vercel_ai_gateway".to_string(),
            "litellm" => "lite_llm".to_string(),
            "huaweicloudmaas" => "huawei_cloud_maas".to_string(),
            other => other.to_string(),
        };
        if !provider.is_empty() {
            return Some(format!("settings/providers/{provider}"));
        }
    }
    None
}

fn provider_family_key(surface: &str) -> Option<&'static str> {
    let provider = surface.strip_prefix("settings/providers/")?;
    match provider {
        "openai_native" | "openai_compatible" | "openai_codex" => Some("openai"),
        "openrouter" | "requesty" | "together" | "lite_llm" | "vercel_ai_gateway" => {
            Some("aggregator")
        }
        "qwen" | "qwen_code" => Some("qwen"),
        "anthropic" | "claude_code" => Some("anthropic"),
        "ollama" | "lmstudio" | "vscode_lm" => Some("local_runtime"),
        "bedrock" | "vertex" | "sap_ai_core" | "huawei_cloud_maas" => Some("enterprise_cloud"),
        _ => Some("independent"),
    }
}

fn clamp_unit(value: f64) -> f64 {
    value.clamp(0.0, 1.0)
}

impl StructuredVerifier {
    pub fn derive_claim_bundle(
        &self,
        task_id: &str,
        evidence_cards: &[EvidenceCard],
    ) -> ClaimBundle {
        let mut claims = Vec::<Claim>::new();
        let mut conflicts = Vec::<String>::new();
        let mut unresolved_questions = Vec::<String>::new();
        for (claim_index, card) in evidence_cards.iter().enumerate() {
            let status = support_status(&card.confidence_vector, &card.summary);
            if status == ClaimStatus::Unsupported {
                unresolved_questions.push(format!(
                    "Need stronger evidence for source {}",
                    card.source_ref
                ));
            }
            let claim_content_id = deterministic_hash(&serde_json::json!({
                "kind":"claim_content",
                "task_id": task_id,
                "evidence_content_id": card.evidence_content_id,
                "text": card.summary,
            }));
            let claim_event_id = deterministic_hash(&serde_json::json!({
                "kind":"claim_event",
                "task_id": task_id,
                "claim_content_id": claim_content_id,
                "evidence_event_id": card.evidence_event_id,
                "claim_index": claim_index,
            }));
            let claim = Claim {
                claim_id: claim_content_id.clone(),
                claim_content_id,
                claim_event_id,
                text: card.summary.clone(),
                evidence_ids: vec![card.evidence_id.clone()],
                status,
                confidence_vector: card.confidence_vector.clone(),
                conflict_refs: Vec::new(),
            };
            claims.push(claim);
        }
        let mut text_index = HashMap::<String, Vec<usize>>::new();
        for (idx, claim) in claims.iter().enumerate() {
            let key = claim.text.to_ascii_lowercase();
            text_index.entry(key).or_default().push(idx);
        }
        for indexes in text_index.values() {
            if indexes.len() < 2 {
                continue;
            }
            let mut has_negative = false;
            let mut has_positive = false;
            for idx in indexes {
                if has_negative_cue(&claims[*idx].text) {
                    has_negative = true;
                } else {
                    has_positive = true;
                }
            }
            if has_negative && has_positive {
                for idx in indexes {
                    claims[*idx].status = ClaimStatus::Conflicting;
                    claims[*idx].conflict_refs = indexes
                        .iter()
                        .filter(|row| **row != *idx)
                        .map(|row| claims[*row].claim_id.clone())
                        .collect::<Vec<_>>();
                    conflicts.push(claims[*idx].claim_id.clone());
                }
            }
        }
        let mut topic_index = HashMap::<String, Vec<usize>>::new();
        for (idx, claim) in claims.iter().enumerate() {
            let topic = contradiction_topic_key(&claim.text);
            if topic.len() >= 8 {
                topic_index.entry(topic).or_default().push(idx);
            }
        }
        for indexes in topic_index.values() {
            if indexes.len() < 2 {
                continue;
            }
            let mut has_negative = false;
            let mut has_positive = false;
            for idx in indexes {
                if has_negative_cue(&claims[*idx].text) {
                    has_negative = true;
                } else {
                    has_positive = true;
                }
            }
            if has_negative && has_positive {
                for idx in indexes {
                    claims[*idx].status = ClaimStatus::Conflicting;
                    claims[*idx].conflict_refs = indexes
                        .iter()
                        .filter(|row| **row != *idx)
                        .map(|row| claims[*row].claim_id.clone())
                        .collect::<Vec<_>>();
                    conflicts.push(claims[*idx].claim_id.clone());
                }
            }
        }
        let supported_or_partial = claims
            .iter()
            .filter(|claim| matches!(claim.status, ClaimStatus::Supported | ClaimStatus::Partial))
            .count();
        let source_diversity_domains = evidence_cards
            .iter()
            .map(|card| source_domain_key(card.source_ref.as_str()))
            .collect::<HashSet<_>>()
            .len();
        let source_languages = evidence_cards
            .iter()
            .filter_map(|card| source_language_key(card.source_ref.as_str()))
            .collect::<HashSet<_>>();
        let source_language_count = source_languages.len();
        let source_surfaces = evidence_cards
            .iter()
            .filter_map(|card| source_surface_key(card.source_ref.as_str()))
            .collect::<HashSet<_>>();
        let source_surface_count = source_surfaces.len();
        let has_surface_evidence = !source_surfaces.is_empty();
        let provider_surface_count = source_surfaces
            .iter()
            .filter(|surface| surface.starts_with("settings/providers/"))
            .count();
        let provider_family_count = source_surfaces
            .iter()
            .filter_map(|surface| provider_family_key(surface.as_str()))
            .collect::<HashSet<_>>()
            .len();
        let shared_contract_surface_count = source_surfaces
            .iter()
            .filter(|surface| surface.starts_with("shared/contracts/"))
            .count();
        let shared_cline_surface_count = source_surfaces
            .iter()
            .filter(|surface| surface.starts_with("shared/cline/"))
            .count();
        let shared_message_surface_count = source_surfaces
            .iter()
            .filter(|surface| surface.starts_with("shared/messages/"))
            .count();
        let shared_internal_surface_count = source_surfaces
            .iter()
            .filter(|surface| surface.starts_with("shared/internal/"))
            .count();
        let shared_service_surface_count = source_surfaces
            .iter()
            .filter(|surface| surface.starts_with("shared/services/"))
            .count();
        let shared_multi_root_surface_count = source_surfaces
            .iter()
            .filter(|surface| surface.starts_with("shared/multi_root/"))
            .count();
        let chat_hook_surface_count = source_surfaces
            .iter()
            .filter(|surface| surface.starts_with("chat/hooks/"))
            .count();
        let chat_util_surface_count = source_surfaces
            .iter()
            .filter(|surface| surface.starts_with("chat/utils/"))
            .count();
        let chat_message_surface_count = source_surfaces
            .iter()
            .filter(|surface| surface.starts_with("chat/messages/"))
            .count();
        let chat_shared_surface_count = source_surfaces
            .iter()
            .filter(|surface| surface.starts_with("chat/shared/"))
            .count();
        let chat_type_surface_count = source_surfaces
            .iter()
            .filter(|surface| surface.starts_with("chat/types/"))
            .count();
        let chat_view_surface_count = source_surfaces
            .iter()
            .filter(|surface| surface.starts_with("chat/view/"))
            .count();
        let chat_layout_surface_count = source_surfaces
            .iter()
            .filter(|surface| surface.starts_with("chat/layout/"))
            .count();
        let chat_root_surface_count = source_surfaces
            .iter()
            .filter(|surface| surface.starts_with("chat/root/"))
            .count();
        let chat_output_surface_count = source_surfaces
            .iter()
            .filter(|surface| {
                surface.starts_with("chat/root/")
                    && (surface.contains("outputrow")
                        || surface.contains("plancompletionoutputrow"))
            })
            .count();
        let chat_error_surface_count = source_surfaces
            .iter()
            .filter(|surface| {
                surface.starts_with("chat/root/")
                    && (surface.contains("error") || surface.contains("creditlimit"))
            })
            .count();
        let chat_preview_surface_count = source_surfaces
            .iter()
            .filter(|surface| surface.starts_with("chat/root/") && surface.contains("preview"))
            .count();
        let chat_interaction_surface_count = source_surfaces
            .iter()
            .filter(|surface| {
                surface.starts_with("chat/root/")
                    && (surface.contains("expandhandle")
                        || surface.contains("featuretip")
                        || surface.contains("hookmessage")
                        || surface.contains("requeststartrow")
                        || surface.contains("quotebutton")
                        || surface.contains("taskfeedbackbuttons")
                        || surface.contains("subagentstatusrow")
                        || surface.contains("thinkingrow"))
            })
            .count();
        let chat_component_surface_count = source_surfaces
            .iter()
            .filter(|surface| surface.starts_with("chat/components/"))
            .count();
        let chat_task_header_surface_count = source_surfaces
            .iter()
            .filter(|surface| surface.starts_with("chat/task_header/"))
            .count();
        let chat_task_header_button_surface_count = source_surfaces
            .iter()
            .filter(|surface| surface.starts_with("chat/task_header_buttons/"))
            .count();
        let chat_auto_approve_surface_count = source_surfaces
            .iter()
            .filter(|surface| surface.starts_with("chat/auto_approve/"))
            .count();
        let cline_rules_surface_count = source_surfaces
            .iter()
            .filter(|surface| surface.starts_with("cline_rules/"))
            .count();
        let common_component_surface_count = source_surfaces
            .iter()
            .filter(|surface| surface.starts_with("common/components/"))
            .count();
        let common_content_surface_count = source_surfaces
            .iter()
            .filter(|surface| {
                surface.starts_with("common/components/")
                    && (surface.contains("lightmarkdown")
                        || surface.contains("markdownblock")
                        || surface.contains("mermaidblock")
                        || surface.contains("thumbnails")
                        || surface.contains("unsafeimage"))
            })
            .count();
        let common_ui_surface_count = source_surfaces
            .iter()
            .filter(|surface| {
                surface.starts_with("common/components/")
                    && (surface.contains("popupmodalcontainer")
                        || surface.contains("screenreaderannounce")
                        || surface.contains("successbutton")
                        || surface.contains("tab")
                        || surface.contains("telemetrybanner"))
            })
            .count();
        let history_surface_count = source_surfaces
            .iter()
            .filter(|surface| surface.starts_with("history/"))
            .count();
        let menu_surface_count = source_surfaces
            .iter()
            .filter(|surface| surface.starts_with("menu/"))
            .count();
        let onboarding_surface_count = source_surfaces
            .iter()
            .filter(|surface| surface.starts_with("onboarding/"))
            .count();
        let browser_surface_count = source_surfaces
            .iter()
            .filter(|surface| surface.starts_with("browser/"))
            .count();
        let settings_utils_surface_count = source_surfaces
            .iter()
            .filter(|surface| surface.starts_with("settings/utils/"))
            .count();
        let hooks_surface_count = source_surfaces
            .iter()
            .filter(|surface| surface.starts_with("hooks/"))
            .count();
        let root_provider_surface_count = source_surfaces
            .iter()
            .filter(|surface| surface.starts_with("root/providers/"))
            .count();
        let settings_component_surface_count = source_surfaces
            .iter()
            .filter(|surface| surface.starts_with("settings/components/"))
            .count();
        let settings_model_picker_surface_count = source_surfaces
            .iter()
            .filter(|surface| {
                surface.starts_with("settings/components/")
                    && (surface.contains("modelpicker")
                        || surface.contains("modelselector")
                        || surface.contains("reasoningeffortselector")
                        || surface.contains("preferredlanguagesetting"))
            })
            .count();
        let settings_common_surface_count = source_surfaces
            .iter()
            .filter(|surface| surface.starts_with("settings/common/"))
            .count();
        let settings_section_surface_count = source_surfaces
            .iter()
            .filter(|surface| surface.starts_with("settings/sections/"))
            .count();
        let settings_test_surface_count = source_surfaces
            .iter()
            .filter(|surface| surface.starts_with("settings/tests/"))
            .count();
        let settings_control_surface_count = source_surfaces
            .iter()
            .filter(|surface| {
                (surface.starts_with("settings/components/")
                    || surface.starts_with("settings/sections/"))
                    && (surface.contains("slider")
                        || surface.contains("checkbox")
                        || surface.contains("section")
                        || surface.contains("settingsview"))
            })
            .count();
        let tooling_coverage_surface_count = source_surfaces
            .iter()
            .filter(|surface| surface.starts_with("tooling/coverage_check/"))
            .count();
        let tooling_workflow_surface_count = source_surfaces
            .iter()
            .filter(|surface| surface.starts_with("tooling/workflows/"))
            .count();
        let tooling_clinerules_workflow_surface_count = source_surfaces
            .iter()
            .filter(|surface| surface.starts_with("tooling/clinerules/workflows/"))
            .count();
        let tooling_clinerules_doc_surface_count = source_surfaces
            .iter()
            .filter(|surface| surface.starts_with("tooling/clinerules/docs/"))
            .count();
        let tooling_agent_skill_surface_count = source_surfaces
            .iter()
            .filter(|surface| surface.starts_with("tooling/agent_skills/"))
            .count();
        let tooling_storybook_surface_count = source_surfaces
            .iter()
            .filter(|surface| surface.starts_with("tooling/storybook/"))
            .count();
        let tooling_cli_surface_count = source_surfaces
            .iter()
            .filter(|surface| surface.starts_with("tooling/cli/"))
            .count();
        let tooling_contract_surface_count = source_surfaces
            .iter()
            .filter(|surface| surface.starts_with("tooling/contracts/"))
            .count();
        let tooling_rtk_surface_count = source_surfaces
            .iter()
            .filter(|surface| surface.starts_with("tooling/rtk/"))
            .count();
        let tooling_docs_surface_count = source_surfaces
            .iter()
            .filter(|surface| surface.starts_with("tooling/docs/"))
            .count();
        if source_diversity_domains < 2 && evidence_cards.len() > 1 {
            unresolved_questions.push(
                "Evidence sources are concentrated in one domain; add a second independent source."
                    .to_string(),
            );
        }
        if source_language_count == 1 && evidence_cards.len() > 1 {
            unresolved_questions.push(
                "Tooling synthesis references one parser/query language only; add a second language view."
                    .to_string(),
            );
        }
        if has_surface_evidence && source_surface_count == 1 && evidence_cards.len() > 1 {
            unresolved_questions.push(
                "MCP/tooling synthesis references one render/config surface only; add a second surface view."
                    .to_string(),
            );
        }
        if provider_surface_count == 1 && evidence_cards.len() > 1 {
            unresolved_questions.push(
                "Provider synthesis references one provider surface only; add cross-provider evidence."
                    .to_string(),
            );
        }
        if provider_surface_count >= 2 && provider_family_count <= 1 && evidence_cards.len() > 1 {
            unresolved_questions.push(
                "Provider synthesis spans one provider family only; add cross-family evidence."
                    .to_string(),
            );
        }
        if shared_contract_surface_count == 1 && evidence_cards.len() > 1 {
            unresolved_questions.push(
                "Shared-contract synthesis references one contract surface only; add cross-contract evidence."
                    .to_string(),
            );
        }
        if shared_cline_surface_count == 1 && evidence_cards.len() > 1 {
            unresolved_questions.push(
                "Shared cline synthesis references one cline surface only; add cross-cline evidence."
                    .to_string(),
            );
        }
        if shared_message_surface_count == 1 && evidence_cards.len() > 1 {
            unresolved_questions.push(
                "Shared message synthesis references one message surface only; add cross-message evidence."
                    .to_string(),
            );
        }
        if shared_internal_surface_count == 1 && evidence_cards.len() > 1 {
            unresolved_questions.push(
                "Shared internal synthesis references one internal surface only; add cross-internal evidence."
                    .to_string(),
            );
        }
        if shared_service_surface_count == 1 && evidence_cards.len() > 1 {
            unresolved_questions.push(
                "Shared service synthesis references one service surface only; add cross-service evidence."
                    .to_string(),
            );
        }
        if shared_multi_root_surface_count == 1 && evidence_cards.len() > 1 {
            unresolved_questions.push(
                "Shared multi-root synthesis references one surface only; add cross-surface evidence."
                    .to_string(),
            );
        }
        if chat_hook_surface_count == 1 && evidence_cards.len() > 1 {
            unresolved_questions.push(
                "Chat hook synthesis references one hook surface only; add cross-hook evidence."
                    .to_string(),
            );
        }
        if chat_util_surface_count == 1 && evidence_cards.len() > 1 {
            unresolved_questions.push(
                "Chat utility synthesis references one util surface only; add cross-util evidence."
                    .to_string(),
            );
        }
        if chat_message_surface_count == 1 && evidence_cards.len() > 1 {
            unresolved_questions.push(
                "Chat message synthesis references one message surface only; add cross-message evidence."
                    .to_string(),
            );
        }
        if chat_shared_surface_count == 1 && evidence_cards.len() > 1 {
            unresolved_questions.push(
                "Chat shared synthesis references one shared surface only; add cross-shared evidence."
                    .to_string(),
            );
        }
        if chat_type_surface_count == 1 && evidence_cards.len() > 1 {
            unresolved_questions.push(
                "Chat type synthesis references one type surface only; add cross-type evidence."
                    .to_string(),
            );
        }
        if chat_view_surface_count == 1 && evidence_cards.len() > 1 {
            unresolved_questions.push(
                "Chat view synthesis references one view surface only; add cross-view evidence."
                    .to_string(),
            );
        }
        if chat_layout_surface_count == 1 && evidence_cards.len() > 1 {
            unresolved_questions.push(
                "Chat layout synthesis references one layout surface only; add cross-layout evidence."
                    .to_string(),
            );
        }
        if chat_root_surface_count == 1 && evidence_cards.len() > 1 {
            unresolved_questions.push(
                "Chat root synthesis references one root surface only; add cross-root evidence."
                    .to_string(),
            );
        }
        if chat_output_surface_count == 1 && evidence_cards.len() > 1 {
            unresolved_questions.push(
                "Chat output synthesis references one output surface only; add cross-output evidence."
                    .to_string(),
            );
        }
        if chat_error_surface_count == 1 && evidence_cards.len() > 1 {
            unresolved_questions.push(
                "Chat error synthesis references one error surface only; add cross-error evidence."
                    .to_string(),
            );
        }
        if chat_preview_surface_count == 1 && evidence_cards.len() > 1 {
            unresolved_questions.push(
                "Chat preview synthesis references one preview surface only; add cross-preview evidence."
                    .to_string(),
            );
        }
        if chat_interaction_surface_count == 1 && evidence_cards.len() > 1 {
            unresolved_questions.push(
                "Chat interaction synthesis references one interaction surface only; add cross-interaction evidence."
                    .to_string(),
            );
        }
        if chat_component_surface_count == 1 && evidence_cards.len() > 1 {
            unresolved_questions.push(
                "Chat component synthesis references one component surface only; add cross-component evidence."
                    .to_string(),
            );
        }
        if chat_task_header_surface_count == 1 && evidence_cards.len() > 1 {
            unresolved_questions.push(
                "Chat task-header synthesis references one surface only; add cross-task-header evidence."
                    .to_string(),
            );
        }
        if chat_task_header_button_surface_count == 1 && evidence_cards.len() > 1 {
            unresolved_questions.push(
                "Chat task-header button synthesis references one surface only; add cross-button evidence."
                    .to_string(),
            );
        }
        if chat_auto_approve_surface_count == 1 && evidence_cards.len() > 1 {
            unresolved_questions.push(
                "Chat auto-approve synthesis references one surface only; add cross-auto-approve evidence."
                    .to_string(),
            );
        }
        if cline_rules_surface_count == 1 && evidence_cards.len() > 1 {
            unresolved_questions.push(
                "Cline-rules synthesis references one surface only; add cross-rule evidence."
                    .to_string(),
            );
        }
        if common_component_surface_count == 1 && evidence_cards.len() > 1 {
            unresolved_questions.push(
                "Common component synthesis references one surface only; add cross-component evidence."
                    .to_string(),
            );
        }
        if common_content_surface_count == 1 && evidence_cards.len() > 1 {
            unresolved_questions.push(
                "Common content synthesis references one surface only; add cross-content evidence."
                    .to_string(),
            );
        }
        if common_ui_surface_count == 1 && evidence_cards.len() > 1 {
            unresolved_questions.push(
                "Common UI synthesis references one surface only; add cross-ui evidence."
                    .to_string(),
            );
        }
        if history_surface_count == 1 && evidence_cards.len() > 1 {
            unresolved_questions.push(
                "History synthesis references one surface only; add cross-history evidence."
                    .to_string(),
            );
        }
        if menu_surface_count == 1 && evidence_cards.len() > 1 {
            unresolved_questions.push(
                "Menu synthesis references one surface only; add cross-menu evidence.".to_string(),
            );
        }
        if onboarding_surface_count == 1 && evidence_cards.len() > 1 {
            unresolved_questions.push(
                "Onboarding synthesis references one surface only; add cross-onboarding evidence."
                    .to_string(),
            );
        }
        if browser_surface_count == 1 && evidence_cards.len() > 1 {
            unresolved_questions.push(
                "Browser synthesis references one surface only; add cross-browser evidence."
                    .to_string(),
            );
        }
        if settings_utils_surface_count == 1 && evidence_cards.len() > 1 {
            unresolved_questions.push(
                "Settings utils synthesis references one surface only; add cross-utils evidence."
                    .to_string(),
            );
        }
        if hooks_surface_count == 1 && evidence_cards.len() > 1 {
            unresolved_questions.push(
                "Hook synthesis references one surface only; add cross-hook evidence.".to_string(),
            );
        }
        if root_provider_surface_count == 1 && evidence_cards.len() > 1 {
            unresolved_questions.push(
                "Root provider synthesis references one surface only; add cross-provider evidence."
                    .to_string(),
            );
        }
        if settings_component_surface_count == 1 && evidence_cards.len() > 1 {
            unresolved_questions.push(
                "Settings synthesis references one component surface only; add cross-component evidence."
                    .to_string(),
            );
        }
        if settings_model_picker_surface_count == 1 && evidence_cards.len() > 1 {
            unresolved_questions.push(
                "Settings synthesis references one model-picker surface only; add cross-picker evidence."
                    .to_string(),
            );
        }
        if settings_common_surface_count == 1 && evidence_cards.len() > 1 {
            unresolved_questions.push(
                "Settings synthesis references one common-input surface only; add cross-common evidence."
                    .to_string(),
            );
        }
        if settings_section_surface_count == 1 && evidence_cards.len() > 1 {
            unresolved_questions.push(
                "Settings synthesis references one section surface only; add cross-section evidence."
                    .to_string(),
            );
        }
        if settings_test_surface_count == 1 && evidence_cards.len() > 1 {
            unresolved_questions.push(
                "Settings synthesis references one test surface only; add cross-test evidence."
                    .to_string(),
            );
        }
        if settings_control_surface_count == 1 && evidence_cards.len() > 1 {
            unresolved_questions.push(
                "Settings synthesis references one control surface only; add cross-control evidence."
                    .to_string(),
            );
        }
        if tooling_coverage_surface_count == 1 && evidence_cards.len() > 1 {
            unresolved_questions.push(
                "Tooling synthesis references one coverage-check surface only; add additional tooling script evidence."
                    .to_string(),
            );
        }
        if tooling_workflow_surface_count == 1 && evidence_cards.len() > 1 {
            unresolved_questions.push(
                "Tooling synthesis references one workflow surface only; add additional CI workflow evidence."
                    .to_string(),
            );
        }
        if tooling_clinerules_workflow_surface_count == 1 && evidence_cards.len() > 1 {
            unresolved_questions.push(
                "Tooling synthesis references one clinerules workflow surface only; add additional workflow policy evidence."
                    .to_string(),
            );
        }
        if tooling_clinerules_doc_surface_count == 1 && evidence_cards.len() > 1 {
            unresolved_questions.push(
                "Tooling synthesis references one clinerules doc surface only; add additional policy doc evidence."
                    .to_string(),
            );
        }
        if tooling_agent_skill_surface_count == 1 && evidence_cards.len() > 1 {
            unresolved_questions.push(
                "Tooling synthesis references one agent-skill surface only; add additional skill/workflow evidence."
                    .to_string(),
            );
        }
        if tooling_storybook_surface_count == 1 && evidence_cards.len() > 1 {
            unresolved_questions.push(
                "Tooling synthesis references one storybook surface only; add additional storybook/dev-surface evidence."
                    .to_string(),
            );
        }
        if tooling_cli_surface_count == 1 && evidence_cards.len() > 1 {
            unresolved_questions.push(
                "Tooling synthesis references one CLI tooling surface only; add additional CLI/tooling evidence."
                    .to_string(),
            );
        }
        if tooling_contract_surface_count == 1 && evidence_cards.len() > 1 {
            unresolved_questions.push(
                "Tooling synthesis references one contract/export surface only; add additional contract evidence."
                    .to_string(),
            );
        }
        if tooling_rtk_surface_count == 1 && evidence_cards.len() > 1 {
            unresolved_questions.push(
                "Tooling synthesis references one RTK surface only; add additional RTK tooling evidence."
                    .to_string(),
            );
        }
        if tooling_docs_surface_count == 1 && evidence_cards.len() > 1 {
            unresolved_questions.push(
                "Tooling synthesis references one docs/changelog surface only; add additional docs evidence."
                    .to_string(),
            );
        }
        let base_coverage = if claims.is_empty() {
            0.0
        } else {
            supported_or_partial as f64 / claims.len() as f64
        };
        let diversity_bonus = if source_diversity_domains >= 3 {
            0.08
        } else if source_diversity_domains == 2 {
            0.04
        } else {
            0.0
        };
        let language_bonus = if source_language_count >= 2 {
            0.03
        } else {
            0.0
        };
        let surface_bonus = if source_surface_count >= 2 { 0.03 } else { 0.0 };
        let provider_bonus = if provider_surface_count >= 2 {
            0.02
        } else {
            0.0
        };
        let provider_family_bonus = if provider_family_count >= 2 {
            0.02
        } else {
            0.0
        };
        let shared_contract_bonus = if shared_contract_surface_count >= 2 {
            0.02
        } else {
            0.0
        };
        let shared_cline_bonus = if shared_cline_surface_count >= 2 {
            0.02
        } else {
            0.0
        };
        let shared_message_bonus = if shared_message_surface_count >= 2 {
            0.02
        } else {
            0.0
        };
        let shared_internal_bonus = if shared_internal_surface_count >= 2 {
            0.02
        } else {
            0.0
        };
        let shared_service_bonus = if shared_service_surface_count >= 2 {
            0.02
        } else {
            0.0
        };
        let shared_multi_root_bonus = if shared_multi_root_surface_count >= 2 {
            0.02
        } else {
            0.0
        };
        let chat_hook_bonus = if chat_hook_surface_count >= 2 {
            0.02
        } else {
            0.0
        };
        let chat_util_bonus = if chat_util_surface_count >= 2 {
            0.02
        } else {
            0.0
        };
        let chat_message_bonus = if chat_message_surface_count >= 2 {
            0.02
        } else {
            0.0
        };
        let chat_shared_bonus = if chat_shared_surface_count >= 2 {
            0.02
        } else {
            0.0
        };
        let chat_type_bonus = if chat_type_surface_count >= 2 {
            0.02
        } else {
            0.0
        };
        let chat_view_bonus = if chat_view_surface_count >= 2 {
            0.02
        } else {
            0.0
        };
        let chat_layout_bonus = if chat_layout_surface_count >= 2 {
            0.02
        } else {
            0.0
        };
        let chat_root_bonus = if chat_root_surface_count >= 2 {
            0.02
        } else {
            0.0
        };
        let chat_output_bonus = if chat_output_surface_count >= 2 {
            0.02
        } else {
            0.0
        };
        let chat_error_bonus = if chat_error_surface_count >= 2 {
            0.02
        } else {
            0.0
        };
        let chat_preview_bonus = if chat_preview_surface_count >= 2 {
            0.02
        } else {
            0.0
        };
        let chat_interaction_bonus = if chat_interaction_surface_count >= 2 {
            0.02
        } else {
            0.0
        };
        let chat_component_bonus = if chat_component_surface_count >= 2 {
            0.02
        } else {
            0.0
        };
        let chat_task_header_bonus = if chat_task_header_surface_count >= 2 {
            0.02
        } else {
            0.0
        };
        let chat_task_header_button_bonus = if chat_task_header_button_surface_count >= 2 {
            0.02
        } else {
            0.0
        };
        let chat_auto_approve_bonus = if chat_auto_approve_surface_count >= 2 {
            0.02
        } else {
            0.0
        };
        let cline_rules_bonus = if cline_rules_surface_count >= 2 {
            0.02
        } else {
            0.0
        };
        let common_component_bonus = if common_component_surface_count >= 2 {
            0.02
        } else {
            0.0
        };
        let common_content_bonus = if common_content_surface_count >= 2 {
            0.02
        } else {
            0.0
        };
        let common_ui_bonus = if common_ui_surface_count >= 2 {
            0.02
        } else {
            0.0
        };
        let history_bonus = if history_surface_count >= 2 {
            0.02
        } else {
            0.0
        };
        let menu_bonus = if menu_surface_count >= 2 { 0.02 } else { 0.0 };
        let onboarding_bonus = if onboarding_surface_count >= 2 {
            0.02
        } else {
            0.0
        };
        let browser_bonus = if browser_surface_count >= 2 {
            0.02
        } else {
            0.0
        };
        let settings_utils_bonus = if settings_utils_surface_count >= 2 {
            0.02
        } else {
            0.0
        };
        let hooks_bonus = if hooks_surface_count >= 2 { 0.02 } else { 0.0 };
        let root_provider_bonus = if root_provider_surface_count >= 2 {
            0.02
        } else {
            0.0
        };
        let settings_component_bonus = if settings_component_surface_count >= 2 {
            0.02
        } else {
            0.0
        };
        let settings_model_picker_bonus = if settings_model_picker_surface_count >= 2 {
            0.02
        } else {
            0.0
        };
        let settings_common_bonus = if settings_common_surface_count >= 2 {
            0.02
        } else {
            0.0
        };
        let settings_section_bonus = if settings_section_surface_count >= 2 {
            0.02
        } else {
            0.0
        };
        let settings_test_bonus = if settings_test_surface_count >= 2 {
            0.02
        } else {
            0.0
        };
        let settings_control_bonus = if settings_control_surface_count >= 2 {
            0.02
        } else {
            0.0
        };
        let tooling_coverage_bonus = if tooling_coverage_surface_count >= 2 {
            0.02
        } else {
            0.0
        };
        let tooling_workflow_bonus = if tooling_workflow_surface_count >= 2 {
            0.02
        } else {
            0.0
        };
        let tooling_clinerules_workflow_bonus = if tooling_clinerules_workflow_surface_count >= 2 {
            0.02
        } else {
            0.0
        };
        let tooling_clinerules_doc_bonus = if tooling_clinerules_doc_surface_count >= 2 {
            0.02
        } else {
            0.0
        };
        let tooling_agent_skill_bonus = if tooling_agent_skill_surface_count >= 2 {
            0.02
        } else {
            0.0
        };
        let tooling_storybook_bonus = if tooling_storybook_surface_count >= 2 {
            0.02
        } else {
            0.0
        };
        let tooling_cli_bonus = if tooling_cli_surface_count >= 2 {
            0.02
        } else {
            0.0
        };
        let tooling_contract_bonus = if tooling_contract_surface_count >= 2 {
            0.02
        } else {
            0.0
        };
        let tooling_rtk_bonus = if tooling_rtk_surface_count >= 2 {
            0.02
        } else {
            0.0
        };
        let tooling_docs_bonus = if tooling_docs_surface_count >= 2 {
            0.02
        } else {
            0.0
        };
        let conflict_penalty = if conflicts.is_empty() {
            0.0
        } else {
            (conflicts.len() as f64 / claims.len().max(1) as f64) * 0.2
        };
        let coverage_score = clamp_unit(
            base_coverage
                + diversity_bonus
                + language_bonus
                + surface_bonus
                + provider_bonus
                + provider_family_bonus
                + shared_contract_bonus
                + shared_cline_bonus
                + shared_message_bonus
                + shared_internal_bonus
                + shared_service_bonus
                + shared_multi_root_bonus
                + chat_hook_bonus
                + chat_util_bonus
                + chat_message_bonus
                + chat_shared_bonus
                + chat_type_bonus
                + chat_view_bonus
                + chat_layout_bonus
                + chat_root_bonus
                + chat_output_bonus
                + chat_error_bonus
                + chat_preview_bonus
                + chat_interaction_bonus
                + chat_component_bonus
                + chat_task_header_bonus
                + chat_task_header_button_bonus
                + chat_auto_approve_bonus
                + cline_rules_bonus
                + common_component_bonus
                + common_content_bonus
                + common_ui_bonus
                + history_bonus
                + menu_bonus
                + onboarding_bonus
                + browser_bonus
                + settings_utils_bonus
                + hooks_bonus
                + root_provider_bonus
                + settings_component_bonus
                + settings_model_picker_bonus
                + settings_common_bonus
                + settings_section_bonus
                + settings_test_bonus
                + settings_control_bonus
                + tooling_coverage_bonus
                + tooling_workflow_bonus
                + tooling_clinerules_workflow_bonus
                + tooling_clinerules_doc_bonus
                + tooling_agent_skill_bonus
                + tooling_storybook_bonus
                + tooling_cli_bonus
                + tooling_contract_bonus
                + tooling_rtk_bonus
                + tooling_docs_bonus
                - conflict_penalty,
        );
        let claim_ids = claims
            .iter()
            .map(|claim| claim.claim_id.clone())
            .collect::<Vec<_>>();
        let claim_bundle_content_id = deterministic_hash(&serde_json::json!({
            "kind":"claim_bundle_content",
            "task_id": task_id,
            "claim_ids": claim_ids
        }));
        let claim_bundle_event_id = deterministic_hash(&serde_json::json!({
            "kind":"claim_bundle_event",
            "task_id": task_id,
            "claim_bundle_content_id": claim_bundle_content_id,
            "evidence_count": evidence_cards.len(),
        }));
        ClaimBundle {
            claim_bundle_id: claim_bundle_content_id.clone(),
            claim_bundle_content_id,
            claim_bundle_event_id,
            task_id: task_id.to_string(),
            claims,
            unresolved_questions,
            conflicts,
            coverage_score,
        }
    }

    pub fn supported_claims_for_synthesis<'a>(&self, bundle: &'a ClaimBundle) -> Vec<&'a Claim> {
        bundle
            .claims
            .iter()
            .filter(|claim| matches!(claim.status, ClaimStatus::Supported | ClaimStatus::Partial))
            .collect::<Vec<_>>()
    }

    pub fn validate_claim_evidence_refs(
        &self,
        bundle: &ClaimBundle,
        evidence_cards: &[EvidenceCard],
    ) -> Result<(), String> {
        let evidence_ids = evidence_cards
            .iter()
            .map(|row| row.evidence_id.as_str())
            .collect::<HashSet<_>>();
        for claim in &bundle.claims {
            if claim.evidence_ids.is_empty() {
                return Err(format!("claim_without_evidence:{}", claim.claim_id));
            }
            for evidence_id in &claim.evidence_ids {
                if !evidence_ids.contains(evidence_id.as_str()) {
                    return Err(format!(
                        "claim_references_unknown_evidence:{}:{}",
                        claim.claim_id, evidence_id
                    ));
                }
            }
        }
        Ok(())
    }
}

fn support_status(confidence: &ConfidenceVector, summary: &str) -> ClaimStatus {
    let avg = (confidence.relevance + confidence.reliability + confidence.freshness) / 3.0;
    if summary.trim().is_empty() {
        return ClaimStatus::Unsupported;
    }
    if avg >= 0.74 {
        ClaimStatus::Supported
    } else if avg >= 0.45 {
        ClaimStatus::Partial
    } else {
        ClaimStatus::Unsupported
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card(id: &str, text: &str, reliability: f64) -> EvidenceCard {
        EvidenceCard {
            evidence_id: id.to_string(),
            evidence_content_id: format!("content-{id}"),
            evidence_event_id: format!("event-{id}"),
            trace_id: "trace-1".to_string(),
            task_id: "task-1".to_string(),
            derived_from_result_id: "r1".to_string(),
            source_ref: "https://example.com".to_string(),
            source_location: "payload".to_string(),
            excerpt: text.to_string(),
            summary: text.to_string(),
            confidence_vector: ConfidenceVector {
                relevance: reliability,
                reliability,
                freshness: reliability,
            },
            dedupe_hash: format!("d-{id}"),
            lineage: vec!["l1".to_string()],
            timestamp: 1,
        }
    }

    #[test]
    fn verifier_labels_supported_and_unsupported_claims() {
        let verifier = StructuredVerifier;
        let bundle = verifier.derive_claim_bundle(
            "task-1",
            &[card("e1", "Result is stable", 0.9), card("e2", "Weak", 0.2)],
        );
        assert_eq!(bundle.claims.len(), 2);
        assert!(bundle
            .claims
            .iter()
            .any(|claim| claim.status == ClaimStatus::Supported));
        assert!(bundle
            .claims
            .iter()
            .any(|claim| claim.status == ClaimStatus::Unsupported));
        verifier
            .validate_claim_evidence_refs(
                &bundle,
                &[card("e1", "Result is stable", 0.9), card("e2", "Weak", 0.2)],
            )
            .expect("claims should always map to evidence");
        let synth = verifier.supported_claims_for_synthesis(&bundle);
        assert!(!synth.is_empty());
    }

    #[test]
    fn verifier_marks_conflicting_claims_by_shared_topic() {
        let verifier = StructuredVerifier;
        let bundle = verifier.derive_claim_bundle(
            "task-2",
            &[
                card("e1", "Tool route is available for workspace search", 0.9),
                card(
                    "e2",
                    "Tool route is not available for workspace search",
                    0.9,
                ),
            ],
        );
        assert!(bundle
            .claims
            .iter()
            .all(|claim| claim.status == ClaimStatus::Conflicting));
        assert!(!bundle.conflicts.is_empty());
    }

    #[test]
    fn verifier_applies_diversity_bonus_for_mixed_sources() {
        let verifier = StructuredVerifier;
        let mixed = [
            EvidenceCard {
                source_ref: "https://example.com".to_string(),
                ..card("e1", "web source claim", 0.9)
            },
            EvidenceCard {
                source_ref: "core/layer2/tooling/src/verifier.rs".to_string(),
                ..card("e2", "workspace source claim", 0.9)
            },
        ];
        let bundle = verifier.derive_claim_bundle("task-3", &mixed);
        assert!(bundle.coverage_score > 0.9);
    }
}
