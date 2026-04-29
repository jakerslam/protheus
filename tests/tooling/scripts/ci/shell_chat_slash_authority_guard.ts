#!/usr/bin/env node
/* eslint-disable no-console */
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

const ROOT = process.cwd();
const DEFAULT_CHAT = 'client/runtime/systems/ui/infring_static/js/pages/chat.ts';
const DEFAULT_OUT_JSON = 'core/local/artifacts/shell_chat_slash_authority_guard_current.json';
const DEFAULT_OUT_MARKDOWN = 'local/workspace/reports/SHELL_CHAT_SLASH_AUTHORITY_GUARD_CURRENT.md';

type Violation = {
  kind: string;
  detail: string;
};

function readText(relPath: string): string {
  return readFileSync(resolve(ROOT, relPath), 'utf8');
}

function markdown(payload: any): string {
  const lines: string[] = [];
  lines.push('# Shell Chat Slash Authority Guard');
  lines.push('');
  lines.push(`Generated: ${payload.generated_at}`);
  lines.push(`Revision: ${payload.revision}`);
  lines.push(`Pass: ${payload.ok}`);
  lines.push('');
  lines.push('## Summary');
  lines.push(`- violations: ${payload.summary.violations}`);
  lines.push(`- workflow_owned_commands_checked: ${payload.summary.workflow_owned_commands_checked}`);
  lines.push('');
  lines.push('## Violations');
  if (!payload.violations.length) lines.push('- none');
  for (const violation of payload.violations) {
    lines.push(`- ${violation.kind}: ${violation.detail}`);
  }
  return `${lines.join('\n')}\n`;
}

function run(argv = process.argv.slice(2)): number {
  const common = parseStrictOutArgs(argv, { strict: true, out: DEFAULT_OUT_JSON });
  const outJson = readFlag(argv, 'out-json') || common.out || DEFAULT_OUT_JSON;
  const outMarkdown = readFlag(argv, 'out-markdown') || DEFAULT_OUT_MARKDOWN;
  const chatPath = readFlag(argv, 'chat') || DEFAULT_CHAT;
  const source = readText(chatPath);
  const violations: Violation[] = [];

  const gateToken = 'if (matched && this.isShellOwnedSlashCommand(matched.cmd)) {';
  if (!source.includes(gateToken)) {
    violations.push({
      kind: 'missing_shell_owned_gate',
      detail: 'chat send path must guard slash pre-execution through isShellOwnedSlashCommand().',
    });
  }

  const helperStart = source.indexOf('isShellOwnedSlashCommand: function(cmd) {');
  const helperEnd = source.indexOf('\n    runSlashMemprobe:', helperStart);
  if (helperStart < 0 || helperEnd <= helperStart) {
    violations.push({
      kind: 'missing_shell_owned_classifier',
      detail: 'chat.ts must expose a bounded shell-owned slash classifier block.',
    });
  }

  const helperBlock = helperStart >= 0 && helperEnd > helperStart
    ? source.slice(helperStart, helperEnd)
    : '';
  const workflowOwnedCommands = [
    '/file',
    '/folder',
    '/memory',
    '/browse',
    '/search',
    '/batch',
    '/capabilities',
    '/cron',
    '/undo',
  ];
  for (const command of workflowOwnedCommands) {
    if (helperBlock.includes(`case '${command}':`)) {
      violations.push({
        kind: 'workflow_command_still_shell_owned',
        detail: `${command} must pass through agent/workflow routing instead of staying shell-owned.`,
      });
    }
  }

  if (source.includes("var result = await InfringAPI.post('/api/route/auto', {")) {
    violations.push({
      kind: 'shell_auto_route_preflight_still_present',
      detail: 'chat.ts must not preflight backend auto-route decisions before sending the message.',
    });
  }

  if (source.includes('inferContextWindowFromModelId(')) {
    violations.push({
      kind: 'shell_context_window_heuristic_still_present',
      detail: 'chat.ts must not infer context windows from model-name heuristics.',
    });
  }

  if (source.includes('contextWindowNeedsFloor(')) {
    violations.push({
      kind: 'shell_context_window_floor_heuristic_still_present',
      detail: 'chat.ts must not keep provider-specific context-window floor heuristics.',
    });
  }

  const restoreStart = source.indexOf('restoreAgentConversation(agentId) {');
  const restoreEnd = source.indexOf('\n\n    loadConversationCache()', restoreStart);
  if (restoreStart < 0 || restoreEnd <= restoreStart) {
    violations.push({
      kind: 'missing_restore_agent_conversation_block',
      detail: 'chat.ts must keep an explicit restoreAgentConversation block so its authority can be audited.',
    });
  } else {
    const restoreBlock = source.slice(restoreStart, restoreEnd);
    if (!restoreBlock.includes("if (!(typeof this.isSystemThreadId === 'function' && this.isSystemThreadId(agentId))) {")) {
      violations.push({
        kind: 'non_system_cache_restore_not_blocked',
        detail: 'restoreAgentConversation must fail closed for non-system threads.',
      });
    }
    if (restoreBlock.includes('this.normalizeSessionMessages({ messages: sanitized })')) {
      violations.push({
        kind: 'cache_restore_still_reconstructs_session_messages',
        detail: 'system-thread cache restore must not rebuild session truth through normalizeSessionMessages.',
      });
    }
  }

  if (source.includes('restoredFromCache = self.restoreAgentConversation(agentId);')) {
    violations.push({
      kind: 'load_session_error_still_uses_cache_truth_fallback',
      detail: 'loadSession error handling must not fall back to cache restoration for normal agent chat.',
    });
  }

  if (source.includes('} else if (data && Array.isArray(data.turns)) {')) {
    violations.push({
      kind: 'session_normalization_still_reconstructs_turn_rows',
      detail: 'normalizeSessionMessages must not reconstruct chat truth from legacy turns arrays inside the Shell.',
    });
  }

  if (source.includes('self.fallbackAssistantTextFromPayload(m, tools)')) {
    violations.push({
      kind: 'session_normalization_still_repairs_assistant_history',
      detail: 'normalizeSessionMessages must not repair assistant history rows from workflow/tool metadata inside the Shell.',
    });
  }

  if (source.includes('this.fallbackAssistantTextFromPayload(duplicate, duplicate.tools || [])')) {
    violations.push({
      kind: 'dedupe_merge_still_repairs_assistant_text',
      detail: 'dedupe merge must preserve authoritative metadata without synthesizing visible assistant text from it inside the Shell.',
    });
  }

  if (source.includes('fallbackAssistantTextFromPayload: function(')) {
    violations.push({
      kind: 'fallback_assistant_text_helper_still_present',
      detail: 'chat.ts must not keep a shell helper for replacing visible assistant text from workflow/tool metadata.',
    });
  }

  if (source.includes('buildPromptSuggestionContextSnapshot(') || source.includes('collectPromptSuggestionContext()')) {
    violations.push({
      kind: 'prompt_suggestion_context_still_shell_derived',
      detail: 'chat.ts must not derive suggestion context from local message history; the backend/session contract owns suggestion context.',
    });
  }

  if (source.includes('payload.recent_context =')) {
    violations.push({
      kind: 'prompt_suggestion_recent_context_still_sent',
      detail: 'suggestion requests must not send shell-derived recent_context payloads.',
    });
  }

  if (source.includes('derivePromptSuggestionFallback(')) {
    violations.push({
      kind: 'prompt_suggestion_fallback_still_shell_authored',
      detail: 'chat.ts must not synthesize fallback prompt suggestions locally when backend suggestions fail.',
    });
  }

  const payload = {
    ok: violations.length === 0,
    type: 'shell_chat_slash_authority_guard',
    generated_at: new Date().toISOString(),
    revision: currentRevision(ROOT),
    inputs: {
      chat_path: chatPath,
    },
    summary: {
      violations: violations.length,
      workflow_owned_commands_checked: workflowOwnedCommands.length,
    },
    violations,
  };

  writeTextArtifact(outMarkdown, markdown(payload));
  return emitStructuredResult(payload, {
    outPath: outJson,
    strict: common.strict,
    ok: payload.ok,
  });
}

process.exit(run(process.argv.slice(2)));
