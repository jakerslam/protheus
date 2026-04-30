// Chat tool status, display-name, and thinking-action label helpers.
'use strict';

function infringChatToolLabelMethods() {
  return {
    toolReceiptDisplayState: function(tool) {
      if (!tool || typeof tool !== 'object') return '';
      return String(
        tool.display_state ||
        tool.receipt_status ||
        tool.tool_receipt_status ||
        tool.status ||
        ''
      ).trim().toLowerCase();
    },

    isBlockedTool: function(tool) {
      if (!tool) return false;
      var state = this.toolReceiptDisplayState(tool);
      return tool.blocked === true || state === 'blocked' || state === 'policy_denied';
    },

    isToolSuccessful: function(tool) {
      if (!tool) return false;
      if (tool.running) return false;
      if (this.isBlockedTool(tool)) return false;
      var state = this.toolReceiptDisplayState(tool);
      return state === 'success' || state === 'ok' || state === 'done' || state === 'ready';
    },

    isThoughtTool: function(tool) {
      return !!(tool && String(tool.name || '').toLowerCase() === 'thought_process');
    },

    toolNameKey: function(tool) {
      if (!tool) return '';
      return String(tool.name || '')
        .trim()
        .toLowerCase()
        .replace(/[\s-]+/g, '_');
    },

    toolInputPayload: function(tool) {
      if (!tool || typeof tool !== 'object') return null;
      var receipt = tool.tool_attempt_receipt && typeof tool.tool_attempt_receipt === 'object'
        ? tool.tool_attempt_receipt
        : null;
      var normalized = receipt && receipt.normalized_result && receipt.normalized_result.normalized_args
        ? receipt.normalized_result.normalized_args
        : null;
      return normalized && typeof normalized === 'object' ? normalized : null;
    },

    toolPayloadCount: function(payload, keys) {
      if (!payload || typeof payload !== 'object') return 0;
      var list = Array.isArray(keys) ? keys : [];
      for (var i = 0; i < list.length; i++) {
        var key = list[i];
        if (!Object.prototype.hasOwnProperty.call(payload, key)) continue;
        var value = payload[key];
        if (Array.isArray(value)) return value.length;
        if (typeof value === 'number' && Number.isFinite(value)) return Math.max(0, Math.round(value));
        if (typeof value === 'string' && value.trim()) return 1;
      }
      return 0;
    },

    prettifyToolLabel: function(value) {
      var raw = String(value || '').trim();
      if (!raw) return 'tool';
      var normalized = raw
        .replace(/[_-]+/g, ' ')
        .replace(/\s+/g, ' ')
        .trim();
      if (!normalized) return 'tool';
      return normalized
        .split(' ')
        .map(function(token) {
          return token ? token.charAt(0).toUpperCase() + token.slice(1) : token;
        })
        .join(' ');
    },

    toolActionName: function(tool) {
      var payload = this.toolInputPayload(tool);
      if (!payload || typeof payload !== 'object') return '';
      return String(
        payload.action ||
        payload.method ||
        payload.operation ||
        payload.op ||
        ''
      ).trim();
    },

    toolDisplayName: function(tool) {
      if (!tool) return 'tool';
      if (this.isThoughtTool(tool)) return 'thought';
      var key = this.toolNameKey(tool);
      var actionName = this.toolActionName(tool);
      switch (key) {
        case 'web_search':
        case 'search_web':
        case 'search':
        case 'web_query':
        case 'batch_query':
          return 'Web search';
        case 'web_fetch':
        case 'browse':
        case 'web_conduit_fetch':
          return 'Web fetch';
        case 'file_read':
        case 'read_file':
        case 'file':
          return 'File read';
        case 'folder_export':
        case 'list_folder':
        case 'folder_tree':
        case 'folder':
          return 'Folder export';
        case 'terminal_exec':
        case 'run_terminal':
        case 'terminal':
        case 'shell_exec':
          return 'Terminal command';
        case 'spawn_subagents':
        case 'spawn_swarm':
        case 'agent_spawn':
        case 'sessions_spawn':
          return 'Swarm spawn';
        case 'memory_semantic_query':
          return 'Memory query';
        case 'cron_schedule':
          return 'Schedule task';
        case 'cron_run':
          return 'Run scheduled task';
        case 'cron_list':
          return 'List schedules';
        case 'session_rollback_last_turn':
          return 'Undo last turn';
        case 'slack':
          return actionName ? ('Slack ' + this.prettifyToolLabel(actionName)) : 'Slack';
        case 'gmail':
          return actionName ? ('Gmail ' + this.prettifyToolLabel(actionName)) : 'Gmail';
        case 'github':
          return actionName ? ('GitHub ' + this.prettifyToolLabel(actionName)) : 'GitHub';
        case 'notion':
          return actionName ? ('Notion ' + this.prettifyToolLabel(actionName)) : 'Notion';
        default:
          return this.prettifyToolLabel(String(tool.name || 'tool'));
      }
    },

    toolThinkingActionLabel: function(tool) {
      if (!tool) return '';
      if (this.isThoughtTool(tool)) return 'Thinking';
      var key = this.toolNameKey(tool);
      var payload = this.toolInputPayload(tool);
      switch (key) {
        case 'web_search':
        case 'search_web':
        case 'search':
        case 'web_query':
          return 'Searching internet';
        case 'web_fetch':
        case 'browse':
        case 'web_conduit_fetch':
          return 'Reading web pages';
        case 'file_read':
        case 'read_file':
        case 'file':
          var fileCount = this.toolPayloadCount(payload, ['paths', 'files', 'file_paths', 'targets', 'path', 'file']);
          if (fileCount > 1) return 'Scanning ' + fileCount + ' files';
          if (fileCount === 1) return 'Scanning 1 file';
          return 'Scanning files';
        case 'folder_export':
        case 'list_folder':
        case 'folder_tree':
        case 'folder':
          var folderCount = this.toolPayloadCount(payload, ['folders', 'paths', 'targets', 'path', 'folder']);
          if (folderCount > 1) return 'Scanning ' + folderCount + ' folders';
          if (folderCount === 1) return 'Scanning 1 folder';
          return 'Scanning folders';
        case 'terminal_exec':
        case 'run_terminal':
        case 'terminal':
        case 'shell_exec':
          return 'Running terminal command';
        case 'spawn_subagents':
        case 'spawn_swarm':
        case 'agent_spawn':
        case 'sessions_spawn':
          var spawnCount = this.toolPayloadCount(payload, ['count', 'agent_count', 'num_agents', 'agents']);
          if (spawnCount > 0) return 'Summoning ' + spawnCount + ' agents';
          return 'Summoning agents';
        case 'memory_semantic_query':
          return 'Searching memory';
        case 'cron_schedule':
          return 'Scheduling follow-up work';
        case 'cron_run':
          return 'Running scheduled work';
        case 'cron_list':
          return 'Checking schedules';
        case 'session_rollback_last_turn':
          return 'Rewinding the last turn';
        default:
          return 'Running ' + this.toolDisplayName(tool);
      }
    },
  };
}
