    // ── Setup Checklist ──
    checklistDismissed: localStorage.getItem('of-checklist-dismissed') === 'true',

    get setupChecklist() {
      return [
        { key: 'provider', label: 'Configure an LLM provider', done: this.configuredProviders.length > 0, action: '#settings' },
        { key: 'agent', label: 'Create your first agent', done: (Alpine.store('app').agents || []).length > 0, action: '#agents' },
        { key: 'chat', label: 'Send your first message', done: localStorage.getItem('of-first-msg') === 'true', action: '#chat' },
        { key: 'channel', label: 'Connect a messaging channel', done: this.channels.length > 0, action: '#channels' },
        { key: 'skill', label: 'Browse or install a skill', done: localStorage.getItem('of-skill-browsed') === 'true', action: '#skills' }
      ];
    },

    get setupProgress() {
      var done = this.setupChecklist.filter(function(item) { return item.done; }).length;
      return (done / 5) * 100;
    },

    get setupDoneCount() {
      return this.setupChecklist.filter(function(item) { return item.done; }).length;
    },

    dismissChecklist() {
      this.checklistDismissed = true;
      localStorage.setItem('of-checklist-dismissed', 'true');
    },

    formatUptime(secs) {
      if (!secs) return '-';
      var d = Math.floor(secs / 86400);
      var h = Math.floor((secs % 86400) / 3600);
      var m = Math.floor((secs % 3600) / 60);
      if (d > 0) return d + 'd ' + h + 'h';
      if (h > 0) return h + 'h ' + m + 'm';
      return m + 'm';
    },

    formatNumber(n) {
      if (!n) return '0';
      if (n >= 1000000) return (n / 1000000).toFixed(1) + 'M';
      if (n >= 1000) return (n / 1000).toFixed(1) + 'K';
      return String(n);
    },

    formatCost(n) {
      if (!n || n === 0) return '$0.00';
      if (n < 0.01) return '<$0.01';
      return '$' + n.toFixed(2);
    },

    // Relative time formatting ("2m ago", "1h ago", "just now")
    timeAgo(timestamp) {
      if (!timestamp) return '';
      var now = Date.now();
      var ts = new Date(timestamp).getTime();
      var diff = Math.floor((now - ts) / 1000);
      if (diff < 10) return 'just now';
      if (diff < 60) return diff + 's ago';
      if (diff < 3600) return Math.floor(diff / 60) + 'm ago';
      if (diff < 86400) return Math.floor(diff / 3600) + 'h ago';
      return Math.floor(diff / 86400) + 'd ago';
    },

    // Map raw audit action names to user-friendly labels
    friendlyAction(action) {
      if (!action) return 'Unknown';
      var map = {
        'AgentSpawn': 'Agent Created',
        'AgentKill': 'Agent Stopped',
        'AgentTerminated': 'Agent Stopped',
        'ToolInvoke': 'Tool Used',
        'ToolResult': 'Tool Completed',
        'MessageReceived': 'Message In',
        'MessageSent': 'Response Sent',
        'SessionReset': 'Session Reset',
        'SessionCompact': 'Compacted',
        'ModelSwitch': 'Model Changed',
        'AuthAttempt': 'Login Attempt',
        'AuthSuccess': 'Login OK',
        'AuthFailure': 'Login Failed',
        'CapabilityDenied': 'Denied',
        'RateLimited': 'Rate Limited',
        'WorkflowRun': 'Workflow Run',
        'TriggerFired': 'Trigger Fired',
        'SkillInstalled': 'Skill Installed',
        'McpConnected': 'MCP Connected'
      };
      return map[action] || action.replace(/([A-Z])/g, ' $1').trim();
    },

    // Audit action icon (small inline SVG)
    actionIcon(action) {
      if (!action) return '';
      var icons = {
        'AgentSpawn': '<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="10"/><path d="M12 8v8M8 12h8"/></svg>',
        'AgentKill': '<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="10"/><path d="M15 9l-6 6M9 9l6 6"/></svg>',
        'AgentTerminated': '<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="10"/><path d="M15 9l-6 6M9 9l6 6"/></svg>',
        'ToolInvoke': '<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M14.7 6.3a1 1 0 0 0 0 1.4l1.6 1.6a1 1 0 0 0 1.4 0l3.77-3.77a6 6 0 0 1-7.94 7.94l-6.91 6.91a2.12 2.12 0 0 1-3-3l6.91-6.91a6 6 0 0 1 7.94-7.94l-3.76 3.76z"/></svg>',
        'MessageReceived': '<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"/></svg>',
        'MessageSent': '<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M22 2L11 13M22 2l-7 20-4-9-9-4 20-7z"/></svg>'
      };
      return icons[action] || '<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="10"/></svg>';
    },

    // Resolve agent UUID to name if possible
    agentName(agentId) {
      if (!agentId) return '-';
      var agents = Alpine.store('app').agents || [];
      var agent = agents.find(function(a) { return a.id === agentId; });
      return agent ? agent.name : agentId.substring(0, 8) + '\u2026';
    }
  };
}
