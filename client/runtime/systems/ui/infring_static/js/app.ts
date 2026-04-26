// Shared rendering helpers split out to keep dashboard part files under size caps.

// Render LaTeX math in the chat message container using KaTeX auto-render.
// Call this after new messages are inserted into the DOM.
function renderLatex(el) {
  if (typeof renderMathInElement !== 'function') return;
  var target = el || document.getElementById('messages');
  if (!target) return;
  try {
    renderMathInElement(target, {
      delimiters: [
        { left: '$$', right: '$$', display: true },
        { left: '\\[', right: '\\]', display: true },
        { left: '$', right: '$', display: false },
        { left: '\\(', right: '\\)', display: false }
      ],
      throwOnError: false,
      trust: false
    });
  } catch (e) { /* KaTeX render error — ignore gracefully */ }
}

function cloneDashboardConfigObject(value) {
  if (typeof structuredClone === 'function') return structuredClone(value);
  return JSON.parse(JSON.stringify(value));
}

function normalizeDashboardOptionalString(value) {
  var text = String(value == null ? '' : value).trim();
  return text || '';
}

var DASHBOARD_FORBIDDEN_CONFIG_KEYS = { '__proto__': true, 'prototype': true, 'constructor': true };

function resolveDashboardConfigPathContainer(obj, path, createMissing) {
  if (!obj || !Array.isArray(path) || !path.length) return null;
  var current = obj;
  for (var i = 0; i < path.length - 1; i += 1) {
    var key = path[i];
    if (typeof key === 'string' && DASHBOARD_FORBIDDEN_CONFIG_KEYS[key]) return null;
    var nextKey = path[i + 1];
    if (typeof key === 'number') {
      if (!Array.isArray(current)) return null;
      if (current[key] == null) {
        if (!createMissing) return null;
        current[key] = typeof nextKey === 'number' ? [] : {};
      }
      current = current[key];
      continue;
    }
    if (!current || typeof current !== 'object') return null;
    if (current[key] == null) {
      if (!createMissing) return null;
      current[key] = typeof nextKey === 'number' ? [] : {};
    }
    current = current[key];
  }
  return { current: current, lastKey: path[path.length - 1] };
}

function setDashboardConfigPathValue(obj, path, value) {
  var container = resolveDashboardConfigPathContainer(obj, path, true);
  if (!container) return;
  if (typeof container.lastKey === 'number') {
    if (Array.isArray(container.current)) container.current[container.lastKey] = value;
    return;
  }
  if (typeof container.lastKey === 'string' && DASHBOARD_FORBIDDEN_CONFIG_KEYS[container.lastKey]) return;
  if (container.current && typeof container.current === 'object') container.current[container.lastKey] = value;
}

function removeDashboardConfigPathValue(obj, path) {
  var container = resolveDashboardConfigPathContainer(obj, path, false);
  if (!container) return;
  if (typeof container.lastKey === 'number') {
    if (Array.isArray(container.current)) container.current.splice(container.lastKey, 1);
    return;
  }
  if (typeof container.lastKey === 'string' && DASHBOARD_FORBIDDEN_CONFIG_KEYS[container.lastKey]) return;
  if (container.current && typeof container.current === 'object') delete container.current[container.lastKey];
}

function normalizeDashboardAgentLabel(agent, agentIdentity) {
  var identityName = normalizeDashboardOptionalString(agentIdentity && agentIdentity.name);
  if (identityName) return identityName;
  var agentName = normalizeDashboardOptionalString(agent && agent.name);
  if (agentName) return agentName;
  var nestedName = normalizeDashboardOptionalString(agent && agent.identity && agent.identity.name);
  if (nestedName) return nestedName;
  return normalizeDashboardOptionalString(agent && agent.id) || 'agent';
}

function resolveDashboardAgentAvatar(agent, agentIdentity) {
  var values = [
    normalizeDashboardOptionalString(agentIdentity && agentIdentity.avatar),
    normalizeDashboardOptionalString(agent && agent.identity && agent.identity.avatar_url),
    normalizeDashboardOptionalString(agent && agent.identity && agent.identity.avatar),
    normalizeDashboardOptionalString(agent && agent.avatar_url)
  ];
  for (var i = 0; i < values.length; i += 1) {
    if (/^(https?:\/\/|data:image\/|\/)/i.test(values[i])) return values[i];
  }
  return '';
}

function resolveDashboardAgentEmoji(agent, agentIdentity) {
  var values = [
    normalizeDashboardOptionalString(agentIdentity && agentIdentity.emoji),
    normalizeDashboardOptionalString(agent && agent.identity && agent.identity.emoji),
    normalizeDashboardOptionalString(agent && agent.identity && agent.identity.avatar),
    normalizeDashboardOptionalString(agent && agent.avatar)
  ];
  for (var i = 0; i < values.length; i += 1) {
    var value = values[i];
    if (!value || value.length > 16) continue;
    if (/[A-Za-z0-9]/.test(value) && value.charCodeAt(0) < 128) continue;
    if (value.indexOf('://') >= 0 || value.indexOf('/') >= 0 || value.indexOf('.') >= 0) continue;
    return value;
  }
  return '';
}

function copyCode(btn) {
  var code = null;
  if (btn && typeof btn.closest === 'function') {
    var block = btn.closest('.chat-codeblock');
    if (block && typeof block.querySelector === 'function') {
      code = block.querySelector('pre > code');
    }
  }
  if (!code && btn && btn.parentElement && btn.parentElement.tagName === 'PRE') {
    code = btn.parentElement.querySelector('code');
  }
  if (!code && btn) {
    var next = btn.nextElementSibling;
    if (next && next.tagName === 'CODE') code = next;
  }
  if (!code) return;
  var setCopyState = function(copied) {
    if (!btn) return;
    var copyIcon = btn.querySelector('.copy-icon');
    var copiedIcon = btn.querySelector('.copied-icon');
    if (copyIcon && copiedIcon) {
      copyIcon.style.display = copied ? 'none' : '';
      copiedIcon.style.display = copied ? '' : 'none';
    }
    btn.classList.toggle('copied', !!copied);
    btn.setAttribute('title', copied ? 'Copied' : 'Copy code');
    btn.setAttribute('aria-label', copied ? 'Copied' : 'Copy code');
  };
  navigator.clipboard.writeText(code.textContent).then(function() {
    if (btn._copyResetTimer) clearTimeout(btn._copyResetTimer);
    setCopyState(true);
    btn._copyResetTimer = setTimeout(function() {
      setCopyState(false);
      btn._copyResetTimer = null;
    }, 1500);
  });
}

function dashboardExtractCodeLanguage(codeAttrs) {
  var attrs = String(codeAttrs || '');
  if (!attrs) return '';
  var classMatch = attrs.match(/\bclass\s*=\s*"([^"]*)"/i) || attrs.match(/\bclass\s*=\s*'([^']*)'/i);
  var classText = classMatch ? String(classMatch[1] || '') : '';
  if (classText) {
    var classes = classText.split(/\s+/).filter(Boolean);
    for (var i = 0; i < classes.length; i += 1) {
      var cls = String(classes[i] || '').trim().toLowerCase();
      if (!cls) continue;
      if (cls.indexOf('language-') === 0) return cls.slice('language-'.length);
      if (cls.indexOf('lang-') === 0) return cls.slice('lang-'.length);
    }
  }
  var directMatch = attrs.match(/\blanguage-([a-z0-9_+-]+)/i) || attrs.match(/\blang-([a-z0-9_+-]+)/i);
  return directMatch ? String(directMatch[1] || '').trim().toLowerCase() : '';
}

function dashboardFormatCodeLanguageLabel(languageKey) {
  var key = String(languageKey || '').trim().toLowerCase();
  if (!key) return 'Code';
  var labels = {
    js: 'JavaScript',
    jsx: 'JSX',
    ts: 'TypeScript',
    tsx: 'TSX',
    py: 'Python',
    rs: 'Rust',
    sh: 'Shell',
    bash: 'Bash',
    zsh: 'Zsh',
    ps1: 'PowerShell',
    yml: 'YAML',
    md: 'Markdown',
    html: 'HTML',
    css: 'CSS',
    json: 'JSON',
    toml: 'TOML',
    sql: 'SQL',
    xml: 'XML'
  };
  return labels[key] || (key.charAt(0).toUpperCase() + key.slice(1));
}

function dashboardEscapeInlineHtmlText(value) {
  var raw = String(value == null ? '' : value);
  if (typeof escapeHtml === 'function') return escapeHtml(raw);
  return raw
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;');
}

function dashboardWrapMarkdownCodeBlocks(html) {
  var input = String(html || '');
  if (!input) return '';
  return input.replace(/<pre><code([^>]*)>([\s\S]*?)<\/code><\/pre>/g, function(_, attrs, body) {
    var codeAttrs = attrs || '';
    var languageKey = dashboardExtractCodeLanguage(codeAttrs);
    var languageLabel = dashboardFormatCodeLanguageLabel(languageKey);
    var bodyText = String(body || '');
    var lineCount = bodyText ? bodyText.split('\n').length : 0;
    var collapsible = lineCount > 30;
    var collapsedClass = collapsible ? ' chat-codeblock-collapsible is-collapsed' : '';
    var toggleBtn = collapsible
      ? (
        '<button class="message-stat-btn chat-codeblock-toggle" type="button" onclick="toggleCodeFold(this)" title="Expand code" aria-label="Expand code" aria-expanded="false">' +
          'Expand' +
        '</button>'
      )
      : '';
    return (
      '<div class="chat-codeblock' + collapsedClass + '" data-code-language="' + dashboardEscapeInlineHtmlText(languageKey || 'code') + '">' +
        '<div class="chat-codeblock-toolbar">' +
          '<span class="chat-codeblock-language">' + dashboardEscapeInlineHtmlText(languageLabel) + '</span>' +
          '<span class="chat-codeblock-toolbar-actions">' +
            toggleBtn +
            '<button class="message-stat-btn chat-codeblock-copy" type="button" onclick="copyCode(this)" title="Copy code" aria-label="Copy code">' +
              '<svg class="copy-icon" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>' +
              '<svg class="copied-icon" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" style="display:none"><path d="M20 6L9 17l-5-5"></path></svg>' +
            '</button>' +
          '</span>' +
        '</div>' +
        '<pre><code' + codeAttrs + '>' + body + '</code></pre>' +
      '</div>'
    );
  });
}

function dashboardWrapMarkdownTables(html) {
  var input = String(html || '');
  if (!input) return '';
  return input.replace(/<table>([\s\S]*?)<\/table>/g, function(_, inner) {
    return '<div class="chat-table-wrap"><table>' + String(inner || '') + '</table></div>';
  });
}

function toggleCodeFold(btn) {
  if (!btn || typeof btn.closest !== 'function') return;
  var block = btn.closest('.chat-codeblock');
  if (!block) return;
  var nextCollapsed = !block.classList.contains('is-collapsed');
  block.classList.toggle('is-collapsed', nextCollapsed);
  btn.textContent = nextCollapsed ? 'Expand' : 'Collapse';
  btn.setAttribute('aria-expanded', nextCollapsed ? 'false' : 'true');
  btn.setAttribute('title', nextCollapsed ? 'Expand code' : 'Collapse code');
  btn.setAttribute('aria-label', nextCollapsed ? 'Expand code' : 'Collapse code');
}

// Tool category icon SVGs — returns inline SVG for each tool category.
function toolIcon(toolName) {
  if (!toolName) return '';
  var n = toolName.toLowerCase();
  var s = 'width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"';
  if (n.indexOf('file_') === 0 || n.indexOf('directory_') === 0) {
    return '<svg ' + s + '><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><path d="M14 2v6h6"/><path d="M16 13H8"/><path d="M16 17H8"/></svg>';
  }
  if (n.indexOf('web_') === 0 || n.indexOf('link_') === 0) {
    return '<svg ' + s + '><circle cx="12" cy="12" r="10"/><path d="M2 12h20"/><path d="M12 2a15 15 0 0 1 4 10 15 15 0 0 1-4 10 15 15 0 0 1-4-10 15 15 0 0 1 4-10z"/></svg>';
  }
  if (n.indexOf('shell') === 0 || n.indexOf('exec_') === 0) {
    return '<svg ' + s + '><polyline points="4 17 10 11 4 5"/><line x1="12" y1="19" x2="20" y2="19"/></svg>';
  }
  if (n.indexOf('agent_') === 0) {
    return '<svg ' + s + '><path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2"/><circle cx="9" cy="7" r="4"/><path d="M23 21v-2a4 4 0 0 0-3-3.87"/><path d="M16 3.13a4 4 0 0 1 0 7.75"/></svg>';
  }
  if (n.indexOf('memory_') === 0 || n.indexOf('knowledge_') === 0) {
    return '<svg ' + s + '><path d="M2 3h6a4 4 0 0 1 4 4v14a3 3 0 0 0-3-3H2z"/><path d="M22 3h-6a4 4 0 0 0-4 4v14a3 3 0 0 1 3-3h7z"/></svg>';
  }
  if (n.indexOf('cron_') === 0 || n.indexOf('schedule_') === 0) {
    return '<svg ' + s + '><circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12 16 14"/></svg>';
  }
  if (n.indexOf('browser_') === 0 || n.indexOf('playwright_') === 0) {
    return '<svg ' + s + '><rect x="2" y="3" width="20" height="14" rx="2"/><path d="M8 21h8"/><path d="M12 17v4"/></svg>';
  }
  if (n.indexOf('container_') === 0 || n.indexOf('docker_') === 0) {
    return '<svg ' + s + '><path d="M22 12H2"/><path d="M5.45 5.11L2 12v6a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2v-6l-3.45-6.89A2 2 0 0 0 16.76 4H7.24a2 2 0 0 0-1.79 1.11z"/></svg>';
  }
  if (n.indexOf('image_') === 0 || n.indexOf('tts_') === 0) {
    return '<svg ' + s + '><rect x="3" y="3" width="18" height="18" rx="2"/><circle cx="8.5" cy="8.5" r="1.5"/><polyline points="21 15 16 10 5 21"/></svg>';
  }
  if (n.indexOf('hand_') === 0) {
    return '<svg ' + s + '><path d="M18 11V6a2 2 0 0 0-2-2 2 2 0 0 0-2 2"/><path d="M14 10V4a2 2 0 0 0-2-2 2 2 0 0 0-2 2v6"/><path d="M10 10.5V6a2 2 0 0 0-2-2 2 2 0 0 0-2 2v8"/><path d="M18 8a2 2 0 1 1 4 0v6a8 8 0 0 1-8 8h-2c-2.8 0-4.5-.9-5.7-2.4L3.4 16a2 2 0 0 1 3.2-2.4L8 15"/></svg>';
  }
  if (n.indexOf('task_') === 0) {
    return '<svg ' + s + '><path d="M9 11l3 3L22 4"/><path d="M21 12v7a2 2 0 01-2 2H5a2 2 0 01-2-2V5a2 2 0 012-2h11"/></svg>';
  }
  return '<svg ' + s + '><path d="M14.7 6.3a1 1 0 0 0 0 1.4l1.6 1.6a1 1 0 0 0 1.4 0l3.77-3.77a6 6 0 0 1-7.94 7.94l-6.91 6.91a2.12 2.12 0 0 1-3-3l6.91-6.91a6 6 0 0 1 7.94-7.94l-3.76 3.76z"/></svg>';
}

// Infring App — Alpine.js init, hash router, global store
'use strict';

// Marked.js configuration
if (typeof marked !== 'undefined') {
  marked.setOptions({
    breaks: true,
    gfm: true,
    highlight: function(code, lang) {
      if (typeof hljs !== 'undefined' && lang && hljs.getLanguage(lang)) {
        try { return hljs.highlight(code, { language: lang }).value; } catch(e) {}
      }
      return code;
    }
  });
}

function escapeHtml(text) {
  var div = document.createElement('div');
  div.textContent = text || '';
  return div.innerHTML;
}

function normalizeChatMarkdownListBreaks(text) {
  var source = String(text || '');
  if (!source) return '';
  var normalized = source.replace(/\r\n/g, '\n');
  normalized = normalized.replace(/[ \t]+•[ \t]+/g, '\n- ');
  normalized = normalized.replace(/(:\s+)(\*\*[^*\n]{1,120}\*\*:)/g, function(_match, prefix, marker) {
    return prefix.replace(/\s+$/, '') + '\n' + marker;
  });
  normalized = normalized.replace(/([.!?])\s+(\*\*[^*\n]{1,120}\*\*:)/g, function(_match, punctuation, marker) {
    return punctuation + '\n' + marker;
  });
  normalized = normalized.replace(/(:\n)(\*\*[^*\n]{1,120}\*\*:)/, '$1- $2');
  var out = '';
  var i = 0;
  var inCodeFence = false;
  var atLineStart = true;
  var lineHasContent = false;
  var isLikelyListMarkerAt = function(value, index) {
    var tail = String(value || '').slice(index);
    var match = tail.match(/^(\*\*\d{1,3}[.)]\s+[^*\n]+\*\*|\d{1,3}[.)]\s+|[*+-]\s+)/);
    if (!match) return null;
    var marker = String(match[1] || '');
    if (/^[*+-]\s+$/.test(marker)) {
      var nextChar = tail.charAt(marker.length);
      if (!nextChar || /\s/.test(nextChar)) return null;
    }
    if (/^\d/.test(marker)) {
      var prev = index > 0 ? value.charAt(index - 1) : '';
      if (/\d/.test(prev)) return null;
    }
    return marker;
  };
  while (i < normalized.length) {
    if ((i === 0 || normalized.charAt(i - 1) === '\n') && normalized.slice(i, i + 3) === '```') {
      inCodeFence = !inCodeFence;
    }
    if (!inCodeFence && !atLineStart) {
      var marker = isLikelyListMarkerAt(normalized, i);
      if (marker && lineHasContent) {
        var prevChar = out.length ? out.charAt(out.length - 1) : '';
        if (prevChar !== '\n') out += '\n';
        atLineStart = true;
        lineHasContent = false;
      }
    }
    var ch = normalized.charAt(i);
    out += ch;
    if (ch === '\n') {
      atLineStart = true;
      lineHasContent = false;
    } else {
      if (!/\s/.test(ch)) lineHasContent = true;
      atLineStart = false;
    }
    i += 1;
  }
  return out.replace(/\n{3,}/g, '\n\n');
}

function renderMarkdown(text) {
  if (!text) return '';
  if (typeof marked !== 'undefined') {
    // Protect LaTeX blocks from marked.js mangling (underscores, backslashes, etc.)
    var latexBlocks = [];
    var protected_ = normalizeChatMarkdownListBreaks(text);
    // Protect display math $$...$$ first (greedy across lines)
    protected_ = protected_.replace(/\$\$([\s\S]+?)\$\$/g, function(match) {
      var idx = latexBlocks.length;
      latexBlocks.push(match);
      return '\x00LATEX' + idx + '\x00';
    });
    // Protect inline math $...$ (single line, not empty, not starting/ending with space)
    protected_ = protected_.replace(/\$([^\s$](?:[^$]*[^\s$])?)\$/g, function(match) {
      var idx = latexBlocks.length;
      latexBlocks.push(match);
      return '\x00LATEX' + idx + '\x00';
    });
    // Protect \[...\] display math
    protected_ = protected_.replace(/\\\[([\s\S]+?)\\\]/g, function(match) {
      var idx = latexBlocks.length;
      latexBlocks.push(match);
      return '\x00LATEX' + idx + '\x00';
    });
    // Protect \(...\) inline math
    protected_ = protected_.replace(/\\\(([\s\S]+?)\\\)/g, function(match) {
      var idx = latexBlocks.length;
      latexBlocks.push(match);
      return '\x00LATEX' + idx + '\x00';
    });

    var html = marked.parse(protected_);
    // Restore LaTeX blocks
    for (var i = 0; i < latexBlocks.length; i++) {
      html = html.replace('\x00LATEX' + i + '\x00', latexBlocks[i]);
    }
    // Upgrade markdown render cards for richer code/table ergonomics.
    if (typeof dashboardWrapMarkdownCodeBlocks === 'function') {
      html = dashboardWrapMarkdownCodeBlocks(html);
    }
    if (typeof dashboardWrapMarkdownTables === 'function') {
      html = dashboardWrapMarkdownTables(html);
    }
    // Open external links in new tab
    html = html.replace(/<a\s+href="(https?:\/\/[^"]*)"(?![^>]*target=)([^>]*)>/gi, '<a href="$1" target="_blank" rel="noopener"$2>');
    return html;
  }
  return escapeHtml(text);
}

// Alpine.js global store
document.addEventListener('alpine:init', function() {
  // Restore saved API key on load
  var savedKey = localStorage.getItem('infring-api-key');
  if (savedKey) InfringAPI.setAuthToken(savedKey);

  Alpine.store('app', {
    agents: [],
    connected: false,
    booting: true,
    agentsLoading: true,
    agentsHydrated: false,
    wsConnected: false,
    connectionState: 'connecting',
    statusFailureStreak: 0,
    lastError: '',
    bootStage: 'starting',
    statusDegraded: false,
    lastStatusLatencyMs: 0,
    lastStatusAt: '',
    version: (window.__INFRING_APP_VERSION || '0.0.0'),
    serverVersion: '',
    gitBranch: '',
    assistantName: 'Assistant',
    assistantAvatar: null,
    assistantAgentId: null,
    agentCount: 0,
    localMediaPreviewRoots: [],
    embedSandboxMode: 'scripts',
    allowExternalEmbedUrls: false,
    pendingAgent: null,
    pendingFreshAgentId: null,
    activeAgentId: (() => {
      try {
        var saved = localStorage.getItem('infring-last-active-agent-id');
        return saved ? String(saved) : null;
      } catch(_) {
        return null;
      }
    })(),
    focusMode: localStorage.getItem('infring-focus') === 'true',
    showOnboarding: false,
    showAuthPrompt: false,
    authMode: 'apikey',
    sessionUser: null,
    notifications: [],
    notificationsOpen: false,
    unreadNotifications: 0,
    notificationBubble: null,
    notificationBellPulse: false,
    _notificationBellPulseTimer: null,
    _notificationBellPulseSeq: 0,
    _notificationBubbleTimer: null,
    _notificationSeq: 0,
    taskbarRefreshTurns: 0,
    taskbarSearchOpen: false,
    taskbarSearchQuery: '',
    _taskbarSearchFocusTimer: 0,
    agentChatPreviews: {},
    agentLiveActivity: {},
    agentsEmptyResponseStreak: 0,
    agentsLastNonEmptyAt: 0,
    agentsFetchAttempts: 0,
    agentsLastError: '',
    agentTransientHoldMs: 20000,
    _refreshAgentsInFlight: null,
    _lastAgentsRefreshAt: 0,
    runtimeSync: null,
    lastErrorCode: '',
    _sessionActivityByAgent: {},
    _sessionActivityBootstrapped: false,
    _lastSessionActivityPollAt: 0,

    toggleFocusMode() {
      this.focusMode = !this.focusMode;
      localStorage.setItem('infring-focus', this.focusMode);
    },

    bumpTaskbarRefreshTurn() {
      var current = Number(this.taskbarRefreshTurns || 0);
      if (!Number.isFinite(current) || current < 0) current = 0;
      this.taskbarRefreshTurns = (current + 1) % 4096;
    },

    setActiveAgentId(agentId) {
      this.activeAgentId = agentId ? String(agentId) : null;
      if (this.activeAgentId && this.agentChatPreviews && this.agentChatPreviews[this.activeAgentId]) {
        this.agentChatPreviews[this.activeAgentId].unread_response = false;
      }
      try {
        if (this.activeAgentId) localStorage.setItem('infring-last-active-agent-id', this.activeAgentId);
        else localStorage.removeItem('infring-last-active-agent-id');
      } catch(_) {}
    },

    isArchivedLikeAgent(agent) {
      if (!agent || typeof agent !== 'object') return false;
      var truthy = function(value) {
        if (value === true || value === 1) return true;
        var text = String(value || '').trim().toLowerCase();
        return text === 'true' || text === '1' || text === 'yes';
      };
      if (truthy(agent.archived) || truthy(agent.sidebar_archived)) return true;
      if (truthy(agent.contract_terminated) || truthy(agent.revive_recommended)) return true;
      if (truthy(agent.is_terminated) || truthy(agent.terminated) || truthy(agent.is_archived) || truthy(agent.inactive)) return true;
      var hardInactivePattern = /\b(archived|inactive|terminated|termed|contract[_\s-]*terminated|expired|revoked|timed[_\s-]*out|timeout|stopped|killed|dead)\b/;
      var lifecycleText = [
        agent.status,
        agent.state,
        agent.lifecycle_state,
        agent.agent_state,
        agent.runtime_state
      ]
        .map(function(value) { return String(value || '').trim().toLowerCase(); })
        .filter(Boolean)
        .join(' ');
      var hasLiveActiveSignal = /\b(active|running|ready|connected)\b/.test(lifecycleText);
      var hasLiveInactiveSignal = hardInactivePattern.test(lifecycleText);
      if (hasLiveInactiveSignal && !hasLiveActiveSignal) return true;
      var reasonText = [
        agent.termination_reason,
        agent.archive_reason,
        agent.inactive_reason
      ]
        .map(function(value) { return String(value || '').trim().toLowerCase(); })
        .filter(Boolean)
        .join(' ');
      if (hardInactivePattern.test(reasonText)) return true;
      var contract = agent.contract && typeof agent.contract === 'object' ? agent.contract : null;
      var contractStatus = String(contract && (contract.status || contract.state) ? (contract.status || contract.state) : '').trim().toLowerCase();
      if (hardInactivePattern.test(contractStatus)) return true;
      var contractRemaining = Number(
        (contract && (contract.remaining_ms != null ? contract.remaining_ms : contract.contract_remaining_ms)) != null
          ? (contract.remaining_ms != null ? contract.remaining_ms : contract.contract_remaining_ms)
          : (agent.contract_remaining_ms != null ? agent.contract_remaining_ms : NaN)
      );
      var contractFiniteExpiry = (contract && contract.finite_expiry != null)
        ? truthy(contract.finite_expiry)
        : truthy(agent.contract_finite_expiry);
      if (contractFiniteExpiry && Number.isFinite(contractRemaining) && contractRemaining <= 0) return true;
      return false;
    },

    markAgentPreviewUnread(agentId, unread) {
      var id = String(agentId || '').trim();
      if (!id) return;
      if (!this.agentChatPreviews) this.agentChatPreviews = {};
      if (!this.agentChatPreviews[id]) this.agentChatPreviews[id] = { text: '', ts: Date.now(), role: 'agent' };
      this.agentChatPreviews[id].unread_response = unread !== false;
    },

    async refreshAgents(opts) {
      // Alpine can invoke store methods through different call paths; guard against lost `this`.
      var store = (this && typeof this === 'object' && Object.prototype.hasOwnProperty.call(this, 'agentsHydrated'))
        ? this
        : Alpine.store('app');
      if (!store) return;
      var options = opts || {};
      var force = options.force === true;
      var now = Date.now();
      if (!force && store._lastAgentsRefreshAt && (now - store._lastAgentsRefreshAt) < 1200) {
        return;
      }
      if (store._refreshAgentsInFlight) {
        return store._refreshAgentsInFlight;
      }
      store._refreshAgentsInFlight = (async () => {
        if (!store.agentsHydrated) store.agentsLoading = true;
        store.agentsFetchAttempts = Number(store.agentsFetchAttempts || 0) + 1;
        var agents = null;
        var fetchError = '';
        try {
          agents = await InfringAPI.get('/api/agents?view=sidebar&authority=runtime');
        } catch(e) {
          fetchError = (e && e.message) ? String(e.message) : 'agent_fetch_failed';
          try {
            await new Promise(function(resolve) { setTimeout(resolve, 250); });
            agents = await InfringAPI.get('/api/agents?view=sidebar&authority=runtime');
          } catch(_) {
            agents = null;
          }
        }
        if (Array.isArray(agents)) {
          var priorAgents = Array.isArray(store.agents) ? store.agents.slice() : [];
          var hadPriorAgents = priorAgents.length > 0;
          var holdMs = Number(store.agentTransientHoldMs || 0);
          var statusAgentCountHint = Number(store.agentCount || 0);
          if (!Number.isFinite(statusAgentCountHint) || statusAgentCountHint < 0) {
            statusAgentCountHint = 0;
          }
          var connectionState = String(store.connectionState || '').toLowerCase();
          if (agents.length === 0 && hadPriorAgents && store.connectionState !== 'disconnected') {
            // Strict runtime authority can momentarily return an empty roster when
            // collab dashboard polling times out. Preserve known-good rows while
            // status still reports active agents so the sidebar/chat selection
            // does not flap to zero.
            if (statusAgentCountHint > 0 || connectionState === 'connecting' || connectionState === 'reconnecting') {
              store.agentsHydrated = true;
              store.agentsLoading = false;
              store.agentsLastError = fetchError || 'strict_roster_transient_empty';
              store.agentCount = Math.max(priorAgents.length, statusAgentCountHint);
              return;
            }
            store.agentsEmptyResponseStreak = Number(store.agentsEmptyResponseStreak || 0) + 1;
            var lastNonEmptyAt = Number(store.agentsLastNonEmptyAt || 0);
            var withinHoldWindow = lastNonEmptyAt > 0 && (Date.now() - lastNonEmptyAt) < holdMs;
            // Buffer transient empty responses so chat selection doesn't flap/reset.
            if (withinHoldWindow || store.agentsEmptyResponseStreak < 3) {
              store.agentsHydrated = true;
              store.agentsLoading = false;
              store.agentCount = priorAgents.length;
              return;
            }
          } else if (agents.length > 0) {
            store.agentsEmptyResponseStreak = 0;
            store.agentsLastNonEmptyAt = Date.now();
          } else {
            store.agentsEmptyResponseStreak = 0;
          }

          // First-load protection: do not finalize empty roster until repeated confirms.
          if (agents.length === 0 && !store.agentsHydrated) {
            var attempts = Number(store.agentsFetchAttempts || 0);
            if (statusAgentCountHint > 0) {
              store.agentsLoading = true;
              store.agentCount = statusAgentCountHint;
              store.agentsLastError = fetchError || 'strict_roster_waiting_for_directory';
              return;
            }
            if (connectionState !== 'connected' || attempts < 3) {
              store.agentsLoading = true;
              store.agentCount = 0;
              return;
            }
          }

          var isSidebarArchivedRow = function(row) {
            if (!row || typeof row !== 'object') return false;
            return typeof store.isArchivedLikeAgent === 'function' ? store.isArchivedLikeAgent(row) : false;
          };
          var nextAgents = (Array.isArray(agents) ? agents : []).filter(function(row) {
            if (!row || !row.id) return false;
            return !isSidebarArchivedRow(row);
          });
          store.agents = nextAgents;
          store.agentsHydrated = true;
          store.agentsLoading = false;
          store.agentsLastError = '';
          var keep = {};
          for (var ai = 0; ai < nextAgents.length; ai++) {
            var row = nextAgents[ai];
            if (row && row.id) keep[String(row.id)] = true;
          }
          var nextActivity = {};
          var now = Date.now();
          var srcActivity = store.agentLiveActivity || {};
          keep.system = true;
          Object.keys(srcActivity).forEach(function(id) {
            var entry = srcActivity[id];
            if (!keep[id] || !entry) return;
            var state = String(entry.state || '').toLowerCase();
            var ts = Number(entry.ts || 0);
            var busyState = state.indexOf('typing') >= 0 || state.indexOf('working') >= 0 || state.indexOf('processing') >= 0;
            var ttlMs = busyState ? 180000 : 20000;
            if (!Number.isFinite(ts) || (now - ts) > ttlMs) return;
            nextActivity[id] = entry;
          });
          store.agentLiveActivity = nextActivity;
          if (store.activeAgentId) {
            var activeId = String(store.activeAgentId || '');
            var pendingFreshId = String(store.pendingFreshAgentId || '');
            var stillActive = activeId === 'system' || nextAgents.some(function(agent) {
              return agent && agent.id === store.activeAgentId;
            });
            if (!stillActive && pendingFreshId && activeId && pendingFreshId === activeId) {
              stillActive = true;
            }
            if (!stillActive) {
              store.setActiveAgentId(null);
            }
          }
          store.agentCount = nextAgents.length;
        } else if (!store.agentsHydrated) {
          store.agentsLoading = true;
          store.agentsLastError = fetchError || 'agent_fetch_failed';
        }
        store._lastAgentsRefreshAt = Date.now();
      })();
      try {
        await store._refreshAgentsInFlight;
      } finally {
        store._refreshAgentsInFlight = null;
      }
    },

    async checkStatus() {
      if (this.booting || this.connectionState === 'disconnected') {
        this.connectionState = 'connecting';
      }
      try {
        var startedAt = Date.now();
        var results = await Promise.all([
          InfringAPI.get('/api/status'),
          InfringAPI.get('/api/version').catch(function() { return null; })
        ]);
        var latencyMs = Math.max(0, Date.now() - startedAt);
        var s = results[0];
        var versionPayload = results[1];
        var statusObj = (s && typeof s === 'object') ? s : {};
        var versionObj = (versionPayload && typeof versionPayload === 'object') ? versionPayload : {};
        var stateRaw = String(
          statusObj.connection_state ||
          statusObj.state ||
          (statusObj.connected === false ? 'disconnected' : 'connected')
        ).toLowerCase();
        var connectedState = stateRaw === 'connected';
        var degraded = !!statusObj.degraded || !!statusObj.warning || statusObj.ok === false;
        var bootStage = String(statusObj.boot_stage || statusObj.last_stage || (connectedState ? 'ready' : 'connecting')).trim();
        if (!connectedState) {
          throw new Error(String(statusObj.error || 'status_unavailable'));
        }
        this.connected = true;
        this.booting = false;
        this.statusFailureStreak = 0;
        this.connectionState = 'connected';
        this.statusDegraded = degraded;
        this.bootStage = bootStage || 'ready';
        this.lastStatusLatencyMs = latencyMs;
        this.lastStatusAt = new Date().toISOString();
        this.lastError = degraded ? String(statusObj.error || statusObj.warning || '') : '';
        this.lastErrorCode = normalizeDashboardOptionalString(statusObj.error_code || statusObj.warning_code || '');
        var liveVersion = String(versionObj.version || versionObj.tag || '').trim().replace(/^[vV]/, '');
        this.version = liveVersion || statusObj.version || this.version || window.__INFRING_APP_VERSION || '0.0.0';
        this.gitBranch = statusObj.git_branch ? String(statusObj.git_branch) : (this.gitBranch || '');
        this.agentCount = statusObj.agent_count || 0;
        this.runtimeSync = (statusObj.runtime_sync && typeof statusObj.runtime_sync === 'object') ? statusObj.runtime_sync : null;
        if (typeof this.applyBootstrapRuntimeState === 'function') {
          this.applyBootstrapRuntimeState(statusObj, versionObj);
        }
        await this.pollSessionActivity(false);
      } catch(e) {
        var streak = Number(this.statusFailureStreak || 0) + 1;
        this.connected = false;
        this.booting = false;
        this.statusFailureStreak = streak;
        this.statusDegraded = false;
        this.connectionState = streak >= 3 ? 'disconnected' : 'reconnecting';
        this.bootStage = streak >= 3 ? 'status_unreachable' : 'status_retrying';
        this.lastStatusLatencyMs = 0;
        this.lastStatusAt = new Date().toISOString();
        this.lastError = e.message || 'Unknown error';
        this.lastErrorCode = normalizeDashboardOptionalString((e && (e.code || e.name)) || '');
        this.runtimeSync = null;
        console.warn('[Infring] Status check failed:', e.message);
      }
    },

    async pollSessionActivity(force) {
      var now = Date.now();
      if (!force && this._lastSessionActivityPollAt && (now - Number(this._lastSessionActivityPollAt || 0)) < 8000) {
        return;
      }
      this._lastSessionActivityPollAt = now;
      try {
        var payload = await InfringAPI.get('/api/sessions');
        var rows = Array.isArray(payload && payload.sessions)
          ? payload.sessions
          : (Array.isArray(payload && payload.rows) ? payload.rows : []);
        var priorMap = this._sessionActivityByAgent && typeof this._sessionActivityByAgent === 'object'
          ? this._sessionActivityByAgent
          : {};
        var nextMap = {};
        var activeId = String(this.activeAgentId || '').trim();
        var noticesEmitted = 0;
        for (var i = 0; i < rows.length; i++) {
          var row = rows[i] && typeof rows[i] === 'object' ? rows[i] : null;
          if (!row) continue;
          var agentId = String(row.agent_id || '').trim();
          if (!agentId) continue;
          var messageCount = Number(row.message_count || 0);
          if (!Number.isFinite(messageCount) || messageCount < 0) messageCount = 0;
          var updatedAt = String(row.updated_at || '').trim();
          nextMap[agentId] = {
            message_count: messageCount,
            updated_at: updatedAt
          };
          if (!this._sessionActivityBootstrapped) continue;
          if (noticesEmitted >= 8) continue;
          var prior = priorMap[agentId];
          if (!prior || typeof prior !== 'object') continue;
          var priorCount = Number(prior.message_count || 0);
          if (!Number.isFinite(priorCount) || priorCount < 0) priorCount = 0;
          var priorUpdated = String(prior.updated_at || '').trim();
          var countIncreased = messageCount > priorCount;
          var updatedChanged = !!updatedAt && updatedAt !== priorUpdated;
          if (!countIncreased && !updatedChanged) continue;
          if (agentId === activeId) continue;

          if (typeof this.addNotification !== 'function') continue;

          var label = agentId === 'system' ? 'System' : ('Agent ' + agentId);
          var agent = null;
          if (Array.isArray(this.agents)) {
            agent = this.agents.find(function(entry) {
              return entry && String(entry.id || '').trim() === agentId;
            });
            if (agent) {
              var agentName = String(agent.name || '').trim();
              if (agentName) label = agentName;
            }
          }
          var serverPreview = agent && agent.sidebar_preview && typeof agent.sidebar_preview === 'object'
            ? agent.sidebar_preview
            : null;
          var preview = this.agentChatPreviews && this.agentChatPreviews[agentId]
            ? this.agentChatPreviews[agentId]
            : null;
          var previewText = '';
          if (serverPreview && typeof serverPreview.text === 'string') {
            previewText = serverPreview.text.replace(/\s+/g, ' ').trim();
          }
          if (!previewText && preview && typeof preview.text === 'string') {
            previewText = preview.text.replace(/\s+/g, ' ').trim();
          }
          if (previewText.length > 120) previewText = previewText.slice(0, 117) + '...';
          var summary = previewText || 'posted a new update.';
          var message = previewText ? (label + ': ' + previewText) : (label + ' posted a new update.');

          this.addNotification({
            type: agentId === 'system' ? 'warn' : 'info',
            message: message,
            ts: now + noticesEmitted,
            source: 'session_activity',
            page: 'chat',
            agent_id: agentId,
            summary: summary
          });
          noticesEmitted += 1;
        }
        this._sessionActivityByAgent = nextMap;
        this._sessionActivityBootstrapped = true;
      } catch(_) {}
    },

    normalizeDashboardAssistantIdentity(payload) {
      var source = payload && typeof payload === 'object' ? payload : {};
      var name = normalizeDashboardOptionalString(
        source.name ||
        source.assistant_name ||
        source.display_name ||
        source.label
      );
      var avatar = normalizeDashboardOptionalString(
        source.avatar ||
        source.avatar_url ||
        source.assistant_avatar
      );
      var agentId = normalizeDashboardOptionalString(
        source.agent_id ||
        source.assistant_agent_id ||
        source.id
      );
      return {
        name: name || 'Assistant',
        avatar: avatar || '',
        agentId: agentId || ''
      };
    },

    applyBootstrapRuntimeState(statusObj, versionObj) {
      var status = statusObj && typeof statusObj === 'object' ? statusObj : {};
      var version = versionObj && typeof versionObj === 'object' ? versionObj : {};
      var assistantPayload =
        (status.assistant_identity && typeof status.assistant_identity === 'object' && status.assistant_identity) ||
        (status.assistant && typeof status.assistant === 'object' && status.assistant) ||
        (version.assistant_identity && typeof version.assistant_identity === 'object' && version.assistant_identity) ||
        (version.assistant && typeof version.assistant === 'object' && version.assistant) ||
        {
          name: status.assistant_name || version.assistant_name || '',
          avatar: status.assistant_avatar || version.assistant_avatar || '',
          agent_id: status.assistant_agent_id || version.assistant_agent_id || ''
        };
      var assistantIdentity = this.normalizeDashboardAssistantIdentity(assistantPayload);
      this.assistantName = assistantIdentity.name || this.assistantName || 'Assistant';
      this.assistantAvatar = assistantIdentity.avatar || this.assistantAvatar || null;
      this.assistantAgentId = assistantIdentity.agentId || this.assistantAgentId || null;

      var serverVersion = normalizeDashboardOptionalString(version.version || version.tag || status.version).replace(/^[vV]/, '');
      if (serverVersion) this.serverVersion = serverVersion;

      var previewRoots = status.local_media_preview_roots || version.local_media_preview_roots;
      if (!Array.isArray(previewRoots) && status.media && typeof status.media === 'object') {
        previewRoots = status.media.local_preview_roots;
      }
      if (!Array.isArray(previewRoots) && version.media && typeof version.media === 'object') {
        previewRoots = version.media.local_preview_roots;
      }
      if (Array.isArray(previewRoots)) {
        this.localMediaPreviewRoots = previewRoots
          .map(function(root) { return normalizeDashboardOptionalString(root); })
          .filter(function(root) { return !!root; });
      }

      var sandboxMode = normalizeDashboardOptionalString(
        status.embed_sandbox_mode ||
        (status.embed && status.embed.sandbox_mode) ||
        version.embed_sandbox_mode ||
        (version.embed && version.embed.sandbox_mode)
      );
      if (sandboxMode) this.embedSandboxMode = sandboxMode;

      var allowExternal = status.allow_external_embed_urls;
      if (typeof allowExternal !== 'boolean' && status.embed && typeof status.embed === 'object') {
        allowExternal = status.embed.allow_external_urls;
      }
      if (typeof allowExternal !== 'boolean') {
        allowExternal = version.allow_external_embed_urls;
      }
      if (typeof allowExternal !== 'boolean' && version.embed && typeof version.embed === 'object') {
        allowExternal = version.embed.allow_external_urls;
      }
      if (typeof allowExternal === 'boolean') this.allowExternalEmbedUrls = allowExternal;
    },

    focusTaskbarSearchInput() {
      var self = this;
      if (this._taskbarSearchFocusTimer) {
        clearTimeout(this._taskbarSearchFocusTimer);
        this._taskbarSearchFocusTimer = 0;
      }
      this._taskbarSearchFocusTimer = window.setTimeout(function() {
        var input = document.getElementById('taskbar-search-input');
        if (input && typeof input.focus === 'function') {
          input.focus({ preventScroll: true });
          if (typeof input.select === 'function') input.select();
        }
        self._taskbarSearchFocusTimer = 0;
      }, 40);
    },

    openTaskbarSearch() {
      this.taskbarSearchOpen = false;
    },

    closeTaskbarSearch() {
      this.taskbarSearchOpen = false;
      if (this._taskbarSearchFocusTimer) {
        clearTimeout(this._taskbarSearchFocusTimer);
        this._taskbarSearchFocusTimer = 0;
      }
    },

    toggleTaskbarSearch() {
      this.taskbarSearchOpen = false;
    },

    async checkOnboarding() {
      if (localStorage.getItem('infring-onboarded')) return;
      try {
        var config = await InfringAPI.get('/api/config');
        var apiKey = config && config.api_key;
        var noKey = !apiKey || apiKey === 'not set' || apiKey === '';
        if (noKey && this.agentCount === 0) {
          this.showOnboarding = true;
        }
      } catch(e) {
        // If config endpoint fails, still show onboarding if no agents
        if (this.agentCount === 0) this.showOnboarding = true;
      }
    },

    dismissOnboarding() {
      this.showOnboarding = false;
      localStorage.setItem('infring-onboarded', 'true');
    },

    async checkAuth() {
      try {
        // First check if session-based auth is configured
        var authInfo = await InfringAPI.get('/api/auth/check');
        if (authInfo.mode === 'none') {
          // No session auth — fall back to API key detection
          this.authMode = 'apikey';
          this.sessionUser = null;
        } else if (authInfo.mode === 'session') {
          this.authMode = 'session';
          if (authInfo.authenticated) {
            this.sessionUser = authInfo.username;
            this.showAuthPrompt = false;
            return;
          }
          // Session auth enabled but not authenticated — show login prompt
          this.showAuthPrompt = true;
          return;
        }
      } catch(e) { /* ignore — fall through to API key check */ }


      // API key mode detection
      try {
        await InfringAPI.get('/api/tools');
        this.showAuthPrompt = false;
      } catch(e) {
        if (e.message && (e.message.indexOf('Not authorized') >= 0 || e.message.indexOf('401') >= 0 || e.message.indexOf('Missing Authorization') >= 0 || e.message.indexOf('Unauthorized') >= 0)) {
          var saved = localStorage.getItem('infring-api-key');
          if (saved) {
            InfringAPI.setAuthToken('');
            localStorage.removeItem('infring-api-key');
          }
          this.showAuthPrompt = true;
        }
      }
    },

    submitApiKey(key) {
      if (!key || !key.trim()) return;
      InfringAPI.setAuthToken(key.trim());
      localStorage.setItem('infring-api-key', key.trim());
      this.showAuthPrompt = false;
      this.refreshAgents();
    },

    async sessionLogin(username, password) {
      try {
        var result = await InfringAPI.post('/api/auth/login', { username: username, password: password });
        if (result.status === 'ok') {
          this.sessionUser = result.username;
          this.showAuthPrompt = false;
          this.refreshAgents();
        } else {
          InfringToast.error(result.error || 'Login failed');
        }
      } catch(e) {
        InfringToast.error(e.message || 'Login failed');
      }
    },

    async sessionLogout() {
      try {
        await InfringAPI.post('/api/auth/logout');
      } catch(e) { /* ignore */ }
      this.sessionUser = null;
      this.showAuthPrompt = true;
    },

    normalizeNotificationType(rawType, message) {
      var value = String(rawType || '').trim().toLowerCase();
      if (!value) {
        var text = String(message || '').toLowerCase();
        if (/(completed|complete|done|success|succeeded|finished|resolved)/.test(text)) {
          value = 'completed';
        } else if (/(error|failed|failure|aborted|abort|exception|crash|denied|timeout)/.test(text)) {
          value = 'error';
        } else {
          value = 'info';
        }
      }
      if (['completed', 'complete', 'done', 'success', 'ok', 'resolved', 'action_completed', 'task_completed'].indexOf(value) >= 0) {
        return 'completed';
      }
      if (['error', 'failed', 'failure', 'fatal', 'critical', 'danger', 'exception', 'aborted', 'abort', 'timeout'].indexOf(value) >= 0) {
        return 'error';
      }
      return 'info';
    },

    addNotification(payload) {
      var p = payload || {};
      var noteTs = Number(p.ts || Date.now());
      if (!Number.isFinite(noteTs) || noteTs <= 0) noteTs = Date.now();
      var noteMessage = String(p.message || '');
      var noteType = this.normalizeNotificationType(p.type, noteMessage);
      var noteAgentId = String(p.agent_id || p.agentId || '').trim();
      if (this.notifications && this.notifications.length) {
        var prior = this.notifications[0] || null;
        if (
          prior &&
          String(prior.message || '') === noteMessage &&
          String(prior.type || '') === noteType &&
          String(prior.agent_id || '') === noteAgentId &&
          Math.abs(noteTs - Number(prior.ts || 0)) <= 2200
        ) {
          return;
        }
      }
      var note = {
        id: p.id || ('notif-' + (++this._notificationSeq) + '-' + Date.now()),
        message: noteMessage,
        type: noteType,
        ts: noteTs,
        read: !!this.notificationsOpen,
        page: String(p.page || '').trim(),
        agent_id: noteAgentId,
        source: String(p.source || '').trim()
      };
      this.notifications.unshift(note);
      if (this.notifications.length > 150) this.notifications = this.notifications.slice(0, 150);
      this.unreadNotifications = this.notifications.filter(function(n) { return !n.read; }).length;
      this.ringNotificationBell();
      this.showNotificationBubble(note);
    },
    ringNotificationBell() {
      var self = this, seq = Number(this._notificationBellPulseSeq || 0) + 1;
      this._notificationBellPulseSeq = seq;
      this.notificationBellPulse = false;
      if (this._notificationBellPulseTimer) {
        clearTimeout(this._notificationBellPulseTimer);
        this._notificationBellPulseTimer = null;
      }
      var arm = function() {
        if (self._notificationBellPulseSeq !== seq) return;
        self.notificationBellPulse = true;
        self._notificationBellPulseTimer = setTimeout(function() {
          if (self._notificationBellPulseSeq !== seq) return;
          self.notificationBellPulse = false;
          self._notificationBellPulseTimer = null;
        }, 760);
      };
      if (typeof requestAnimationFrame === 'function') {
        requestAnimationFrame(arm);
      } else {
        setTimeout(arm, 0);
      }
    },
    showNotificationBubble(note) {
      var n = note || null;
      if (!n) return;
      this.notificationBubble = {
        id: n.id,
        message: n.message,
        type: n.type,
        ts: n.ts,
      };
      if (this._notificationBubbleTimer) clearTimeout(this._notificationBubbleTimer);
      var self = this;
      this._notificationBubbleTimer = setTimeout(function() {
        self.notificationBubble = null;
      }, 5200);
    },

    toggleNotifications() {
      this.notificationsOpen = !this.notificationsOpen;
      if (this.notificationsOpen) this.markAllNotificationsRead();
    },

    markNotificationRead(id) {
      this.notifications = this.notifications.map(function(n) {
        if (n.id === id) n.read = true;
        return n;
      });
      this.unreadNotifications = this.notifications.filter(function(n) { return !n.read; }).length;
    },

    markAllNotificationsRead() {
      this.notifications = this.notifications.map(function(n) {
        n.read = true;
        return n;
      });
      this.unreadNotifications = 0;
    },

    dismissNotification(id) {
      var targetId = String(id || '').trim();
      if (!targetId) return;
      this.notifications = this.notifications.filter(function(n) {
        return String(n && n.id ? n.id : '') !== targetId;
      });
      this.unreadNotifications = this.notifications.filter(function(n) { return !n.read; }).length;
      if (this.notificationBubble && String(this.notificationBubble.id || '') === targetId) {
        this.dismissNotificationBubble();
      }
    },

    clearNotifications() {
      this.notifications = [];
      this.notificationsOpen = false;
      this.unreadNotifications = 0;
      this.notificationBubble = null;
      this.notificationBellPulse = false;
      this._notificationBellPulseSeq = 0;
      if (this._notificationBellPulseTimer) {
        clearTimeout(this._notificationBellPulseTimer);
        this._notificationBellPulseTimer = null;
      }
      if (this._notificationBubbleTimer) {
        clearTimeout(this._notificationBubbleTimer);
        this._notificationBubbleTimer = null;
      }
    },

    reopenNotification(note) {
      if (!note) return;
      this.markNotificationRead(note.id);
      this.showNotificationBubble(note);
      this.notificationsOpen = false;
      var targetAgentId = String(note.agent_id || '').trim();
      var targetPage = String(note.page || '').trim();
      if (targetAgentId) {
        if (typeof this.setActiveAgentId === 'function') {
          this.setActiveAgentId(targetAgentId);
        } else {
          this.activeAgentId = targetAgentId;
        }
      }
      if (targetPage) {
        window.location.hash = targetPage;
      } else if (targetAgentId) {
        window.location.hash = 'chat';
      }
    },

    dismissNotificationBubble() {
      this.notificationBubble = null;
      if (this._notificationBubbleTimer) {
        clearTimeout(this._notificationBubbleTimer);
        this._notificationBubbleTimer = null;
      }
    },

    saveAgentChatPreview(agentId, messages) {
      if (!agentId) return;
      var list = Array.isArray(messages) ? messages : [];
      var previewKey = String(agentId);
      var existingPreview = this.agentChatPreviews && this.agentChatPreviews[previewKey]
        ? this.agentChatPreviews[previewKey]
        : null;
      var preview = {
        text: '',
        ts: Date.now(),
        role: 'agent',
        has_tools: false,
        tool_state: '',
        tool_label: '',
        unread_response: !!(existingPreview && existingPreview.unread_response)
      };
      var toolStateRank = { success: 1, warning: 2, error: 3 };
      var classifyTool = function(tool) {
        if (!tool) return '';
        if (tool.running) return 'warning';
        var status = String(tool.status || '').toLowerCase();
        var result = String(tool.result || '').toLowerCase();
        var blocked = tool.blocked === true || status === 'blocked' ||
          result.indexOf('blocked') >= 0 ||
          result.indexOf('policy') >= 0 ||
          result.indexOf('denied') >= 0 ||
          result.indexOf('not allowed') >= 0 ||
          result.indexOf('forbidden') >= 0 ||

          result.indexOf('approval') >= 0 ||
          result.indexOf('permission') >= 0 ||
          result.indexOf('fail-closed') >= 0;
        if (blocked) return 'warning';
        if (tool.is_error) return 'error';
        return 'success';
      };
      var summarizeTools = function(tools) {
        if (!Array.isArray(tools) || !tools.length) return { has_tools: false, tool_state: '', tool_label: '' };
        var state = 'success';
        for (var ti = 0; ti < tools.length; ti++) {
          var s = classifyTool(tools[ti]) || 'success';
          if ((toolStateRank[s] || 0) > (toolStateRank[state] || 0)) state = s;
        }
        var label = state === 'error'
          ? 'Tool error'
          : (state === 'warning' ? 'Tool warning' : 'Tool success');
        return { has_tools: true, tool_state: state, tool_label: label };
      };
      for (var i = list.length - 1; i >= 0; i--) {
        var msg = list[i] || {};
        var text = '';
        var toolInfo = summarizeTools(msg.tools);
        if (typeof msg.text === 'string' && msg.text.trim()) {
          text = msg.text.replace(/\s+/g, ' ').trim();
        } else if (Array.isArray(msg.tools) && msg.tools.length) {
          text = '[Processes] ' + msg.tools.map(function(tool) {
            return tool && tool.name ? tool.name : 'tool';
          }).join(', ');
        }
        if (text) {
          preview.text = text;
          preview.ts = Number(msg.ts || Date.now());
          preview.role = String(msg.role || 'agent');
          preview.has_tools = !!toolInfo.has_tools;
          preview.tool_state = toolInfo.tool_state || '';
          preview.tool_label = toolInfo.tool_label || '';
          break;
        }
      }
      if (preview.role === 'agent') {
        preview.unread_response = String(this.activeAgentId || '') !== previewKey;
      } else if (String(this.activeAgentId || '') === previewKey) {
        preview.unread_response = false;
      }
      var previewChanged = !!existingPreview && (
        Number(preview.ts || 0) > Number(existingPreview.ts || 0) ||
        String(preview.text || '') !== String(existingPreview.text || '') ||
        String(preview.role || '') !== String(existingPreview.role || '') ||
        String(preview.tool_state || '') !== String(existingPreview.tool_state || '')
      );
      var inactiveAgent = String(this.activeAgentId || '') !== previewKey;
      if (previewChanged && inactiveAgent && preview.role === 'agent' && String(preview.text || '').trim()) {
        var label = 'Agent';
        if (Array.isArray(this.agents)) {
          var found = this.agents.find(function(row) {
            return row && String(row.id || '') === previewKey;
          });
          if (found) {
            var foundName = String(found.name || '').trim();
            if (foundName) label = foundName;
          }
        }
        var compact = String(preview.text || '').replace(/\s+/g, ' ').trim();
        if (compact.length > 120) compact = compact.slice(0, 117) + '...';
        this.addNotification({
          type: 'info',
          message: label + ': ' + compact,
          agent_id: previewKey,
          page: 'chat',
          source: 'agent_preview',
          ts: Number(preview.ts || Date.now())
        });
      }
      this.agentChatPreviews[previewKey] = preview;
    },

    getAgentChatPreview(agentId) {
      if (!agentId) return null;
      return this.agentChatPreviews[String(agentId)] || null;
    },

    coerceAgentTimestamp(value) {
      if (value === null || typeof value === 'undefined' || value === '') return 0;
      if (typeof value === 'number') {
        if (!Number.isFinite(value)) return 0;
        return value < 1e12 ? Math.round(value * 1000) : Math.round(value);
      }
      var asNum = Number(value);
      if (Number.isFinite(asNum) && String(value).trim() !== '') {
        return asNum < 1e12 ? Math.round(asNum * 1000) : Math.round(asNum);
      }
      var asDate = Number(new Date(value).getTime());
      return Number.isFinite(asDate) ? asDate : 0;
    },

    agentLastActivityTs(agent) {
      if (!agent) return 0;
      var latest = 0;
      var keys = ['last_active_at', 'last_activity_at', 'last_message_at', 'last_seen_at', 'updated_at'];
      for (var i = 0; i < keys.length; i++) {
        var ts = this.coerceAgentTimestamp(agent[keys[i]]);
        if (ts > latest) latest = ts;
      }
      if (agent.id) {
        var preview = this.getAgentChatPreview(agent.id);
        var previewTs = this.coerceAgentTimestamp(preview && preview.ts);
        if (previewTs > latest) latest = previewTs;
      }
      return latest;
    },

    agentStatusFreshness(agent) {
      var raw = agent && agent.sidebar_status_freshness && typeof agent.sidebar_status_freshness === 'object'
        ? agent.sidebar_status_freshness
        : {};
      var source = String((raw.source || (agent && agent.sidebar_status_source) || '')).trim();
      var sourceSequence = String((raw.source_sequence || (agent && agent.sidebar_status_source_sequence) || '')).trim();
      var ageRaw = Number(
        typeof raw.age_seconds !== 'undefined'
          ? raw.age_seconds
          : (agent && agent.sidebar_status_age_seconds)
      );
      var ageSeconds = Number.isFinite(ageRaw) && ageRaw >= 0 ? ageRaw : 0;
      var staleRaw = raw.stale;
      if (typeof staleRaw !== 'boolean' && agent && typeof agent.sidebar_status_stale === 'boolean') {
        staleRaw = agent.sidebar_status_stale;
      }
      var stale = staleRaw === true;
      return {
        source: source,
        source_sequence: sourceSequence,
        age_seconds: ageSeconds,
        stale: stale
      };
    },

    agentStatusState(agent) {
      if (!agent) return 'offline';
      var rawServerState = (typeof agent.sidebar_status_state === 'string')
        ? agent.sidebar_status_state
        : '';
      var serverState = String(rawServerState).trim().toLowerCase();
      if (serverState === 'active' || serverState === 'idle' || serverState === 'offline') return serverState;
      var freshness = this.agentStatusFreshness(agent);
      if (freshness.stale) return 'offline';
      return 'offline';
    },

    agentStatusLabel(agent) {
      var rawServerLabel = (agent && typeof agent.sidebar_status_label === 'string')
        ? agent.sidebar_status_label
        : '';
      var serverLabel = String(rawServerLabel).trim().toLowerCase();
      if (serverLabel === 'active' || serverLabel === 'idle' || serverLabel === 'offline') return serverLabel;
      var rawServerState = (agent && typeof agent.sidebar_status_state === 'string')
        ? agent.sidebar_status_state
        : '';
      var serverState = String(rawServerState).trim().toLowerCase();
      if (serverState === 'active' || serverState === 'idle' || serverState === 'offline') return serverState;
      var freshness = this.agentStatusFreshness(agent);
      if (freshness.stale) return 'offline';
      return 'offline';
    },

    setAgentLiveActivity(agentId, state) {
      var id = String(agentId || '').trim();
      if (!id) return;
      var normalized = String(state || '').trim().toLowerCase();
      if (!normalized || normalized === 'idle' || normalized === 'done' || normalized === 'stop' || normalized === 'stopped') {
        if (this.agentLiveActivity && Object.prototype.hasOwnProperty.call(this.agentLiveActivity, id)) {
          delete this.agentLiveActivity[id];
          this.agentLiveActivity = Object.assign({}, this.agentLiveActivity);
        }
        return;
      }
      this.agentLiveActivity = Object.assign({}, this.agentLiveActivity || {}, {
        [id]: { state: normalized, ts: Date.now() }
      });
    },

    clearAgentLiveActivity(agentId) {
      this.setAgentLiveActivity(agentId, 'idle');
    },

    isAgentLiveBusy(agent) {
      if (!agent || !agent.id) return false;
      var id = String(agent.id);
      var entry = this.agentLiveActivity ? this.agentLiveActivity[id] : null;
      if (entry) {
        var state = String(entry.state || '').toLowerCase();
        var ts = Number(entry.ts || 0);
        var busyState = state.indexOf('typing') >= 0 || state.indexOf('working') >= 0 || state.indexOf('processing') >= 0;
        // Allow longer-lived busy windows so long tool/reasoning phases keep
        // the avatar pulse visible until completion events clear the state.
        if (busyState && Number.isFinite(ts) && (Date.now() - ts) <= 180000) return true;
      }
      return false;
    },

    formatNotificationTime(ts) {
      if (!ts) return '';
      var d = new Date(ts);
      if (Number.isNaN(d.getTime())) return '';
      return d.toLocaleTimeString([], { hour: 'numeric', minute: '2-digit' });
    },

    clearApiKey() {
      InfringAPI.setAuthToken('');
      localStorage.removeItem('infring-api-key');
    }
  });
});

function infringTaskbarDockService() {
  var services = typeof window !== 'undefined' ? window.InfringSharedShellServices : null;
  return services && services.taskbarDock ? services.taskbarDock : null;
}

function infringShellLayoutDefaultProfile() {
  var service = infringTaskbarDockService();
  if (service && typeof service.defaultProfile === 'function') return service.defaultProfile();
  var raw = '';
  try {
    raw = String((navigator && (navigator.userAgent || navigator.platform)) || '').toLowerCase();
  } catch(_) {}
  if (raw.indexOf('mac') >= 0 || raw.indexOf('darwin') >= 0) return 'mac';
  if (raw.indexOf('win') >= 0) return 'windows';
  if (raw.indexOf('linux') >= 0 || raw.indexOf('x11') >= 0) return 'linux';
  return 'other';
}

function infringShellLayoutDefaultConfig() {
  var service = infringTaskbarDockService();
  if (service && typeof service.defaultLayoutConfig === 'function') return service.defaultLayoutConfig();
  var profile = infringShellLayoutDefaultProfile();
  var macLike = profile === 'mac';
  return {
    version: 1,
    profile: profile,
    dock: {
      placement: 'center',
      wallLock: macLike ? '' : 'bottom',
      order: ['chat', 'overview', 'agents', 'scheduler', 'skills', 'runtime', 'settings']
    },
    taskbar: {
      edge: macLike ? 'top' : 'bottom',
      orderLeft: ['nav_cluster'],
      orderRight: ['connectivity', 'theme', 'notifications', 'search', 'auth']
    },
    chatMap: { placementX: 1, placementY: 0.38, wallLock: 'right' },
    chatBar: { placementX: 1, placementY: 0.5, placementTopPx: null, wallLock: 'right' }
  };
}

function infringLocalStorageHasAny(keys) {
  var service = infringTaskbarDockService();
  if (service && typeof service.hasAnyStorage === 'function') return service.hasAnyStorage(keys);
  try {
    for (var i = 0; i < keys.length; i += 1) {
      if (localStorage.getItem(keys[i]) !== null) return true;
    }
  } catch(_) {}
  return false;
}

function infringReadShellLayoutConfig() {
  var service = infringTaskbarDockService();
  if (service && typeof service.readLayoutConfig === 'function') return service.readLayoutConfig();
  var key = 'infring-shell-layout-config';
  var config = null;
  try {
    var raw = localStorage.getItem(key);
    config = raw ? JSON.parse(raw) : null;
  } catch(_) {
    config = null;
  }
  if (!config || typeof config !== 'object') config = infringShellLayoutDefaultConfig();
  var defaults = infringShellLayoutDefaultConfig();
  config.dock = config.dock && typeof config.dock === 'object' ? config.dock : {};
  config.taskbar = config.taskbar && typeof config.taskbar === 'object' ? config.taskbar : {};
  config.chatMap = config.chatMap && typeof config.chatMap === 'object' ? config.chatMap : {};
  config.chatBar = config.chatBar && typeof config.chatBar === 'object' ? config.chatBar : {};
  config.dock.placement = String(config.dock.placement || defaults.dock.placement);
  config.dock.wallLock = String(config.dock.wallLock || defaults.dock.wallLock || '');
  config.taskbar.edge = String(config.taskbar.edge || defaults.taskbar.edge);
  config.chatMap.placementX = Number.isFinite(Number(config.chatMap.placementX)) ? Number(config.chatMap.placementX) : defaults.chatMap.placementX;
  config.chatMap.placementY = Number.isFinite(Number(config.chatMap.placementY)) ? Number(config.chatMap.placementY) : defaults.chatMap.placementY;
  config.chatMap.wallLock = String(config.chatMap.wallLock || defaults.chatMap.wallLock || '');
  config.chatBar.placementX = Number.isFinite(Number(config.chatBar.placementX)) ? Number(config.chatBar.placementX) : defaults.chatBar.placementX;
  config.chatBar.placementY = Number.isFinite(Number(config.chatBar.placementY)) ? Number(config.chatBar.placementY) : defaults.chatBar.placementY;
  config.chatBar.placementTopPx = Number.isFinite(Number(config.chatBar.placementTopPx)) ? Number(config.chatBar.placementTopPx) : null;
  config.chatBar.wallLock = String(config.chatBar.wallLock || defaults.chatBar.wallLock || '');
  if (!Array.isArray(config.dock.order)) config.dock.order = defaults.dock.order.slice();
  if (!Array.isArray(config.taskbar.orderLeft)) config.taskbar.orderLeft = defaults.taskbar.orderLeft.slice();
  if (!Array.isArray(config.taskbar.orderRight)) config.taskbar.orderRight = defaults.taskbar.orderRight.slice();
  return config;
}

function infringWriteShellLayoutConfig(config) {
  var service = infringTaskbarDockService();
  if (service && typeof service.writeLayoutConfig === 'function') {
    service.writeLayoutConfig(config);
    return;
  }
  try {
    localStorage.setItem('infring-shell-layout-config', JSON.stringify(config));
  } catch(_) {}
}

function infringUpdateShellLayoutConfig(mutator) {
  var service = infringTaskbarDockService();
  if (service && typeof service.updateLayoutConfig === 'function') {
    infringShellLayoutConfig = service.updateLayoutConfig(mutator);
    return;
  }
  var config = infringReadShellLayoutConfig();
  try { mutator(config); } catch(_) {}
  infringShellLayoutConfig = config;
  infringWriteShellLayoutConfig(config);
}

function infringSeedShellLayoutConfig() {
  var service = infringTaskbarDockService();
  if (service && typeof service.seedLayoutConfig === 'function') return service.seedLayoutConfig();
  var config = infringReadShellLayoutConfig();
  var existed = false;
  try { existed = localStorage.getItem('infring-shell-layout-config') !== null; } catch(_) {}
  if (!existed) {
    var dockKeys = ['infring-bottom-dock-placement', 'infring-bottom-dock-wall-lock', 'infring-bottom-dock-order'];
    var taskbarKeys = ['infring-taskbar-dock-edge', 'infring-taskbar-order-left', 'infring-taskbar-order-right'];
    var chatMapKeys = ['infring-chat-map-placement-x', 'infring-chat-map-placement-y', 'infring-chat-map-wall-lock'];
    var chatBarKeys = ['infring-chat-sidebar-placement-x', 'infring-chat-sidebar-placement-y', 'infring-chat-sidebar-placement-top-px', 'infring-chat-sidebar-wall-lock'];
    try {
      if (localStorage.getItem(dockKeys[0])) config.dock.placement = localStorage.getItem(dockKeys[0]);
      if (localStorage.getItem(dockKeys[1])) config.dock.wallLock = localStorage.getItem(dockKeys[1]);
      if (localStorage.getItem(dockKeys[2])) config.dock.order = JSON.parse(localStorage.getItem(dockKeys[2]) || '[]');
      if (localStorage.getItem(taskbarKeys[0])) config.taskbar.edge = localStorage.getItem(taskbarKeys[0]);
      if (localStorage.getItem(taskbarKeys[1])) config.taskbar.orderLeft = JSON.parse(localStorage.getItem(taskbarKeys[1]) || '[]');
      if (localStorage.getItem(taskbarKeys[2])) config.taskbar.orderRight = JSON.parse(localStorage.getItem(taskbarKeys[2]) || '[]');
      if (localStorage.getItem(chatMapKeys[0])) config.chatMap.placementX = Number(localStorage.getItem(chatMapKeys[0]));
      if (localStorage.getItem(chatMapKeys[1])) config.chatMap.placementY = Number(localStorage.getItem(chatMapKeys[1]));
      if (localStorage.getItem(chatMapKeys[2])) config.chatMap.wallLock = localStorage.getItem(chatMapKeys[2]);
      if (localStorage.getItem(chatBarKeys[0])) config.chatBar.placementX = Number(localStorage.getItem(chatBarKeys[0]));
      if (localStorage.getItem(chatBarKeys[1])) config.chatBar.placementY = Number(localStorage.getItem(chatBarKeys[1]));
      if (localStorage.getItem(chatBarKeys[2])) config.chatBar.placementTopPx = Number(localStorage.getItem(chatBarKeys[2]));
      if (localStorage.getItem(chatBarKeys[3])) config.chatBar.wallLock = localStorage.getItem(chatBarKeys[3]);
    } catch(_) {}
  }
  try {
    if (!infringLocalStorageHasAny(['infring-bottom-dock-placement'])) localStorage.setItem('infring-bottom-dock-placement', String(config.dock.placement || 'center'));
    if (!infringLocalStorageHasAny(['infring-bottom-dock-wall-lock', 'infring-bottom-dock-smash-wall']) && config.dock.wallLock) localStorage.setItem('infring-bottom-dock-wall-lock', String(config.dock.wallLock));
    if (!infringLocalStorageHasAny(['infring-bottom-dock-order'])) localStorage.setItem('infring-bottom-dock-order', JSON.stringify(config.dock.order || []));
    if (!infringLocalStorageHasAny(['infring-taskbar-dock-edge'])) localStorage.setItem('infring-taskbar-dock-edge', String(config.taskbar.edge || 'top'));
    if (!infringLocalStorageHasAny(['infring-taskbar-order-left'])) localStorage.setItem('infring-taskbar-order-left', JSON.stringify(config.taskbar.orderLeft || []));
    if (!infringLocalStorageHasAny(['infring-taskbar-order-right'])) localStorage.setItem('infring-taskbar-order-right', JSON.stringify(config.taskbar.orderRight || []));
    if (!infringLocalStorageHasAny(['infring-chat-map-placement-x'])) localStorage.setItem('infring-chat-map-placement-x', String(config.chatMap.placementX));
    if (!infringLocalStorageHasAny(['infring-chat-map-placement-y'])) localStorage.setItem('infring-chat-map-placement-y', String(config.chatMap.placementY));
    if (!infringLocalStorageHasAny(['infring-chat-map-wall-lock', 'infring-chat-map-smash-wall']) && config.chatMap.wallLock) localStorage.setItem('infring-chat-map-wall-lock', String(config.chatMap.wallLock));
    if (!infringLocalStorageHasAny(['infring-chat-sidebar-placement-x'])) localStorage.setItem('infring-chat-sidebar-placement-x', String(config.chatBar.placementX));
    if (!infringLocalStorageHasAny(['infring-chat-sidebar-placement-y'])) localStorage.setItem('infring-chat-sidebar-placement-y', String(config.chatBar.placementY));
    if (!infringLocalStorageHasAny(['infring-chat-sidebar-placement-top-px']) && Number.isFinite(Number(config.chatBar.placementTopPx))) localStorage.setItem('infring-chat-sidebar-placement-top-px', String(config.chatBar.placementTopPx));
    if (!infringLocalStorageHasAny(['infring-chat-sidebar-wall-lock', 'infring-chat-sidebar-smash-wall']) && config.chatBar.wallLock) localStorage.setItem('infring-chat-sidebar-wall-lock', String(config.chatBar.wallLock));
  } catch(_) {}
  infringWriteShellLayoutConfig(config);
  return config;
}

var infringShellLayoutConfig = infringSeedShellLayoutConfig();

// Main app component
function app() {
  return {
    page: 'agents',
    themeMode: localStorage.getItem('infring-theme-mode') || 'system',
    overlayGlassTemplate: 'simple-glass',
    uiBackgroundTemplate: (() => {
      var service = infringTaskbarDockService();
      if (service && typeof service.readDisplayBackground === 'function') return service.readDisplayBackground();
      var mode = 'light-wood';
      try {
        var rawDisplaySettings = localStorage.getItem('infring-display-settings') || '';
        var displaySettings = rawDisplaySettings ? JSON.parse(rawDisplaySettings) : {};
        mode = String(displaySettings && displaySettings.background ? displaySettings.background : mode);
        if (mode === 'sand') {
          mode = 'light-wood';
          displaySettings = displaySettings && typeof displaySettings === 'object' ? displaySettings : {};
          displaySettings.background = mode;
          localStorage.setItem('infring-display-settings', JSON.stringify(displaySettings));
        }
        if (!rawDisplaySettings || !displaySettings.background) {
          displaySettings = displaySettings && typeof displaySettings === 'object' ? displaySettings : {};
          displaySettings.background = mode;
          localStorage.setItem('infring-display-settings', JSON.stringify(displaySettings));
        }
      } catch (_) {}
      if (mode === 'unsplash-paper') mode = 'light-wood';
      if (mode !== 'default-grid' && mode !== 'light-wood' && mode !== 'sand') mode = 'light-wood';
      try {
        document.documentElement.setAttribute('data-ui-background-template', mode);
      } catch (_) {}
      return mode;
    })(),
    theme: (() => {
      var mode = localStorage.getItem('infring-theme-mode') || 'system';
      if (mode === 'system') return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
      return mode;
    })(),
    sidebarCollapsed: localStorage.getItem('infring-sidebar') === 'collapsed',
    mobileMenuOpen: false,
    chatSidebarMode: 'default',
    chatSidebarQuery: '',
    chatSidebarSearchResults: [],
    chatSidebarSearchLoading: false,
    chatSidebarSearchError: '',
    chatSidebarSearchSeq: 0,
    _chatSidebarSearchTimer: 0,
    agentChatsSectionCollapsed: false,
    chatSidebarSortMode: (() => {
      try {
        var saved = String(localStorage.getItem('infring-chat-sidebar-sort-mode') || '').trim().toLowerCase();
        return saved === 'topology' ? 'topology' : 'age';
      } catch(_) {
        return 'age';
      }
    })(),
    chatSidebarTopologyOrder: (() => {
      try {
        var raw = localStorage.getItem('infring-chat-sidebar-topology-order');
        var parsed = raw ? JSON.parse(raw) : [];
        if (!Array.isArray(parsed)) return [];
        return parsed.map(function(id) { return String(id || '').trim(); }).filter(Boolean);
      } catch(_) {
        return [];
      }
    })(),
    chatSidebarDragAgentId: '',

    chatSidebarDropTargetId: '',
    chatSidebarDropAfter: false,
    chatSidebarVisibleBase: 7,
    chatSidebarVisibleStep: 5,
    chatSidebarVisibleCount: 7,
    dashboardPopup: {
      id: '',
      active: false,
      source: '',
      title: '',
      body: '',
      meta_origin: '',
      meta_time: '',
      unread: false,
      left: 0,
      top: 0,
      side: 'bottom',
      inline_away: 'right',
      block_away: 'bottom',
      compact: false
    },
    confirmArchiveAgentId: '',
    sidebarSpawningAgent: false,
    connected: false,
    wsConnected: false,
    connectionState: 'connecting',
    connectionIndicatorState: 'connecting',
    healthSummary: null,
    healthSummaryError: '',
    version: (window.__INFRING_APP_VERSION || '0.0.0'),
    agentCount: 0,
    bootSelectionApplied: false,
    clockTick: Date.now(),
    _dashboardClockTimer: 0,
    _dashboardStatusTimer: 0,
    _dashboardVisibilityHandler: null,
    _themeSwitchReset: 0,
    _lastConnectionIndicatorAt: 0,
    _connectionIndicatorTimer: null,
    _pendingConnectionIndicatorState: '',
    _healthSummaryLoadedAt: 0,
    _healthSummaryLoading: null,
    _healthSummaryLoadSeq: 0,
    _pollStatusInFlight: null,
    _pollStatusQueued: false,
    sidebarHasOverflowAbove: false,
    sidebarHasOverflowBelow: false,
    chatSidebarHasOverflowAbove: false,
    chatSidebarHasOverflowBelow: false,
    _sidebarScrollIndicatorRaf: 0,
    _chatSidebarFlipDurationMs: 240,
    _chatSidebarFlipRaf: 0,
    _chatSidebarLastSnapshot: null,
    _dragSurfaceLockTransformMs: 500,
    _dragSurfaceVisualStates: {},
    chatSidebarDragActive: false,
    chatSidebarDragLeft: 0,
    chatSidebarDragTop: 0,
    _chatSidebarDragRowsCache: null,
    _chatSidebarDragRenderMaxRows: 10,
    _chatSidebarDragRenderRowHeight: 56,
    chatSidebarPlacementX: (() => {
      try {
        var raw = Number(localStorage.getItem('infring-chat-sidebar-placement-x'));
        if (Number.isFinite(raw)) return Math.max(0, Math.min(1, raw));
      } catch(_) {}
      return 0;
    })(),
    chatSidebarPlacementY: (() => {
      try {
        var raw = Number(localStorage.getItem('infring-chat-sidebar-placement-y'));
        if (Number.isFinite(raw)) return Math.max(0, Math.min(1, raw));
      } catch(_) {}
      return 0.5;
    })(),
    chatSidebarPlacementTopPx: (() => {
      try {
        var raw = Number(localStorage.getItem('infring-chat-sidebar-placement-top-px'));
        if (Number.isFinite(raw)) return raw;
      } catch(_) {}
      return Number.NaN;
    })(),
    chatSidebarWallLock: (() => {
      try {
        var raw = String(
          localStorage.getItem('infring-chat-sidebar-wall-lock')
          || localStorage.getItem('infring-chat-sidebar-smash-wall')
          || ''
        ).trim().toLowerCase();
        if (raw === 'left' || raw === 'right' || raw === 'top' || raw === 'bottom') return raw;
      } catch(_) {}
      return '';
    })(),
    _chatSidebarMoveDurationMs: 280,
    _chatSidebarPointerActive: false,
    _chatSidebarPointerMoved: false,
    _chatSidebarPointerStartX: 0,
    _chatSidebarPointerStartY: 0,
    _chatSidebarPointerOriginLeft: 0,
    _chatSidebarPointerOriginTop: 0,
    _chatSidebarPointerLastX: 0,
    _chatSidebarPointerLastY: 0,
    _chatSidebarPointerLastAt: 0,
    _chatSidebarPointerVelocity: 0,
    _chatSidebarPointerMoveHandler: null,
    _chatSidebarPointerUpHandler: null,
    _sidebarToggleSuppressUntil: 0,
    chatMapDragActive: false,
    chatMapDragLeft: 0,
    chatMapDragTop: 0,
    chatMapPlacementX: (() => {
      try {
        var raw = Number(localStorage.getItem('infring-chat-map-placement-x'));
        if (Number.isFinite(raw)) return Math.max(0, Math.min(1, raw));
      } catch(_) {}
      return 1;
    })(),
    chatMapPlacementY: (() => {
      try {
        var raw = Number(localStorage.getItem('infring-chat-map-placement-y'));
        if (Number.isFinite(raw)) return Math.max(0, Math.min(1, raw));
      } catch(_) {}
      return 0.38;
    })(),
    chatMapWallLock: (() => {
      try {
        var raw = String(
          localStorage.getItem('infring-chat-map-wall-lock')
          || localStorage.getItem('infring-chat-map-smash-wall')
          || ''
        ).trim().toLowerCase();
        if (raw === 'left' || raw === 'right' || raw === 'top' || raw === 'bottom') return raw;
      } catch(_) {}
      return '';
    })(),
    _chatMapMoveDurationMs: 280,
    _chatMapPointerActive: false,
    _chatMapPointerMoved: false,
    _chatMapPointerStartX: 0,
    _chatMapPointerStartY: 0,
    _chatMapPointerOriginLeft: 0,
    _chatMapPointerOriginTop: 0,
    _chatMapPointerLastX: 0,
    _chatMapPointerLastY: 0,
    _chatMapPointerLastAt: 0,
    _chatMapPointerVelocity: 0,
    _chatMapPointerMoveHandler: null,
    _chatMapPointerUpHandler: null,
    bootSplashVisible: true,
    _bootSplashStartedAt: Date.now(),
    _bootSplashMinMs: 850,
    _bootSplashMaxMs: 5000,
    _bootSplashHideTimer: 0,
    _bootSplashMaxTimer: 0,
    bootProgressPercent: 6,
    bootProgressEvent: 'splash_visible',
    _bootProgressUpdatedAt: Date.now(),
    _taskbarRefreshOverlayTimer: 0,
    _taskbarRefreshReloadTimer: 0,
    taskbarHeroMenuOpen: false,
    taskbarTextMenuOpen: '',
    helpManualWindowOpen: false,
    reportIssueWindowOpen: false,
    reportIssueDraft: '',
    popupWindowPlacements: {
      manual: { left: null, top: null },
      report: { left: null, top: null }
    },
    popupWindowWallLocks: {
      manual: (() => {
        try {
          var raw = String(
            localStorage.getItem('infring-popup-window-manual-wall-lock')
            || localStorage.getItem('infring-popup-window-manual-smash-wall')
            || ''
          ).trim().toLowerCase();
          if (raw === 'left' || raw === 'right' || raw === 'top' || raw === 'bottom') return raw;
        } catch(_) {}
        return '';
      })(),
      report: (() => {
        try {
          var raw = String(
            localStorage.getItem('infring-popup-window-report-wall-lock')
            || localStorage.getItem('infring-popup-window-report-smash-wall')
            || ''
          ).trim().toLowerCase();
          if (raw === 'left' || raw === 'right' || raw === 'top' || raw === 'bottom') return raw;
        } catch(_) {}
        return '';
      })()
    },
    popupWindowDragActive: false,
    popupWindowDragKind: '',
    popupWindowDragLeft: 0,
    popupWindowDragTop: 0,
    popupWindowDragWallLock: '',
    _popupWindowMoveDurationMs: 260,
    _popupWindowPointerActive: false,
    _popupWindowPointerMoved: false,
    _popupWindowPointerStartX: 0,
    _popupWindowPointerStartY: 0,
    _popupWindowPointerOriginLeft: 0,
    _popupWindowPointerOriginTop: 0,
    _popupWindowPointerLastX: 0,
    _popupWindowPointerLastY: 0,
    _popupWindowPointerLastAt: 0,
    _popupWindowPointerVelocity: 0,
    _popupWindowPointerMoveHandler: null,
    _popupWindowPointerUpHandler: null,
    taskbarHeroActionPending: '',
    taskbarDockEdge: (() => {
      var service = infringTaskbarDockService();
      if (service && typeof service.readLayoutConfig === 'function') return service.readLayoutConfig().taskbar.edge;
      try {
        var raw = String(localStorage.getItem('infring-taskbar-dock-edge') || '').trim().toLowerCase();
        if (raw === 'bottom') return 'bottom';
      } catch(_) {}
      return 'top';
    })(),
    taskbarDockDragActive: false,
    taskbarDockDragY: 0,
    _taskbarDockPointerActive: false,
    _taskbarDockPointerMoved: false,
    _taskbarDockPointerStartX: 0,
    _taskbarDockPointerStartY: 0,
    _taskbarDockOriginY: 0,
    _taskbarDockPointerMoveHandler: null,
    _taskbarDockPointerUpHandler: null,
    taskbarReorderLeft: (() => {
      var service = infringTaskbarDockService();
      if (service && typeof service.readTaskbarOrder === 'function') return service.readTaskbarOrder('left');
      var defaults = ['nav_cluster'];
      try {
        var raw = localStorage.getItem('infring-taskbar-order-left');
        var parsed = raw ? JSON.parse(raw) : [];
        if (!Array.isArray(parsed)) return defaults.slice();
        var seen = {};
        var ordered = [];
        for (var i = 0; i < parsed.length; i += 1) {
          var id = String(parsed[i] || '').trim();
          if (!id || seen[id] || defaults.indexOf(id) < 0) continue;
          seen[id] = true;
          ordered.push(id);
        }
        for (var j = 0; j < defaults.length; j += 1) {
          var fallbackId = defaults[j];
          if (seen[fallbackId]) continue;
          seen[fallbackId] = true;
          ordered.push(fallbackId);
        }
        return ordered;
      } catch(_) {
        return defaults.slice();
      }
    })(),
    taskbarReorderRight: (() => {
      var service = infringTaskbarDockService();
      if (service && typeof service.readTaskbarOrder === 'function') return service.readTaskbarOrder('right');
      var defaults = ['connectivity', 'theme', 'notifications', 'search', 'auth'];
      try {
        var raw = localStorage.getItem('infring-taskbar-order-right');
        var parsed = raw ? JSON.parse(raw) : [];
        if (!Array.isArray(parsed)) return defaults.slice();
        var seen = {};
        var ordered = [];
        for (var i = 0; i < parsed.length; i += 1) {
          var id = String(parsed[i] || '').trim();
          if (!id || seen[id] || defaults.indexOf(id) < 0) continue;
          seen[id] = true;
          ordered.push(id);
        }
        for (var j = 0; j < defaults.length; j += 1) {
          var fallbackId = defaults[j];
          if (seen[fallbackId]) continue;
          seen[fallbackId] = true;
          ordered.push(fallbackId);
        }
        return ordered;
      } catch(_) {
        return defaults.slice();
      }
    })(),
    taskbarDragGroup: '',
    taskbarDragItem: '',
    taskbarDragStartOrder: [],
    _taskbarDragHoldTimer: 0,
    _taskbarDragHoldGroup: '',
    _taskbarDragHoldItem: '',
    _taskbarDragArmedGroup: '',
    _taskbarDragArmedItem: '',
    navBackStack: [],
    navForwardStack: [],
    _navCurrentPage: '',
    _navHistoryAction: '',
    _navHistoryCap: 48,

    appsIconBottomRowFill(index) {
      var idx = Number(index);
      if (!Number.isFinite(idx) || idx < 0) idx = 0;
      idx = Math.floor(idx);
      var colors = Array.isArray(this.appsIconBottomRowColors) ? this.appsIconBottomRowColors : [];
      return String(colors[idx] || '#22c55e');
    },

    chatSidebarFlipDurationMs() {
      var raw = Number(this._chatSidebarFlipDurationMs || 240);
      if (!Number.isFinite(raw)) raw = 240;
      return Math.max(120, Math.min(420, Math.round(raw)));
    },

    readChatSidebarSnapshot() {
      var refs = this.$refs || {};
      var nav = refs.sidebarNav;
      if (!nav || typeof nav.querySelectorAll !== 'function') return null;
      var nodes = nav.querySelectorAll('.nav-agent-row[data-agent-id]');
      var rects = {};
      var ids = [];
      for (var i = 0; i < nodes.length; i += 1) {
        var node = nodes[i];
        if (!node) continue;
        var id = String(node.getAttribute('data-agent-id') || '').trim();
        if (!id || Object.prototype.hasOwnProperty.call(rects, id)) continue;
        var rect = node.getBoundingClientRect();
        rects[id] = {
          left: Number(rect.left || 0),
          top: Number(rect.top || 0)
        };
        ids.push(id);
      }
      return {
        order: ids.join('|'),
        scrollTop: Number(nav.scrollTop || 0),
        rects: rects
      };
    },

    animateChatSidebarFromSnapshot(snapshot) {
      if (!snapshot || typeof snapshot !== 'object') return;
      if (typeof requestAnimationFrame !== 'function') return;
      var refs = this.$refs || {};
      var nav = refs.sidebarNav;
      if (!nav || typeof nav.querySelectorAll !== 'function') return;
      var durationMs = this.chatSidebarFlipDurationMs();
      requestAnimationFrame(function() {
        var nodes = nav.querySelectorAll('.nav-agent-row[data-agent-id]');
        for (var i = 0; i < nodes.length; i += 1) {
          var node = nodes[i];
          if (!node || (node.classList && node.classList.contains('dragging'))) continue;
          var id = String(node.getAttribute('data-agent-id') || '').trim();
          if (!id || !Object.prototype.hasOwnProperty.call(snapshot.rects || {}, id)) continue;
          var from = snapshot.rects[id] || {};
          var rect = node.getBoundingClientRect();
          var dx = Number(from.left || 0) - Number(rect.left || 0);
          var dy = Number(from.top || 0) - Number(rect.top || 0);
          if (Math.abs(dx) < 0.5 && Math.abs(dy) < 0.5) continue;
          node.style.transition = 'none';
          node.style.transform = 'translate(' + Math.round(dx) + 'px,' + Math.round(dy) + 'px)';
          void node.offsetHeight;
          node.style.transition = 'transform ' + durationMs + 'ms var(--ease-smooth)';
          node.style.transform = 'translate(0px, 0px)';
          (function(el) {
            window.setTimeout(function() {
              if (!el.classList.contains('dragging')) {
                el.style.transform = '';
              }
              el.style.transition = '';
            }, durationMs + 24);
          })(node);
        }
      });
    },

    maybeAnimateChatSidebarRows() {
      if (String(this.chatSidebarDragAgentId || '').trim()) {
        this._chatSidebarLastSnapshot = this.readChatSidebarSnapshot();
        return;
      }
      if (this._chatSidebarFlipRaf) return;
      var self = this;
      this._chatSidebarFlipRaf = requestAnimationFrame(function() {
        self._chatSidebarFlipRaf = 0;
        var current = self.readChatSidebarSnapshot();
        if (!current) {
          self._chatSidebarLastSnapshot = null;
          return;
        }
        var previous = self._chatSidebarLastSnapshot;
        self._chatSidebarLastSnapshot = current;
        if (!previous) return;
        if (Math.abs(Number(current.scrollTop || 0) - Number(previous.scrollTop || 0)) > 1) return;
        if (String(current.order || '') === String(previous.order || '')) return;
        self.animateChatSidebarFromSnapshot(previous);
      });
    },

    cleanupBottomDockDragGhost() {
      if (this._bottomDockGhostRaf && typeof cancelAnimationFrame === 'function') {
        try { cancelAnimationFrame(this._bottomDockGhostRaf); } catch(_) {}
      }
      if (this._bottomDockGhostCleanupTimer) {
        try { clearTimeout(this._bottomDockGhostCleanupTimer); } catch(_) {}
      }
      this._bottomDockGhostRaf = 0;
      this._bottomDockGhostCleanupTimer = 0;
      this._bottomDockGhostTargetX = 0;
      this._bottomDockGhostTargetY = 0;
      this._bottomDockGhostCurrentX = 0;
      this._bottomDockGhostCurrentY = 0;
      this._bottomDockDragBoundaries = [];
      this._bottomDockLastInsertionIndex = -1;
      this._bottomDockReorderLockUntil = 0;
      var node = this._bottomDockDragGhostEl;
      if (node && node.parentNode) {
        try { node.parentNode.removeChild(node); } catch(_) {}
      }
      this._bottomDockDragGhostEl = null;
      this._bottomDockRevealTargetDuringSettle = false;
    },

    setBottomDockGhostTarget(x, y) {
      var nextX = Number(x || 0);
      var nextY = Number(y || 0);
      var targetX = Number.isFinite(nextX) ? nextX : 0;
      var targetY = Number.isFinite(nextY) ? nextY : 0;
      this._bottomDockGhostTargetX = targetX;
      this._bottomDockGhostTargetY = targetY;
      this._bottomDockGhostCurrentX = targetX;
      this._bottomDockGhostCurrentY = targetY;
      var ghost = this._bottomDockDragGhostEl;
      if (!ghost) return;
      ghost.style.left = Math.round(targetX) + 'px';
      ghost.style.top = Math.round(targetY) + 'px';
    },

    dragbarService() {
      var services = typeof window !== 'undefined' ? window.InfringSharedShellServices : null;
      return services && services.dragbar ? services.dragbar : null;
    },

    taskbarDockService() {
      var services = typeof window !== 'undefined' ? window.InfringSharedShellServices : null;
      return services && services.taskbarDock ? services.taskbarDock : null;
    },

    dragSurfaceMoveDurationMs(rawValue, fallbackMs) {
      var service = this.dragbarService();
      if (service && typeof service.moveDurationMs === 'function') {
        return service.moveDurationMs(rawValue, fallbackMs);
      }
      var fallback = Number(fallbackMs || 280);
      if (!Number.isFinite(fallback)) fallback = 280;
      fallback = Math.max(80, Math.round(fallback));
      var raw = Number(rawValue);
      if (!Number.isFinite(raw)) raw = fallback;
      return Math.max(80, Math.round(raw));
    },

    readBottomDockScale(el) {
      if (!el || typeof window === 'undefined' || typeof window.getComputedStyle !== 'function') {
        return 0.95;
      }
      try {
        var transform = String(window.getComputedStyle(el).transform || '').trim();
        if (!transform || transform === 'none') return 0.95;
        var matrix2d = transform.match(/^matrix\(([^)]+)\)$/);
        if (matrix2d && matrix2d[1]) {
          var parts2d = matrix2d[1].split(',').map(function(v) { return Number(String(v || '').trim()); });
          if (parts2d.length >= 2 && Number.isFinite(parts2d[0]) && Number.isFinite(parts2d[1])) {
            var scale2d = Math.sqrt((parts2d[0] * parts2d[0]) + (parts2d[1] * parts2d[1]));
            if (Number.isFinite(scale2d) && scale2d > 0.01) return scale2d;
          }
        }
        var matrix3d = transform.match(/^matrix3d\(([^)]+)\)$/);
        if (matrix3d && matrix3d[1]) {
          var parts3d = matrix3d[1].split(',').map(function(v) { return Number(String(v || '').trim()); });
          if (parts3d.length >= 1 && Number.isFinite(parts3d[0]) && parts3d[0] > 0.01) return parts3d[0];
        }
      } catch(_) {}
      return 0.95;
    },

    bootProgressClamped(rawPercent) {
      var next = Number(rawPercent);
      if (!Number.isFinite(next)) next = 0;
      return Math.max(0, Math.min(100, Math.round(next)));
    },

    resetBootProgress() {
      this.bootProgressPercent = 6;
      this.bootProgressEvent = 'splash_visible';
      this._bootProgressUpdatedAt = Date.now();
    },

    bootProgressFromBootStage(rawStage) {
      var stage = String(rawStage || '').trim().toLowerCase();
      if (!stage) return 38;
      if (
        stage === 'ready' ||
        stage === 'connected' ||
        stage === 'boot_complete' ||
        stage === 'runtime_ready'
      ) {
        return 70;
      }
      if (stage.indexOf('agent') >= 0) return 66;
      if (stage.indexOf('connect') >= 0) return 28;
      var isRecoveringStage = stage.indexOf('retry') >= 0; if (isRecoveringStage) return 24;
      if (stage.indexOf('unreachable') >= 0 || stage.indexOf('disconnected') >= 0) return 20;
      if (stage.indexOf('start') >= 0 || stage.indexOf('init') >= 0 || stage.indexOf('boot') >= 0) return 16;
      return 42;
    },

    setBootProgressPercent(rawPercent, opts) {
      var options = opts && typeof opts === 'object' ? opts : {};
      var next = this.bootProgressClamped(rawPercent);
      var current = this.bootProgressClamped(this.bootProgressPercent);
      var allowDecrease = options.allowDecrease === true;
      if (!allowDecrease && next < current) next = current;
      if (next === current) return;
      this.bootProgressPercent = next;
      this._bootProgressUpdatedAt = Date.now();
    },

    setBootProgressEvent(eventName, meta) {
      var event = String(eventName || '').trim().toLowerCase();
      if (!event) return;
      var target = 0;
      if (event === 'splash_visible') target = 6;
      else if (event === 'status_requesting') target = 18;
      else if (event === 'status_connected') target = 42;
      else if (event === 'status_retrying') target = 24;
      else if (event === 'agents_refresh_started') target = 56;
      else if (event === 'agents_hydrated') target = 76;
      else if (event === 'selection_applied') target = 90;
      else if (event === 'releasing') target = 97;
      else if (event === 'complete') target = 100;
      else target = 12;

      var stageTarget = this.bootProgressFromBootStage(meta && meta.bootStage);
      if (event === 'status_connected' || event === 'status_retrying') {
        target = Math.max(target, stageTarget);
      }
      if (event === 'complete') {
        this.setBootProgressPercent(100, { allowDecrease: true });
      } else {
        this.setBootProgressPercent(target);
      }
      this.bootProgressEvent = event;
    },
    normalizeConnectionIndicatorState(state) {
      var raw = String(state || '').trim().toLowerCase();
      if (raw === 'connected') return 'connected';
      if (raw === 'disconnected') return 'disconnected';
      return 'connecting';
    },

    queueConnectionIndicatorState(state) {
      var next = this.normalizeConnectionIndicatorState(state);
      var now = Date.now();
      var minIntervalMs = next === 'connecting' ? 1200 : 250;
      if (next !== 'connecting') {
        this.connectionIndicatorState = next;
        this._lastConnectionIndicatorAt = now;
        this._pendingConnectionIndicatorState = '';
        if (this._connectionIndicatorTimer) {
          clearTimeout(this._connectionIndicatorTimer);
          this._connectionIndicatorTimer = null;
        }
        return;
      }
      if (!this._lastConnectionIndicatorAt || (now - this._lastConnectionIndicatorAt) >= minIntervalMs) {
        this.connectionIndicatorState = next;
        this._lastConnectionIndicatorAt = now;
        this._pendingConnectionIndicatorState = '';
        if (this._connectionIndicatorTimer) {
          clearTimeout(this._connectionIndicatorTimer);
          this._connectionIndicatorTimer = null;
        }
        return;
      }
      this._pendingConnectionIndicatorState = next;
      if (this._connectionIndicatorTimer) return;
      var delay = Math.max(0, minIntervalMs - (now - this._lastConnectionIndicatorAt));
      var self = this;
      this._connectionIndicatorTimer = setTimeout(function() {
        self._connectionIndicatorTimer = null;
        var pending = self._pendingConnectionIndicatorState || next;
        self._pendingConnectionIndicatorState = '';
        self.connectionIndicatorState = self.normalizeConnectionIndicatorState(pending);
        self._lastConnectionIndicatorAt = Date.now();
      }, delay);
    },

    _computeScrollHintState(el) {
      if (!el) return { above: false, below: false };
      var scrollHeight = Number(el.scrollHeight || 0);
      var clientHeight = Number(el.clientHeight || 0);
      var scrollTop = Math.max(0, Number(el.scrollTop || 0));
      var maxScroll = Math.max(0, scrollHeight - clientHeight);
      if (maxScroll <= 2) return { above: false, below: false };
      return {
        above: scrollTop > 2,
        below: (maxScroll - scrollTop) > 2
      };
    },

    bottomDockOrder: (() => {
      var service = infringTaskbarDockService();
      if (service && typeof service.readDockOrder === 'function') return service.readDockOrder();
      var defaults = ['chat', 'overview', 'agents', 'scheduler', 'skills', 'runtime', 'settings'];
      try {
        var raw = localStorage.getItem('infring-bottom-dock-order');
        var parsed = raw ? JSON.parse(raw) : [];
        if (!Array.isArray(parsed)) return defaults.slice();
        var seen = {};
        var ordered = [];
        for (var i = 0; i < parsed.length; i++) {
          var id = String(parsed[i] || '').trim();
          if (!id || seen[id] || defaults.indexOf(id) < 0) continue;
          seen[id] = true;
          ordered.push(id);
        }
        for (var j = 0; j < defaults.length; j++) {
          var fallbackId = defaults[j];
          if (seen[fallbackId]) continue;
          seen[fallbackId] = true;
          ordered.push(fallbackId);
        }
        return ordered;
      } catch(_) {
        return defaults.slice();
      }
    })(),
    bottomDockTileConfig: (() => {
      var service = infringTaskbarDockService();
      if (service && typeof service.dockTileConfig === 'function') return service.dockTileConfig();
      return {
      chat: { icon: 'messages', tone: 'message', tooltip: 'Messages', label: 'Messages' },
      overview: { icon: 'home', tone: 'bright', tooltip: 'Home', label: 'Home' },
      agents: { icon: 'agents', tone: 'bright', tooltip: 'Agents', label: 'Agents' },
      scheduler: { icon: 'automation', tone: 'muted', tooltip: 'Automation', label: 'Automation', animation: ['automation-gears', 1200] },
      skills: { icon: 'apps', tone: 'default', tooltip: 'Apps', label: 'Apps' },
      runtime: { icon: 'system', tone: 'bright', tooltip: 'System', label: 'System', animation: ['system-terminal', 2000] },
      settings: { icon: 'settings', tone: 'muted', tooltip: 'Settings', label: 'Settings', animation: ['spin', 4000] }
      };
    })(),
    appsIconBottomRowColors: (() => {
      var palette = ['#14b8a6', '#06b6d4', '#38bdf8', '#22c55e', '#f59e0b', '#ef4444', '#a855f7', '#f43f5e', '#64748b'];
      var out = [];
      for (var i = 0; i < 3; i += 1) {
        out.push(palette[Math.floor(Math.random() * palette.length)]);
      }
      return out;
    })(),
    bottomDockDragId: '',
    bottomDockDragStartOrder: [],
    bottomDockDragCommitted: false,
    bottomDockHoverId: '',
    bottomDockHoverWeightById: {},
    bottomDockPointerX: 0,
    bottomDockPointerY: 0,
    bottomDockPreviewText: '',
    bottomDockPreviewMorphFromText: '',
    bottomDockPreviewHoverKey: '',
    bottomDockPreviewX: 0,
    bottomDockPreviewY: 0,
    bottomDockPreviewWidth: 0,
    bottomDockPreviewVisible: false,
    bottomDockPreviewLabelMorphing: false,
    bottomDockPreviewLabelFxReady: true,
    _bottomDockPreviewHideTimer: 0,
    _bottomDockPreviewReflowRaf: 0,
    _bottomDockPreviewReflowFrames: 0,
    _bottomDockPreviewWidthRaf: 0,
    _bottomDockPreviewLabelFxRaf: 0,
    _bottomDockPreviewLabelFxTimer: 0,
    _bottomDockPreviewLabelMorphTimer: 0,
    bottomDockClickAnimId: '',
    _bottomDockDragGhostEl: null,
    _bottomDockClickAnimTimer: 0,
    _bottomDockClickAnimDurationMs: 980,
    _bottomDockSuppressClickUntil: 0,
    _bottomDockPointerActive: false,
    _bottomDockPointerMoved: false,
    _bottomDockPointerCandidateId: '',
    _bottomDockPointerStartX: 0,
    _bottomDockPointerStartY: 0,
    _bottomDockPointerLastX: 0,
    _bottomDockPointerLastY: 0,
    _bottomDockPointerGrabOffsetX: 16,
    _bottomDockPointerGrabOffsetY: 16,
    _bottomDockDragGhostWidth: 32,
    _bottomDockDragGhostHeight: 32,
    _bottomDockPointerMoveHandler: null,
    _bottomDockPointerUpHandler: null,
    _bottomDockGhostTargetX: 0,
    _bottomDockGhostTargetY: 0,
    _bottomDockGhostCurrentX: 0,
    _bottomDockGhostCurrentY: 0,
    _bottomDockGhostRaf: 0,
    _bottomDockGhostCleanupTimer: 0,
    _bottomDockMoveDurationMs: 360,
    _bottomDockExpandedScale: 1.54,
    bottomDockRotationDeg: Number.NaN,
    _bottomDockRevealTargetDuringSettle: false,
    _bottomDockDragBoundaries: [],
    _bottomDockLastInsertionIndex: -1,
    _bottomDockReorderLockUntil: 0,
    bottomDockPlacementId: (() => {
      var service = infringTaskbarDockService();
      if (service && typeof service.readLayoutConfig === 'function') return service.readLayoutConfig().dock.placement;
      try {
        var raw = String(localStorage.getItem('infring-bottom-dock-placement') || '').trim().toLowerCase();
        var allowed = {
          left: true,
          center: true,
          right: true,
          'top-left': true,
          'top-center': true,
          'top-right': true,
          'left-top': true,
          'left-bottom': true,
          'right-top': true,
          'right-bottom': true
        };
        if (allowed[raw]) return raw;
        if (raw === 'left-center') return 'left-top';
        if (raw === 'right-center') return 'right-top';
      } catch(_) {}
      return 'center';
    })(),
    bottomDockSnapPoints: [
      { id: 'left', x: 0.16, y: 0.995, side: 'bottom' },
      { id: 'center', x: 0.50, y: 0.995, side: 'bottom' },
      { id: 'right', x: 0.84, y: 0.995, side: 'bottom' },
      { id: 'top-left', x: 0.16, y: 0.005, side: 'top' },
      { id: 'top-center', x: 0.50, y: 0.005, side: 'top' },
      { id: 'top-right', x: 0.84, y: 0.005, side: 'top' },
      { id: 'left-top', x: 0.005, y: (1 / 3), side: 'left' },
      { id: 'left-bottom', x: 0.005, y: (2 / 3), side: 'left' },
      { id: 'right-top', x: 0.995, y: (1 / 3), side: 'right' },
      { id: 'right-bottom', x: 0.995, y: (2 / 3), side: 'right' }
    ],
    bottomDockContainerDragActive: false,
    bottomDockContainerSettling: false,
    bottomDockContainerDragX: 0,
    bottomDockContainerDragY: 0,
    bottomDockContainerWallLock: (() => {
      var service = infringTaskbarDockService();
      if (service && typeof service.readLayoutConfig === 'function') return service.readLayoutConfig().dock.wallLock;
      try {
        var raw = String(
          localStorage.getItem('infring-bottom-dock-wall-lock')
          || localStorage.getItem('infring-bottom-dock-smash-wall')
          || ''
        ).trim().toLowerCase();
        if (raw === 'left' || raw === 'right' || raw === 'top' || raw === 'bottom') return raw;
      } catch(_) {}
      return '';
    })(),
    _bottomDockContainerDragWallLock: '',
    _bottomDockContainerPointerActive: false,
    _bottomDockContainerPointerMoved: false,
    _bottomDockContainerPointerStartX: 0,
    _bottomDockContainerPointerStartY: 0,
    _bottomDockContainerPointerLastX: 0,
    _bottomDockContainerPointerLastY: 0,
    _bottomDockContainerOriginX: 0,
    _bottomDockContainerOriginY: 0,
    _bottomDockContainerPointerMoveHandler: null,
    _bottomDockContainerPointerUpHandler: null,
    _bottomDockContainerSettleTimer: 0,

    bottomDockMoveDurationMs() {
      return this.dragSurfaceMoveDurationMs(this._bottomDockMoveDurationMs, 360);
    },

    bottomDockExpandedScale() {
      var raw = Number(this._bottomDockExpandedScale || 1.54);
      if (!Number.isFinite(raw) || raw <= 1) raw = 1.54;
      return raw;
    },

    bottomDockReadViewportSize() {
      var width = 0;
      var height = 0;
      try {
        width = Number(window && window.innerWidth || 0);
        height = Number(window && window.innerHeight || 0);
      } catch(_) {
        width = 0;
        height = 0;
      }
      if (!Number.isFinite(width) || width <= 0) {
        width = Number(document && document.documentElement && document.documentElement.clientWidth || 1440);
      }
      if (!Number.isFinite(height) || height <= 0) {
        height = Number(document && document.documentElement && document.documentElement.clientHeight || 900);
      }
      if (!Number.isFinite(width) || width <= 0) width = 1440;
      if (!Number.isFinite(height) || height <= 0) height = 900;
      return { width: width, height: height };
    },

    bottomDockReadBaseSize() {
      var width = 0;
      var height = 0;
      try {
        var node = document && typeof document.querySelector === 'function'
          ? document.querySelector('.bottom-dock')
          : null;
        if (node) {
          width = Number(node.offsetWidth || 0);
          height = Number(node.offsetHeight || 0);
        }
      } catch(_) {
        width = 0;
        height = 0;
      }
      if (!Number.isFinite(width) || width <= 0) width = 420;
      if (!Number.isFinite(height) || height <= 0) height = 54;
      return { width: width, height: height };
    },

    bottomDockNormalizeSide(side) {
      var key = String(side || '').trim().toLowerCase();
      if (key === 'top' || key === 'left' || key === 'right') return key;
      return 'bottom';
    },

    bottomDockIsVerticalSide(side) {
      var key = this.bottomDockNormalizeSide(side);
      return key === 'left' || key === 'right';
    },

    bottomDockRotationDegForSide(side) {
      var key = this.bottomDockNormalizeSide(side);
      if (key === 'left') return -90;
      if (key === 'right') return 90;
      return 0;
    },

    bottomDockIconRotationDegForSide(side) {
      var key = this.bottomDockNormalizeSide(side);
      if (key === 'left') return 90;
      if (key === 'right') return -90;
      return 0;
    },

    bottomDockUpDegForSide(side) {
      var key = this.bottomDockNormalizeSide(side);
      if (key === 'left' || key === 'right' || key === 'top' || key === 'bottom') return 0;
      return 0;
    },

    bottomDockOrientation(sideHint) {
      var side = this.bottomDockNormalizeSide(sideHint || this.bottomDockActiveSide());
      var horizontal = !this.bottomDockIsVerticalSide(side);
      var axis = horizontal ? 'x' : 'y';
      return {
        side: side,
        horizontal: horizontal,
        axis: axis,
        primarySign: 1,
        upDeg: Number(this.bottomDockUpDegForSide(side) || 0)
      };
    },

    bottomDockOppositeSide(sideHint) {
      var side = this.bottomDockNormalizeSide(sideHint);
      if (side === 'left') return 'right';
      if (side === 'right') return 'left';
      if (side === 'top') return 'bottom';
      return 'top';
    },

    bottomDockWallSide() {
      return this.bottomDockNormalizeSide(this.bottomDockActiveSide());
    },

    bottomDockOpenSide() {
      return this.bottomDockOppositeSide(this.bottomDockWallSide());
    },

    bottomDockRotationDegResolved(sideHint) {
      var side = this.bottomDockNormalizeSide(sideHint || this.bottomDockActiveSide());
      var rotationDeg = Number(this.bottomDockRotationDeg);
      if (!Number.isFinite(rotationDeg)) {
        rotationDeg = Number(this.bottomDockRotationDegForSide(side));
      }
      return Number(this.bottomDockNormalizeRotationDeg(rotationDeg) || 0);
    },

    bottomDockScreenDeltaToLocal(dx, dy, sideHint) {
      var screenDx = Number(dx || 0);
      var screenDy = Number(dy || 0);
      var rotationDeg = this.bottomDockRotationDegResolved(sideHint);
      var theta = (rotationDeg * Math.PI) / 180;
      var cos = Math.cos(theta);
      var sin = Math.sin(theta);
      return {
        x: (screenDx * cos) + (screenDy * sin),
        y: (-screenDx * sin) + (screenDy * cos)
      };
    },

    bottomDockCanonicalRotationCandidatesForSide(side) {
      var key = this.bottomDockNormalizeSide(side);
      if (key === 'left' || key === 'right') return [90, -90];
      return [0];
    },

    bottomDockNormalizeRotationDeg(value) {
      var raw = Number(value);
      var canonical = [-90, 0, 90];
      if (!Number.isFinite(raw)) return 0;
      var best = canonical[0];
      var bestDist = Number.POSITIVE_INFINITY;
      for (var i = 0; i < canonical.length; i += 1) {
        var candidate = canonical[i];
        var dist = Math.abs(raw - candidate);
        if (dist < bestDist) {
          bestDist = dist;
          best = candidate;
        }
      }
      return best;
    },

    bottomDockResolveShortestRotationDeg(currentDeg, targetDeg) {
      var current = Number(currentDeg);
      var target = Number(targetDeg);
      if (!Number.isFinite(target)) target = 0;
      if (!Number.isFinite(current)) return target;
      var best = target;
      var bestDelta = Number.POSITIVE_INFINITY;
      for (var k = -2; k <= 2; k += 1) {
        var candidate = target + (k * 360);
        var delta = Math.abs(candidate - current);
        if (delta < bestDelta) {
          bestDelta = delta;
          best = candidate;
        }
      }
      return best;
    },

    bottomDockPreferredRotationDirectionForAnchor(anchorX, anchorY) {
      var view = this.bottomDockReadViewportSize();
      var x = Number(anchorX);
      var y = Number(anchorY);
      if (!Number.isFinite(x)) x = Number(view.width || 0) * 0.5;
      if (!Number.isFinite(y)) y = Number(view.height || 0) * 0.5;
      var left = x < (Number(view.width || 0) * 0.5);
      var top = y < (Number(view.height || 0) * 0.5);
      // TL + BR => counterclockwise. TR + BL => clockwise.
      return (left === top) ? 'ccw' : 'cw';
    },

    bottomDockResolveDirectionalRotationDeg(currentDeg, targetDeg, direction) {
      var current = Number(currentDeg);
      var target = Number(targetDeg);
      var dir = String(direction || '').trim().toLowerCase();
      if (!Number.isFinite(target)) target = 0;
      if (!Number.isFinite(current)) return target;
      if (dir !== 'cw' && dir !== 'ccw') {
        return this.bottomDockResolveShortestRotationDeg(current, target);
      }
      var best = null;
      var bestAbs = Number.POSITIVE_INFINITY;
      for (var k = -2; k <= 2; k += 1) {
        var candidate = target + (k * 360);
        var delta = candidate - current;
        if (dir === 'cw' && delta < 0) continue;
        if (dir === 'ccw' && delta > 0) continue;
        var absDelta = Math.abs(delta);
        if (absDelta < bestAbs) {
          bestAbs = absDelta;
          best = candidate;
        }
      }
      if (best === null) {
        return this.bottomDockResolveShortestRotationDeg(current, target);
      }
      return best;
    },

    bottomDockResolveRotationForSide(side, anchorX, anchorY) {
      var current = this.bottomDockNormalizeRotationDeg(this.bottomDockRotationDeg);
      var dir = this.bottomDockPreferredRotationDirectionForAnchor(anchorX, anchorY);
      var candidates = this.bottomDockCanonicalRotationCandidatesForSide(side);
      if (!Array.isArray(candidates) || !candidates.length) return current;
      var best = Number(candidates[0] || 0);
      var bestScore = Number.POSITIVE_INFINITY;
      var bestDeltaAbs = Number.POSITIVE_INFINITY;
      for (var i = 0; i < candidates.length; i += 1) {
        var target = Number(candidates[i] || 0);
        var delta = target - current;
        var deltaAbs = Math.abs(delta);
        var directionPenalty = 0;
        if (dir === 'cw' && delta < 0) directionPenalty = 0.35;
        if (dir === 'ccw' && delta > 0) directionPenalty = 0.35;
        var score = deltaAbs + directionPenalty;
        if (score < bestScore || (score === bestScore && deltaAbs < bestDeltaAbs)) {
          best = target;
          bestScore = score;
          bestDeltaAbs = deltaAbs;
        }
      }
      var chosenDelta = best - current;
      if (Math.abs(chosenDelta) > 90) {
        if (dir === 'cw') return current + 90;
        if (dir === 'ccw') return current - 90;
        return current + (chosenDelta > 0 ? 90 : -90);
      }
      return best;
    },

    bottomDockSnapDefinitions() {
      var source = Array.isArray(this.bottomDockSnapPoints) ? this.bottomDockSnapPoints : [];
      var out = [];
      var seen = {};
      for (var i = 0; i < source.length; i += 1) {
        var row = source[i];
        if (!row || typeof row !== 'object') continue;
        var id = String(row.id || '').trim().toLowerCase();
        if (!id || seen[id]) continue;
        var nx = Number(row.x);
        var ny = Number(row.y);
        var side = this.bottomDockNormalizeSide(row.side);
        if (!Number.isFinite(nx)) nx = 0.5;
        if (!Number.isFinite(ny)) ny = 0.995;
        nx = Math.max(0, Math.min(1, nx));
        ny = Math.max(0, Math.min(1, ny));
        seen[id] = true;
        out.push({ id: id, x: nx, y: ny, side: side });
      }
      if (!out.length) {
        out.push({ id: 'center', x: 0.5, y: 0.995, side: 'bottom' });
      }
      return out;
    },

    bottomDockSnapDefinitionById(id) {
      var key = String(id || '').trim().toLowerCase();
      var defs = this.bottomDockSnapDefinitions();
      if (!defs.length) return null;
      for (var i = 0; i < defs.length; i += 1) {
        if (defs[i] && defs[i].id === key) return defs[i];
      }
      for (var j = 0; j < defs.length; j += 1) {
        if (defs[j] && defs[j].id === 'center') return defs[j];
      }
      return defs[0] || null;
    },

    bottomDockSideForSnapId(id) {
      var snap = this.bottomDockSnapDefinitionById(id);
      return this.bottomDockNormalizeSide(snap && snap.side || 'bottom');
    },

    bottomDockActiveSnapId() {
      if (this.bottomDockContainerDragActive) {
        var anchor = this.bottomDockClampDragAnchor(this.bottomDockContainerDragX, this.bottomDockContainerDragY);
        return this.bottomDockNearestSnapId(anchor.x, anchor.y);
      }
      var snap = this.bottomDockSnapDefinitionById(this.bottomDockPlacementId);
      return String(snap && snap.id || 'center');
    },

    bottomDockActiveSide() {
      return this.bottomDockSideForSnapId(this.bottomDockActiveSnapId());
    },

    bottomDockWallLockNormalized() {
      return this.dragSurfaceNormalizeWall(this.bottomDockContainerWallLock);
    },

    bottomDockTaskbarContained() {
      var service = this.taskbarDockService();
      if (service && typeof service.dockTaskbarContained === 'function') {
        return service.dockTaskbarContained(
          this.bottomDockWallLockNormalized(),
          this.taskbarDockEdge,
          this.taskbarDockDragActive,
          this._taskbarDockDraggingContainedBottomDock
        );
      }
      var wall = this.bottomDockWallLockNormalized();
      if (wall !== 'top' && wall !== 'bottom') return false;
      if (this.taskbarDockDragActive && String(this._taskbarDockDraggingContainedBottomDock || '') === wall) return true;
      return wall === this.taskbarDockEdgeNormalized(this.taskbarDockEdge);
    },

    bottomDockHoverExpansionDisabled() {
      return this.bottomDockTaskbarContained();
    },

    bottomDockTaskbarContainedAnchorX(sideHint) {
      var side = this.bottomDockNormalizeSide(sideHint || this.bottomDockActiveSide());
      var view = this.bottomDockReadViewportSize();
      var size = this.bottomDockVisualSizeForSide(side);
      var dockWidth = Math.max(1, Number(size && size.width || 1));
      var left = 16;
      try {
        var textMenu = document.querySelector('.global-taskbar .taskbar-text-menus');
        var rect = textMenu && typeof textMenu.getBoundingClientRect === 'function' ? textMenu.getBoundingClientRect() : null;
        if (rect && Number.isFinite(Number(rect.right))) left = Number(rect.right) + 8;
      } catch(_) {}
      var service = this.taskbarDockService();
      if (service && typeof service.dockTaskbarContainedAnchorX === 'function') {
        return service.dockTaskbarContainedAnchorX({
          side: side,
          viewportWidth: Number(view.width || 0),
          dockWidth: dockWidth,
          leftAnchor: left
        });
      }
      var minX = dockWidth / 2;
      var maxX = Math.max(minX, Number(view.width || 0) - minX - 10);
      return Math.max(minX, Math.min(maxX, left + (dockWidth / 2)));
    },

    bottomDockTaskbarContainedMetrics() {
      var edge = this.taskbarDockEdgeNormalized(this.taskbarDockEdge);
      var height = 32;
      var centerY = edge === 'bottom' ? this.taskbarReadViewportHeight() - 23 : 23;
      try {
        var group = document.querySelector('.global-taskbar .taskbar-visual-group-left');
        var rect = group && typeof group.getBoundingClientRect === 'function' ? group.getBoundingClientRect() : null;
        if (rect && Number.isFinite(Number(rect.height)) && Number(rect.height) > 0) {
          height = Number(rect.height);
          centerY = Number(rect.top || 0) + (height / 2);
        }
      } catch(_) {}
      if (this.taskbarDockDragActive && String(this._taskbarDockDraggingContainedBottomDock || '')) {
        centerY = this.taskbarClampDragY(this.taskbarDockDragY) + (this.taskbarReadHeight() / 2);
      }
      var service = this.taskbarDockService();
      if (service && typeof service.dockTaskbarContainedMetrics === 'function') {
        return service.dockTaskbarContainedMetrics({
          edge: edge,
          viewportHeight: this.taskbarReadViewportHeight(),
          fallbackHeight: 32,
          groupHeight: height,
          groupTop: centerY - (height / 2),
          dragging: this.taskbarDockDragActive && String(this._taskbarDockDraggingContainedBottomDock || ''),
          dragY: this.taskbarClampDragY(this.taskbarDockDragY),
          taskbarHeight: this.taskbarReadHeight()
        });
      }
      return { height: height, centerY: centerY };
    },

    bottomDockSetWallLock(wallRaw) {
      var wall = this.dragSurfaceNormalizeWall(wallRaw);
      this.bottomDockContainerWallLock = wall;
      try {
        if (wall) localStorage.setItem('infring-bottom-dock-wall-lock', wall);
        else localStorage.removeItem('infring-bottom-dock-wall-lock');
        localStorage.removeItem('infring-bottom-dock-smash-wall');
        infringUpdateShellLayoutConfig(function(config) { config.dock.wallLock = wall; });
      } catch(_) {}
      return wall;
    },

    bottomDockBoundsScaleForSide(sideHint) {
      var side = this.bottomDockNormalizeSide(sideHint || this.bottomDockActiveSide());
      if (this.bottomDockTaskbarContained()) return 1;
      if (side === 'left' || side === 'right') return 1;
      var expandedScale = this.bottomDockExpandedScale();
      var baseScale = 0.95;
      var dragging = !!this.bottomDockContainerDragActive
        || !!this.bottomDockContainerSettling
        || !!String(this.bottomDockDragId || '').trim();
      var hovering = !!String(this.bottomDockHoverId || '').trim();
      if (this.bottomDockHoverExpansionDisabled()) hovering = false;
      if (dragging || hovering) baseScale = expandedScale;
      if (!Number.isFinite(baseScale) || baseScale <= 0.01) baseScale = 0.95;
      return baseScale;
    },

    bottomDockVisualSizeForSide(sideHint) {
      var side = this.bottomDockNormalizeSide(sideHint || this.bottomDockActiveSide());
      var dock = this.bottomDockReadBaseSize();
      var scale = this.bottomDockBoundsScaleForSide(side);
      var baseWidth = Math.max(20, Number(dock.width || 0) * scale);
      var baseHeight = Math.max(20, Number(dock.height || 0) * scale);
      var visualWidth = this.bottomDockIsVerticalSide(side) ? baseHeight : baseWidth;
      var visualHeight = this.bottomDockIsVerticalSide(side) ? baseWidth : baseHeight;
      return { side: side, width: visualWidth, height: visualHeight };
    },

    bottomDockHardBoundsForSide(sideHint) {
      var size = this.bottomDockVisualSizeForSide(sideHint);
      var view = this.bottomDockReadViewportSize();
      var width = Number(size && size.width || 0);
      var height = Number(size && size.height || 0);
      if (!Number.isFinite(width) || width < 1) width = 1;
      if (!Number.isFinite(height) || height < 1) height = 1;
      var viewportWidth = Number(view && view.width || 0);
      var viewportHeight = Number(view && view.height || 0);
      if (!Number.isFinite(viewportWidth) || viewportWidth <= 0) viewportWidth = 1440;
      if (!Number.isFinite(viewportHeight) || viewportHeight <= 0) viewportHeight = 900;
      return {
        minLeft: 0,
        maxLeft: Math.max(0, viewportWidth - width),
        minTop: 0,
        maxTop: Math.max(0, viewportHeight - height)
      };
    },

    bottomDockTopLeftFromAnchor(anchorX, anchorY, sideHint) {
      var size = this.bottomDockVisualSizeForSide(sideHint);
      var x = Number(anchorX);
      var y = Number(anchorY);
      if (!Number.isFinite(x)) x = Number(this.bottomDockReadViewportSize().width || 0) * 0.5;
      if (!Number.isFinite(y)) y = Number(this.bottomDockReadViewportSize().height || 0) * 0.5;
      var side = this.bottomDockNormalizeSide(size && size.side);
      var top = y - (Number(size.height || 0) / 2);
      if (side === 'top') top = y;
      else if (side === 'bottom') top = y - Number(size.height || 0);
      return {
        left: x - (Number(size.width || 0) / 2),
        top: top,
        side: side
      };
    },

    bottomDockAnchorFromTopLeft(leftRaw, topRaw, sideHint) {
      var size = this.bottomDockVisualSizeForSide(sideHint);
      var left = Number(leftRaw);
      var top = Number(topRaw);
      if (!Number.isFinite(left)) left = Number(size.width || 0) / -2;
      if (!Number.isFinite(top)) top = Number(size.height || 0) / -2;
      var side = this.bottomDockNormalizeSide(size && size.side);
      var y = top + (Number(size.height || 0) / 2);
      if (side === 'top') y = top;
      else if (side === 'bottom') y = top + Number(size.height || 0);
      return {
        x: left + (Number(size.width || 0) / 2),
        y: y,
        side: side
      };
    },

    bottomDockLocalWallForRotation(wallRaw, rotationDegRaw) {
      var wall = this.dragSurfaceNormalizeWall(wallRaw);
      if (!wall) return '';
      var rotationDeg = Number(rotationDegRaw);
      if (!Number.isFinite(rotationDeg)) rotationDeg = 0;
      var theta = (this.bottomDockNormalizeRotationDeg(rotationDeg) * Math.PI) / 180;
      var vx = 0;
      var vy = 0;
      if (wall === 'left') vx = -1;
      else if (wall === 'right') vx = 1;
      else if (wall === 'top') vy = -1;
      else vy = 1;
      var localX = (vx * Math.cos(theta)) + (vy * Math.sin(theta));
      var localY = (-vx * Math.sin(theta)) + (vy * Math.cos(theta));
      if (Math.abs(localX) >= Math.abs(localY)) {
        return localX >= 0 ? 'right' : 'left';
      }
      return localY >= 0 ? 'bottom' : 'top';
    },

    bottomDockLockRadiusCssVars(wallRaw, rotationDegRaw) {
      var wall = this.dragSurfaceNormalizeWall(wallRaw);
      if (!wall) return '';
      var localWall = this.bottomDockLocalWallForRotation(wall, rotationDegRaw);
      return '--bottom-dock-radius-override:' + this.dragSurfaceRadiusByWall(localWall) + ';';
    },

    bottomDockClampDragAnchor(anchorX, anchorY) {
      var view = this.bottomDockReadViewportSize();
      var margin = 8;
      var minX = margin;
      var maxX = Number(view.width || 0) - margin;
      var minY = margin;
      var maxY = Number(view.height || 0) - margin;
      var x = Number(anchorX);
      var y = Number(anchorY);
      if (!Number.isFinite(x)) x = Number(view.width || 0) * 0.5;
      if (!Number.isFinite(y)) y = Number(view.height || 0) * 0.5;
      x = Math.max(minX, Math.min(maxX, x));
      y = Math.max(minY, Math.min(maxY, y));
      return { x: x, y: y };
    },

    bottomDockClampAnchor(anchorX, anchorY, sideOverride) {
      void sideOverride;
      return this.bottomDockClampDragAnchor(anchorX, anchorY);
    },

    bottomDockAnchorForSnapId(id) {
      var snap = this.bottomDockSnapDefinitionById(id);
      var view = this.bottomDockReadViewportSize();
      var x = Number(view.width || 0) * Number(snap && snap.x || 0.5);
      var y = Number(view.height || 0) * Number(snap && snap.y || 0.995);
      var side = this.bottomDockNormalizeSide(snap && snap.side || 'bottom');
      return this.bottomDockClampAnchor(x, y, side);
    },

    bottomDockNearestSnapId(anchorX, anchorY) {
      var defs = this.bottomDockSnapDefinitions();
      if (!defs.length) return 'center';
      var anchor = this.bottomDockClampDragAnchor(anchorX, anchorY);
      var bestId = defs[0].id;
      var bestDist = Number.POSITIVE_INFINITY;
      for (var i = 0; i < defs.length; i += 1) {
        var row = defs[i];
        if (!row) continue;
        var snapAnchor = this.bottomDockAnchorForSnapId(row.id);
        var dx = Number(anchor.x || 0) - Number(snapAnchor.x || 0);
        var dy = Number(anchor.y || 0) - Number(snapAnchor.y || 0);
        var dist = (dx * dx) + (dy * dy);
        if (!Number.isFinite(dist)) continue;
        if (dist >= bestDist) continue;
        bestDist = dist;
        bestId = row.id;
      }
      return String(bestId || 'center');
    },

    persistBottomDockPlacement() {
      var key = String(this.bottomDockPlacementId || '').trim().toLowerCase();
      var snap = this.bottomDockSnapDefinitionById(key);
      this.bottomDockPlacementId = String(snap && snap.id || 'center');
      try {
        localStorage.setItem('infring-bottom-dock-placement', this.bottomDockPlacementId);
        infringUpdateShellLayoutConfig(function(config) { config.dock.placement = this.bottomDockPlacementId; }.bind(this));
      } catch(_) {}
    },

    syncDragWallCapHostNode(node, wallRaw) {
      if (!node || !node.classList) return;
      var wall = this.dragSurfaceNormalizeWall(wallRaw);
      node.classList.add('drag-wall-cap-host');
      node.classList.remove('wall-lock-left', 'wall-lock-right', 'wall-lock-top', 'wall-lock-bottom');
      if (wall) node.classList.add('wall-lock-' + wall);
      var capA = null;
      var capB = null;
      var kids = node.children || [];
      for (var i = 0; i < kids.length; i += 1) {
        var child = kids[i];
        if (!child || !child.classList) continue;
        if (child.classList.contains('drag-bar-wall-cap--a')) capA = child;
        if (child.classList.contains('drag-bar-wall-cap--b')) capB = child;
      }
      if (!capA) {
        capA = document.createElement('span');
        capA.className = 'drag-bar-wall-cap drag-bar-wall-cap--a';
        capA.setAttribute('aria-hidden', 'true');
        node.appendChild(capA);
      }
      if (!capB) {
        capB = document.createElement('span');
        capB.className = 'drag-bar-wall-cap drag-bar-wall-cap--b';
        capB.setAttribute('aria-hidden', 'true');
        node.appendChild(capB);
      }
    },

    syncDragWallCaps() {
      if (typeof document === 'undefined') return;
      var sidebarNode = null;
      var chatMapSurfaceNode = null;
      var dockNode = null;
      try { sidebarNode = document.querySelector('.sidebar.drag-bar'); } catch(_) {}
      try { chatMapSurfaceNode = document.querySelector('.chat-map .chat-map-surface.drag-bar'); } catch(_) {}
      try { dockNode = document.querySelector('.bottom-dock.drag-bar'); } catch(_) {}
      this.syncDragWallCapHostNode(sidebarNode, this.page === 'chat' ? this.chatSidebarWallLockNormalized() : '');
      this.syncDragWallCapHostNode(chatMapSurfaceNode, this.chatMapPlacementEnabled() ? this.chatMapWallLockNormalized() : '');
      this.syncDragWallCapHostNode(dockNode, this.bottomDockTaskbarContained() ? '' : this.bottomDockWallLockNormalized());
    },

    bottomDockContainerStyle() {
      this.syncDragWallCaps();
      var lockWall = this.bottomDockWallLockNormalized();
      var taskbarContained = this.bottomDockTaskbarContained();
      var activeSnapId = this.bottomDockContainerDragActive
        ? this.bottomDockNearestSnapId(this.bottomDockContainerDragX, this.bottomDockContainerDragY)
        : this.bottomDockPlacementId;
      var side = this.bottomDockSideForSnapId(activeSnapId);
      if (lockWall) side = lockWall;
      var anchor = this.bottomDockContainerDragActive
        ? this.bottomDockClampDragAnchor(this.bottomDockContainerDragX, this.bottomDockContainerDragY)
        : this.bottomDockAnchorForSnapId(this.bottomDockPlacementId);
      if (lockWall) {
        var topLeft = this.bottomDockTopLeftFromAnchor(anchor.x, anchor.y, side);
        var hardBounds = this.bottomDockHardBoundsForSide(side);
        var snapped = this.dragSurfaceApplyWallLock(hardBounds, topLeft.left, topLeft.top, lockWall);
        var lockedAnchor = this.bottomDockAnchorFromTopLeft(snapped.left, snapped.top, side);
        anchor = { x: Number(lockedAnchor.x || 0), y: Number(lockedAnchor.y || 0) };
      }
      if (taskbarContained) {
        var taskbarContainedMetrics = this.bottomDockTaskbarContainedMetrics();
        anchor.x = this.bottomDockTaskbarContainedAnchorX(side);
        anchor.y = Number(taskbarContainedMetrics.centerY || anchor.y || 0);
      }
      var rotationDeg = Number(this.bottomDockRotationDeg);
      if (!Number.isFinite(rotationDeg)) {
        rotationDeg = this.bottomDockResolveRotationForSide(side, anchor.x, anchor.y);
        this.bottomDockRotationDeg = rotationDeg;
      }
      var upDeg = Number(this.bottomDockUpDegForSide(side) || 0);
      var tileRotationDeg = upDeg - Number(rotationDeg || 0);
      var iconRotationDeg = 0;
      var carriedByTaskbar = taskbarContained && this.taskbarDockDragActive;
      var durationMs = (this.bottomDockContainerDragActive || carriedByTaskbar) ? 0 : this.bottomDockMoveDurationMs();
      var localLockWall = lockWall && !taskbarContained ? this.bottomDockLocalWallForRotation(lockWall, rotationDeg) : '';
      var lockCss = this.dragSurfaceLockVisualCssVars('bottom-dock', localLockWall, {
        transformMs: this._dragSurfaceLockTransformMs
      });
      return (
        lockCss +
        '--bottom-dock-anchor-x:' + Math.round(Number(anchor.x || 0)) + 'px;' +
        '--bottom-dock-anchor-y:' + Math.round(Number(anchor.y || 0)) + 'px;' +
        '--bottom-dock-taskbar-contained-height:' + Math.round(Number((taskbarContainedMetrics && taskbarContainedMetrics.height) || 32)) + 'px;' +
        '--bottom-dock-taskbar-contained-tile-size:' + Math.max(18, Math.round(Number((taskbarContainedMetrics && taskbarContainedMetrics.height) || 32) - 10)) + 'px;' +
        '--bottom-dock-position-transition:' + Math.max(0, Math.round(Number(durationMs || 0))) + 'ms;' +
        '--bottom-dock-up-deg:' + Math.round(Number(upDeg || 0)) + 'deg;' +
        '--bottom-dock-rotation-deg:' + Math.round(Number(rotationDeg || 0)) + 'deg;' +
        '--bottom-dock-tile-rotation-deg:' + Math.round(Number(tileRotationDeg || 0)) + 'deg;' +
        '--bottom-dock-icon-rotation-deg:' + Math.round(Number(iconRotationDeg || 0)) + 'deg;'
      );
    },

    bindBottomDockContainerPointerListeners() {
      if (this._bottomDockContainerPointerMoveHandler || this._bottomDockContainerPointerUpHandler) return;
      var self = this;
      this._bottomDockContainerPointerMoveHandler = function(ev) { self.handleBottomDockContainerPointerMove(ev); };
      this._bottomDockContainerPointerUpHandler = function(ev) { self.endBottomDockContainerPointerDrag(ev); };
      window.addEventListener('pointermove', this._bottomDockContainerPointerMoveHandler, true);
      window.addEventListener('pointerup', this._bottomDockContainerPointerUpHandler, true);
      window.addEventListener('pointercancel', this._bottomDockContainerPointerUpHandler, true);
      window.addEventListener('mousemove', this._bottomDockContainerPointerMoveHandler, true);
      window.addEventListener('mouseup', this._bottomDockContainerPointerUpHandler, true);
    },

    unbindBottomDockContainerPointerListeners() {
      if (this._bottomDockContainerPointerMoveHandler) {
        try { window.removeEventListener('pointermove', this._bottomDockContainerPointerMoveHandler, true); } catch(_) {}
        try { window.removeEventListener('mousemove', this._bottomDockContainerPointerMoveHandler, true); } catch(_) {}
      }
      if (this._bottomDockContainerPointerUpHandler) {
        try { window.removeEventListener('pointerup', this._bottomDockContainerPointerUpHandler, true); } catch(_) {}
        try { window.removeEventListener('pointercancel', this._bottomDockContainerPointerUpHandler, true); } catch(_) {}
        try { window.removeEventListener('mouseup', this._bottomDockContainerPointerUpHandler, true); } catch(_) {}
      }
      this._bottomDockContainerPointerMoveHandler = null;
      this._bottomDockContainerPointerUpHandler = null;
    },

    startBottomDockContainerPointerDrag(ev) {
      if (!ev || Number(ev.button) !== 0) return;
      if (String(this.bottomDockDragId || '').trim()) return;
      var target = ev && ev.target ? ev.target : null;
      if (target && typeof target.closest === 'function') {
        var tileNode = target.closest('.bottom-dock-btn[data-dock-id]');
        if (tileNode) return;
      }
      if (this._bottomDockContainerSettleTimer) {
        try { clearTimeout(this._bottomDockContainerSettleTimer); } catch(_) {}
      }
      this._bottomDockContainerSettleTimer = 0;
      this.bottomDockContainerSettling = false;
      var anchor = this.bottomDockAnchorForSnapId(this.bottomDockPlacementId);
      this._bottomDockContainerPointerActive = true;
      this._bottomDockContainerPointerMoved = false;
      this._bottomDockContainerPointerStartX = Number(ev.clientX || 0);
      this._bottomDockContainerPointerStartY = Number(ev.clientY || 0);
      this._bottomDockContainerPointerLastX = Number(ev.clientX || 0);
      this._bottomDockContainerPointerLastY = Number(ev.clientY || 0);
      this._bottomDockContainerOriginX = Number(anchor.x || 0);
      this._bottomDockContainerOriginY = Number(anchor.y || 0);
      this.bottomDockContainerDragX = Number(anchor.x || 0);
      this.bottomDockContainerDragY = Number(anchor.y || 0);
      this._bottomDockContainerDragWallLock = this.bottomDockWallLockNormalized();
      this.bindBottomDockContainerPointerListeners();
      try {
        if (ev.currentTarget && typeof ev.currentTarget.setPointerCapture === 'function' && Number.isFinite(ev.pointerId)) {
          ev.currentTarget.setPointerCapture(ev.pointerId);
        }
      } catch(_) {}
      if (ev.cancelable && typeof ev.preventDefault === 'function') ev.preventDefault();
    },

    handleBottomDockContainerPointerMove(ev) {
      if (!this._bottomDockContainerPointerActive) return;
      var nextX = Number(ev.clientX || 0);
      var nextY = Number(ev.clientY || 0);
      this._bottomDockContainerPointerLastX = nextX;
      this._bottomDockContainerPointerLastY = nextY;
      var movedX = Math.abs(nextX - Number(this._bottomDockContainerPointerStartX || 0));
      var movedY = Math.abs(nextY - Number(this._bottomDockContainerPointerStartY || 0));
      if (!this._bottomDockContainerPointerMoved) {
        if (movedX < 4 && movedY < 4) return;
        this._bottomDockContainerPointerMoved = true;
        this.bottomDockContainerDragActive = true;
        this.bottomDockHoverId = '';
        this.bottomDockHoverWeightById = {};
        this.bottomDockPointerX = 0;
        this.bottomDockPointerY = 0;
        this.bottomDockPreviewVisible = false;
        this.bottomDockPreviewText = '';
        this.bottomDockPreviewMorphFromText = '';
        this.bottomDockPreviewLabelMorphing = false;
        this.bottomDockPreviewWidth = 0;
        this.cancelBottomDockPreviewReflow();
      }
      var candidateX = Number(this._bottomDockContainerOriginX || 0) + (nextX - Number(this._bottomDockContainerPointerStartX || 0));
      var candidateY = Number(this._bottomDockContainerOriginY || 0) + (nextY - Number(this._bottomDockContainerPointerStartY || 0));
      var lockedWall = this.dragSurfaceNormalizeWall(this._bottomDockContainerDragWallLock || this.bottomDockWallLockNormalized());
      if (lockedWall) {
        var lockedTopLeft = this.bottomDockTopLeftFromAnchor(candidateX, candidateY, lockedWall);
        var lockedHardBounds = this.bottomDockHardBoundsForSide(lockedWall);
        var unlockDistance = this.dragSurfaceDistanceFromWall(
          lockedHardBounds,
          lockedTopLeft.left,
          lockedTopLeft.top,
          lockedWall
        );
        if (unlockDistance >= this.dragSurfaceWallUnlockDistanceThreshold()) {
          lockedWall = '';
          this._bottomDockContainerDragWallLock = '';
          this.bottomDockSetWallLock('');
        } else {
          var holdTopLeft = this.bottomDockTopLeftFromAnchor(
            this.bottomDockContainerDragX,
            this.bottomDockContainerDragY,
            lockedWall
          );
          var holdLocked = this.dragSurfaceApplyWallLock(
            lockedHardBounds,
            holdTopLeft.left,
            holdTopLeft.top,
            lockedWall
          );
          var holdAnchor = this.bottomDockAnchorFromTopLeft(holdLocked.left, holdLocked.top, lockedWall);
          this.bottomDockContainerDragX = Number(holdAnchor.x || 0);
          this.bottomDockContainerDragY = Number(holdAnchor.y || 0);
          this.bottomDockRotationDeg = this.bottomDockResolveRotationForSide(
            lockedWall,
            this.bottomDockContainerDragX,
            this.bottomDockContainerDragY
          );
          if (ev.cancelable && typeof ev.preventDefault === 'function') ev.preventDefault();
          return;
        }
      }
      var anchor = this.bottomDockClampDragAnchor(candidateX, candidateY);
      var nearestId = this.bottomDockNearestSnapId(anchor.x, anchor.y);
      var side = this.bottomDockSideForSnapId(nearestId);
      var candidateTopLeft = this.bottomDockTopLeftFromAnchor(anchor.x, anchor.y, side);
      var hardBounds = this.bottomDockHardBoundsForSide(side);
      var clampedTopLeft = this.dragSurfaceClampWithBounds(hardBounds, candidateTopLeft.left, candidateTopLeft.top);
      var nearestWall = this.dragSurfaceNearestWall(hardBounds, clampedTopLeft.left, clampedTopLeft.top);
      var lockWall = this.dragSurfaceResolveWallLock(
        hardBounds,
        candidateTopLeft.left,
        candidateTopLeft.top,
        nearestWall,
        nextX - Number(this._bottomDockContainerPointerStartX || 0),
        nextY - Number(this._bottomDockContainerPointerStartY || 0)
      );
      if (lockWall) {
        this._bottomDockContainerDragWallLock = this.bottomDockSetWallLock(lockWall);
        var lockTopLeft = this.bottomDockTopLeftFromAnchor(anchor.x, anchor.y, lockWall);
        var lockHardBounds = this.bottomDockHardBoundsForSide(lockWall);
        var lockClamped = this.dragSurfaceClampWithBounds(lockHardBounds, lockTopLeft.left, lockTopLeft.top);
        var snapped = this.dragSurfaceApplyWallLock(lockHardBounds, lockClamped.left, lockClamped.top, lockWall);
        var snappedAnchor = this.bottomDockAnchorFromTopLeft(snapped.left, snapped.top, lockWall);
        this.bottomDockContainerDragX = Number(snappedAnchor.x || 0);
        this.bottomDockContainerDragY = Number(snappedAnchor.y || 0);
        side = lockWall;
      } else {
        var freeAnchor = this.bottomDockAnchorFromTopLeft(clampedTopLeft.left, clampedTopLeft.top, side);
        this.bottomDockContainerDragX = Number(freeAnchor.x || 0);
        this.bottomDockContainerDragY = Number(freeAnchor.y || 0);
      }
      this.bottomDockRotationDeg = this.bottomDockResolveRotationForSide(
        side,
        this.bottomDockContainerDragX,
        this.bottomDockContainerDragY
      );
      if (ev.cancelable && typeof ev.preventDefault === 'function') ev.preventDefault();
    },

    endBottomDockContainerPointerDrag() {
      if (!this._bottomDockContainerPointerActive) return;
      this._bottomDockContainerPointerActive = false;
      this.unbindBottomDockContainerPointerListeners();
      if (!this._bottomDockContainerPointerMoved) {
        this.bottomDockContainerDragActive = false;
        this._bottomDockContainerPointerMoved = false;
        this._bottomDockContainerDragWallLock = '';
        return;
      }
      var lockWall = this.dragSurfaceNormalizeWall(this._bottomDockContainerDragWallLock || this.bottomDockWallLockNormalized());
      var anchor = this.bottomDockClampDragAnchor(this.bottomDockContainerDragX, this.bottomDockContainerDragY);
      if (!lockWall) {
        var freeNearest = this.bottomDockNearestSnapId(anchor.x, anchor.y);
        var freeSide = this.bottomDockSideForSnapId(freeNearest);
        var freeTopLeft = this.bottomDockTopLeftFromAnchor(anchor.x, anchor.y, freeSide);
        var freeHardBounds = this.bottomDockHardBoundsForSide(freeSide);
        var freeNearestWall = this.dragSurfaceNearestWall(freeHardBounds, freeTopLeft.left, freeTopLeft.top);
        if (Number(freeNearestWall.distance || 0) <= this.dragSurfaceWallLockDistanceThreshold()) {
          lockWall = this.bottomDockSetWallLock(freeNearestWall.wall);
          this._bottomDockContainerDragWallLock = lockWall;
        }
      }
      if (lockWall) {
        var lockedTopLeft = this.bottomDockTopLeftFromAnchor(anchor.x, anchor.y, lockWall);
        var lockedHardBounds = this.bottomDockHardBoundsForSide(lockWall);
        var finalLocked = this.dragSurfaceApplyWallLock(
          lockedHardBounds,
          lockedTopLeft.left,
          lockedTopLeft.top,
          lockWall
        );
        var finalAnchor = this.bottomDockAnchorFromTopLeft(finalLocked.left, finalLocked.top, lockWall);
        anchor = { x: Number(finalAnchor.x || 0), y: Number(finalAnchor.y || 0) };
      }
      var nearestId = this.bottomDockNearestSnapId(anchor.x, anchor.y);
      this.bottomDockPlacementId = nearestId;
      this.bottomDockRotationDeg = this.bottomDockResolveRotationForSide(this.bottomDockSideForSnapId(nearestId), anchor.x, anchor.y);
      this.persistBottomDockPlacement();
      this.bottomDockContainerDragActive = false;
      this.bottomDockContainerSettling = true;
      this._bottomDockContainerPointerMoved = false;
      this._bottomDockContainerDragWallLock = '';
      if (this._bottomDockContainerSettleTimer) {
        try { clearTimeout(this._bottomDockContainerSettleTimer); } catch(_) {}
      }
      var self = this;
      var settleMs = this.bottomDockMoveDurationMs() + 36;
      this._bottomDockContainerSettleTimer = window.setTimeout(function() {
        self._bottomDockContainerSettleTimer = 0;
        self.bottomDockContainerSettling = false;
      }, settleMs);
    },

    settleBottomDockDragGhost(dragId, done) {
      var finish = typeof done === 'function' ? done : function() {};
      var ghost = this._bottomDockDragGhostEl;
      if (!ghost || !document) {
        this.cleanupBottomDockDragGhost();
        finish();
        return;
      }
      var key = String(dragId || '').trim();
      if (!key) {
        this.cleanupBottomDockDragGhost();
        finish();
        return;
      }
      var slot = document.querySelector('.bottom-dock-btn[data-dock-id="' + key + '"]');
      if (!slot || typeof slot.getBoundingClientRect !== 'function') {
        this.cleanupBottomDockDragGhost();
        finish();
        return;
      }
      var rect = slot.getBoundingClientRect();
      var durationMs = this.bottomDockMoveDurationMs();
      var targetWidth = Number(rect && rect.width ? rect.width : 0);
      var targetHeight = Number(rect && rect.height ? rect.height : 0);
      var slotStyle = null;
      if (!Number.isFinite(targetWidth) || targetWidth <= 0) {
        targetWidth = Number(ghost.offsetWidth || 32);
      }
      if (!Number.isFinite(targetHeight) || targetHeight <= 0) {
        targetHeight = Number(ghost.offsetHeight || 32);
      }
      var slotRadiusPx = 0;
      try {
        if (typeof window !== 'undefined' && typeof window.getComputedStyle === 'function') {
          slotStyle = window.getComputedStyle(slot);
        }
        var rawRadius = slotStyle ? String(slotStyle.borderTopLeftRadius || slotStyle.borderRadius || '') : '';
        var rawWidth = slotStyle ? String(slotStyle.width || '') : '';
        var parsedRadius = parseFloat(rawRadius);
        var parsedWidth = parseFloat(rawWidth);
        if (Number.isFinite(parsedRadius) && parsedRadius >= 0) {
          if (Number.isFinite(parsedWidth) && parsedWidth > 0) {
            slotRadiusPx = (parsedRadius / parsedWidth) * targetWidth;
          } else {
            slotRadiusPx = parsedRadius;
          }
        }
      } catch(_) {}
      if (!slotRadiusPx) {
        slotRadiusPx = Math.round((targetWidth / 32) * 11);
      }
      ghost.style.transition =
        'left ' + durationMs + 'ms var(--ease-smooth), ' +
        'top ' + durationMs + 'ms var(--ease-smooth), ' +
        'width ' + durationMs + 'ms var(--ease-smooth), ' +
        'height ' + durationMs + 'ms var(--ease-smooth), ' +
        'border-radius ' + durationMs + 'ms var(--ease-smooth), ' +
        'opacity ' + durationMs + 'ms var(--ease-smooth)';
      var targetX = Number(rect.left || 0) + ((Number(rect.width || 0) - targetWidth) / 2);
      var targetY = Number(rect.top || 0) + ((Number(rect.height || 0) - targetHeight) / 2);
      var self = this;
      var moveGhost = function() {
        if (slotStyle) {
          ghost.style.background = String(slotStyle.background || ghost.style.background || '');
          ghost.style.border = String(slotStyle.border || ghost.style.border || '');
          ghost.style.borderWidth = String(slotStyle.borderTopWidth || ghost.style.borderWidth || '');
          ghost.style.borderStyle = String(slotStyle.borderTopStyle || ghost.style.borderStyle || '');
          ghost.style.borderColor = String(slotStyle.borderColor || ghost.style.borderColor || '');
          ghost.style.boxShadow = String(slotStyle.boxShadow || ghost.style.boxShadow || '');
          ghost.style.color = String(slotStyle.color || ghost.style.color || '');
        }
        ghost.style.left = targetX + 'px';
        ghost.style.top = targetY + 'px';
        ghost.style.width = targetWidth + 'px';
        ghost.style.height = targetHeight + 'px';
        ghost.style.borderRadius = slotRadiusPx + 'px';
        ghost.style.setProperty(
          '--dock-ghost-scale',
          String(Math.max(0.8, Math.min(4, targetWidth / 32)))
        );
        ghost.style.opacity = '1';
      };
      if (typeof requestAnimationFrame === 'function') requestAnimationFrame(moveGhost);
      else moveGhost();
      if (this._bottomDockGhostCleanupTimer) {
        try { clearTimeout(this._bottomDockGhostCleanupTimer); } catch(_) {}
      }
      this._bottomDockGhostCleanupTimer = window.setTimeout(function() {
        self._bottomDockRevealTargetDuringSettle = true;
        var settleHoldMs = 54;
        var completeSettle = function() {
          self._bottomDockGhostCleanupTimer = 0;
          finish();
          if (typeof requestAnimationFrame !== 'function') {
            self.cleanupBottomDockDragGhost();
            return;
          }
          requestAnimationFrame(function() {
            requestAnimationFrame(function() {
              self.cleanupBottomDockDragGhost();
            });
          });
        };
        self._bottomDockGhostCleanupTimer = window.setTimeout(completeSettle, settleHoldMs);
      }, durationMs + 40);
    },

    taskbarDockEdgeNormalized(raw) {
      var service = this.taskbarDockService();
      if (service && typeof service.normalizeTaskbarEdge === 'function') return service.normalizeTaskbarEdge(raw);
      var key = String(raw || '').trim().toLowerCase();
      return key === 'bottom' ? 'bottom' : 'top';
    },

    taskbarPersistDockEdge() {
      this.taskbarDockEdge = this.taskbarDockEdgeNormalized(this.taskbarDockEdge);
      try {
        localStorage.setItem('infring-taskbar-dock-edge', this.taskbarDockEdge);
      } catch(_) {}
      infringUpdateShellLayoutConfig(function(config) {
        config.taskbar.edge = this.taskbarDockEdge;
      }.bind(this));
    },

    taskbarReadHeight() {
      if (typeof document === 'undefined') return 46;
      try {
        var node = document.querySelector('.global-taskbar');
        var height = Number(node && node.offsetHeight || 0);
        if (Number.isFinite(height) && height > 0) return height;
      } catch(_) {}
      return 46;
    },

    taskbarReadViewportHeight() {
      var h = 0;
      try { h = Number(window && window.innerHeight || 0); } catch(_) { h = 0; }
      if (!Number.isFinite(h) || h <= 0) {
        h = Number(document && document.documentElement && document.documentElement.clientHeight || 900);
      }
      if (!Number.isFinite(h) || h <= 0) h = 900;
      return h;
    },

    chatOverlayViewportWidth() {
      var w = 0;
      try { w = Number(window && window.innerWidth || 0); } catch(_) { w = 0; }
      if (!Number.isFinite(w) || w <= 0) {
        w = Number(document && document.documentElement && document.documentElement.clientWidth || 1440);
      }
      if (!Number.isFinite(w) || w <= 0) w = 1440;
      return w;
    },

    taskbarAnchorForDockEdge(edgeRaw) {
      var service = this.taskbarDockService();
      if (service && typeof service.normalizeTaskbarEdge === 'function') {
        var edgeFromService = service.normalizeTaskbarEdge(edgeRaw);
        if (edgeFromService === 'bottom') return Math.max(0, this.taskbarReadViewportHeight() - this.taskbarReadHeight());
        return 0;
      }
      var edge = this.taskbarDockEdgeNormalized(edgeRaw);
      if (edge === 'bottom') {
        return Math.max(0, this.taskbarReadViewportHeight() - this.taskbarReadHeight());
      }
      return 0;
    },

    taskbarClampDragY(yRaw) {
      var service = this.taskbarDockService();
      if (service && typeof service.normalizeTaskbarEdge === 'function') {
        var yFromService = Number(yRaw);
        if (!Number.isFinite(yFromService)) yFromService = this.taskbarAnchorForDockEdge(this.taskbarDockEdge);
        var maxFromService = Math.max(0, this.taskbarReadViewportHeight() - this.taskbarReadHeight());
        return Math.max(0, Math.min(maxFromService, yFromService));
      }
      var y = Number(yRaw);
      if (!Number.isFinite(y)) y = this.taskbarAnchorForDockEdge(this.taskbarDockEdge);
      var maxY = Math.max(0, this.taskbarReadViewportHeight() - this.taskbarReadHeight());
      return Math.max(0, Math.min(maxY, y));
    },

    taskbarNearestDockEdge(yRaw) {
      var service = this.taskbarDockService();
      if (service && typeof service.normalizeTaskbarEdge === 'function') {
        var yFromService = this.taskbarClampDragY(yRaw);
        var topYFromService = this.taskbarAnchorForDockEdge('top');
        var bottomYFromService = this.taskbarAnchorForDockEdge('bottom');
        return Math.abs(yFromService - bottomYFromService) < Math.abs(yFromService - topYFromService) ? 'bottom' : 'top';
      }
      var y = this.taskbarClampDragY(yRaw);
      var topY = this.taskbarAnchorForDockEdge('top');
      var bottomY = this.taskbarAnchorForDockEdge('bottom');
      var topDist = Math.abs(y - topY);
      var bottomDist = Math.abs(y - bottomY);
      return bottomDist < topDist ? 'bottom' : 'top';
    },

    taskbarContainerStyle() {
      var service = this.taskbarDockService();
      if (service && typeof service.taskbarContainerStyle === 'function') {
        return service.taskbarContainerStyle({
          page: this.page,
          edge: this.taskbarDockEdge,
          dragging: this.taskbarDockDragActive,
          dragY: this.taskbarClampDragY(this.taskbarDockDragY),
          transitionMs: 220
        });
      }
      var styles = [];
      if (this.page !== 'chat') {
        styles.push('background:transparent;border-bottom:none;box-shadow:none;-webkit-backdrop-filter:none;backdrop-filter:none;');
      }
      var transitionMs = this.taskbarDockDragActive ? 0 : 220;
      styles.push('--taskbar-dock-transition:' + Math.max(0, Math.round(Number(transitionMs || 0))) + 'ms;');
      if (this.taskbarDockDragActive) {
        var y = this.taskbarClampDragY(this.taskbarDockDragY);
        styles.push('top:' + Math.round(Number(y || 0)) + 'px;bottom:auto;');
      } else if (this.taskbarDockEdgeNormalized(this.taskbarDockEdge) === 'bottom') {
        styles.push('top:auto;bottom:0;');
      } else {
        styles.push('top:0;bottom:auto;');
      }
      return styles.join('');
    },

    shouldIgnoreTaskbarDockDragTarget(target) {
      var service = this.dragbarService();
      if (service && typeof service.shouldIgnoreTarget === 'function') {
        return service.shouldIgnoreTarget(target, {
          ignoreSelector: 'button, a, input, textarea, select, [role="button"], [draggable="true"], .taskbar-reorder-item, .taskbar-hero-menu-anchor, .taskbar-hero-menu, .theme-switcher, .notif-wrap, .taskbar-search-popup, .taskbar-search-popup-anchor, .taskbar-clock'
        });
      }
      if (!target || typeof target.closest !== 'function') return false;
      return Boolean(
        target.closest(
          'button, a, input, textarea, select, [role="button"], [draggable="true"], .taskbar-reorder-item, .taskbar-hero-menu-anchor, .taskbar-hero-menu, .theme-switcher, .notif-wrap, .taskbar-search-popup, .taskbar-search-popup-anchor, .taskbar-clock'
        )
      );
    },

    bindTaskbarDockPointerListeners() {
      if (this._taskbarDockPointerMoveHandler || this._taskbarDockPointerUpHandler) return;
      var self = this;
      this._taskbarDockPointerMoveHandler = function(ev) { self.handleTaskbarDockPointerMove(ev); };
      this._taskbarDockPointerUpHandler = function(ev) { self.endTaskbarDockPointerDrag(ev); };
      window.addEventListener('pointermove', this._taskbarDockPointerMoveHandler, true);
      window.addEventListener('pointerup', this._taskbarDockPointerUpHandler, true);
      window.addEventListener('pointercancel', this._taskbarDockPointerUpHandler, true);
      window.addEventListener('mousemove', this._taskbarDockPointerMoveHandler, true);
      window.addEventListener('mouseup', this._taskbarDockPointerUpHandler, true);
    },

    unbindTaskbarDockPointerListeners() {
      if (this._taskbarDockPointerMoveHandler) {
        try { window.removeEventListener('pointermove', this._taskbarDockPointerMoveHandler, true); } catch(_) {}
        try { window.removeEventListener('mousemove', this._taskbarDockPointerMoveHandler, true); } catch(_) {}
      }
      if (this._taskbarDockPointerUpHandler) {
        try { window.removeEventListener('pointerup', this._taskbarDockPointerUpHandler, true); } catch(_) {}
        try { window.removeEventListener('pointercancel', this._taskbarDockPointerUpHandler, true); } catch(_) {}
        try { window.removeEventListener('mouseup', this._taskbarDockPointerUpHandler, true); } catch(_) {}
      }
      this._taskbarDockPointerMoveHandler = null;
      this._taskbarDockPointerUpHandler = null;
    },

    startTaskbarDockPointerDrag(ev) {
      if (!ev || Number(ev.button) !== 0) return;
      if (String(this.taskbarDragGroup || '').trim()) return;
      var target = ev && ev.target ? ev.target : null;
      if (this.shouldIgnoreTaskbarDockDragTarget(target)) return;
      this._taskbarDockDraggingContainedBottomDock = this.bottomDockTaskbarContained()
        ? this.bottomDockWallLockNormalized()
        : '';
      this._taskbarDockPointerActive = true;
      this._taskbarDockPointerMoved = false;
      this._taskbarDockPointerStartX = Number(ev.clientX || 0);
      this._taskbarDockPointerStartY = Number(ev.clientY || 0);
      this._taskbarDockOriginY = this.taskbarAnchorForDockEdge(this.taskbarDockEdge);
      this.taskbarDockDragY = this._taskbarDockOriginY;
      this.bindTaskbarDockPointerListeners();
      try {
        if (ev.currentTarget && typeof ev.currentTarget.setPointerCapture === 'function' && Number.isFinite(ev.pointerId)) {
          ev.currentTarget.setPointerCapture(ev.pointerId);
        }
      } catch(_) {}
      if (ev.cancelable && typeof ev.preventDefault === 'function') ev.preventDefault();
    },

    handleTaskbarDockPointerMove(ev) {
      if (!this._taskbarDockPointerActive) return;
      var x = Number(ev.clientX || 0);
      var y = Number(ev.clientY || 0);
      var movedX = Math.abs(x - Number(this._taskbarDockPointerStartX || 0));
      var movedY = Math.abs(y - Number(this._taskbarDockPointerStartY || 0));
      if (!this._taskbarDockPointerMoved) {
        if (movedX < 4 && movedY < 4) return;
        this._taskbarDockPointerMoved = true;
        this.taskbarDockDragActive = true;
      }
      var candidateY = Number(this._taskbarDockOriginY || 0) + (y - Number(this._taskbarDockPointerStartY || 0));
      this.taskbarDockDragY = this.taskbarClampDragY(candidateY);
      if (ev.cancelable && typeof ev.preventDefault === 'function') ev.preventDefault();
    },

    endTaskbarDockPointerDrag() {
      if (!this._taskbarDockPointerActive) return;
      this._taskbarDockPointerActive = false;
      this.unbindTaskbarDockPointerListeners();
      if (!this._taskbarDockPointerMoved) {
        this.taskbarDockDragActive = false;
        this._taskbarDockDraggingContainedBottomDock = '';
        return;
      }
      this._taskbarDockPointerMoved = false;
      this.taskbarDockEdge = this.taskbarNearestDockEdge(this.taskbarDockDragY);
      var carriedBottomDock = String(this._taskbarDockDraggingContainedBottomDock || '');
      if (carriedBottomDock) {
        this.bottomDockSetWallLock(this.taskbarDockEdge);
        this.taskbarDockDragY = this.taskbarAnchorForDockEdge(this.taskbarDockEdge);
        this.taskbarPersistDockEdge();
        var self = this;
        window.requestAnimationFrame(function() {
          self._taskbarDockDraggingContainedBottomDock = '';
          self.taskbarDockDragActive = false;
        });
        return;
      }
      this._taskbarDockDraggingContainedBottomDock = '';
      this.taskbarDockDragActive = false;
      this.taskbarPersistDockEdge();
    },

    overlayWallGapPx() {
      var fallback = 16;
      if (typeof window !== 'undefined' && typeof window.getComputedStyle === 'function' && document && document.documentElement) {
        try {
          var raw = String(window.getComputedStyle(document.documentElement).getPropertyValue('--overlay-wall-gap') || '').trim();
          var parsed = parseFloat(raw);
          if (Number.isFinite(parsed) && parsed >= 0) fallback = parsed;
        } catch(_) {}
      }
      return Math.max(0, Math.round(fallback));
    },

    chatOverlayVerticalBounds() {
      var viewportHeight = this.taskbarReadViewportHeight();
      var wallGap = this.overlayWallGapPx();
      var edge = this.taskbarDockEdgeNormalized(this.taskbarDockEdge);
      var taskbarH = this.taskbarReadHeight();
      var topInset = edge === 'top' ? taskbarH : 0;
      var bottomInset = edge === 'bottom' ? taskbarH : 0;
      return {
        minTop: topInset + wallGap,
        maxBottom: viewportHeight - bottomInset - wallGap,
        viewportHeight: viewportHeight,
        wallGap: wallGap
      };
    },

    dragSurfaceHardBounds(widthRaw, heightRaw, ignoreTaskbarBoundaryRaw) {
      var width = Number(widthRaw || 0);
      var height = Number(heightRaw || 0);
      if (!Number.isFinite(width) || width < 1) width = 1;
      if (!Number.isFinite(height) || height < 1) height = 1;
      var ignoreTaskbarBoundary = true;
      if (typeof ignoreTaskbarBoundaryRaw === 'boolean') {
        ignoreTaskbarBoundary = ignoreTaskbarBoundaryRaw;
      } else if (ignoreTaskbarBoundaryRaw && typeof ignoreTaskbarBoundaryRaw === 'object') {
        if (Object.prototype.hasOwnProperty.call(ignoreTaskbarBoundaryRaw, 'ignoreTaskbarBoundary')) {
          ignoreTaskbarBoundary = Boolean(ignoreTaskbarBoundaryRaw.ignoreTaskbarBoundary);
        }
      }
      var viewportWidth = this.chatOverlayViewportWidth();
      var viewportHeight = this.taskbarReadViewportHeight();
      var minTop = 0;
      var maxBottom = viewportHeight;
      if (!ignoreTaskbarBoundary) {
        var edge = this.taskbarDockEdgeNormalized(this.taskbarDockEdge);
        var taskbarH = this.taskbarReadHeight();
        minTop = edge === 'top' ? taskbarH : 0;
        maxBottom = viewportHeight - (edge === 'bottom' ? taskbarH : 0);
      }
      var service = this.dragbarService();
      if (service && typeof service.hardBounds === 'function') {
        return service.hardBounds({
          width: width,
          height: height,
          viewportWidth: viewportWidth,
          viewportHeight: viewportHeight,
          minTop: minTop,
          maxBottom: maxBottom
        });
      }
      return {
        minLeft: 0,
        maxLeft: Math.max(0, viewportWidth - width),
        minTop: minTop,
        maxTop: Math.max(minTop, maxBottom - height)
      };
    },

    dragSurfaceSoftBounds(widthRaw, heightRaw) {
      var width = Number(widthRaw || 0);
      var height = Number(heightRaw || 0);
      if (!Number.isFinite(width) || width < 1) width = 1;
      if (!Number.isFinite(height) || height < 1) height = 1;
      var vertical = this.chatOverlayVerticalBounds();
      var wallGap = this.overlayWallGapPx();
      var minLeft = wallGap;
      var maxLeft = Math.max(minLeft, this.chatOverlayViewportWidth() - wallGap - width);
      var minTop = Number(vertical.minTop || 0);
      var maxTop = Math.max(minTop, Number(vertical.maxBottom || 0) - height);
      var service = this.dragbarService();
      if (service && typeof service.softBounds === 'function') {
        return service.softBounds({
          width: width,
          height: height,
          wallGap: wallGap,
          viewportWidth: this.chatOverlayViewportWidth(),
          minTop: minTop,
          maxBottom: Number(vertical.maxBottom || 0)
        });
      }
      return { minLeft: minLeft, maxLeft: maxLeft, minTop: minTop, maxTop: maxTop };
    },

    dragSurfaceClampWithBounds(bounds, leftRaw, topRaw) {
      var service = this.dragbarService();
      if (service && typeof service.clampWithBounds === 'function') {
        return service.clampWithBounds(bounds, leftRaw, topRaw);
      }
      var box = bounds && typeof bounds === 'object' ? bounds : { minLeft: 0, maxLeft: 0, minTop: 0, maxTop: 0 };
      var left = Number(leftRaw); if (!Number.isFinite(left)) left = Number(box.minLeft || 0);
      var top = Number(topRaw); if (!Number.isFinite(top)) top = Number(box.minTop || 0);
      return {
        left: Math.max(Number(box.minLeft || 0), Math.min(Number(box.maxLeft || 0), left)),
        top: Math.max(Number(box.minTop || 0), Math.min(Number(box.maxTop || 0), top))
      };
    },

    dragSurfaceNearestWall(bounds, leftRaw, topRaw) {
      var service = this.dragbarService();
      if (service && typeof service.nearestWall === 'function') {
        return service.nearestWall(bounds, leftRaw, topRaw);
      }
      var clamped = this.dragSurfaceClampWithBounds(bounds, leftRaw, topRaw);
      var distances = {
        left: Math.max(0, clamped.left - Number(bounds.minLeft || 0)),
        right: Math.max(0, Number(bounds.maxLeft || 0) - clamped.left),
        top: Math.max(0, clamped.top - Number(bounds.minTop || 0)),
        bottom: Math.max(0, Number(bounds.maxTop || 0) - clamped.top)
      };
      var wall = 'left';
      var distance = Number(distances.left || 0);
      ['right', 'top', 'bottom'].forEach(function(key) {
        var next = Number(distances[key] || 0);
        if (next < distance) { wall = key; distance = next; }
      });
      return { wall: wall, distance: Math.max(0, distance), distances: distances, left: clamped.left, top: clamped.top };
    },

    dragSurfaceNormalizeWall(wallRaw) {
      var service = this.dragbarService();
      if (service && typeof service.normalizeWall === 'function') {
        return service.normalizeWall(wallRaw);
      }
      var wall = String(wallRaw || '').trim().toLowerCase();
      if (wall === 'left' || wall === 'right' || wall === 'top' || wall === 'bottom') return wall;
      return '';
    },

    dragSurfaceApplyWallLock(bounds, leftRaw, topRaw, wallRaw) {
      var service = this.dragbarService();
      if (service && typeof service.applyWallLock === 'function') {
        return service.applyWallLock(bounds, leftRaw, topRaw, wallRaw);
      }
      var wall = this.dragSurfaceNormalizeWall(wallRaw);
      var clamped = this.dragSurfaceClampWithBounds(bounds, leftRaw, topRaw);
      if (!wall) return { left: clamped.left, top: clamped.top, wall: '' };
      if (wall === 'left') clamped.left = Number(bounds.minLeft || 0);
      else if (wall === 'right') clamped.left = Number(bounds.maxLeft || 0);
      else if (wall === 'top') clamped.top = Number(bounds.minTop || 0);
      else if (wall === 'bottom') clamped.top = Number(bounds.maxTop || 0);
      return { left: clamped.left, top: clamped.top, wall: wall };
    },

    dragSurfaceDistanceFromWall(bounds, leftRaw, topRaw, wallRaw) {
      var service = this.dragbarService();
      if (service && typeof service.distanceFromWall === 'function') {
        return service.distanceFromWall(bounds, leftRaw, topRaw, wallRaw);
      }
      var wall = this.dragSurfaceNormalizeWall(wallRaw);
      if (!wall) return Number.POSITIVE_INFINITY;
      var clamped = this.dragSurfaceClampWithBounds(bounds, leftRaw, topRaw);
      if (wall === 'left') return Math.max(0, clamped.left - Number(bounds.minLeft || 0));
      if (wall === 'right') return Math.max(0, Number(bounds.maxLeft || 0) - clamped.left);
      if (wall === 'top') return Math.max(0, clamped.top - Number(bounds.minTop || 0));
      return Math.max(0, Number(bounds.maxTop || 0) - clamped.top);
    },

    dragSurfaceWallLockOvershoot(bounds, leftRaw, topRaw, wallRaw) {
      var service = this.dragbarService();
      if (service && typeof service.wallLockOvershoot === 'function') {
        return service.wallLockOvershoot(bounds, leftRaw, topRaw, wallRaw);
      }
      var wall = this.dragSurfaceNormalizeWall(wallRaw);
      if (!wall) return 0;
      var left = Number(leftRaw);
      var top = Number(topRaw);
      if (!Number.isFinite(left)) left = Number(bounds.minLeft || 0);
      if (!Number.isFinite(top)) top = Number(bounds.minTop || 0);
      if (wall === 'left') return Math.max(0, Number(bounds.minLeft || 0) - left);
      if (wall === 'right') return Math.max(0, left - Number(bounds.maxLeft || 0));
      if (wall === 'top') return Math.max(0, Number(bounds.minTop || 0) - top);
      return Math.max(0, top - Number(bounds.maxTop || 0));
    },

    dragSurfaceCenteredPoint(bounds) {
      var service = this.dragbarService();
      if (service && typeof service.centeredPoint === 'function') {
        return service.centeredPoint(bounds);
      }
      var box = bounds && typeof bounds === 'object' ? bounds : { minLeft: 0, maxLeft: 0, minTop: 0, maxTop: 0 };
      var left = Number(box.minLeft || 0) + ((Number(box.maxLeft || 0) - Number(box.minLeft || 0)) * 0.5);
      var top = Number(box.minTop || 0) + ((Number(box.maxTop || 0) - Number(box.minTop || 0)) * 0.5);
      return { left: left, top: top };
    },

    dragSurfaceWallLockContactThreshold() {
      var service = this.dragbarService();
      if (service && typeof service.wallLockThresholds === 'function') return service.wallLockThresholds(this.overlayWallGapPx()).contact;
      return Math.max(2, Math.round(this.overlayWallGapPx() * 0.12));
    },
    dragSurfaceWallLockDistanceThreshold() {
      var service = this.dragbarService();
      if (service && typeof service.wallLockThresholds === 'function') return service.wallLockThresholds(this.overlayWallGapPx()).distance;
      return Math.max(8, Math.round(this.overlayWallGapPx() * 0.7));
    },
    dragSurfaceWallUnlockDistanceThreshold() {
      var service = this.dragbarService();
      if (service && typeof service.wallLockThresholds === 'function') return service.wallLockThresholds(this.overlayWallGapPx()).unlock;
      return Math.max(42, Math.round(this.overlayWallGapPx() * 2.6));
    },
    dragSurfaceWallLockOvershootThreshold() {
      var service = this.dragbarService();
      if (service && typeof service.wallLockThresholds === 'function') return service.wallLockThresholds(this.overlayWallGapPx()).overshoot;
      return Math.max(5, Math.round(this.overlayWallGapPx() * 0.34));
    },
    dragSurfaceResolveWallLock(bounds, candidateLeft, candidateTop, nearest, motionDxRaw, motionDyRaw) {
      var service = this.dragbarService();
      if (service && typeof service.resolveWallLock === 'function') {
        return service.resolveWallLock(bounds, candidateLeft, candidateTop, nearest, motionDxRaw, motionDyRaw, {
          wallGap: this.overlayWallGapPx()
        });
      }
      var walls = ['left', 'right', 'top', 'bottom'];
      var overshootThreshold = this.dragSurfaceWallLockOvershootThreshold();
      var contactThreshold = this.dragSurfaceWallLockContactThreshold();
      var distanceThreshold = this.dragSurfaceWallLockDistanceThreshold();

      var overshootWall = '';
      var overshootValue = 0;
      for (var i = 0; i < walls.length; i += 1) {
        var wall = walls[i];
        var overshoot = this.dragSurfaceWallLockOvershoot(bounds, candidateLeft, candidateTop, wall);
        if (overshoot >= overshootThreshold && overshoot > overshootValue) {
          overshootValue = overshoot;
          overshootWall = wall;
        }
      }
      if (overshootWall) return overshootWall;

      var clamped = this.dragSurfaceClampWithBounds(bounds, candidateLeft, candidateTop);
      var touchedWalls = [];
      if (Math.abs(clamped.left - Number(bounds.minLeft || 0)) <= contactThreshold) touchedWalls.push('left');
      if (Math.abs(Number(bounds.maxLeft || 0) - clamped.left) <= contactThreshold) touchedWalls.push('right');
      if (Math.abs(clamped.top - Number(bounds.minTop || 0)) <= contactThreshold) touchedWalls.push('top');
      if (Math.abs(Number(bounds.maxTop || 0) - clamped.top) <= contactThreshold) touchedWalls.push('bottom');

      if (touchedWalls.length === 1) return touchedWalls[0];
      if (touchedWalls.length > 1) {
        var motionDx = Number(motionDxRaw || 0);
        var motionDy = Number(motionDyRaw || 0);
        var absDx = Math.abs(motionDx);
        var absDy = Math.abs(motionDy);
        if (absDx > absDy + 0.25) {
          if (motionDx >= 0 && touchedWalls.indexOf('right') >= 0) return 'right';
          if (motionDx < 0 && touchedWalls.indexOf('left') >= 0) return 'left';
        } else if (absDy > absDx + 0.25) {
          if (motionDy >= 0 && touchedWalls.indexOf('bottom') >= 0) return 'bottom';
          if (motionDy < 0 && touchedWalls.indexOf('top') >= 0) return 'top';
        }
        var nearestWall = nearest && typeof nearest.wall === 'string' ? this.dragSurfaceNormalizeWall(nearest.wall) : '';
        if (nearestWall && touchedWalls.indexOf(nearestWall) >= 0) return nearestWall;
        return touchedWalls[0];
      }

      var edgeDistance = nearest && Number.isFinite(Number(nearest.distance)) ? Number(nearest.distance) : Number.POSITIVE_INFINITY;
      if (!Number.isFinite(edgeDistance) || edgeDistance > distanceThreshold) return '';
      return this.dragSurfaceNormalizeWall(nearest && nearest.wall ? nearest.wall : '');
    },

    dragSurfaceRadiusByWall(wallRaw) {
      var service = this.dragbarService();
      if (service && typeof service.radiusByWall === 'function') {
        return service.radiusByWall(wallRaw);
      }
      var r = 'var(--overlay-shared-surface-radius, var(--overlay-surface-radius, 18px))';
      var wall = this.dragSurfaceNormalizeWall(wallRaw);
      if (wall === 'left') return '0 ' + r + ' ' + r + ' 0';
      if (wall === 'right') return r + ' 0 0 ' + r;
      if (wall === 'top') return '0 0 ' + r + ' ' + r;
      if (wall === 'bottom') return r + ' ' + r + ' 0 0';
      return r;
    },

    dragSurfaceLockTransformTimeMs(rawValue) {
      var service = this.dragbarService();
      if (service && typeof service.lockTransformTimeMs === 'function') {
        return service.lockTransformTimeMs(rawValue, this._dragSurfaceLockTransformMs || 500);
      }
      var fallback = Number(this._dragSurfaceLockTransformMs || 500);
      if (!Number.isFinite(fallback)) fallback = 500;
      var raw = Number(rawValue);
      if (!Number.isFinite(raw)) raw = fallback;
      return Math.max(120, Math.round(raw));
    },

    dragSurfaceLockBorderFadeDurationMs(transformMsRaw) {
      var service = this.dragbarService();
      if (service && typeof service.lockBorderFadeDurationMs === 'function') {
        return service.lockBorderFadeDurationMs(transformMsRaw);
      }
      var transformMs = this.dragSurfaceLockTransformTimeMs(transformMsRaw);
      return Math.max(80, Math.round(transformMs * 0.24));
    },

    dragSurfaceVisualStateStore() {
      if (!this._dragSurfaceVisualStates || typeof this._dragSurfaceVisualStates !== 'object') {
        this._dragSurfaceVisualStates = {};
      }
      return this._dragSurfaceVisualStates;
    },

    dragSurfaceLockVisualCssVars(surfaceKeyRaw, wallRaw, optionsRaw) {
      var service = this.dragbarService();
      if (service && typeof service.lockVisualCssVars === 'function') {
        return service.lockVisualCssVars(surfaceKeyRaw, wallRaw, optionsRaw, this.dragSurfaceVisualStateStore());
      }
      var key = String(surfaceKeyRaw || 'drag-surface').trim().toLowerCase(); if (!key) key = 'drag-surface';
      var wall = this.dragSurfaceNormalizeWall(wallRaw);
      var options = optionsRaw && typeof optionsRaw === 'object' ? optionsRaw : {};
      var transformMs = this.dragSurfaceLockTransformTimeMs(options.transformMs);
      var fadeMs = this.dragSurfaceLockBorderFadeDurationMs(transformMs);
      var delayMs = 0; var durationMs = 0;
      var store = this.dragSurfaceVisualStateStore();
      var prev = store[key] && typeof store[key] === 'object' ? store[key] : { initialized: false, wall: wall };
      var initialized = prev.initialized === true;
      var previousWall = this.dragSurfaceNormalizeWall(prev.wall); if (!initialized) previousWall = wall;
      var wallChanged = previousWall !== wall;
      if (wall && wallChanged) { delayMs = transformMs; durationMs = fadeMs; }
      store[key] = { initialized: true, wall: wall };
      var baseBorder = 'var(--drag-bar-border)';
      var borderTop = baseBorder; var borderRight = baseBorder; var borderBottom = baseBorder; var borderLeft = baseBorder;
      if (wall === 'left') borderLeft = 'transparent';
      else if (wall === 'right') borderRight = 'transparent';
      else if (wall === 'top') borderTop = 'transparent';
      else if (wall === 'bottom') borderBottom = 'transparent';

      var shellPaddingInline = Object.prototype.hasOwnProperty.call(options, 'shellPaddingInline') ? String(options.shellPaddingInline || '') : '';
      var shellPaddingInlineLocked = Object.prototype.hasOwnProperty.call(options, 'shellPaddingInlineLocked') ? String(options.shellPaddingInlineLocked || '') : '';
      var shellPaddingBlock = Object.prototype.hasOwnProperty.call(options, 'shellPaddingBlock') ? String(options.shellPaddingBlock || '') : '';
      var shellPaddingBlockLocked = Object.prototype.hasOwnProperty.call(options, 'shellPaddingBlockLocked') ? String(options.shellPaddingBlockLocked || '') : '';
      var shellAlignItems = Object.prototype.hasOwnProperty.call(options, 'shellAlignItems') ? String(options.shellAlignItems || '') : '';
      var shellAlignItemsLocked = shellAlignItems;
      if (wall === 'left' && Object.prototype.hasOwnProperty.call(options, 'shellAlignItemsLeft')) shellAlignItemsLocked = String(options.shellAlignItemsLeft || shellAlignItemsLocked || '');
      else if (wall === 'right' && Object.prototype.hasOwnProperty.call(options, 'shellAlignItemsRight')) shellAlignItemsLocked = String(options.shellAlignItemsRight || shellAlignItemsLocked || '');
      else if (wall === 'top' && Object.prototype.hasOwnProperty.call(options, 'shellAlignItemsTop')) shellAlignItemsLocked = String(options.shellAlignItemsTop || shellAlignItemsLocked || '');
      else if (wall === 'bottom' && Object.prototype.hasOwnProperty.call(options, 'shellAlignItemsBottom')) shellAlignItemsLocked = String(options.shellAlignItemsBottom || shellAlignItemsLocked || '');

      var surfaceMarginInline = Object.prototype.hasOwnProperty.call(options, 'surfaceMarginInline') ? String(options.surfaceMarginInline || '') : '';
      var surfaceMarginInlineLocked = Object.prototype.hasOwnProperty.call(options, 'surfaceMarginInlineLocked') ? String(options.surfaceMarginInlineLocked || '') : '';
      var resolvedSurfaceMarginInline = wall ? (surfaceMarginInlineLocked || surfaceMarginInline) : surfaceMarginInline;

      var radius = this.dragSurfaceRadiusByWall(wall);
      var css = '';
      css += '--drag-bar-lock-wall:' + (wall || 'none') + ';';
      css += '--drag-bar-lock-state:' + (wall ? '1' : '0') + ';';
      css += '--drag-bar-transform-time:' + transformMs + 'ms;';
      css += '--drag-bar-radius-transition:' + transformMs + 'ms var(--ease-smooth);';
      css += '--drag-bar-radius-override:' + radius + ';';
      css += '--drag-bar-border-top-color:' + borderTop + ';';
      css += '--drag-bar-border-right-color:' + borderRight + ';';
      css += '--drag-bar-border-bottom-color:' + borderBottom + ';';
      css += '--drag-bar-border-left-color:' + borderLeft + ';';
      css += '--drag-bar-border-transition-duration:' + Math.max(0, Math.round(durationMs)) + 'ms;';
      css += '--drag-bar-border-transition-delay:' + Math.max(0, Math.round(delayMs)) + 'ms;';
      if (shellPaddingInline || shellPaddingInlineLocked) {
        css += '--drag-bar-shell-padding-inline:' + (wall ? (shellPaddingInlineLocked || shellPaddingInline || '0px') : (shellPaddingInline || '0px')) + ';';
      }
      if (shellPaddingBlock || shellPaddingBlockLocked) {
        css += '--drag-bar-shell-padding-block:' + (wall ? (shellPaddingBlockLocked || shellPaddingBlock || '0px') : (shellPaddingBlock || '0px')) + ';';
      }
      if (shellAlignItems || shellAlignItemsLocked) {
        css += '--drag-bar-shell-align-items:' + (wall ? (shellAlignItemsLocked || shellAlignItems || 'stretch') : (shellAlignItems || 'stretch')) + ';';
      }
      if (resolvedSurfaceMarginInline) {
        css += '--drag-bar-surface-margin-inline:' + resolvedSurfaceMarginInline + ';';
      }
      return css;
    },

    dragSurfaceLockRadiusCssVars(wallRaw) {
      var service = this.dragbarService();
      if (service && typeof service.lockRadiusCssVars === 'function') {
        return service.lockRadiusCssVars(wallRaw);
      }
      var wall = this.dragSurfaceNormalizeWall(wallRaw);
      if (!wall) return '';
      var radius = this.dragSurfaceRadiusByWall(wall);
      return '--drag-bar-radius-override:' + radius + ';';
    },

    readChatMapElement() {
      if (typeof document === 'undefined' || typeof document.querySelector !== 'function') return null;
      try { return document.querySelector('.chat-map'); } catch(_) {}
      return null;
    },

    readChatMapHeight() {
      var node = this.readChatMapElement();
      var height = Number(node && node.offsetHeight || 0);
      if (!Number.isFinite(height) || height <= 0) {
        height = Math.max(180, this.taskbarReadViewportHeight() - 276);
      }
      return height;
    },

    chatMapPlacementEnabled() {
      return this.page === 'chat' || (this.page === 'agents' && !!this.activeChatAgent);
    },

    chatMapClampTop(topRaw) {
      var bounds = this.chatOverlayVerticalBounds();
      var height = this.readChatMapHeight();
      var minTop = Number(bounds.minTop || 0);
      var maxTop = Number(bounds.maxBottom || 0) - height;
      if (!Number.isFinite(maxTop) || maxTop < minTop) maxTop = minTop;
      var top = Number(topRaw);
      if (!Number.isFinite(top)) top = minTop + ((maxTop - minTop) * 0.38);
      return Math.max(minTop, Math.min(maxTop, top));
    },

    chatMapPersistPlacementFromTop(topRaw) {
      var bounds = this.chatOverlayVerticalBounds();
      var height = this.readChatMapHeight();
      var minTop = Number(bounds.minTop || 0);
      var maxTop = Number(bounds.maxBottom || 0) - height;
      if (!Number.isFinite(maxTop) || maxTop < minTop) maxTop = minTop;
      var top = this.chatMapClampTop(topRaw);
      var ratio = maxTop > minTop ? (top - minTop) / (maxTop - minTop) : 0.38;
      ratio = Math.max(0, Math.min(1, ratio));
      this.chatMapPlacementY = ratio;
      try {
        localStorage.setItem('infring-chat-map-placement-y', String(ratio));
      } catch(_) {}
      infringUpdateShellLayoutConfig(function(config) {
        config.chatMap.placementY = ratio;
      });
    },

    shouldIgnoreChatMapDragTarget(target) {
      var service = this.dragbarService();
      if (service && typeof service.shouldIgnoreTarget === 'function') {
        return service.shouldIgnoreTarget(target, {
          ignoreSelector: 'button, a, input, textarea, select, [role="button"], [contenteditable="true"], .chat-map-item, .chat-map-day, .chat-map-jump'
        });
      }
      var node = target;
      if (node && typeof node.closest !== 'function' && node.parentElement) {
        node = node.parentElement;
      }
      if (!node || typeof node.closest !== 'function') return false;
      return Boolean(
        node.closest(
          'button, a, input, textarea, select, [role="button"], [contenteditable="true"], .chat-map-item, .chat-map-day, .chat-map-jump'
        )
      );
    },

    bindChatMapPointerListeners() {
      if (this._chatMapPointerMoveHandler || this._chatMapPointerUpHandler) return;
      var self = this;
      this._chatMapPointerMoveHandler = function(ev) { self.handleChatMapPointerMove(ev); };
      this._chatMapPointerUpHandler = function() { self.endChatMapPointerDrag(); };
      window.addEventListener('pointermove', this._chatMapPointerMoveHandler, true);
      window.addEventListener('pointerup', this._chatMapPointerUpHandler, true);
      window.addEventListener('pointercancel', this._chatMapPointerUpHandler, true);
      window.addEventListener('mousemove', this._chatMapPointerMoveHandler, true);
      window.addEventListener('mouseup', this._chatMapPointerUpHandler, true);
    },

    unbindChatMapPointerListeners() {
      if (this._chatMapPointerMoveHandler) {
        try { window.removeEventListener('pointermove', this._chatMapPointerMoveHandler, true); } catch(_) {}
        try { window.removeEventListener('mousemove', this._chatMapPointerMoveHandler, true); } catch(_) {}
      }
      if (this._chatMapPointerUpHandler) {
        try { window.removeEventListener('pointerup', this._chatMapPointerUpHandler, true); } catch(_) {}
        try { window.removeEventListener('pointercancel', this._chatMapPointerUpHandler, true); } catch(_) {}
        try { window.removeEventListener('mouseup', this._chatMapPointerUpHandler, true); } catch(_) {}
      }
      this._chatMapPointerMoveHandler = null;
      this._chatMapPointerUpHandler = null;
    },

    taskbarReorderDefaults(group) {
      var service = this.taskbarDockService();
      if (service && typeof service.taskbarOrderDefaults === 'function') return service.taskbarOrderDefaults(group);
      var key = String(group || '').trim().toLowerCase();
      if (key === 'right') return ['connectivity', 'theme', 'notifications', 'search', 'auth'];
      return ['nav_cluster'];
    },

    taskbarReorderStorageKey(group) {
      var service = this.taskbarDockService();
      if (service && typeof service.taskbarStorageKey === 'function') return service.taskbarStorageKey(group);
      var key = String(group || '').trim().toLowerCase();
      return key === 'right' ? 'infring-taskbar-order-right' : 'infring-taskbar-order-left';
    },

    taskbarReorderOrderForGroup(group) {
      var key = String(group || '').trim().toLowerCase();
      return key === 'right' ? this.taskbarReorderRight : this.taskbarReorderLeft;
    },

    setTaskbarReorderOrderForGroup(group, nextOrder) {
      var key = String(group || '').trim().toLowerCase();
      if (key === 'right') {
        this.taskbarReorderRight = nextOrder;
        return;
      }
      this.taskbarReorderLeft = nextOrder;
    },

    normalizeTaskbarReorder(group, rawOrder) {
      var service = this.taskbarDockService();
      if (service && typeof service.normalizeOrder === 'function') return service.normalizeOrder(rawOrder, this.taskbarReorderDefaults(group));
      var defaults = this.taskbarReorderDefaults(group);
      var source = Array.isArray(rawOrder) ? rawOrder : [];
      var seen = {};
      var ordered = [];
      for (var i = 0; i < source.length; i += 1) {
        var id = String(source[i] || '').trim();
        if (!id || seen[id] || defaults.indexOf(id) < 0) continue;
        seen[id] = true;
        ordered.push(id);
      }
      for (var j = 0; j < defaults.length; j += 1) {
        var fallbackId = defaults[j];
        if (seen[fallbackId]) continue;
        seen[fallbackId] = true;
        ordered.push(fallbackId);
      }
      return ordered;
    },

    persistTaskbarReorder(group) {
      var key = String(group || '').trim().toLowerCase();
      if (key !== 'right') key = 'left';
      var normalized = this.normalizeTaskbarReorder(key, this.taskbarReorderOrderForGroup(key));
      this.setTaskbarReorderOrderForGroup(key, normalized);
      try {
        var service = this.taskbarDockService();
        if (service && typeof service.persistTaskbarOrder === 'function') normalized = service.persistTaskbarOrder(key, normalized);
        else localStorage.setItem(this.taskbarReorderStorageKey(key), JSON.stringify(normalized));
      } catch(_) {}
      infringUpdateShellLayoutConfig(function(config) {
        if (key === 'right') config.taskbar.orderRight = normalized.slice();
        else config.taskbar.orderLeft = normalized.slice();
      });
    },

    taskbarReorderOrderIndex(group, item) {
      var key = String(group || '').trim().toLowerCase();
      if (key !== 'right') key = 'left';
      var itemId = String(item || '').trim();
      if (!itemId) return 999;
      var service = this.taskbarDockService();
      if (service && typeof service.orderIndex === 'function') {
        return service.orderIndex(itemId, this.taskbarReorderOrderForGroup(key), this.taskbarReorderDefaults(key));
      }
      var order = this.normalizeTaskbarReorder(key, this.taskbarReorderOrderForGroup(key));
      var idx = order.indexOf(itemId);
      if (idx >= 0) return idx;
      var fallback = this.taskbarReorderDefaults(key).indexOf(itemId);
      return fallback >= 0 ? fallback : 999;
    },

    taskbarReorderItemStyle(group, item) {
      return 'order:' + this.taskbarReorderOrderIndex(group, item);
    },

    taskbarReorderItemRects(group) {
      if (typeof document === 'undefined') return {};
      var key = String(group || '').trim().toLowerCase();
      if (key !== 'right') key = 'left';
      var out = {};
      var box = null;
      try {
        box = document.querySelector('.taskbar-reorder-box-' + key);
      } catch(_) {
        box = null;
      }
      if (!box || typeof box.querySelectorAll !== 'function') return out;
      var nodes = box.querySelectorAll('.taskbar-reorder-item[data-taskbar-item]');
      for (var i = 0; i < nodes.length; i += 1) {
        var node = nodes[i];
        if (!node || typeof node.getBoundingClientRect !== 'function') continue;
        var id = String(node.getAttribute('data-taskbar-item') || '').trim();
        if (!id || Object.prototype.hasOwnProperty.call(out, id)) continue;
        var rect = node.getBoundingClientRect();
        out[id] = { left: Number(rect.left || 0), top: Number(rect.top || 0) };
      }
      return out;
    },

    animateTaskbarReorderFromRects(group, beforeRects) {
      if (!beforeRects || typeof beforeRects !== 'object') return;
      if (typeof requestAnimationFrame !== 'function' || typeof document === 'undefined') return;
      var key = String(group || '').trim().toLowerCase();
      if (key !== 'right') key = 'left';
      requestAnimationFrame(function() {
        var box = null;
        try {
          box = document.querySelector('.taskbar-reorder-box-' + key);
        } catch(_) {
          box = null;
        }
        if (!box || typeof box.querySelectorAll !== 'function') return;
        var nodes = box.querySelectorAll('.taskbar-reorder-item[data-taskbar-item]');
        for (var i = 0; i < nodes.length; i += 1) {
          var node = nodes[i];
          if (!node || node.classList.contains('dragging')) continue;
          var id = String(node.getAttribute('data-taskbar-item') || '').trim();
          if (!id || !Object.prototype.hasOwnProperty.call(beforeRects, id)) continue;
          var from = beforeRects[id] || {};
          var rect = node.getBoundingClientRect();
          var dx = Number(from.left || 0) - Number(rect.left || 0);
          var dy = Number(from.top || 0) - Number(rect.top || 0);
          if (Math.abs(dx) < 0.5 && Math.abs(dy) < 0.5) continue;
          node.style.transition = 'none';
          node.style.transform = 'translate(' + Math.round(dx) + 'px,' + Math.round(dy) + 'px)';
          void node.offsetHeight;
          node.style.transition = 'transform 220ms var(--ease-smooth)';
          node.style.transform = 'translate(0px, 0px)';
          (function(el) {
            window.setTimeout(function() {
              if (!el.classList.contains('dragging')) el.style.transform = '';
              el.style.transition = '';
            }, 250);
          })(node);
        }
      });
    },

    applyTaskbarReorder(group, dragItem, targetItem, preferAfter, animate) {
      var key = String(group || '').trim().toLowerCase();
      if (key !== 'right') key = 'left';
      var dragId = String(dragItem || '').trim();
      var targetId = String(targetItem || '').trim();
      if (!dragId || !targetId || dragId === targetId) return false;
      var current = this.normalizeTaskbarReorder(key, this.taskbarReorderOrderForGroup(key));
      var fromIndex = current.indexOf(dragId);
      var toIndex = current.indexOf(targetId);
      if (fromIndex < 0 || toIndex < 0) return false;
      var next = current.slice();
      next.splice(fromIndex, 1);
      if (fromIndex < toIndex) toIndex -= 1;
      if (Boolean(preferAfter)) toIndex += 1;
      if (toIndex < 0) toIndex = 0;
      if (toIndex > next.length) toIndex = next.length;
      next.splice(toIndex, 0, dragId);
      if (JSON.stringify(next) === JSON.stringify(current)) return false;
      var beforeRects = Boolean(animate) ? this.taskbarReorderItemRects(key) : null;
      this.setTaskbarReorderOrderForGroup(key, next);
      if (beforeRects) this.animateTaskbarReorderFromRects(key, beforeRects);
      return true;
    },
    handleTaskbarReorderPointerDown(group, ev) {
      if (String(this.taskbarDragGroup || '').trim()) return;
      if (!ev || Number(ev.button) !== 0) return;
      var key = String(group || '').trim().toLowerCase();
      if (key !== 'right') key = 'left';
      var target = ev && ev.target && typeof ev.target.closest === 'function'
        ? ev.target.closest('.taskbar-reorder-item[data-taskbar-item]')
        : null;
      var item = target ? String(target.getAttribute('data-taskbar-item') || '').trim() : '';
      if (!item) return;
      this.cancelTaskbarDragHold();
      this._taskbarDragHoldGroup = key;
      this._taskbarDragHoldItem = item;
      var self = this;
      if (typeof window !== 'undefined' && typeof window.setTimeout === 'function') {
        this._taskbarDragHoldTimer = window.setTimeout(function() {
          self._taskbarDragHoldTimer = 0;
          self._taskbarDragArmedGroup = key;
          self._taskbarDragArmedItem = item;
        }, 180);
      }
    },
    cancelTaskbarDragHold() {
      if (this._taskbarDragHoldTimer) {
        try { clearTimeout(this._taskbarDragHoldTimer); } catch(_) {}
      }
      this._taskbarDragHoldTimer = 0;
      this._taskbarDragHoldGroup = '';
      this._taskbarDragHoldItem = '';
      if (!String(this.taskbarDragGroup || '').trim()) {
        this._taskbarDragArmedGroup = '';
        this._taskbarDragArmedItem = '';
      }
    },
    forceTaskbarMoveDragEffect(ev) {
      if (!ev || !ev.dataTransfer) return;
      try { ev.dataTransfer.effectAllowed = 'move'; } catch(_) {}
      try { ev.dataTransfer.dropEffect = 'move'; } catch(_) {}
    },
    setTaskbarDragBodyActive(active) {
      if (typeof document === 'undefined' || !document.body || !document.body.classList) return;
      if (active) {
        document.body.classList.add('taskbar-drag-active');
      } else {
        document.body.classList.remove('taskbar-drag-active');
      }
    },
    handleTaskbarReorderDragStart(group, ev) {
      var key = String(group || '').trim().toLowerCase();
      if (key !== 'right') key = 'left';
      var target = ev && ev.target && typeof ev.target.closest === 'function'
        ? ev.target.closest('.taskbar-reorder-item[data-taskbar-item]')
        : null;
      var item = target ? String(target.getAttribute('data-taskbar-item') || '').trim() : '';
      if (!item || this._taskbarDragArmedGroup !== key || this._taskbarDragArmedItem !== item) {
        if (ev && typeof ev.preventDefault === 'function') ev.preventDefault();
        return;
      }
      this.taskbarDragGroup = key;
      this.taskbarDragItem = item;
      this.taskbarDragStartOrder = this.normalizeTaskbarReorder(key, this.taskbarReorderOrderForGroup(key));
      this._taskbarDragArmedGroup = '';
      this._taskbarDragArmedItem = '';
      this.cancelTaskbarDragHold();
      if (ev && ev.dataTransfer) {
        this.forceTaskbarMoveDragEffect(ev);
        try { ev.dataTransfer.setData('application/x-infring-taskbar', key + ':' + item); } catch(_) {}
        try { ev.dataTransfer.setData('text/plain', key + ':' + item); } catch(_) {}
        try {
          if (
            typeof document !== 'undefined' &&
            document.body &&
            typeof ev.dataTransfer.setDragImage === 'function'
          ) {
            var ghost = target && typeof target.cloneNode === 'function'
              ? target.cloneNode(true)
              : document.createElement('span');
            ghost.style.position = 'fixed';
            ghost.style.left = '-9999px';
            ghost.style.top = '-9999px';
            ghost.style.margin = '0';
            ghost.style.pointerEvents = 'none';
            ghost.style.transform = 'none';
            ghost.style.opacity = '1';
            if (ghost.classList && ghost.classList.contains('dragging')) {
              ghost.classList.remove('dragging');
            }
            var rect = target && typeof target.getBoundingClientRect === 'function'
              ? target.getBoundingClientRect()
              : null;
            var offsetX = 0;
            var offsetY = 0;
            if (rect) {
              var width = Math.max(1, Math.round(Number(rect.width || 0)));
              var height = Math.max(1, Math.round(Number(rect.height || 0)));
              ghost.style.width = width + 'px';
              ghost.style.height = height + 'px';
              ghost.style.boxSizing = 'border-box';
              if (typeof ev.clientX === 'number') {
                offsetX = Math.round(Math.max(0, Math.min(width, ev.clientX - rect.left)));
              }
              if (typeof ev.clientY === 'number') {
                offsetY = Math.round(Math.max(0, Math.min(height, ev.clientY - rect.top)));
              }
            } else {
              ghost.style.width = '1px';
              ghost.style.height = '1px';
            }
            document.body.appendChild(ghost);
            ev.dataTransfer.setDragImage(ghost, offsetX, offsetY);
            window.setTimeout(function() {
              if (ghost.parentNode) ghost.parentNode.removeChild(ghost);
            }, 0);
          }
        } catch(_) {}
      }
      if (target && target.classList) target.classList.add('dragging');
      this.setTaskbarDragBodyActive(true);
    },
    handleTaskbarReorderDragMove(ev) {
      this.forceTaskbarMoveDragEffect(ev);
    },
    handleTaskbarReorderDragEnter(group, ev) {
      var key = String(group || '').trim().toLowerCase();
      if (key !== 'right') key = 'left';
      if (String(this.taskbarDragGroup || '').trim() !== key) return;
      if (ev && typeof ev.preventDefault === 'function') ev.preventDefault();
      this.forceTaskbarMoveDragEffect(ev);
    },
    handleTaskbarReorderDragOver(group, ev) {
      var key = String(group || '').trim().toLowerCase();
      if (key !== 'right') key = 'left';
      if (String(this.taskbarDragGroup || '').trim() !== key) return;
      if (ev && typeof ev.preventDefault === 'function') ev.preventDefault();
      this.forceTaskbarMoveDragEffect(ev);
      var dragItem = String(this.taskbarDragItem || '').trim();
      if (!dragItem) return;
      var target = ev && ev.target && typeof ev.target.closest === 'function'
        ? ev.target.closest('.taskbar-reorder-item[data-taskbar-item]')
        : null;
      var targetItem = target ? String(target.getAttribute('data-taskbar-item') || '').trim() : '';
      if (!targetItem || targetItem === dragItem) return;
      var preferAfter = false;
      if (target && typeof target.getBoundingClientRect === 'function') {
        var rect = target.getBoundingClientRect();
        var midX = Number(rect.left || 0) + (Number(rect.width || 0) / 2);
        preferAfter = Number(ev && ev.clientX || 0) >= midX;
      }
      this.applyTaskbarReorder(key, dragItem, targetItem, preferAfter, true);
    },
    clearTaskbarReorderDraggingClass() {
      if (typeof document === 'undefined') return;
      try {
        var draggingNodes = document.querySelectorAll('.taskbar-reorder-item.dragging');
        for (var i = 0; i < draggingNodes.length; i += 1) {
          draggingNodes[i].classList.remove('dragging');
        }
      } catch(_) {}
    },
    handleTaskbarReorderDrop(group, ev) {
      var key = String(group || '').trim().toLowerCase();
      if (key !== 'right') key = 'left';
      if (String(this.taskbarDragGroup || '').trim() !== key) return;
      if (ev && typeof ev.preventDefault === 'function') ev.preventDefault();
      this.persistTaskbarReorder(key);
      this.taskbarDragGroup = '';
      this.taskbarDragItem = '';
      this.taskbarDragStartOrder = [];
      this.cancelTaskbarDragHold();
      this.setTaskbarDragBodyActive(false);
      this.clearTaskbarReorderDraggingClass();
    },
    handleTaskbarDragEnd() {
      var key = String(this.taskbarDragGroup || '').trim();
      if (key) this.persistTaskbarReorder(key);
      this.taskbarDragGroup = '';
      this.taskbarDragItem = '';
      this.taskbarDragStartOrder = [];
      this.cancelTaskbarDragHold();
      this.setTaskbarDragBodyActive(false);
      this.clearTaskbarReorderDraggingClass();
    },
    chatSidebarSnapDefinitions() {
      return [
        { id: 'left-top', x: 0, y: 0 },
        { id: 'left-middle', x: 0, y: 0.5 },
        { id: 'left-bottom', x: 0, y: 1 }
      ];
    },
    chatSidebarSnapDefinitionById(id) {
      var key = String(id || '').trim().toLowerCase();
      var defs = this.chatSidebarSnapDefinitions();
      for (var i = 0; i < defs.length; i += 1) {
        var row = defs[i];
        if (!row || row.id !== key) continue;
        return row;
      }
      return defs[1] || defs[0] || { id: 'left-middle', x: 0, y: 0.5 };
    },
    chatSidebarAnchorForSnapId(id) {
      var snap = this.chatSidebarSnapDefinitionById(id);
      var wallGap = this.overlayWallGapPx();
      var minLeft = wallGap;
      var maxLeft = this.chatOverlayViewportWidth() - wallGap - this.readChatSidebarWidth() - this.readChatSidebarPulltabWidth();
      if (!Number.isFinite(maxLeft) || maxLeft < minLeft) maxLeft = minLeft;
      var bounds = this.chatOverlayVerticalBounds();
      var height = this.readChatSidebarHeight();
      var minTop = Number(bounds.minTop || 0);
      var maxTop = Number(bounds.maxBottom || 0) - height;
      if (!Number.isFinite(maxTop) || maxTop < minTop) maxTop = minTop;
      var nx = Number(snap && snap.x);
      var ny = Number(snap && snap.y);
      if (!Number.isFinite(nx)) nx = 0;
      if (!Number.isFinite(ny)) ny = 0.5;
      nx = Math.max(0, Math.min(1, nx));
      ny = Math.max(0, Math.min(1, ny));
      return {
        id: String(snap && snap.id || 'left-middle'),
        left: this.chatSidebarClampLeft(minLeft + ((maxLeft - minLeft) * nx)),
        top: this.chatSidebarClampTop(minTop + ((maxTop - minTop) * ny))
      };
    },
    chatSidebarNearestSnapId(leftRaw, topRaw) {
      var defs = this.chatSidebarSnapDefinitions();
      if (!defs.length) return 'left-middle';
      var left = this.chatSidebarClampLeft(leftRaw);
      var top = this.chatSidebarClampTop(topRaw);
      var bestId = String(defs[0].id || 'left-middle');
      var bestDist = Number.POSITIVE_INFINITY;
      for (var i = 0; i < defs.length; i += 1) {
        var row = defs[i];
        if (!row) continue;
        var anchor = this.chatSidebarAnchorForSnapId(row.id);
        var dx = Number(left || 0) - Number(anchor.left || 0);
        var dy = Number(top || 0) - Number(anchor.top || 0);
        var dist = (dx * dx) + (dy * dy);
        if (!Number.isFinite(dist) || dist >= bestDist) continue;
        bestDist = dist;
        bestId = String(row.id || bestId);
      }
      return bestId || 'left-middle';
    },
    chatSidebarResolvedLeftFromRatio() {
      var ratio = 0;
      try {
        var raw = Number(localStorage.getItem('infring-chat-sidebar-placement-x'));
        if (Number.isFinite(raw)) ratio = Math.max(0, Math.min(1, raw));
      } catch(_) {}
      if (Number.isFinite(this.chatSidebarPlacementX)) ratio = Math.max(0, Math.min(1, Number(this.chatSidebarPlacementX)));
      var wallGap = this.overlayWallGapPx();
      var minLeft = wallGap;
      var maxLeft = this.chatOverlayViewportWidth() - wallGap - this.readChatSidebarWidth() - this.readChatSidebarPulltabWidth();
      if (!Number.isFinite(maxLeft) || maxLeft < minLeft) maxLeft = minLeft;
      return this.chatSidebarClampLeft(minLeft + ((maxLeft - minLeft) * ratio));
    },
    chatSidebarResolvedTopFromRatio() {
      var topPx = Number(this.chatSidebarPlacementTopPx);
      if (!Number.isFinite(topPx)) {
        try {
          var rawTop = Number(localStorage.getItem('infring-chat-sidebar-placement-top-px'));
          if (Number.isFinite(rawTop)) topPx = rawTop;
        } catch(_) {}
      }
      if (Number.isFinite(topPx)) {
        return this.chatSidebarClampTop(topPx);
      }
      var bounds = this.chatOverlayVerticalBounds();
      var height = this.readChatSidebarHeight();
      var minTop = Number(bounds.minTop || 0);
      var maxTop = Number(bounds.maxBottom || 0) - height;
      if (!Number.isFinite(maxTop) || maxTop < minTop) maxTop = minTop;
      var ratio = Number(this.chatSidebarPlacementY);
      if (!Number.isFinite(ratio)) ratio = 0.5;
      ratio = Math.max(0, Math.min(1, ratio));
      return this.chatSidebarClampTop(minTop + ((maxTop - minTop) * ratio));
    },
    chatSidebarActiveSnapId() {
      if (this.chatSidebarDragActive) {
        return this.chatSidebarNearestSnapId(this.chatSidebarDragLeft, this.chatSidebarDragTop);
      }
      var storedId = String(this.chatSidebarPlacementAnchorId || '').trim().toLowerCase();
      if (!storedId) {
        try {
          var raw = String(localStorage.getItem('infring-chat-sidebar-placement-anchor') || '').trim().toLowerCase();
          if (raw) storedId = raw;
        } catch(_) {}
      }
      if (storedId) return this.chatSidebarSnapDefinitionById(storedId).id;
      var fallbackLeft = this.chatSidebarClampLeft(this.chatSidebarResolvedLeftFromRatio());
      var fallbackTop = this.chatSidebarClampTop(this.chatSidebarResolvedTopFromRatio());
      return this.chatSidebarNearestSnapId(fallbackLeft, fallbackTop);
    },
    chatSidebarPersistSnapId(id) {
      var snap = this.chatSidebarSnapDefinitionById(id);
      this.chatSidebarPlacementAnchorId = String(snap && snap.id || 'left-middle');
      try {
        localStorage.setItem('infring-chat-sidebar-placement-anchor', this.chatSidebarPlacementAnchorId);
      } catch(_) {}
    },
    readChatMapWidth() {
      var lockedWall = this.chatMapWallLockNormalized();
      if (lockedWall) {
        var surface = null;
        if (typeof document !== 'undefined' && typeof document.querySelector === 'function') {
          try { surface = document.querySelector('.chat-map .chat-map-surface'); } catch(_) {}
        }
        var lockedWidth = Number(surface && surface.offsetWidth || 0);
        if (Number.isFinite(lockedWidth) && lockedWidth > 0) return lockedWidth;
        return 60;
      }
      var node = this.readChatMapElement();
      var width = Number(node && node.offsetWidth || 0);
      if (Number.isFinite(width) && width > 0) return width;
      return 76;
    },
    chatMapSnapDefinitions() {
      return [
        { id: 'right-top', x: 1, y: 0 },
        { id: 'right-middle', x: 1, y: 0.5 },
        { id: 'right-bottom', x: 1, y: 1 }
      ];
    },
    chatMapSnapDefinitionById(id) {
      var key = String(id || '').trim().toLowerCase();
      var defs = this.chatMapSnapDefinitions();
      for (var i = 0; i < defs.length; i += 1) {
        var row = defs[i];
        if (!row || row.id !== key) continue;
        return row;
      }
      return defs[1] || defs[0] || { id: 'right-middle', x: 1, y: 0.5 };
    },
    chatMapAnchorForSnapId(id) {
      var snap = this.chatMapSnapDefinitionById(id);
      var wallGap = this.overlayWallGapPx();
      var minLeft = wallGap;
      var maxLeft = this.chatOverlayViewportWidth() - wallGap - this.readChatMapWidth();
      if (!Number.isFinite(maxLeft) || maxLeft < minLeft) maxLeft = minLeft;
      var bounds = this.chatOverlayVerticalBounds();
      var height = this.readChatMapHeight();
      var minTop = Number(bounds.minTop || 0);
      var maxTop = Number(bounds.maxBottom || 0) - height;
      if (!Number.isFinite(maxTop) || maxTop < minTop) maxTop = minTop;
      var nx = Number(snap && snap.x);
      var ny = Number(snap && snap.y);
      if (!Number.isFinite(nx)) nx = 1;
      if (!Number.isFinite(ny)) ny = 0.5;
      nx = Math.max(0, Math.min(1, nx));
      ny = Math.max(0, Math.min(1, ny));
      return {
        id: String(snap && snap.id || 'right-middle'),
        left: this.chatMapClampLeft(minLeft + ((maxLeft - minLeft) * nx)),
        top: this.chatMapClampTop(minTop + ((maxTop - minTop) * ny))
      };
    },
    chatMapNearestSnapId(leftRaw, topRaw) {
      var defs = this.chatMapSnapDefinitions();
      if (!defs.length) return 'right-middle';
      var left = this.chatMapClampLeft(leftRaw);
      var top = this.chatMapClampTop(topRaw);
      var bestId = String(defs[0].id || 'right-middle');
      var bestDist = Number.POSITIVE_INFINITY;
      for (var i = 0; i < defs.length; i += 1) {
        var row = defs[i];
        if (!row) continue;
        var anchor = this.chatMapAnchorForSnapId(row.id);
        var dx = Number(left || 0) - Number(anchor.left || 0);
        var dy = Number(top || 0) - Number(anchor.top || 0);
        var dist = (dx * dx) + (dy * dy);
        if (!Number.isFinite(dist) || dist >= bestDist) continue;
        bestDist = dist;
        bestId = String(row.id || bestId);
      }
      return bestId || 'right-middle';
    },
    chatMapResolvedLeftFromRatio() {
      var ratio = 1;
      try {
        var raw = Number(localStorage.getItem('infring-chat-map-placement-x'));
        if (Number.isFinite(raw)) ratio = Math.max(0, Math.min(1, raw));
      } catch(_) {}
      if (Number.isFinite(this.chatMapPlacementX)) ratio = Math.max(0, Math.min(1, Number(this.chatMapPlacementX)));
      var wallGap = this.overlayWallGapPx();
      var minLeft = wallGap;
      var maxLeft = this.chatOverlayViewportWidth() - wallGap - this.readChatMapWidth();
      if (!Number.isFinite(maxLeft) || maxLeft < minLeft) maxLeft = minLeft;
      return this.chatMapClampLeft(minLeft + ((maxLeft - minLeft) * ratio));
    },
    chatMapResolvedTopFromRatio() {
      var bounds = this.chatOverlayVerticalBounds();
      var height = this.readChatMapHeight();
      var minTop = Number(bounds.minTop || 0);
      var maxTop = Number(bounds.maxBottom || 0) - height;
      if (!Number.isFinite(maxTop) || maxTop < minTop) maxTop = minTop;
      var ratio = Number(this.chatMapPlacementY);
      if (!Number.isFinite(ratio)) ratio = 0.38;
      ratio = Math.max(0, Math.min(1, ratio));
      return this.chatMapClampTop(minTop + ((maxTop - minTop) * ratio));
    },
    chatMapActiveSnapId() {
      if (this.chatMapDragActive) {
        return this.chatMapNearestSnapId(this.chatMapDragLeft, this.chatMapDragTop);
      }
      var storedId = String(this.chatMapPlacementAnchorId || '').trim().toLowerCase();
      if (!storedId) {
        try {
          var raw = String(localStorage.getItem('infring-chat-map-placement-anchor') || '').trim().toLowerCase();
          if (raw) storedId = raw;
        } catch(_) {}
      }
      if (storedId) return this.chatMapSnapDefinitionById(storedId).id;
      var fallbackLeft = this.chatMapClampLeft(this.chatMapResolvedLeftFromRatio());
      var fallbackTop = this.chatMapClampTop(this.chatMapResolvedTopFromRatio());
      return this.chatMapNearestSnapId(fallbackLeft, fallbackTop);
    },
    chatMapPersistSnapId(id) {
      var snap = this.chatMapSnapDefinitionById(id);
      this.chatMapPlacementAnchorId = String(snap && snap.id || 'right-middle');
      try {
        localStorage.setItem('infring-chat-map-placement-anchor', this.chatMapPlacementAnchorId);
      } catch(_) {}
    },
    chatMapClampLeft(leftRaw) {
      var wallGap = this.overlayWallGapPx();
      var minLeft = wallGap;
      var maxLeft = this.chatOverlayViewportWidth() - wallGap - this.readChatMapWidth();
      if (!Number.isFinite(maxLeft) || maxLeft < minLeft) maxLeft = minLeft;
      var left = Number(leftRaw);
      if (!Number.isFinite(left)) left = maxLeft;
      return Math.max(minLeft, Math.min(maxLeft, left));
    },
    chatMapHardBounds() {
      return this.dragSurfaceHardBounds(this.readChatMapWidth(), this.readChatMapHeight());
    },
    chatMapWallLockNormalized() {
      var wall = this.dragSurfaceNormalizeWall(this.chatMapWallLock);
      return wall === 'left' || wall === 'right' ? wall : '';
    },
    chatMapSetWallLock(wallRaw) {
      var wall = this.dragSurfaceNormalizeWall(wallRaw);
      if (wall !== 'left' && wall !== 'right') wall = '';
      this.chatMapWallLock = wall;
      try {
        if (wall) localStorage.setItem('infring-chat-map-wall-lock', wall);
        else localStorage.removeItem('infring-chat-map-wall-lock');
        localStorage.removeItem('infring-chat-map-smash-wall');
      } catch(_) {}
      infringUpdateShellLayoutConfig(function(config) {
        config.chatMap.wallLock = wall;
      });
      return wall;
    },
    chatMapResolvedLeft() {
      if (this.chatMapDragActive) return Number(this.chatMapDragLeft || 0);
      var left = this.chatMapClampLeft(this.chatMapResolvedLeftFromRatio());
      var top = this.chatMapClampTop(this.chatMapResolvedTopFromRatio());
      var wall = this.chatMapWallLockNormalized();
      if (!wall) return left;
      return this.dragSurfaceApplyWallLock(this.chatMapHardBounds(), left, top, wall).left;
    },
    chatMapResolvedTop() {
      if (this.chatMapDragActive) return Number(this.chatMapDragTop || 0);
      var left = this.chatMapClampLeft(this.chatMapResolvedLeftFromRatio());
      var top = this.chatMapClampTop(this.chatMapResolvedTopFromRatio());
      var wall = this.chatMapWallLockNormalized();
      if (!wall) return top;
      return this.dragSurfaceApplyWallLock(this.chatMapHardBounds(), left, top, wall).top;
    },
    chatMapPersistPlacementFromLeft(leftRaw) {
      var wallGap = this.overlayWallGapPx();
      var minLeft = wallGap;
      var maxLeft = this.chatOverlayViewportWidth() - wallGap - this.readChatMapWidth();
      if (!Number.isFinite(maxLeft) || maxLeft < minLeft) maxLeft = minLeft;
      var left = this.chatMapClampLeft(leftRaw);
      var ratio = maxLeft > minLeft ? (left - minLeft) / (maxLeft - minLeft) : 1;
      ratio = Math.max(0, Math.min(1, ratio));
      this.chatMapPlacementX = ratio;
      try {
        localStorage.setItem('infring-chat-map-placement-x', String(ratio));
      } catch(_) {}
      infringUpdateShellLayoutConfig(function(config) {
        config.chatMap.placementX = ratio;
      });
    },
    chatMapContainerStyle() {
      if (!this.chatMapPlacementEnabled()) return '';
      var top = this.chatMapResolvedTop();
      var left = this.chatMapResolvedLeft();
      var height = this.readChatMapHeight();
      var durationMs = this.chatMapDragActive ? 0 : this.dragSurfaceMoveDurationMs(this._chatMapMoveDurationMs, 280);
      var wall = this.chatMapWallLockNormalized();
      var lockCss = this.dragSurfaceLockVisualCssVars('chat-map', wall, {
        transformMs: this._dragSurfaceLockTransformMs,
        shellPaddingInline: '8px',
        shellPaddingInlineLocked: '0px',
        shellPaddingBlock: '2px',
        shellPaddingBlockLocked: '0px',
        shellAlignItems: 'flex-end',
        shellAlignItemsLeft: 'flex-start',
        shellAlignItemsRight: 'flex-end',
        surfaceMarginInline: 'auto',
        surfaceMarginInlineLocked: '0'
      });
      return (
        'left:' + Math.round(left) + 'px;' +
        'top:' + Math.round(top) + 'px;' +
        'right:auto;' +
        'bottom:auto;' +
        'height:' + Math.round(height) + 'px;' +
        lockCss +
        'transition:top ' + Math.max(0, Math.round(durationMs)) + 'ms var(--ease-smooth), left ' + Math.max(0, Math.round(durationMs)) + 'ms var(--ease-smooth);'
      );
    },
    startChatMapPointerDrag(ev) {
      if (!ev || !this.chatMapPlacementEnabled()) return;
      var button = Number(ev.button);
      if (Number.isFinite(button) && button > 0) return;
      var target = ev && ev.target ? ev.target : null;
      if (this.shouldIgnoreChatMapDragTarget(target)) return;
      this._chatMapPointerActive = true;
      this._chatMapPointerMoved = false;
      this._chatMapPointerStartX = Number(ev.clientX || 0);
      this._chatMapPointerStartY = Number(ev.clientY || 0);
      this._chatMapPointerOriginLeft = this.chatMapResolvedLeft();
      this._chatMapPointerOriginTop = this.chatMapResolvedTop();
      this._chatMapPointerLastX = this._chatMapPointerStartX;
      this._chatMapPointerLastY = this._chatMapPointerStartY;
      this._chatMapPointerLastAt = Date.now();
      this._chatMapPointerVelocity = 0;
      this.chatMapDragLeft = this._chatMapPointerOriginLeft;
      this.chatMapDragTop = this._chatMapPointerOriginTop;
      this.bindChatMapPointerListeners();
      try {
        if (ev.currentTarget && typeof ev.currentTarget.setPointerCapture === 'function' && Number.isFinite(ev.pointerId)) {
          ev.currentTarget.setPointerCapture(ev.pointerId);
        }
      } catch(_) {}
    },
    handleChatMapPointerMove(ev) {
      if (!this._chatMapPointerActive || !this.chatMapPlacementEnabled()) return;
      var nextX = Number(ev.clientX || 0);
      var nextY = Number(ev.clientY || 0);
      var now = Date.now();
      var prevX = Number(this._chatMapPointerLastX || nextX);
      var prevY = Number(this._chatMapPointerLastY || nextY);
      var prevAt = Number(this._chatMapPointerLastAt || now);
      var dt = Math.max(1, now - prevAt);
      var stepDx = nextX - prevX;
      var stepDy = nextY - prevY;
      this._chatMapPointerVelocity = Math.sqrt((stepDx * stepDx) + (stepDy * stepDy)) / dt;
      this._chatMapPointerLastX = nextX;
      this._chatMapPointerLastY = nextY;
      this._chatMapPointerLastAt = now;
      var movedX = Math.abs(nextX - Number(this._chatMapPointerStartX || 0));
      var movedY = Math.abs(nextY - Number(this._chatMapPointerStartY || 0));
      if (!this._chatMapPointerMoved) {
        if (movedX < 4 && movedY < 4) return;
        this._chatMapPointerMoved = true;
        this.chatMapDragActive = true;
        this.hideDashboardPopupBySource('chat-map');
      }
      var dragDx = nextX - Number(this._chatMapPointerStartX || 0);
      var dragDy = nextY - Number(this._chatMapPointerStartY || 0);
      var candidateLeft = Number(this._chatMapPointerOriginLeft || 0) + dragDx;
      var candidateTop = Number(this._chatMapPointerOriginTop || 0) + dragDy;
      var hardBounds = this.chatMapHardBounds();
      var lockedWall = this.chatMapWallLockNormalized();
      if (lockedWall) {
        var unlockDistance = this.dragSurfaceDistanceFromWall(hardBounds, candidateLeft, candidateTop, lockedWall);
        if (unlockDistance >= this.dragSurfaceWallUnlockDistanceThreshold()) {
          lockedWall = this.chatMapSetWallLock('');
        } else {
          var holdLeft = Number.isFinite(Number(this.chatMapDragLeft))
            ? Number(this.chatMapDragLeft)
            : Number(this._chatMapPointerOriginLeft || 0);
          var holdTop = Number.isFinite(Number(this.chatMapDragTop))
            ? Number(this.chatMapDragTop)
            : Number(this._chatMapPointerOriginTop || 0);
          var stayLocked = this.dragSurfaceApplyWallLock(hardBounds, holdLeft, holdTop, lockedWall);
          this.chatMapDragLeft = stayLocked.left;
          this.chatMapDragTop = stayLocked.top;
          if (ev.cancelable && typeof ev.preventDefault === 'function') ev.preventDefault();
          return;
        }
      }
      var clamped = this.dragSurfaceClampWithBounds(hardBounds, candidateLeft, candidateTop);
      var nearest = this.dragSurfaceNearestWall(hardBounds, clamped.left, clamped.top);
      var lockWall = this.dragSurfaceResolveWallLock(
        hardBounds,
        candidateLeft,
        candidateTop,
        nearest,
        dragDx,
        dragDy
      );
      if (lockWall) {
        var persistedLockWall = this.chatMapSetWallLock(lockWall);
        var snapped = this.dragSurfaceApplyWallLock(hardBounds, clamped.left, clamped.top, persistedLockWall);
        this.chatMapDragLeft = snapped.left;
        this.chatMapDragTop = snapped.top;
      } else {
        this.chatMapDragLeft = clamped.left;
        this.chatMapDragTop = clamped.top;
      }
      if (ev.cancelable && typeof ev.preventDefault === 'function') ev.preventDefault();
    },
    endChatMapPointerDrag() {
      if (!this._chatMapPointerActive) return;
      this._chatMapPointerActive = false;
      this.unbindChatMapPointerListeners();
      if (!this._chatMapPointerMoved) {
        this.chatMapDragActive = false;
        return;
      }
      this._chatMapPointerMoved = false;
      var hardBounds = this.chatMapHardBounds();
      var lockedWall = this.chatMapWallLockNormalized();
      var final;
      if (lockedWall) {
        final = this.dragSurfaceApplyWallLock(hardBounds, this.chatMapDragLeft, this.chatMapDragTop, lockedWall);
        this.chatMapPlacementAnchorId = '';
        try { localStorage.removeItem('infring-chat-map-placement-anchor'); } catch(_) {}
      } else {
        var clamped = this.dragSurfaceClampWithBounds(hardBounds, this.chatMapDragLeft, this.chatMapDragTop);
        var snapId = this.chatMapNearestSnapId(clamped.left, clamped.top);
        var snap = this.chatMapAnchorForSnapId(snapId);
        final = this.dragSurfaceClampWithBounds(hardBounds, snap.left, snap.top);
        this.chatMapPersistSnapId(snapId);
      }
      this.chatMapDragLeft = final.left;
      this.chatMapDragTop = final.top;
      this.chatMapPersistPlacementFromLeft(this.chatMapDragLeft);
      this.chatMapPersistPlacementFromTop(this.chatMapDragTop);
      this.chatMapDragActive = false;
    },

    readChatSidebarElement() {
      if (typeof document === 'undefined' || typeof document.querySelector !== 'function') return null;
      try { return document.querySelector('.sidebar'); } catch(_) {}
      return null;
    },
    readChatSidebarHeight() {
      var node = this.readChatSidebarElement();
      var height = Number(node && node.offsetHeight || 0);
      if (!Number.isFinite(height) || height <= 0) {
        height = Math.max(180, Math.round(this.taskbarReadViewportHeight() * 0.52));
      }
      return height;
    },
    readChatSidebarWidth() {
      var node = this.readChatSidebarElement();
      var width = Number(node && node.offsetWidth || 0);
      if (Number.isFinite(width) && width > 0) return width;
      var fallback = this.sidebarCollapsed ? 72 : 248;
      if (typeof window !== 'undefined' && typeof window.getComputedStyle === 'function' && document && document.documentElement) {
        try {
          var key = this.sidebarCollapsed ? '--sidebar-collapsed' : '--sidebar-width';
          var raw = String(window.getComputedStyle(document.documentElement).getPropertyValue(key) || '').trim();
          var parsed = parseFloat(raw);
          if (Number.isFinite(parsed) && parsed > 0) fallback = parsed;
        } catch(_) {}
      }
      return Math.max(1, Math.round(fallback));
    },
    readChatSidebarPulltabWidth() {
      var fallback = 22;
      if (typeof window !== 'undefined' && typeof window.getComputedStyle === 'function' && document && document.documentElement) {
        try {
          var raw = String(window.getComputedStyle(document.documentElement).getPropertyValue('--sidebar-pulltab-width') || '').trim();
          var parsed = parseFloat(raw);
          if (Number.isFinite(parsed) && parsed > 0) fallback = parsed;
        } catch(_) {}
      }
      return Math.max(1, Math.round(fallback));
    },
    chatSidebarClampLeft(leftRaw) {
      var wallGap = this.overlayWallGapPx();
      var minLeft = wallGap;
      var maxLeft = this.chatOverlayViewportWidth() - wallGap - this.readChatSidebarWidth() - this.readChatSidebarPulltabWidth();
      if (!Number.isFinite(maxLeft) || maxLeft < minLeft) maxLeft = minLeft;
      var left = Number(leftRaw);
      if (!Number.isFinite(left)) left = minLeft;
      return Math.max(minLeft, Math.min(maxLeft, left));
    },
    chatSidebarHardBounds() {
      return this.dragSurfaceHardBounds(this.readChatSidebarWidth(), this.readChatSidebarHeight());
    },
    chatSidebarWallLockNormalized() {
      var wall = this.dragSurfaceNormalizeWall(this.chatSidebarWallLock);
      return wall === 'left' || wall === 'right' ? wall : '';
    },
    chatSidebarSetWallLock(wallRaw) {
      var wall = this.dragSurfaceNormalizeWall(wallRaw);
      if (wall !== 'left' && wall !== 'right') wall = '';
      this.chatSidebarWallLock = wall;
      try {
        if (wall) localStorage.setItem('infring-chat-sidebar-wall-lock', wall);
        else localStorage.removeItem('infring-chat-sidebar-wall-lock');
        localStorage.removeItem('infring-chat-sidebar-smash-wall');
      } catch(_) {}
      infringUpdateShellLayoutConfig(function(config) {
        config.chatBar.wallLock = wall;
      });
      return wall;
    },
    chatSidebarResolvedLeft() {
      if (this.chatSidebarDragActive) return Number(this.chatSidebarDragLeft || 0);
      var left = this.chatSidebarClampLeft(this.chatSidebarResolvedLeftFromRatio());
      var top = this.chatSidebarClampTop(this.chatSidebarResolvedTopFromRatio());
      var wall = this.chatSidebarWallLockNormalized();
      if (!wall) return left;
      return this.dragSurfaceApplyWallLock(this.chatSidebarHardBounds(), left, top, wall).left;
    },
    chatSidebarPersistPlacementFromLeft(leftRaw) {
      var wallGap = this.overlayWallGapPx();
      var minLeft = wallGap;
      var maxLeft = this.chatOverlayViewportWidth() - wallGap - this.readChatSidebarWidth() - this.readChatSidebarPulltabWidth();
      if (!Number.isFinite(maxLeft) || maxLeft < minLeft) maxLeft = minLeft;
      var left = this.chatSidebarClampLeft(leftRaw);
      var ratio = maxLeft > minLeft ? (left - minLeft) / (maxLeft - minLeft) : 0;
      ratio = Math.max(0, Math.min(1, ratio));
      this.chatSidebarPlacementX = ratio;
      try {
        localStorage.setItem('infring-chat-sidebar-placement-x', String(ratio));
      } catch(_) {}
      infringUpdateShellLayoutConfig(function(config) {
        config.chatBar.placementX = ratio;
      });
    },
    chatSidebarClampTop(topRaw) {
      var bounds = this.chatOverlayVerticalBounds();
      var height = this.readChatSidebarHeight();
      var minTop = Number(bounds.minTop || 0);
      var maxTop = Number(bounds.maxBottom || 0) - height;
      if (!Number.isFinite(maxTop) || maxTop < minTop) maxTop = minTop;
      var top = Number(topRaw);
      if (!Number.isFinite(top)) top = minTop + ((maxTop - minTop) * 0.5);
      return Math.max(minTop, Math.min(maxTop, top));
    },
    chatSidebarResolvedTop() {
      if (this.chatSidebarDragActive) return Number(this.chatSidebarDragTop || 0);
      var left = this.chatSidebarClampLeft(this.chatSidebarResolvedLeftFromRatio());
      var top = this.chatSidebarClampTop(this.chatSidebarResolvedTopFromRatio());
      var wall = this.chatSidebarWallLockNormalized();
      if (!wall) return top;
      return this.dragSurfaceApplyWallLock(this.chatSidebarHardBounds(), left, top, wall).top;
    },
    chatSidebarPersistPlacementFromTop(topRaw) {
      var bounds = this.chatOverlayVerticalBounds();
      var height = this.readChatSidebarHeight();
      var minTop = Number(bounds.minTop || 0);
      var maxTop = Number(bounds.maxBottom || 0) - height;
      if (!Number.isFinite(maxTop) || maxTop < minTop) maxTop = minTop;
      var top = this.chatSidebarClampTop(topRaw);
      this.chatSidebarPlacementTopPx = top;
      try {
        localStorage.setItem('infring-chat-sidebar-placement-top-px', String(top));
      } catch(_) {}
      var ratio = maxTop > minTop ? (top - minTop) / (maxTop - minTop) : 0.5;
      ratio = Math.max(0, Math.min(1, ratio));
      this.chatSidebarPlacementY = ratio;
      try {
        localStorage.setItem('infring-chat-sidebar-placement-y', String(ratio));
      } catch(_) {}
      infringUpdateShellLayoutConfig(function(config) {
        config.chatBar.placementTopPx = top;
        config.chatBar.placementY = ratio;
      });
    },
    chatSidebarContainerStyle() {
      if (this.page !== 'chat') return '';
      var top = this.chatSidebarResolvedTop();
      var left = this.chatSidebarResolvedLeft();
      var durationMs = this.chatSidebarDragActive ? 0 : this.dragSurfaceMoveDurationMs(this._chatSidebarMoveDurationMs, 280);
      var wall = this.chatSidebarWallLockNormalized();
      var lockCss = this.dragSurfaceLockVisualCssVars('chat-sidebar', wall, {
        transformMs: this._dragSurfaceLockTransformMs
      });
      return (
        'position:fixed;' +
        'left:' + Math.round(left) + 'px;' +
        'top:' + Math.round(top) + 'px;' +
        'bottom:auto;' +
        'height:fit-content;' +
        'min-height:calc(56px * 3);' +
        'max-height:80vh;' +
        'transform:none;' +
        '--sidebar-position-transition:' + Math.max(0, Math.round(durationMs)) + 'ms;' +
        lockCss
      );
    },
    chatSidebarNavShellStyle() {
      return this.page === 'chat'
        ? 'flex:0 1 auto;min-height:0;max-height:calc(80vh - 16px);'
        : '';
    },
    chatSidebarNavStyle() {
      return this.page === 'chat'
        ? 'height:auto;flex:0 1 auto;max-height:calc(80vh - 16px);'
        : '';
    },
    chatSidebarPulltabStyle() {
      if (this.page !== 'chat') return '';
      var durationMs = this.chatSidebarDragActive ? 0 : this.dragSurfaceMoveDurationMs(this._chatSidebarMoveDurationMs, 280);
      var wall = this.chatSidebarWallLockNormalized();
      var service = this.dragbarService();
      if (service && typeof service.pulltabStyle === 'function') {
        return service.pulltabStyle({
          active: this.page === 'chat',
          dragging: this.chatSidebarDragActive,
          durationMs: durationMs,
          fallbackMs: 280,
          transitionVar: '--sidebar-position-transition',
          wall: wall
        });
      }
      var dockRight = wall === 'right';
      return [
        'position:absolute;',
        'left:' + (dockRight ? 'auto' : '100%') + ';',
        'right:' + (dockRight ? '100%' : 'auto') + ';',
        'top:50%;',
        'transform:translateY(-50%);',
        '--sidebar-position-transition:' + Math.max(0, Math.round(durationMs)) + 'ms;'
      ].join('');
    },
    shouldIgnoreChatSidebarDragTarget(target) {
      var service = this.dragbarService();
      if (service && typeof service.shouldIgnoreTarget === 'function') {
        return service.shouldIgnoreTarget(target, {
          ignoreSelector: 'input,textarea,select,[contenteditable="true"],button,a,[role="button"],.sidebar-pulltab,.nav-item,.nav-agent-row,[data-agent-id]'
        });
      }
      var node = target;
      if (node && typeof node.closest !== 'function' && node.parentElement) {
        node = node.parentElement;
      }
      if (!node || typeof node.closest !== 'function') return false;
      if (node.closest('.sidebar-pulltab')) return true;
      return Boolean(
        node.closest(
          'input,textarea,select,[contenteditable="true"],button,a,[role="button"],.nav-item,.nav-agent-row,[data-agent-id]'
        )
      );
    },

    bindChatSidebarPointerListeners() {
      if (this._chatSidebarPointerMoveHandler || this._chatSidebarPointerUpHandler) return;
      var self = this;
      this._chatSidebarPointerMoveHandler = function(ev) { self.handleChatSidebarPointerMove(ev); };
      this._chatSidebarPointerUpHandler = function() { self.endChatSidebarPointerDrag(); };
      var supportsPointer = typeof window !== 'undefined' && ('PointerEvent' in window);
      if (supportsPointer) {
        window.addEventListener('pointermove', this._chatSidebarPointerMoveHandler, true);
        window.addEventListener('pointerup', this._chatSidebarPointerUpHandler, true);
        window.addEventListener('pointercancel', this._chatSidebarPointerUpHandler, true);
      } else {
        window.addEventListener('mousemove', this._chatSidebarPointerMoveHandler, true);
        window.addEventListener('mouseup', this._chatSidebarPointerUpHandler, true);
      }
    },

    unbindChatSidebarPointerListeners() {
      if (this._chatSidebarPointerMoveHandler) {
        try { window.removeEventListener('pointermove', this._chatSidebarPointerMoveHandler, true); } catch(_) {}
        try { window.removeEventListener('mousemove', this._chatSidebarPointerMoveHandler, true); } catch(_) {}
      }
      if (this._chatSidebarPointerUpHandler) {
        try { window.removeEventListener('pointerup', this._chatSidebarPointerUpHandler, true); } catch(_) {}
        try { window.removeEventListener('pointercancel', this._chatSidebarPointerUpHandler, true); } catch(_) {}
        try { window.removeEventListener('mouseup', this._chatSidebarPointerUpHandler, true); } catch(_) {}
      }
      this._chatSidebarPointerMoveHandler = null;
      this._chatSidebarPointerUpHandler = null;
    },

    startChatSidebarPointerDrag(ev) {
      if (!ev || this.page !== 'chat') return;
      if (this._chatSidebarPointerActive) return;
      var button = Number(ev.button);
      if (Number.isFinite(button) && button !== 0) return;
      var target = ev && ev.target ? ev.target : null;
      if (this.shouldIgnoreChatSidebarDragTarget(target)) return;
      this._chatSidebarPointerActive = true;
      this._chatSidebarPointerMoved = false;
      this._chatSidebarPointerStartX = Number(ev.clientX || 0);
      this._chatSidebarPointerStartY = Number(ev.clientY || 0);
      this._chatSidebarPointerOriginLeft = this.chatSidebarResolvedLeft();
      this._chatSidebarPointerOriginTop = this.chatSidebarResolvedTop();
      this._chatSidebarPointerLastX = this._chatSidebarPointerStartX;
      this._chatSidebarPointerLastY = this._chatSidebarPointerStartY;
      this._chatSidebarPointerLastAt = Date.now();
      this._chatSidebarPointerVelocity = 0;
      this.chatSidebarDragLeft = this._chatSidebarPointerOriginLeft;
      this.chatSidebarDragTop = this._chatSidebarPointerOriginTop;
      this.bindChatSidebarPointerListeners();
      try {
        if (ev.currentTarget && typeof ev.currentTarget.setPointerCapture === 'function' && Number.isFinite(ev.pointerId)) {
          ev.currentTarget.setPointerCapture(ev.pointerId);
        }
      } catch(_) {}
    },

    handleChatSidebarPointerMove(ev) {
      if (!this._chatSidebarPointerActive || this.page !== 'chat') return;
      var nextX = Number(ev.clientX || 0);
      var nextY = Number(ev.clientY || 0);
      var now = Date.now();
      var prevX = Number(this._chatSidebarPointerLastX || nextX);
      var prevY = Number(this._chatSidebarPointerLastY || nextY);
      var prevAt = Number(this._chatSidebarPointerLastAt || now);
      var dt = Math.max(1, now - prevAt);
      var stepDx = nextX - prevX;
      var stepDy = nextY - prevY;
      this._chatSidebarPointerVelocity = Math.sqrt((stepDx * stepDx) + (stepDy * stepDy)) / dt;
      this._chatSidebarPointerLastX = nextX;
      this._chatSidebarPointerLastY = nextY;
      this._chatSidebarPointerLastAt = now;
      var movedX = Math.abs(nextX - Number(this._chatSidebarPointerStartX || 0));
      var movedY = Math.abs(nextY - Number(this._chatSidebarPointerStartY || 0));
      if (!this._chatSidebarPointerMoved) {
        if (movedX < 4 && movedY < 4) return;
        this._chatSidebarPointerMoved = true;
        this.chatSidebarDragActive = true;
        this.hideDashboardPopupBySource('sidebar');
      }
      var dragDx = nextX - Number(this._chatSidebarPointerStartX || 0);
      var dragDy = nextY - Number(this._chatSidebarPointerStartY || 0);
      var candidateLeft = Number(this._chatSidebarPointerOriginLeft || 0) + dragDx;
      var candidateTop = Number(this._chatSidebarPointerOriginTop || 0) + dragDy;
      var hardBounds = this.chatSidebarHardBounds();
      var lockedWall = this.chatSidebarWallLockNormalized();
      if (lockedWall) {
        var unlockDistance = this.dragSurfaceDistanceFromWall(hardBounds, candidateLeft, candidateTop, lockedWall);
        if (unlockDistance >= this.dragSurfaceWallUnlockDistanceThreshold()) {
          lockedWall = this.chatSidebarSetWallLock('');
        } else {
          var holdLeft = Number.isFinite(Number(this.chatSidebarDragLeft))
            ? Number(this.chatSidebarDragLeft)
            : Number(this._chatSidebarPointerOriginLeft || 0);
          var holdTop = Number.isFinite(Number(this.chatSidebarDragTop))
            ? Number(this.chatSidebarDragTop)
            : Number(this._chatSidebarPointerOriginTop || 0);
          var stayLocked = this.dragSurfaceApplyWallLock(hardBounds, holdLeft, holdTop, lockedWall);
          this.chatSidebarDragLeft = stayLocked.left;
          this.chatSidebarDragTop = stayLocked.top;
          if (ev.cancelable && typeof ev.preventDefault === 'function') ev.preventDefault();
          return;
        }
      }
      var clamped = this.dragSurfaceClampWithBounds(hardBounds, candidateLeft, candidateTop);
      var nearest = this.dragSurfaceNearestWall(hardBounds, clamped.left, clamped.top);
      var lockWall = this.dragSurfaceResolveWallLock(
        hardBounds,
        candidateLeft,
        candidateTop,
        nearest,
        dragDx,
        dragDy
      );
      if (lockWall) {
        var persistedLockWall = this.chatSidebarSetWallLock(lockWall);
        var snapped = this.dragSurfaceApplyWallLock(hardBounds, clamped.left, clamped.top, persistedLockWall);
        this.chatSidebarDragLeft = snapped.left;
        this.chatSidebarDragTop = snapped.top;
      } else {
        this.chatSidebarDragLeft = clamped.left;
        this.chatSidebarDragTop = clamped.top;
      }
      if (ev.cancelable && typeof ev.preventDefault === 'function') ev.preventDefault();
    },

    endChatSidebarPointerDrag() {
      if (!this._chatSidebarPointerActive) return;
      this._chatSidebarPointerActive = false;
      this.unbindChatSidebarPointerListeners();
      if (!this._chatSidebarPointerMoved) {
        this.chatSidebarDragActive = false;
        this._chatSidebarDragRowsCache = null;
        return;
      }
      this._chatSidebarPointerMoved = false;
      var hardBounds = this.chatSidebarHardBounds();
      var lockedWall = this.chatSidebarWallLockNormalized();
      var final;
      if (lockedWall) {
        final = this.dragSurfaceApplyWallLock(hardBounds, this.chatSidebarDragLeft, this.chatSidebarDragTop, lockedWall);
      } else {
        final = this.dragSurfaceClampWithBounds(hardBounds, this.chatSidebarDragLeft, this.chatSidebarDragTop);
      }
      this.chatSidebarPlacementAnchorId = '';
      try { localStorage.removeItem('infring-chat-sidebar-placement-anchor'); } catch(_) {}
      this.chatSidebarDragLeft = final.left;
      this.chatSidebarDragTop = final.top;
      this.chatSidebarPersistPlacementFromLeft(this.chatSidebarDragLeft);
      this.chatSidebarPersistPlacementFromTop(this.chatSidebarDragTop);
      this.chatSidebarDragActive = false;
      this._chatSidebarDragRowsCache = null;
      this._sidebarToggleSuppressUntil = Date.now() + 260;
    },

    shouldSuppressSidebarToggle() {
      var until = Number(this._sidebarToggleSuppressUntil || 0);
      return Number.isFinite(until) && until > Date.now();
    },

    popupWindowStorageKey(kind, axis) {
      var key = String(kind || '').trim().toLowerCase();
      var lane = String(axis || '').trim().toLowerCase() === 'top' ? 'top' : 'left';
      return 'infring-popup-window-' + (key || 'manual') + '-' + lane;
    },
    popupWindowWallLockStorageKey(kind) {
      var key = String(kind || '').trim().toLowerCase() || 'manual';
      return 'infring-popup-window-' + key + '-wall-lock';
    },
    popupWindowWallLock(kind) {
      void kind;
      return '';
    },
    popupWindowSetWallLock(kind, wallRaw) {
      var key = String(kind || '').trim().toLowerCase();
      void wallRaw;
      if (!key) return '';
      if (!this.popupWindowWallLocks || typeof this.popupWindowWallLocks !== 'object') {
        this.popupWindowWallLocks = {};
      }
      this.popupWindowWallLocks[key] = '';
      try {
        localStorage.removeItem(this.popupWindowWallLockStorageKey(key));
        localStorage.removeItem('infring-popup-window-' + key + '-smash-wall');
      } catch(_) {}
      return '';
    },

    popupWindowOpenState(kind) {
      var key = String(kind || '').trim().toLowerCase();
      if (key === 'report') return !!this.reportIssueWindowOpen;
      return !!this.helpManualWindowOpen;
    },

    popupWindowSetOpenState(kind, open) {
      var key = String(kind || '').trim().toLowerCase();
      var nextOpen = open !== false;
      if (key === 'report') {
        this.reportIssueWindowOpen = nextOpen;
        return;
      }
      this.helpManualWindowOpen = nextOpen;
    },

    readPopupWindowElement(kind) {
      if (typeof document === 'undefined' || typeof document.querySelector !== 'function') return null;
      var key = String(kind || '').trim().toLowerCase();
      if (!key) return null;
      try {
        return document.querySelector('.popup-window[data-popup-window-kind="' + key + '"]');
      } catch(_) {}
      return null;
    },

    popupWindowDefaultSize(kind) {
      var key = String(kind || '').trim().toLowerCase();
      if (key === 'report') return { width: 540, height: 360 };
      return { width: 760, height: 560 };
    },

    readPopupWindowSize(kind) {
      var node = this.readPopupWindowElement(kind);
      var fallback = this.popupWindowDefaultSize(kind);
      var width = Number(node && node.offsetWidth || 0);
      var height = Number(node && node.offsetHeight || 0);
      if (!Number.isFinite(width) || width <= 0) width = Number(fallback.width || 640);
      if (!Number.isFinite(height) || height <= 0) height = Number(fallback.height || 420);
      return {
        width: Math.max(280, Math.round(width)),
        height: Math.max(180, Math.round(height))
      };
    },

    popupWindowBounds(kind, widthRaw, heightRaw) {
      void kind;
      var wallGap = this.overlayWallGapPx();
      var width = Number(widthRaw || 0);
      var height = Number(heightRaw || 0);
      if (!Number.isFinite(width) || width <= 0) width = 640;
      if (!Number.isFinite(height) || height <= 0) height = 420;
      var minLeft = wallGap;
      var maxLeft = this.chatOverlayViewportWidth() - wallGap - width;
      if (!Number.isFinite(maxLeft) || maxLeft < minLeft) maxLeft = minLeft;
      var vertical = this.chatOverlayVerticalBounds();
      var minTop = Number(vertical && vertical.minTop || wallGap) + 2;
      var maxTop = Number(vertical && vertical.maxBottom || this.taskbarReadViewportHeight()) - wallGap - height;
      if (!Number.isFinite(maxTop) || maxTop < minTop) maxTop = minTop;
      return {
        minLeft: minLeft,
        maxLeft: maxLeft,
        minTop: minTop,
        maxTop: maxTop
      };
    },

    popupWindowClampPlacement(kind, leftRaw, topRaw) {
      var size = this.readPopupWindowSize(kind);
      var bounds = this.popupWindowBounds(kind, size.width, size.height);
      var left = Number(leftRaw);
      var top = Number(topRaw);
      if (!Number.isFinite(left)) left = bounds.minLeft + ((bounds.maxLeft - bounds.minLeft) * 0.5);
      if (!Number.isFinite(top)) top = bounds.minTop + ((bounds.maxTop - bounds.minTop) * 0.48);
      return {
        left: Math.max(bounds.minLeft, Math.min(bounds.maxLeft, left)),
        top: Math.max(bounds.minTop, Math.min(bounds.maxTop, top))
      };
    },
    popupWindowHardBounds(kind) {
      var size = this.readPopupWindowSize(kind);
      return this.dragSurfaceHardBounds(size.width, size.height);
    },

    popupWindowEnsurePlacement(kind, forceCenter) {
      var key = String(kind || '').trim().toLowerCase();
      if (!key) return { left: 0, top: 0 };
      if (forceCenter) {
        var centerSize = this.readPopupWindowSize(key);
        var centerBounds = this.popupWindowBounds(key, centerSize.width, centerSize.height);
        var centerPoint = this.dragSurfaceCenteredPoint(centerBounds);
        var centered = this.popupWindowClampPlacement(key, centerPoint.left, centerPoint.top);
        if (!this.popupWindowPlacements || typeof this.popupWindowPlacements !== 'object') {
          this.popupWindowPlacements = {};
        }
        this.popupWindowPlacements[key] = { left: centered.left, top: centered.top };
        return centered;
      }
      var map = (this.popupWindowPlacements && typeof this.popupWindowPlacements === 'object')
        ? this.popupWindowPlacements
        : {};
      var row = map[key] && typeof map[key] === 'object' ? map[key] : { left: null, top: null };
      var left = Number(row.left);
      var top = Number(row.top);
      var hasStored = Number.isFinite(left) && Number.isFinite(top);
      if (!hasStored) {
        try {
          left = Number(localStorage.getItem(this.popupWindowStorageKey(key, 'left')));
          top = Number(localStorage.getItem(this.popupWindowStorageKey(key, 'top')));
        } catch(_) {}
      }
      if (!Number.isFinite(left) || !Number.isFinite(top)) {
        var size = this.readPopupWindowSize(key);
        var bounds = this.popupWindowBounds(key, size.width, size.height);
        left = bounds.minLeft + ((bounds.maxLeft - bounds.minLeft) * 0.5);
        top = bounds.minTop + ((bounds.maxTop - bounds.minTop) * (key === 'report' ? 0.56 : 0.44));
      }
      var clamped = this.popupWindowClampPlacement(key, left, top);
      if (!this.popupWindowPlacements || typeof this.popupWindowPlacements !== 'object') {
        this.popupWindowPlacements = {};
      }
      this.popupWindowPlacements[key] = { left: clamped.left, top: clamped.top };
      return clamped;
    },

    popupWindowPersistPlacement(kind, leftRaw, topRaw) {
      var key = String(kind || '').trim().toLowerCase();
      if (!key) return;
      var clamped = this.popupWindowClampPlacement(key, leftRaw, topRaw);
      if (!this.popupWindowPlacements || typeof this.popupWindowPlacements !== 'object') {
        this.popupWindowPlacements = {};
      }
      this.popupWindowPlacements[key] = { left: clamped.left, top: clamped.top };
      try {
        localStorage.setItem(this.popupWindowStorageKey(key, 'left'), String(clamped.left));
        localStorage.setItem(this.popupWindowStorageKey(key, 'top'), String(clamped.top));
      } catch(_) {}
    },

    popupWindowResolvedLeft(kind) {
      var key = String(kind || '').trim().toLowerCase();
      if (!key) return this.overlayWallGapPx();
      if (this.popupWindowDragActive && this.popupWindowDragKind === key) {
        return Number(this.popupWindowDragLeft || 0);
      }
      var base = this.popupWindowEnsurePlacement(key);
      return this.popupWindowClampPlacement(key, base.left, base.top).left;
    },

    popupWindowResolvedTop(kind) {
      var key = String(kind || '').trim().toLowerCase();
      if (!key) return this.overlayWallGapPx();
      if (this.popupWindowDragActive && this.popupWindowDragKind === key) {
        return Number(this.popupWindowDragTop || 0);
      }
      var base = this.popupWindowEnsurePlacement(key);
      return this.popupWindowClampPlacement(key, base.left, base.top).top;
    },

    popupWindowStyle(kind) {
      var key = String(kind || '').trim().toLowerCase();
      if (!key || !this.popupWindowOpenState(key)) return 'display:none;';
      var left = this.popupWindowResolvedLeft(key);
      var top = this.popupWindowResolvedTop(key);
      var durationMs = (this.popupWindowDragActive && this.popupWindowDragKind === key)
        ? 0
        : this.dragSurfaceMoveDurationMs(this._popupWindowMoveDurationMs, 260);
      return (
        'left:' + Math.round(left) + 'px;' +
        'top:' + Math.round(top) + 'px;' +
        'transition:left ' + Math.max(0, Math.round(durationMs)) + 'ms var(--ease-smooth), top ' + Math.max(0, Math.round(durationMs)) + 'ms var(--ease-smooth);'
      );
    },

    openPopupWindow(kind) {
      var key = String(kind || '').trim().toLowerCase();
      if (!key) return;
      this.popupWindowSetOpenState(key, true);
      this.popupWindowSetWallLock(key, '');
      this.popupWindowEnsurePlacement(key, true);
      var self = this;
      this.$nextTick(function() {
        self.popupWindowEnsurePlacement(key, true);
      });
    },

    closePopupWindow(kind) {
      var key = String(kind || '').trim().toLowerCase();
      if (!key) return;
      if (this._popupWindowPointerActive && this.popupWindowDragKind === key) {
        this.endPopupWindowPointerDrag();
      }
      this.popupWindowSetOpenState(key, false);
    },

    bindPopupWindowPointerListeners() {
      if (this._popupWindowPointerMoveHandler || this._popupWindowPointerUpHandler) return;
      var self = this;
      this._popupWindowPointerMoveHandler = function(ev) { self.handlePopupWindowPointerMove(ev); };
      this._popupWindowPointerUpHandler = function() { self.endPopupWindowPointerDrag(); };
      window.addEventListener('pointermove', this._popupWindowPointerMoveHandler, true);
      window.addEventListener('pointerup', this._popupWindowPointerUpHandler, true);
      window.addEventListener('pointercancel', this._popupWindowPointerUpHandler, true);
      window.addEventListener('mousemove', this._popupWindowPointerMoveHandler, true);
      window.addEventListener('mouseup', this._popupWindowPointerUpHandler, true);
    },

    unbindPopupWindowPointerListeners() {
      if (this._popupWindowPointerMoveHandler) {
        try { window.removeEventListener('pointermove', this._popupWindowPointerMoveHandler, true); } catch(_) {}
        try { window.removeEventListener('mousemove', this._popupWindowPointerMoveHandler, true); } catch(_) {}
      }
      if (this._popupWindowPointerUpHandler) {
        try { window.removeEventListener('pointerup', this._popupWindowPointerUpHandler, true); } catch(_) {}
        try { window.removeEventListener('pointercancel', this._popupWindowPointerUpHandler, true); } catch(_) {}
        try { window.removeEventListener('mouseup', this._popupWindowPointerUpHandler, true); } catch(_) {}
      }
      this._popupWindowPointerMoveHandler = null;
      this._popupWindowPointerUpHandler = null;
    },

    startPopupWindowPointerDrag(kind, ev) {
      var key = String(kind || '').trim().toLowerCase();
      if (!ev || !key || !this.popupWindowOpenState(key)) return;
      var button = Number(ev.button);
      if (Number.isFinite(button) && button !== 0) return;
      var target = ev && ev.target ? ev.target : null;
      if (target && typeof target.closest === 'function') {
        if (target.closest('button, input, textarea, select, a, [contenteditable="true"]')) return;
      }
      this._popupWindowPointerActive = true;
      this._popupWindowPointerMoved = false;
      this.popupWindowDragKind = key;
      this._popupWindowPointerStartX = Number(ev.clientX || 0);
      this._popupWindowPointerStartY = Number(ev.clientY || 0);
      this._popupWindowPointerOriginLeft = this.popupWindowResolvedLeft(key);
      this._popupWindowPointerOriginTop = this.popupWindowResolvedTop(key);
      this._popupWindowPointerLastX = this._popupWindowPointerStartX;
      this._popupWindowPointerLastY = this._popupWindowPointerStartY;
      this._popupWindowPointerLastAt = Date.now();
      this._popupWindowPointerVelocity = 0;
      this.popupWindowDragLeft = this._popupWindowPointerOriginLeft;
      this.popupWindowDragTop = this._popupWindowPointerOriginTop;
      this.popupWindowDragWallLock = '';
      this.bindPopupWindowPointerListeners();
      try {
        if (ev.currentTarget && typeof ev.currentTarget.setPointerCapture === 'function' && Number.isFinite(ev.pointerId)) {
          ev.currentTarget.setPointerCapture(ev.pointerId);
        }
      } catch(_) {}
    },

    handlePopupWindowPointerMove(ev) {
      if (!this._popupWindowPointerActive) return;
      var key = String(this.popupWindowDragKind || '').trim().toLowerCase();
      if (!key || !this.popupWindowOpenState(key)) return;
      var nextX = Number(ev.clientX || 0);
      var nextY = Number(ev.clientY || 0);
      var now = Date.now();
      var prevX = Number(this._popupWindowPointerLastX || nextX);
      var prevY = Number(this._popupWindowPointerLastY || nextY);
      var prevAt = Number(this._popupWindowPointerLastAt || now);
      var dt = Math.max(1, now - prevAt);
      var stepDx = nextX - prevX;
      var stepDy = nextY - prevY;
      this._popupWindowPointerVelocity = Math.sqrt((stepDx * stepDx) + (stepDy * stepDy)) / dt;
      this._popupWindowPointerLastX = nextX;
      this._popupWindowPointerLastY = nextY;
      this._popupWindowPointerLastAt = now;
      var movedX = Math.abs(nextX - Number(this._popupWindowPointerStartX || 0));
      var movedY = Math.abs(nextY - Number(this._popupWindowPointerStartY || 0));
      if (!this._popupWindowPointerMoved) {
        if (movedX < 4 && movedY < 4) return;
        this._popupWindowPointerMoved = true;
        this.popupWindowDragActive = true;
      }
      var dragDx = nextX - Number(this._popupWindowPointerStartX || 0);
      var dragDy = nextY - Number(this._popupWindowPointerStartY || 0);
      var candidateLeft = Number(this._popupWindowPointerOriginLeft || 0) + dragDx;
      var candidateTop = Number(this._popupWindowPointerOriginTop || 0) + dragDy;
      var hardBounds = this.popupWindowHardBounds(key);
      var clamped = this.dragSurfaceClampWithBounds(hardBounds, candidateLeft, candidateTop);
      this.popupWindowDragLeft = clamped.left;
      this.popupWindowDragTop = clamped.top;
      if (ev.cancelable && typeof ev.preventDefault === 'function') ev.preventDefault();
    },

    endPopupWindowPointerDrag() {
      if (!this._popupWindowPointerActive) return;
      var key = String(this.popupWindowDragKind || '').trim().toLowerCase();
      var moved = !!this._popupWindowPointerMoved;
      this._popupWindowPointerActive = false;
      this._popupWindowPointerMoved = false;
      this.unbindPopupWindowPointerListeners();
      if (key && moved) {
        var hardBounds = this.popupWindowHardBounds(key);
        var finalPlacement = this.dragSurfaceClampWithBounds(hardBounds, this.popupWindowDragLeft, this.popupWindowDragTop);
        this.popupWindowDragLeft = finalPlacement.left;
        this.popupWindowDragTop = finalPlacement.top;
        this.popupWindowPersistPlacement(key, this.popupWindowDragLeft, this.popupWindowDragTop);
      }
      this.popupWindowDragActive = false;
      this.popupWindowDragWallLock = '';
      this.popupWindowDragKind = '';
    },

    bottomDockDefaultOrder() {
      var service = this.taskbarDockService();
      if (service && typeof service.dockDefaultOrder === 'function') return service.dockDefaultOrder(this.bottomDockTileConfig);
      var registry = (this.bottomDockTileConfig && typeof this.bottomDockTileConfig === 'object')
        ? this.bottomDockTileConfig
        : null;
      if (registry) {
        var ids = Object.keys(registry);
        if (ids.length) return ids;
      }
      return ['chat', 'overview', 'agents', 'scheduler', 'skills', 'runtime', 'settings'];
    },

    bottomDockTileConfigById(id) {
      var key = String(id || '').trim();
      if (!key) return null;
      var registry = (this.bottomDockTileConfig && typeof this.bottomDockTileConfig === 'object')
        ? this.bottomDockTileConfig
        : null;
      var tile = registry && Object.prototype.hasOwnProperty.call(registry, key) ? registry[key] : null;
      return tile && typeof tile === 'object' ? tile : null;
    },

    bottomDockTileData(id, field, fallback) {
      var key = String(field || '').trim();
      var tile = this.bottomDockTileConfigById(id);
      var value = (key && tile && Object.prototype.hasOwnProperty.call(tile, key)) ? tile[key] : fallback;
      return (value === undefined || value === null) ? String(fallback || '') : String(value);
    },

    bottomDockTileAnimationName(id) {
      var tile = this.bottomDockTileConfigById(id);
      var animation = tile && Array.isArray(tile.animation) ? tile.animation : null;
      var name = animation ? String(animation[0] || '').trim() : '';
      return name || 'none';
    },

    bottomDockTileAnimationDurationAttr(id) {
      var tile = this.bottomDockTileConfigById(id);
      var animation = tile && Array.isArray(tile.animation) ? tile.animation : null;
      if (!animation) return null;
      var durationMs = Number(animation[1]);
      if (!Number.isFinite(durationMs) || durationMs < 120) return null;
      return String(Math.round(durationMs));
    },

    bottomDockSlotStyle(id) {
      var key = String(id || '').trim();
      var weight = this.bottomDockHoverWeight(key);
      var service = this.taskbarDockService();
      if (service && typeof service.dockSlotStyle === 'function') {
        return service.dockSlotStyle(key, this.bottomDockOrder, weight, this.bottomDockTileConfig);
      }
      var order = key ? this.bottomDockOrderIndex(key) : 999;
      if (!Number.isFinite(weight) || weight < 0) weight = 0;
      if (weight > 1) weight = 1;
      return 'order:' + order + ';--bottom-dock-hover-weight:' + weight.toFixed(4);
    },

    bottomDockTileStyle(id) {
      var key = String(id || '').trim();
      var tile = this.bottomDockTileConfigById(key);
      var style = tile && typeof tile.style === 'string' ? String(tile.style || '').trim() : '';
      return style || '';
    },

    normalizeBottomDockOrder(rawOrder) {
      var service = this.taskbarDockService();
      if (service && typeof service.normalizeOrder === 'function') return service.normalizeOrder(rawOrder, this.bottomDockDefaultOrder());
      var defaults = this.bottomDockDefaultOrder();
      var source = Array.isArray(rawOrder) ? rawOrder : [];
      var seen = {};
      var ordered = [];
      for (var i = 0; i < source.length; i++) {
        var id = String(source[i] || '').trim();
        if (!id || seen[id] || defaults.indexOf(id) < 0) continue;
        seen[id] = true;
        ordered.push(id);
      }
      for (var j = 0; j < defaults.length; j++) {
        var fallbackId = defaults[j];
        if (seen[fallbackId]) continue;
        seen[fallbackId] = true;
        ordered.push(fallbackId);
      }
      return ordered;
    },

    persistBottomDockOrder() {
      this.bottomDockOrder = this.normalizeBottomDockOrder(this.bottomDockOrder);
      try {
        var service = this.taskbarDockService();
        if (service && typeof service.persistDockOrder === 'function') this.bottomDockOrder = service.persistDockOrder(this.bottomDockOrder, this.bottomDockTileConfig);
        else localStorage.setItem('infring-bottom-dock-order', JSON.stringify(this.bottomDockOrder));
      } catch(_) {}
      infringUpdateShellLayoutConfig(function(config) {
        config.dock.order = this.bottomDockOrder.slice();
      }.bind(this));
    },

    bottomDockOrderIndex(id) {
      var key = String(id || '').trim();
      if (!key) return 999;
      var service = this.taskbarDockService();
      if (service && typeof service.orderIndex === 'function') {
        return service.orderIndex(key, this.bottomDockOrder, this.bottomDockDefaultOrder());
      }
      var order = this.normalizeBottomDockOrder(this.bottomDockOrder);
      var idx = order.indexOf(key);
      if (idx >= 0) return idx;
      var fallback = this.bottomDockDefaultOrder().indexOf(key);
      return fallback >= 0 ? fallback : 999;
    },

    bottomDockAxisBasis(sideHint) {
      var rotationDeg = this.bottomDockRotationDegResolved(sideHint);
      var theta = (Number(rotationDeg || 0) * Math.PI) / 180;
      var ux = Math.cos(theta);
      var uy = Math.sin(theta);
      if (Math.abs(ux) < 0.0001) ux = 0;
      if (Math.abs(uy) < 0.0001) uy = 0;
      return { ux: ux, uy: uy, vx: -uy, vy: ux };
    },

    bottomDockProjectPointToAxis(x, y, basis) {
      var axis = basis && typeof basis === 'object'
        ? basis
        : this.bottomDockAxisBasis();
      var ux = Number(axis.ux || 0);
      var uy = Number(axis.uy || 0);
      var vx = Number(axis.vx || (-uy));
      var vy = Number(axis.vy || ux);
      var px = Number(x || 0);
      var py = Number(y || 0);
      return {
        primary: (px * ux) + (py * uy),
        secondary: (px * vx) + (py * vy)
      };
    },

    bottomDockAxisHalfExtent(width, height, basis) {
      var axis = basis && typeof basis === 'object'
        ? basis
        : this.bottomDockAxisBasis();
      var w = Number(width || 0);
      var h = Number(height || 0);
      if (!Number.isFinite(w) || w < 0) w = 0;
      if (!Number.isFinite(h) || h < 0) h = 0;
      var ux = Math.abs(Number(axis.ux || 0));
      var uy = Math.abs(Number(axis.uy || 0));
      var vx = Math.abs(Number(axis.vx || 0));
      var vy = Math.abs(Number(axis.vy || 0));
      return {
        primary: ((ux * w) + (uy * h)) / 2,
        secondary: ((vx * w) + (vy * h)) / 2
      };
    },

    bottomDockProjectedRectBounds(rect, basis) {
      if (!rect) return null;
      var axis = basis && typeof basis === 'object'
        ? basis
        : this.bottomDockAxisBasis();
      var left = Number(rect.left || 0);
      var top = Number(rect.top || 0);
      var right = Number(rect.right || left);
      var bottom = Number(rect.bottom || top);
      var p1 = this.bottomDockProjectPointToAxis(left, top, axis);
      var p2 = this.bottomDockProjectPointToAxis(right, top, axis);
      var p3 = this.bottomDockProjectPointToAxis(left, bottom, axis);
      var p4 = this.bottomDockProjectPointToAxis(right, bottom, axis);
      var primaryMin = Math.min(p1.primary, p2.primary, p3.primary, p4.primary);
      var primaryMax = Math.max(p1.primary, p2.primary, p3.primary, p4.primary);
      var secondaryMin = Math.min(p1.secondary, p2.secondary, p3.secondary, p4.secondary);
      var secondaryMax = Math.max(p1.secondary, p2.secondary, p3.secondary, p4.secondary);
      return {
        primaryMin: primaryMin,
        primaryMax: primaryMax,
        secondaryMin: secondaryMin,
        secondaryMax: secondaryMax
      };
    },

    bottomDockButtonRects() {
      var out = {};
      var root = document.querySelector('.bottom-dock');
      if (!root) return out;
      var nodes = root.querySelectorAll('.bottom-dock-btn[data-dock-id]');
      for (var i = 0; i < nodes.length; i++) {
        var node = nodes[i];
        if (!node) continue;
        var id = String(node.getAttribute('data-dock-id') || '').trim();
        if (!id) continue;
        var rect = node.getBoundingClientRect();
        var width = Number(rect.width || 0);
        var height = Number(rect.height || 0);
        var left = Number(rect.left || 0);
        var top = Number(rect.top || 0);
        out[id] = {
          left: left,
          top: top,
          width: width,
          height: height,
          cx: left + (width / 2),
          cy: top + (height / 2)
        };
      }
      return out;
    },

    animateBottomDockFromRects(beforeRects) {
      if (!beforeRects || typeof beforeRects !== 'object') return;
      if (typeof requestAnimationFrame !== 'function') return;
      var durationMs = this.bottomDockMoveDurationMs();
      var self = this;
      requestAnimationFrame(function() {
        var root = document.querySelector('.bottom-dock');
        if (!root) return;
        var rootScale = self.readBottomDockScale(root);
        if (!Number.isFinite(rootScale) || rootScale <= 0.01) rootScale = 1;
        var side = self.bottomDockActiveSide();
        var nodes = root.querySelectorAll('.bottom-dock-btn[data-dock-id]');
        for (var i = 0; i < nodes.length; i++) {
          var node = nodes[i];
          if (!node || node.classList.contains('dragging')) continue;
          var id = String(node.getAttribute('data-dock-id') || '').trim();
          if (!id || !Object.prototype.hasOwnProperty.call(beforeRects, id)) continue;
          var from = beforeRects[id] || {};
          var rect = node.getBoundingClientRect();
          var fromCx = Number(from.cx);
          var fromCy = Number(from.cy);
          if (!Number.isFinite(fromCx)) fromCx = Number(from.left || 0) + (Number(from.width || 0) / 2);
          if (!Number.isFinite(fromCy)) fromCy = Number(from.top || 0) + (Number(from.height || 0) / 2);
          var toCx = Number(rect.left || 0) + (Number(rect.width || 0) / 2);
          var toCy = Number(rect.top || 0) + (Number(rect.height || 0) / 2);
          var screenDx = Number(fromCx || 0) - Number(toCx || 0);
          var screenDy = Number(fromCy || 0) - Number(toCy || 0);
          if (Math.abs(screenDx) < 0.5 && Math.abs(screenDy) < 0.5) continue;
          var localDelta = self.bottomDockScreenDeltaToLocal(screenDx, screenDy, side);
          var tx = Number(localDelta.x || 0) / rootScale;
          var ty = Number(localDelta.y || 0) / rootScale;
          if (Math.abs(tx) < 0.25 && Math.abs(ty) < 0.25) continue;
          node.style.setProperty('--dock-reorder-transition', '0ms');
          node.style.setProperty('--dock-reorder-translate-x', Math.round(tx) + 'px');
          node.style.setProperty('--dock-reorder-translate-y', Math.round(ty) + 'px');
          void node.offsetHeight;
          node.style.setProperty('--dock-reorder-transition', Math.max(0, Math.round(durationMs)) + 'ms');
          node.style.setProperty('--dock-reorder-translate-x', '0px');
          node.style.setProperty('--dock-reorder-translate-y', '0px');
          (function(el) {
            window.setTimeout(function() {
              if (
                !el.classList.contains('dragging') &&
                !el.classList.contains('hovered') &&
                !el.classList.contains('neighbor-hover') &&
                !el.classList.contains('second-neighbor-hover')
              ) {
                el.style.removeProperty('--dock-reorder-translate-x');
                el.style.removeProperty('--dock-reorder-translate-y');
              }
              el.style.removeProperty('--dock-reorder-transition');
            }, durationMs + 30);
          })(node);
        }
      });
    },

    setBottomDockHover(id, ev) {
      if (String(this.bottomDockDragId || '').trim()) return;
      if (this.bottomDockContainerDragActive || this._bottomDockContainerPointerActive) return;
      var key = String(id || '').trim();
      this.bottomDockHoverId = key;
      if (ev) {
        var evX = Number(ev.clientX || 0);
        var evY = Number(ev.clientY || 0);
        if (Number.isFinite(evX) && evX > 0) this.bottomDockPointerX = evX;
        if (Number.isFinite(evY) && evY > 0) this.bottomDockPointerY = evY;
      }
      if (this._bottomDockPreviewHideTimer) {
        try { clearTimeout(this._bottomDockPreviewHideTimer); } catch(_) {}
        this._bottomDockPreviewHideTimer = 0;
      }
      if (!Number.isFinite(this.bottomDockPointerX) || this.bottomDockPointerX <= 0) {
        try {
          var slot = document.querySelector('.bottom-dock .dock-tile-slot[data-dock-slot-id="' + key + '"]');
          if (slot && typeof slot.getBoundingClientRect === 'function') {
            var slotRect = slot.getBoundingClientRect();
            this.bottomDockPointerX = Number(slotRect.left || 0) + (Number(slotRect.width || 0) / 2);
            this.bottomDockPointerY = Number(slotRect.top || 0) + (Number(slotRect.height || 0) / 2);
          }
        } catch(_) {}
      }
      this.refreshBottomDockHoverWeights();
      this.syncBottomDockPreview();
      this.scheduleBottomDockPreviewReflow();
    },

    clearBottomDockHover(id) {
      if (id) return;
      this.bottomDockHoverId = '';
      if (!this.bottomDockHoverId) {
        this.bottomDockHoverWeightById = {};
        this.bottomDockPointerX = 0;
        this.bottomDockPointerY = 0;
        this.cancelBottomDockPreviewReflow();
        var self = this;
        if (this._bottomDockPreviewHideTimer) {
          try { clearTimeout(this._bottomDockPreviewHideTimer); } catch(_) {}
        }
        this._bottomDockPreviewHideTimer = window.setTimeout(function() {
          self._bottomDockPreviewHideTimer = 0;
          if (!String(self.bottomDockHoverId || '').trim()) {
            self.bottomDockPreviewVisible = false;
            self.bottomDockPreviewText = '';
            self.bottomDockPreviewMorphFromText = '';
            self.bottomDockPreviewLabelMorphing = false;
            self.bottomDockPreviewWidth = 0;
          }
        }, 40);
        return;
      }
      this.syncBottomDockPreview();
    },

    readBottomDockSlotCenters() {
      var out = [];
      if (typeof document === 'undefined') return out;
      var root = document.querySelector('.bottom-dock');
      if (!root || typeof root.querySelectorAll !== 'function') return out;
      var nodes = root.querySelectorAll('.dock-tile-slot[data-dock-slot-id]');
      for (var i = 0; i < nodes.length; i += 1) {
        var node = nodes[i];
        if (!node || typeof node.getAttribute !== 'function' || typeof node.getBoundingClientRect !== 'function') continue;
        var id = String(node.getAttribute('data-dock-slot-id') || '').trim();
        if (!id) continue;
        var rect = node.getBoundingClientRect();
        var centerX = Number(rect.left || 0) + (Number(rect.width || 0) / 2);
        var centerY = Number(rect.top || 0) + (Number(rect.height || 0) / 2);
        if (!Number.isFinite(centerX) || !Number.isFinite(centerY)) continue;
        out.push({ id: id, centerX: centerX, centerY: centerY });
      }
      return out;
    },

    bottomDockWeightForDistance(distancePx) {
      var d = Math.abs(Number(distancePx || 0));
      if (!Number.isFinite(d)) return 0;
      var sigma = 52;
      var exponent = -((d * d) / (2 * sigma * sigma));
      var weight = Math.exp(exponent);
      if (!Number.isFinite(weight) || weight < 0.008) return 0;
      if (weight > 1) return 1;
      return weight;
    },

    refreshBottomDockHoverWeights() {
      var side = this.bottomDockActiveSide();
      var vertical = this.bottomDockIsVerticalSide(side);
      var primaryPointer = vertical
        ? Number(this.bottomDockPointerY || 0)
        : Number(this.bottomDockPointerX || 0);
      if (!Number.isFinite(primaryPointer) || primaryPointer <= 0) {
        this.bottomDockHoverWeightById = {};
        return;
      }
      var centers = this.readBottomDockSlotCenters();
      if (!centers.length) {
        this.bottomDockHoverWeightById = {};
        return;
      }
      var nearestId = '';
      var nearestDistance = Number.POSITIVE_INFINITY;
      var weights = {};
      for (var i = 0; i < centers.length; i += 1) {
        var item = centers[i];
        if (!item || !item.id) continue;
        var anchor = vertical ? Number(item.centerY || 0) : Number(item.centerX || 0);
        var dist = Math.abs(primaryPointer - anchor);
        if (!Number.isFinite(dist)) continue;
        if (dist < nearestDistance) {
          nearestDistance = dist;
          nearestId = item.id;
        }
        weights[item.id] = this.bottomDockWeightForDistance(dist);
      }
      this.bottomDockHoverWeightById = weights;
      if (nearestId) this.bottomDockHoverId = nearestId;
    },

    updateBottomDockPointer(ev) {
      if (!ev) return;
      if (String(this.bottomDockDragId || '').trim()) return;
      if (this.bottomDockContainerDragActive || this._bottomDockContainerPointerActive) return;
      var x = Number(ev.clientX || 0);
      var y = Number(ev.clientY || 0);
      if (!Number.isFinite(x) || x <= 0) return;
      this.bottomDockPointerX = x;
      if (Number.isFinite(y) && y > 0) this.bottomDockPointerY = y;
      this.refreshBottomDockHoverWeights();
      this.syncBottomDockPreview();
    },

    reviveBottomDockHoverFromPoint(clientX, clientY) {
      if (String(this.bottomDockDragId || '').trim()) return;
      if (this.bottomDockContainerDragActive || this._bottomDockContainerPointerActive) return;
      var x = Number(clientX || 0);
      var y = Number(clientY || 0);
      if (!Number.isFinite(x) || !Number.isFinite(y) || x <= 0 || y <= 0) return;
      var root = document.querySelector('.bottom-dock');
      if (!root || typeof root.getBoundingClientRect !== 'function') return;
      var rect = root.getBoundingClientRect();
      var withinX = x >= (Number(rect.left || 0) - 16) && x <= (Number(rect.right || 0) + 16);
      var withinY = y >= (Number(rect.top || 0) - 18) && y <= (Number(rect.bottom || 0) + 18);
      if (!withinX || !withinY) return;
      this.bottomDockPointerX = x;
      this.bottomDockPointerY = y;
      this.refreshBottomDockHoverWeights();
      this.syncBottomDockPreview();
      this.scheduleBottomDockPreviewReflow();
    },

    scheduleBottomDockPreviewReflow() {
      this.cancelBottomDockPreviewReflow();
      var self = this;
      this._bottomDockPreviewReflowFrames = 10;
      var step = function() {
        if (!String(self.bottomDockHoverId || '').trim()) {
          self._bottomDockPreviewReflowRaf = 0;
          self._bottomDockPreviewReflowFrames = 0;
          return;
        }
        self.syncBottomDockPreview();
        self._bottomDockPreviewReflowFrames = Math.max(0, Number(self._bottomDockPreviewReflowFrames || 0) - 1);
        if (self._bottomDockPreviewReflowFrames <= 0) {
          self._bottomDockPreviewReflowRaf = 0;
          return;
        }
        self._bottomDockPreviewReflowRaf = requestAnimationFrame(step);
      };
      this._bottomDockPreviewReflowRaf = requestAnimationFrame(step);
    },

    cancelBottomDockPreviewReflow() {
      if (this._bottomDockPreviewReflowRaf && typeof cancelAnimationFrame === 'function') {
        try { cancelAnimationFrame(this._bottomDockPreviewReflowRaf); } catch(_) {}
      }
      this._bottomDockPreviewReflowRaf = 0;
      this._bottomDockPreviewReflowFrames = 0;
    },

    syncBottomDockPreview() {
      var key = String(this.bottomDockHoverId || '').trim();
      if (!key) {
        this.bottomDockPreviewVisible = false;
        this.bottomDockPreviewText = '';
        this.bottomDockPreviewMorphFromText = '';
        this.bottomDockPreviewHoverKey = '';
        this.bottomDockPreviewLabelMorphing = false;
        this.bottomDockPreviewWidth = 0;
        this.bottomDockPreviewLabelFxReady = true;
        return;
      }
      var text = this.bottomDockTileData(key, 'tooltip', '');
      var label = String(text || '').trim();
      if (!label) {
        this.bottomDockPreviewVisible = false;
        this.bottomDockPreviewText = '';
        this.bottomDockPreviewMorphFromText = '';
        this.bottomDockPreviewHoverKey = '';
        this.bottomDockPreviewLabelMorphing = false;
        this.bottomDockPreviewWidth = 0;
        this.bottomDockPreviewLabelFxReady = true;
        return;
      }
      var root = document.querySelector('.bottom-dock');
      var slot = document.querySelector('.bottom-dock .dock-tile-slot[data-dock-slot-id="' + key + '"]');
      if (!root || !slot) {
        this.bottomDockPreviewVisible = false;
        this.bottomDockPreviewText = '';
        this.bottomDockPreviewMorphFromText = '';
        this.bottomDockPreviewHoverKey = '';
        this.bottomDockPreviewLabelMorphing = false;
        this.bottomDockPreviewWidth = 0;
        this.bottomDockPreviewLabelFxReady = true;
        return;
      }
      var centerX = 0;
      var centerY = 0;
      var anchorY = 0;
      var anchorX = 0;
      var wallSide = this.bottomDockWallSide();
      var openSide = this.bottomDockOpenSide();
      var vertical = this.bottomDockIsVerticalSide(wallSide);
      var dockRect = (typeof root.getBoundingClientRect === 'function')
        ? root.getBoundingClientRect()
        : null;
      if (typeof slot.getBoundingClientRect === 'function' && dockRect) {
        var slotRect = slot.getBoundingClientRect();
        centerX = Number(slotRect.left || 0) + (Number(slotRect.width || 0) / 2);
        centerY = Number(slotRect.top || 0) + (Number(slotRect.height || 0) / 2);
        if (openSide === 'top') {
          anchorY = Number(dockRect.top || 0) - 8;
        } else if (openSide === 'bottom') {
          anchorY = Number(dockRect.bottom || 0) + 8;
        } else if (openSide === 'left') {
          anchorX = Number(dockRect.left || 0) - 8;
        } else {
          anchorX = Number(dockRect.right || 0) + 8;
        }
      } else if (slot.offsetParent === root) {
        var rootRect = root.getBoundingClientRect();
        centerX = Number(rootRect.left || 0) + Number(slot.offsetLeft || 0) + (Number(slot.offsetWidth || 0) / 2);
        centerY = Number(rootRect.top || 0) + Number(slot.offsetTop || 0) + (Number(slot.offsetHeight || 0) / 2);
        if (openSide === 'top') {
          anchorY = Number(rootRect.top || 0) - 8;
        } else if (openSide === 'bottom') {
          anchorY = Number(rootRect.bottom || 0) + 8;
        } else if (openSide === 'left') {
          anchorX = Number(rootRect.left || 0) - 8;
        } else {
          anchorX = Number(rootRect.right || 0) + 8;
        }
      }
      var pointerX = Number(this.bottomDockPointerX || 0);
      var pointerY = Number(this.bottomDockPointerY || 0);
      if (!vertical && Number.isFinite(pointerX) && pointerX > 0) {
        if (dockRect) {
          var minX = Number(dockRect.left || 0);
          var maxX = Number(dockRect.right || 0);
          if (Number.isFinite(minX) && Number.isFinite(maxX) && maxX > minX) {
            pointerX = Math.max(minX, Math.min(maxX, pointerX));
          }
        }
        centerX = pointerX;
      }
      if (vertical && Number.isFinite(pointerY) && pointerY > 0) {
        if (dockRect) {
          var minY = Number(dockRect.top || 0);
          var maxY = Number(dockRect.bottom || 0);
          if (Number.isFinite(minY) && Number.isFinite(maxY) && maxY > minY) {
            pointerY = Math.max(minY, Math.min(maxY, pointerY));
          }
        }
        centerY = pointerY;
      }
      if (!Number.isFinite(centerX)) centerX = 0;
      if (!Number.isFinite(centerY)) centerY = 0;
      if (!Number.isFinite(anchorX)) anchorX = 0;
      if (!Number.isFinite(anchorY)) anchorY = 0;
      this.bottomDockPreviewX = vertical ? anchorX : centerX;
      this.bottomDockPreviewY = vertical ? centerY : anchorY;
      this.bottomDockPreviewHoverKey = key;
      this.bottomDockPreviewVisible = true;
      this.bottomDockPreviewText = label;
      this.bottomDockPreviewMorphFromText = '';
      this.bottomDockPreviewLabelMorphing = false;
      this.bottomDockPreviewWidth = 0;
      this.bottomDockPreviewLabelFxReady = true;
    },

    bindBottomDockPointerListeners() {
      if (this._bottomDockPointerMoveHandler || this._bottomDockPointerUpHandler) return;
      var self = this;
      this._bottomDockPointerMoveHandler = function(ev) { self.handleBottomDockPointerMove(ev); };
      this._bottomDockPointerUpHandler = function(ev) { self.endBottomDockPointerDrag(ev); };
      window.addEventListener('pointermove', this._bottomDockPointerMoveHandler, true);
      window.addEventListener('pointerup', this._bottomDockPointerUpHandler, true);
      window.addEventListener('pointercancel', this._bottomDockPointerUpHandler, true);
      window.addEventListener('mousemove', this._bottomDockPointerMoveHandler, true);
      window.addEventListener('mouseup', this._bottomDockPointerUpHandler, true);
    },

    unbindBottomDockPointerListeners() {
      if (this._bottomDockPointerMoveHandler) {
        try { window.removeEventListener('pointermove', this._bottomDockPointerMoveHandler, true); } catch(_) {}
        try { window.removeEventListener('mousemove', this._bottomDockPointerMoveHandler, true); } catch(_) {}
      }
      if (this._bottomDockPointerUpHandler) {
        try { window.removeEventListener('pointerup', this._bottomDockPointerUpHandler, true); } catch(_) {}
        try { window.removeEventListener('pointercancel', this._bottomDockPointerUpHandler, true); } catch(_) {}
        try { window.removeEventListener('mouseup', this._bottomDockPointerUpHandler, true); } catch(_) {}
      }
      this._bottomDockPointerMoveHandler = null;
      this._bottomDockPointerUpHandler = null;
    },

    startBottomDockPointerDrag(id, ev) {
      if (!ev || Number(ev.button) !== 0) return;
      if (this.bottomDockContainerDragActive || this._bottomDockContainerPointerActive) return;
      var key = String(id || '').trim();
      if (!key) return;
      var hostEl = ev && ev.currentTarget ? ev.currentTarget : null;
      if (hostEl && typeof hostEl.getBoundingClientRect === 'function') {
        try {
          var rect = hostEl.getBoundingClientRect();
          var width = Number(rect.width || 32);
          var height = Number(rect.height || 32);
          var baseWidth = Number(hostEl && hostEl.offsetWidth ? hostEl.offsetWidth : width || 32);
          var baseHeight = Number(hostEl && hostEl.offsetHeight ? hostEl.offsetHeight : height || 32);
          if (!Number.isFinite(width) || width <= 0) width = 32;
          if (!Number.isFinite(height) || height <= 0) height = 32;
          if (!Number.isFinite(baseWidth) || baseWidth <= 0) baseWidth = width;
          if (!Number.isFinite(baseHeight) || baseHeight <= 0) baseHeight = height;
          var expandedScale = this.bottomDockExpandedScale();
          var expandedWidth = baseWidth * expandedScale;
          var expandedHeight = baseHeight * expandedScale;
          this._bottomDockDragGhostWidth = Math.max(20, Math.min(112, Math.max(width, expandedWidth)));
          this._bottomDockDragGhostHeight = Math.max(20, Math.min(112, Math.max(height, expandedHeight)));
          var offsetX = Number(ev.clientX || 0) - Number(rect.left || 0);
          var offsetY = Number(ev.clientY || 0) - Number(rect.top || 0);
          var relX = Number.isFinite(offsetX) && width > 0 ? (offsetX / width) : 0.5;
          var relY = Number.isFinite(offsetY) && height > 0 ? (offsetY / height) : 0.5;
          relX = Math.max(0, Math.min(1, relX));
          relY = Math.max(0, Math.min(1, relY));
          this._bottomDockPointerGrabOffsetX = relX * this._bottomDockDragGhostWidth;
          this._bottomDockPointerGrabOffsetY = relY * this._bottomDockDragGhostHeight;
        } catch(_) {
          this._bottomDockPointerGrabOffsetX = 16;
          this._bottomDockPointerGrabOffsetY = 16;
          this._bottomDockDragGhostWidth = 32;
          this._bottomDockDragGhostHeight = 32;
        }
      } else {
        this._bottomDockPointerGrabOffsetX = 16;
        this._bottomDockPointerGrabOffsetY = 16;
        this._bottomDockDragGhostWidth = 32;
        this._bottomDockDragGhostHeight = 32;
      }
      try {
        if (hostEl && typeof hostEl.setPointerCapture === 'function' && Number.isFinite(ev.pointerId)) {
          hostEl.setPointerCapture(ev.pointerId);
        }
      } catch(_) {}
      this._bottomDockPointerActive = true;
      this._bottomDockPointerMoved = false;
      this._bottomDockPointerCandidateId = key;
      this._bottomDockPointerStartX = Number(ev.clientX || 0);
      this._bottomDockPointerStartY = Number(ev.clientY || 0);
      this._bottomDockPointerLastX = Number(ev.clientX || 0);
      this._bottomDockPointerLastY = Number(ev.clientY || 0);
      this._bottomDockReorderLockUntil = 0;
      this.bindBottomDockPointerListeners();
    },

    activateBottomDockPointerDrag(ev) {
      if (this._bottomDockPointerMoved) return;
      var dragId = String(this._bottomDockPointerCandidateId || '').trim();
      if (!dragId) return;
      this._bottomDockPointerMoved = true;
      this.bottomDockHoverId = '';
      this.bottomDockHoverWeightById = {};
      this.bottomDockPointerX = 0;
      this.bottomDockPointerY = 0;
      this.bottomDockPreviewVisible = false;
      this.bottomDockPreviewText = '';
      this.bottomDockPreviewMorphFromText = '';
      this.bottomDockPreviewLabelMorphing = false;
      this.bottomDockPreviewWidth = 0;
      this.cancelBottomDockPreviewReflow();
      this._bottomDockRevealTargetDuringSettle = false;
      this.bottomDockDragId = dragId;
      this.bottomDockDragCommitted = false;
      this.bottomDockDragStartOrder = this.normalizeBottomDockOrder(this.bottomDockOrder);
      this.cleanupBottomDockDragGhost();
      this.captureBottomDockDragBoundaries(dragId);
      var originNode = document.querySelector('.bottom-dock-btn[data-dock-id="' + dragId + '"]');
      if (!originNode || !document || !document.body) return;
      var dockEl = document.querySelector('.bottom-dock');
      if (dockEl && dockEl.style && typeof dockEl.style.setProperty === 'function') {
        dockEl.style.setProperty('--bottom-dock-drag-scale', String(this.readBottomDockScale(dockEl)));
      }
      var ghost = document.createElement('div');
      ghost.className = 'bottom-dock-drag-ghost bottom-dock-btn dock-tile';
      var tone = '';
      var iconKind = '';
      try {
        tone = String(originNode.getAttribute('data-dock-tone') || '').trim();
        iconKind = String(originNode.getAttribute('data-dock-icon') || '').trim();
      } catch(_) {
        tone = '';
        iconKind = '';
      }
      if (tone) ghost.setAttribute('data-dock-tone', tone);
      if (iconKind) ghost.setAttribute('data-dock-icon', iconKind);
      if (originNode.classList && typeof originNode.classList.contains === 'function') {
        if (originNode.classList.contains('active')) ghost.classList.add('active');
      }
      ghost.setAttribute('aria-hidden', 'true');
      ghost.innerHTML = String(originNode.innerHTML || '');
      ghost.style.position = 'fixed';
      ghost.style.width = Math.round(Number(this._bottomDockDragGhostWidth || 32)) + 'px';
      ghost.style.height = Math.round(Number(this._bottomDockDragGhostHeight || 32)) + 'px';
      ghost.style.borderRadius = Math.round((Number(this._bottomDockDragGhostWidth || 32) / 32) * 11) + 'px';
      ghost.style.setProperty(
        '--dock-ghost-scale',
        String(Math.max(0.8, Math.min(4, Number(this._bottomDockDragGhostWidth || 32) / 32)))
      );
      var ghostUpDeg = Number(this.bottomDockUpDegForSide(this.bottomDockActiveSide()) || 0);
      var ghostTileRotation = Math.round(ghostUpDeg) + 'deg';
      var ghostIconRotation = '0deg';
      ghost.style.setProperty('--bottom-dock-tile-rotation-deg', ghostTileRotation);
      ghost.style.setProperty('--bottom-dock-icon-rotation-deg', ghostIconRotation);
      var ghostX = Number(ev.clientX || 0) - Number(this._bottomDockPointerGrabOffsetX || 16);
      var ghostY = Number(ev.clientY || 0) - Number(this._bottomDockPointerGrabOffsetY || 16);
      this._bottomDockGhostCurrentX = ghostX;
      this._bottomDockGhostCurrentY = ghostY;
      ghost.style.left = Math.round(ghostX) + 'px';
      ghost.style.top = Math.round(ghostY) + 'px';
      ghost.style.margin = '0';
      ghost.style.pointerEvents = 'none';
      ghost.style.opacity = '1';
      document.body.appendChild(ghost);
      this._bottomDockDragGhostEl = ghost;
      this.setBottomDockGhostTarget(ghostX, ghostY);
    },

    handleBottomDockPointerMove(ev) {
      if (!this._bottomDockPointerActive) return;
      this._bottomDockPointerLastX = Number(ev.clientX || 0);
      this._bottomDockPointerLastY = Number(ev.clientY || 0);
      var movedX = Math.abs(Number(ev.clientX || 0) - Number(this._bottomDockPointerStartX || 0));
      var movedY = Math.abs(Number(ev.clientY || 0) - Number(this._bottomDockPointerStartY || 0));
      if (!this._bottomDockPointerMoved) {
        if (movedX < 5 && movedY < 5) return;
        this.activateBottomDockPointerDrag(ev);
      }
      if (!this._bottomDockPointerMoved) return;
      if (ev && typeof ev.preventDefault === 'function' && ev.cancelable) ev.preventDefault();
      var ghost = this._bottomDockDragGhostEl;
      if (ghost) {
        this.setBottomDockGhostTarget(
          Number(ev.clientX || 0) - Number(this._bottomDockPointerGrabOffsetX || 16),
          Number(ev.clientY || 0) - Number(this._bottomDockPointerGrabOffsetY || 16)
        );
      }
      var dragId = String(this.bottomDockDragId || '').trim();
      if (!dragId) return;
      var insertionIndex = this.bottomDockInsertionIndexFromPointer(dragId, ev);
      if (Number.isFinite(insertionIndex)) {
        var normalizedIndex = Math.max(0, Math.round(Number(insertionIndex || 0)));
        var nowMs = Date.now();
        var lockUntil = Number(this._bottomDockReorderLockUntil || 0);
        if (
          normalizedIndex !== Number(this._bottomDockLastInsertionIndex || -1) &&
          (!Number.isFinite(lockUntil) || lockUntil <= nowMs)
        ) {
          var changed = this.applyBottomDockReorderByIndex(dragId, normalizedIndex, true);
          this._bottomDockLastInsertionIndex = normalizedIndex;
          if (changed) {
            var moveDuration = this.bottomDockMoveDurationMs();
            var lockMs = Math.max(220, Math.min(420, Math.round(moveDuration * 0.55)));
            this._bottomDockReorderLockUntil = nowMs + lockMs;
          }
        }
        return;
      }
      var targetId = '';
      var targetEl = null;
      try {
        var pointerEl = typeof document !== 'undefined' && typeof document.elementFromPoint === 'function'
          ? document.elementFromPoint(Number(ev.clientX || 0), Number(ev.clientY || 0))
          : null;
        targetEl = pointerEl && typeof pointerEl.closest === 'function'
          ? pointerEl.closest('.bottom-dock-btn[data-dock-id]')
          : null;
        targetId = targetEl ? String(targetEl.getAttribute('data-dock-id') || '').trim() : '';
      } catch(_) {}
      if (targetId && targetId !== dragId) {
        this._bottomDockLastInsertionIndex = -1;
        var preferAfter = this.bottomDockShouldInsertAfter(targetId, ev, targetEl);
        this.handleBottomDockDragOver(targetId, ev, preferAfter);
        return;
      }
      if (!this.bottomDockShouldAppendFromPointer(dragId, ev)) return;
      var appendTargetId = this.bottomDockAppendTargetId(dragId);
      if (!appendTargetId) return;
      this._bottomDockLastInsertionIndex = -1;
      this.handleBottomDockDragOver(appendTargetId, ev, true);
    },

    endBottomDockPointerDrag() {
      if (!this._bottomDockPointerActive) return;
      this._bottomDockPointerActive = false;
      this.unbindBottomDockPointerListeners();
      if (!this._bottomDockPointerMoved) {
        this._bottomDockPointerCandidateId = '';
        return;
      }
      var dragId = String(this.bottomDockDragId || this._bottomDockPointerCandidateId || '').trim();
      if (dragId) {
        var finalPointerEvent = {
          clientX: Number(this._bottomDockPointerLastX || 0),
          clientY: Number(this._bottomDockPointerLastY || 0)
        };
        var finalInsertionIndex = this.bottomDockInsertionIndexFromPointer(dragId, finalPointerEvent);
        if (Number.isFinite(finalInsertionIndex)) {
          this.applyBottomDockReorderByIndex(dragId, finalInsertionIndex, false);
        } else if (this.bottomDockShouldAppendFromPointer(dragId, finalPointerEvent)) {
          var appendTargetId = this.bottomDockAppendTargetId(dragId);
          if (appendTargetId) {
            this.handleBottomDockDragOver(appendTargetId, finalPointerEvent, true);
          }
        }
      }
      var current = this.normalizeBottomDockOrder(this.bottomDockOrder);
      var start = this.normalizeBottomDockOrder(this.bottomDockDragStartOrder);
      if (JSON.stringify(current) !== JSON.stringify(start)) {
        this.bottomDockOrder = current;
        this.persistBottomDockOrder();
        this.bottomDockDragCommitted = true;
      }
      this._bottomDockSuppressClickUntil = Date.now() + 220;
      var self = this;
      var finalizeDrag = function() {
        var dockEl = document.querySelector('.bottom-dock');
        if (dockEl && dockEl.style && typeof dockEl.style.removeProperty === 'function') {
          dockEl.style.removeProperty('--bottom-dock-drag-scale');
        }
        var dropX = Number(self._bottomDockPointerLastX || 0);
        var dropY = Number(self._bottomDockPointerLastY || 0);
        self.bottomDockDragId = '';
        self.bottomDockHoverId = '';
        self.bottomDockDragStartOrder = [];
        self._bottomDockPointerGrabOffsetX = 16;
        self._bottomDockPointerGrabOffsetY = 16;
        self._bottomDockDragGhostWidth = 32;
        self._bottomDockDragGhostHeight = 32;
        self._bottomDockPointerCandidateId = '';
        self._bottomDockPointerMoved = false;
        self._bottomDockDragBoundaries = [];
        self._bottomDockLastInsertionIndex = -1;
        self.reviveBottomDockHoverFromPoint(dropX, dropY);
        self._bottomDockPointerLastX = 0;
        self._bottomDockPointerLastY = 0;
      };
      this.settleBottomDockDragGhost(dragId, finalizeDrag);
    },

    shouldSuppressBottomDockClick() {
      var until = Number(this._bottomDockSuppressClickUntil || 0);
      return Number.isFinite(until) && until > Date.now();
    },

    clearBottomDockClickAnimation() {
      if (this._bottomDockClickAnimTimer) {
        try { clearTimeout(this._bottomDockClickAnimTimer); } catch(_) {}
      }
      this._bottomDockClickAnimTimer = 0;
      this.bottomDockClickAnimId = '';
    },

    triggerBottomDockClickAnimation(id, durationOverrideMs) {
      var key = String(id || '').trim();
      if (!key || typeof window === 'undefined' || typeof window.setTimeout !== 'function') return;
      this.clearBottomDockClickAnimation();
      this.bottomDockClickAnimId = key;
      var self = this;
      var durationMs = Number(durationOverrideMs);
      if (!Number.isFinite(durationMs) || durationMs < 120) {
        durationMs = Number(self._bottomDockClickAnimDurationMs || 980);
      }
      if (!Number.isFinite(durationMs) || durationMs < 120) durationMs = 980;
      if (typeof document !== 'undefined') {
        try {
          var tileNode = document.querySelector('.bottom-dock-btn[data-dock-id="' + key + '"]');
          if (tileNode && tileNode.style && typeof tileNode.style.setProperty === 'function') {
            tileNode.style.setProperty('--dock-click-duration', Math.round(durationMs) + 'ms');
          }
        } catch(_) {}
      }
      self._bottomDockClickAnimTimer = window.setTimeout(function() {
        if (typeof document !== 'undefined') {
          try {
            var activeNode = document.querySelector('.bottom-dock-btn[data-dock-id="' + key + '"]');
            if (activeNode && activeNode.style && typeof activeNode.style.removeProperty === 'function') {
              activeNode.style.removeProperty('--dock-click-duration');
            }
          } catch(_) {}
        }
        self._bottomDockClickAnimTimer = 0;
        self.bottomDockClickAnimId = '';
      }, durationMs);
    },

    bottomDockIsClickAnimating(id) {
      var key = String(id || '').trim();
      if (!key) return false;
      return String(this.bottomDockClickAnimId || '').trim() === key;
    },

    handleBottomDockTileClick(id, targetPage, ev) {
      if (this.shouldSuppressBottomDockClick()) return;
      var key = String(id || '').trim();
      var pageKey = String(targetPage || '').trim();
      var clickAnimation = '';
      var clickDurationMs = 0;
      try {
        var triggerEl = ev && ev.currentTarget ? ev.currentTarget : null;
        clickAnimation = String(
          triggerEl && typeof triggerEl.getAttribute === 'function'
            ? (triggerEl.getAttribute('data-dock-click-animation') || '')
            : ''
        ).trim();
        clickDurationMs = Number(
          triggerEl && typeof triggerEl.getAttribute === 'function'
            ? (triggerEl.getAttribute('data-dock-click-duration-ms') || '')
            : ''
        );
      } catch(_) {
        clickAnimation = '';
        clickDurationMs = 0;
      }
      if (!Number.isFinite(clickDurationMs) || clickDurationMs < 120) clickDurationMs = 0;
      if (key && clickAnimation && clickAnimation !== 'none') {
        this.triggerBottomDockClickAnimation(key, clickDurationMs);
      }
      if (pageKey) this.navigate(pageKey);
    },

    normalizeSidebarPopupText(rawText) {
      var text = String(rawText || '').trim();
      if (!text) return '';
      if (this.isSidebarPopupPlaceholderText(text)) return '';
      return text;
    },

    isSidebarPopupPlaceholderText(text) {
      var normalized = String(text || '').trim().toLowerCase();
      return normalized === 'no messages yet'
        || normalized === 'system events and terminal output'
        || normalized === 'no matching text'
        || normalized === 'agent';
    },

    sidebarPopupMetaOrigin(preview, fallbackLabel) {
      var role = String(preview && preview.role || '').trim().toLowerCase();
      if (role === 'user') return 'User';
      if (role === 'assistant' || role === 'agent') return 'Agent';
      if (role) return role.charAt(0).toUpperCase() + role.slice(1);
      return String(fallbackLabel || 'Sidebar').trim() || 'Sidebar';
    },

    hideDashboardPopupBySource(source) {
      var expected = String(source || '').trim();
      if (!expected) return;
      var popup = this.dashboardPopup || {};
      var currentSource = String(popup.source || '').trim();
      if (currentSource !== expected) return;
      this.hideDashboardPopup(String(popup.id || '').trim());
    },

    showCollapsedSidebarAgentPopup(agent, ev) {
      if (!this.sidebarCollapsed || !agent) {
        this.hideDashboardPopupBySource('sidebar');
        return;
      }
      var rawId = String(agent.id || '').trim();
      var rawIdLower = rawId.toLowerCase();
      var isSystemThread = (typeof this.isSystemSidebarThread === 'function')
        ? this.isSystemSidebarThread(agent)
        : (agent.is_system_thread === true || rawIdLower === 'system');
      if (isSystemThread || rawIdLower === 'settings') {
        this.hideDashboardPopupBySource('sidebar');
        return;
      }
      var preview = this.chatSidebarPreview(agent) || {};
      var previewText = this.normalizeSidebarPopupText(preview.text || '');
      var title = String(agent.name || rawId).trim();
      if (!rawId || !title || !previewText) {
        this.hideDashboardPopupBySource('sidebar');
        return;
      }
      this.showDashboardPopup('sidebar-agent:' + rawId, title, ev, {
        source: 'sidebar',
        side: 'right',
        body: previewText,
        meta_origin: this.sidebarPopupMetaOrigin(preview, 'Agent'),
        meta_time: typeof this.formatChatSidebarTime === 'function'
          ? String(this.formatChatSidebarTime(preview.ts) || '').trim()
          : '',
        unread: !!preview.unread_response
      });
    },

    showCollapsedSidebarNavPopup(label, ev) {
      if (!this.sidebarCollapsed) {
        this.hideDashboardPopupBySource('sidebar');
        return;
      }
      var navLabel = String(label || '').trim();
      var navLabelLower = navLabel.toLowerCase();
      if (!navLabel || navLabelLower === 'system' || navLabelLower === 'settings') {
        this.hideDashboardPopupBySource('sidebar');
        return;
      }
      this.showDashboardPopup('sidebar-nav:' + navLabelLower.replace(/[^a-z0-9_-]+/g, '-'), navLabel, ev, {
        source: 'sidebar',
        side: 'right',
        meta_origin: 'Sidebar'
      });
    },

    dashboardPopupService() {
      var root = typeof window !== 'undefined' ? window : {};
      var services = root && root.InfringSharedShellServices;
      return services && services.popup ? services.popup : null;
    },

    clearDashboardPopupState() {
      var service = this.dashboardPopupService();
      this.dashboardPopup = service && typeof service.emptyState === 'function'
        ? service.emptyState()
        : {
          id: '',
          active: false,
          source: '',
          title: '',
          body: '',
          meta_origin: '',
          meta_time: '',
          unread: false,
          left: 0,
          top: 0,
          side: 'bottom',
          inline_away: 'right',
          block_away: 'bottom',
          compact: false
        };
    },

    normalizeDashboardPopupSide(sideValue, fallbackSide) {
      var service = this.dashboardPopupService();
      if (service && typeof service.normalizeSide === 'function') {
        return service.normalizeSide(sideValue, fallbackSide);
      }
      var fallback = String(fallbackSide || 'bottom').trim().toLowerCase();
      if (fallback !== 'top' && fallback !== 'left' && fallback !== 'right') fallback = 'bottom';
      var side = String(sideValue || fallback).trim().toLowerCase();
      if (side !== 'top' && side !== 'left' && side !== 'right') side = 'bottom';
      return side;
    },

    dashboardOppositeSide(sideValue) {
      var service = this.dashboardPopupService();
      if (service && typeof service.oppositeSide === 'function') {
        return service.oppositeSide(sideValue);
      }
      var side = this.normalizeDashboardPopupSide(sideValue, 'bottom');
      if (side === 'top') return 'bottom';
      if (side === 'left') return 'right';
      if (side === 'right') return 'left';
      return 'top';
    },

    dashboardPopupWallAffinity(rect) {
      var service = this.dashboardPopupService();
      if (service && typeof service.wallAffinity === 'function') {
        return service.wallAffinity(rect);
      }
      if (!rect || typeof window === 'undefined') return null;
      var viewportWidth = Number(window.innerWidth || 0);
      var viewportHeight = Number(window.innerHeight || 0);
      if (!Number.isFinite(viewportWidth) || viewportWidth <= 0) viewportWidth = 1;
      if (!Number.isFinite(viewportHeight) || viewportHeight <= 0) viewportHeight = 1;
      var left = Number(rect.left || 0);
      var right = Number(rect.right || 0);
      var top = Number(rect.top || 0);
      var bottom = Number(rect.bottom || 0);
      if (!Number.isFinite(left) || !Number.isFinite(right) || !Number.isFinite(top) || !Number.isFinite(bottom)) {
        return null;
      }
      var width = Math.max(1, Math.abs(right - left));
      var height = Math.max(1, Math.abs(bottom - top));
      var distanceToLeft = Math.max(0, left);
      var distanceToRight = Math.max(0, viewportWidth - right);
      var distanceToTop = Math.max(0, top);
      var distanceToBottom = Math.max(0, viewportHeight - bottom);
      var proximityScore = function(distance) {
        var normalized = Number(distance || 0);
        if (!Number.isFinite(normalized) || normalized < 0) normalized = 0;
        return 1 / (1 + normalized);
      };
      return {
        scores: {
          top: width * proximityScore(distanceToTop),
          bottom: width * proximityScore(distanceToBottom),
          left: height * proximityScore(distanceToLeft),
          right: height * proximityScore(distanceToRight)
        },
        distances: {
          top: distanceToTop,
          bottom: distanceToBottom,
          left: distanceToLeft,
          right: distanceToRight
        }
      };
    },

    dashboardPopupWallAnchorNode(node) {
      if (!node || typeof node.closest !== 'function') return null;
      try {
        return node.closest(
          '[data-popup-wall-anchor], .global-taskbar, .sidebar, .bottom-dock, .doc-window, .chat-window'
        );
      } catch(_) {
        return null;
      }
    },

    dashboardPopupWallRectForNode(node) {
      var anchor = this.dashboardPopupWallAnchorNode(node);
      if (!anchor || typeof anchor.getBoundingClientRect !== 'function') return null;
      try {
        return anchor.getBoundingClientRect();
      } catch(_) {
        return null;
      }
    },

    dashboardPopupUsableAnchorRect(node) {
      if (!node || typeof node.getBoundingClientRect !== 'function') return null;
      var rect = null;
      try {
        rect = node.getBoundingClientRect();
      } catch(_) {
        rect = null;
      }
      var width = rect ? Math.abs(Number(rect.right || 0) - Number(rect.left || 0)) : 0;
      var height = rect ? Math.abs(Number(rect.bottom || 0) - Number(rect.top || 0)) : 0;
      if (rect && width > 0 && height > 0) return rect;
      if (node && typeof node.closest === 'function') {
        try {
          var fallback = node.closest('[data-popup-origin-anchor], .composer-menu-pill, .composer-input-pill, .taskbar-text-menu-anchor, .taskbar-hero-menu-anchor, .notif-wrap');
          if (fallback && fallback !== node && typeof fallback.getBoundingClientRect === 'function') {
            rect = fallback.getBoundingClientRect();
            width = rect ? Math.abs(Number(rect.right || 0) - Number(rect.left || 0)) : 0;
            height = rect ? Math.abs(Number(rect.bottom || 0) - Number(rect.top || 0)) : 0;
            if (rect && width > 0 && height > 0) return rect;
          }
        } catch(_) {}
      }
      return null;
    },

    dashboardPopupSideAwayFromNearestWall(rect, fallbackSide) {
      var service = this.dashboardPopupService();
      if (service && typeof service.sideAwayFromNearestWall === 'function') {
        return service.sideAwayFromNearestWall(rect, fallbackSide);
      }
      var fallback = this.normalizeDashboardPopupSide('', fallbackSide);
      var affinity = this.dashboardPopupWallAffinity(rect);
      if (!affinity || !affinity.scores || !affinity.distances) return fallback;
      var scores = affinity.scores;
      var distances = affinity.distances;
      var walls = ['top', 'bottom', 'left', 'right'];
      var fallbackWall = this.dashboardOppositeSide(fallback);
      var winner = walls[0];
      var winnerScore = Number(scores[winner] || 0);
      var epsilon = 0.000001;
      var i;
      for (i = 1; i < walls.length; i += 1) {
        var wall = walls[i];
        var score = Number(scores[wall] || 0);
        if (score > winnerScore + epsilon) {
          winner = wall;
          winnerScore = score;
          continue;
        }
        if (Math.abs(score - winnerScore) <= epsilon) {
          if (wall === fallbackWall && winner !== fallbackWall) {
            winner = wall;
            winnerScore = score;
            continue;
          }
          var wallDistance = Number(distances[wall] || 0);
          var winnerDistance = Number(distances[winner] || 0);
          if (wallDistance < winnerDistance) {
            winner = wall;
            winnerScore = score;
          }
        }
      }
      return this.dashboardOppositeSide(winner);
    },

    dashboardPopupHorizontalAwayFromNearestWall(rect, fallbackSide) {
      var service = this.dashboardPopupService();
      if (service && typeof service.horizontalAwayFromNearestWall === 'function') {
        return service.horizontalAwayFromNearestWall(rect, fallbackSide);
      }
      var fallback = String(fallbackSide || 'right').trim().toLowerCase();
      if (fallback !== 'left') fallback = 'right';
      var affinity = this.dashboardPopupWallAffinity(rect);
      if (!affinity || !affinity.distances) return fallback;
      var distances = affinity.distances;
      var nearest = Number(distances.left || 0) <= Number(distances.right || 0)
        ? 'left'
        : 'right';
      return nearest === 'left' ? 'right' : 'left';
    },

    dashboardPopupVerticalAwayFromNearestWall(rect, fallbackSide) {
      var service = this.dashboardPopupService();
      if (service && typeof service.verticalAwayFromNearestWall === 'function') {
        return service.verticalAwayFromNearestWall(rect, fallbackSide);
      }
      var fallback = String(fallbackSide || 'bottom').trim().toLowerCase();
      if (fallback !== 'top') fallback = 'bottom';
      var affinity = this.dashboardPopupWallAffinity(rect);
      if (!affinity || !affinity.distances) return fallback;
      var distances = affinity.distances;
      var nearest = Number(distances.top || 0) <= Number(distances.bottom || 0)
        ? 'top'
        : 'bottom';
      return nearest === 'top' ? 'bottom' : 'top';
    },

    dashboardPopupAxisAwareSideAway(rect, fallbackSide) {
      var service = this.dashboardPopupService();
      if (service && typeof service.axisAwareSideAway === 'function') {
        return service.axisAwareSideAway(rect, fallbackSide);
      }
      var fallback = this.normalizeDashboardPopupSide('', fallbackSide || 'bottom');
      if (fallback === 'left' || fallback === 'right') {
        return this.dashboardPopupHorizontalAwayFromNearestWall(rect, fallback);
      }
      return this.dashboardPopupVerticalAwayFromNearestWall(rect, fallback);
    },

    taskbarAnchoredDropdownClass(anchorNode, fallbackSide, layoutKey) {
      var fallback = this.normalizeDashboardPopupSide('', fallbackSide || 'bottom');
      var anchorRect = anchorNode && typeof anchorNode.getBoundingClientRect === 'function'
        ? this.dashboardPopupUsableAnchorRect(anchorNode)
        : null;
      var service = this.dashboardPopupService();
      if (service && typeof service.dropdownClass === 'function') {
        return service.dropdownClass(anchorRect, fallback, layoutKey);
      }
      String(layoutKey == null ? '' : layoutKey);
      var side = fallback;
      var inlineAway = 'right';
      var blockAway = 'bottom';
      if (anchorRect) {
        side = this.dashboardPopupAxisAwareSideAway(anchorRect, fallback);
        inlineAway = this.dashboardPopupHorizontalAwayFromNearestWall(anchorRect, 'right');
        blockAway = this.dashboardPopupVerticalAwayFromNearestWall(anchorRect, 'bottom');
      }
      return {
        'taskbar-anchored-dropdown': true,
        'is-side-top': side === 'top',
        'is-side-bottom': side === 'bottom',
        'is-side-left': side === 'left',
        'is-side-right': side === 'right',
        'is-inline-away-left': inlineAway === 'left',
        'is-inline-away-right': inlineAway === 'right',
        'is-block-away-top': blockAway === 'top',
        'is-block-away-bottom': blockAway === 'bottom'
      };
    },

    dashboardPopupAnchorPoint(ev, sideOverride) {
      var preferredSide = this.normalizeDashboardPopupSide(sideOverride, 'bottom');
      var node = ev && ev.currentTarget ? ev.currentTarget : null;
      if (!node && ev && ev.target && typeof ev.target.closest === 'function') {
        try {
          node = ev.target.closest('button,[role="button"],.taskbar-reorder-item');
        } catch(_) {
          node = null;
        }
      }
      if (!node || typeof node.getBoundingClientRect !== 'function') {
        return { left: 0, top: 0, side: preferredSide, inline_away: 'right', block_away: 'bottom' };
      }
      var rect = node.getBoundingClientRect();
      var service = this.dashboardPopupService();
      if (service && typeof service.anchorPoint === 'function') {
        return service.anchorPoint(rect, preferredSide);
      }
      var side = this.dashboardPopupAxisAwareSideAway(rect, preferredSide);
      var inlineAway = this.dashboardPopupHorizontalAwayFromNearestWall(rect, 'right');
      var blockAway = this.dashboardPopupVerticalAwayFromNearestWall(rect, 'bottom');
      var left = Math.round(Number(rect.left || 0));
      var top = Math.round(Number(rect.bottom || 0));
      if (side === 'top') {
        left = inlineAway === 'left'
          ? Math.round(Number(rect.right || 0))
          : Math.round(Number(rect.left || 0));
        top = Math.round(Number(rect.top || 0));
      } else if (side === 'bottom') {
        left = inlineAway === 'left'
          ? Math.round(Number(rect.right || 0))
          : Math.round(Number(rect.left || 0));
        top = Math.round(Number(rect.bottom || 0));
      } else if (side === 'left') {
        left = Math.round(Number(rect.left || 0));
        top = blockAway === 'top'
          ? Math.round(Number(rect.bottom || 0))
          : Math.round(Number(rect.top || 0));
      } else if (side === 'right') {
        left = Math.round(Number(rect.right || 0));
        top = blockAway === 'top'
          ? Math.round(Number(rect.bottom || 0))
          : Math.round(Number(rect.top || 0));
      }
      return {
        left: left,
        top: top,
        side: side,
        inline_away: inlineAway === 'left' ? 'left' : 'right',
        block_away: blockAway === 'top' ? 'top' : 'bottom'
      };
    },

    showDashboardPopup(id, label, ev, overrides) {
      var popupId = String(id || '').trim();
      var title = String(label || '').trim();
      if (!popupId || !title) {
        this.hideDashboardPopup();
        return;
      }
      var eventType = String((ev && ev.type) || '').toLowerCase();
      if (
        eventType === 'mouseleave' ||
        eventType === 'pointerleave' ||
        eventType === 'blur' ||
        eventType === 'focusout'
      ) {
        this.hideDashboardPopup(popupId);
        return;
      }
      if (ev && ev.isTrusted === false) return;
      var config = overrides && typeof overrides === 'object' ? overrides : {};
      var anchor = this.dashboardPopupAnchorPoint(ev, config.side);
      var service = this.dashboardPopupService();
      this.dashboardPopup = service && typeof service.openState === 'function'
        ? service.openState(popupId, title, config, anchor)
        : {
          id: popupId,
          active: true,
          source: String(config.source || '').trim(),
          title: title,
          body: String(config.body || '').trim(),
          meta_origin: String(config.meta_origin || 'Taskbar').trim(),
          meta_time: String(config.meta_time || '').trim(),
          unread: !!config.unread,
          left: anchor.left,
          top: anchor.top,
          side: anchor.side,
          inline_away: anchor.inline_away === 'left' ? 'left' : 'right',
          block_away: anchor.block_away === 'top' ? 'top' : 'bottom',
          compact: false
        };
    },

    showTaskbarNavPopup(label, ev) {
      var navLabel = String(label || '').trim();
      if (!navLabel) {
        this.hideDashboardPopup();
        return;
      }
      var navKey = navLabel.toLowerCase().replace(/[^a-z0-9_-]+/g, '-');
      var body = navKey === 'back'
        ? (this.canNavigateBack() ? 'Go to the previous page in this session' : 'No earlier page in this session')
        : (this.canNavigateForward() ? 'Go to the next page in this session' : 'No later page in this session');
      this.showDashboardPopup('taskbar-nav:' + navKey, navLabel, ev, {
        source: 'taskbar',
        side: 'bottom',
        compact: false,
        body: body,
        meta_origin: 'Chat nav'
      });
    },

    showTaskbarUtilityPopup(label, body, ev) {
      var utilityLabel = String(label || '').trim();
      if (!utilityLabel) {
        this.hideDashboardPopup();
        return;
      }
      this.showDashboardPopup(
        'taskbar-utility:' + utilityLabel.toLowerCase().replace(/[^a-z0-9_-]+/g, '-'),
        utilityLabel,
        ev,
        {
          source: 'taskbar',
          side: 'bottom',
          compact: false,
          body: String(body || '').trim(),
          meta_origin: 'Taskbar'
        }
      );
    },

    hideDashboardPopup(rawId) {
      var service = this.dashboardPopupService();
      if (service && typeof service.closeState === 'function') {
        this.dashboardPopup = service.closeState(this.dashboardPopup, rawId);
        return;
      }
      var popupId = String(rawId || '').trim();
      var currentId = String(this.dashboardPopup && this.dashboardPopup.id || '').trim();
      if (popupId && currentId && popupId !== currentId) return;
      this.clearDashboardPopupState();
    },

    bottomDockIsDraggingVisual(id) {
      var key = String(id || '').trim();
      if (!key) return false;
      if (this._bottomDockRevealTargetDuringSettle) return false;
      return String(this.bottomDockDragId || '').trim() === key;
    },

    bottomDockIsNeighbor(id) {
      var hoverId = String(this.bottomDockHoverId || '').trim();
      var key = String(id || '').trim();
      if (!hoverId || !key || hoverId === key) return false;
      return Math.abs(this.bottomDockOrderIndex(hoverId) - this.bottomDockOrderIndex(key)) === 1;
    },

    bottomDockIsSecondNeighbor(id) {
      var hoverId = String(this.bottomDockHoverId || '').trim();
      var key = String(id || '').trim();
      if (!hoverId || !key || hoverId === key) return false;
      return Math.abs(this.bottomDockOrderIndex(hoverId) - this.bottomDockOrderIndex(key)) === 2;
    },

    bottomDockHoverWeight(id) {
      var key = String(id || '').trim();
      if (!key) return 0;
      var weights = this.bottomDockHoverWeightById && typeof this.bottomDockHoverWeightById === 'object'
        ? this.bottomDockHoverWeightById
        : null;
      if (weights && Object.prototype.hasOwnProperty.call(weights, key)) {
        var exact = Number(weights[key] || 0);
        if (Number.isFinite(exact)) return Math.max(0, Math.min(1, exact));
      }
      if (key === String(this.bottomDockHoverId || '').trim()) return 1;
      if (this.bottomDockIsNeighbor(key)) return 0.33;
      if (this.bottomDockIsSecondNeighbor(key)) return 0.11;
      return 0;
    },

    startBottomDockDrag(id, ev) {
      var key = String(id || '').trim();
      if (!key) return;
      this.cleanupBottomDockDragGhost();
      this.bottomDockHoverId = '';
      this.bottomDockHoverWeightById = {};
      this.bottomDockPointerX = 0;
      this.bottomDockPointerY = 0;
      this.bottomDockPreviewVisible = false;
      this.bottomDockPreviewText = '';
      this.bottomDockPreviewMorphFromText = '';
      this.bottomDockPreviewLabelMorphing = false;
      this.bottomDockPreviewWidth = 0;
      this.cancelBottomDockPreviewReflow();
      this.bottomDockDragId = key;
      this.bottomDockDragCommitted = false;
      this.bottomDockDragStartOrder = this.normalizeBottomDockOrder(this.bottomDockOrder);
      this._bottomDockReorderLockUntil = 0;
      this.captureBottomDockDragBoundaries(key);
      if (ev && ev.dataTransfer) {
        try { ev.dataTransfer.effectAllowed = 'move'; } catch(_) {}
        try { ev.dataTransfer.dropEffect = 'move'; } catch(_) {}
        try {
          var dragNode = ev.currentTarget;
          if (dragNode && typeof ev.dataTransfer.setDragImage === 'function') {
            var rect = dragNode.getBoundingClientRect();
            var ghost = dragNode.cloneNode(true);
            if (ghost && document && document.body) {
              ghost.classList.add('bottom-dock-drag-ghost');
              ghost.style.position = 'fixed';
              ghost.style.left = '-9999px';
              ghost.style.top = '-9999px';
              ghost.style.margin = '0';
              ghost.style.transform = 'none';
              ghost.style.pointerEvents = 'none';
              ghost.style.opacity = '1';
              document.body.appendChild(ghost);
              this._bottomDockDragGhostEl = ghost;
              ev.dataTransfer.setDragImage(
                ghost,
                Math.max(0, Math.round(Number(rect.width || 0) / 2)),
                Math.max(0, Math.round(Number(rect.height || 0) / 2))
              );
            } else {
              ev.dataTransfer.setDragImage(
                dragNode,
                Math.max(0, Math.round(Number(rect.width || 0) / 2)),
                Math.max(0, Math.round(Number(rect.height || 0) / 2))
              );
            }
          }
        } catch(_) {}
        try { ev.dataTransfer.setData('application/x-infring-dock', key); } catch(_) {}
        try { ev.dataTransfer.setData('text/plain', key); } catch(_) {}
      }
    },

    bottomDockShouldInsertAfter(targetId, ev, targetEl) {
      var key = String(targetId || '').trim();
      if (!key) return false;
      if (!ev) return false;
      var clientX = Number(ev.clientX || 0);
      var clientY = Number(ev.clientY || 0);
      if (!Number.isFinite(clientX) || !Number.isFinite(clientY)) return false;
      var node = targetEl || null;
      if (!node && typeof document !== 'undefined') {
        try {
          node = document.querySelector('.bottom-dock-btn[data-dock-id="' + key + '"]');
        } catch(_) {
          node = null;
        }
      }
      if (!node || typeof node.getBoundingClientRect !== 'function') return false;
      var rect = node.getBoundingClientRect();
      var width = Number(rect.width || 0);
      var height = Number(rect.height || 0);
      if (!Number.isFinite(width) || width <= 0) return false;
      if (!Number.isFinite(height) || height <= 0) return false;
      var basis = this.bottomDockAxisBasis();
      var centerX = Number(rect.left || 0) + (width / 2);
      var centerY = Number(rect.top || 0) + (height / 2);
      var centerProj = this.bottomDockProjectPointToAxis(centerX, centerY, basis);
      var pointerProj = this.bottomDockProjectPointToAxis(clientX, clientY, basis);
      var half = this.bottomDockAxisHalfExtent(width, height, basis).primary;
      if (!Number.isFinite(half) || half <= 0) half = Math.max(width, height) / 2;
      if (!Number.isFinite(half) || half <= 0) return false;
      var ratio = (pointerProj.primary - (centerProj.primary - half)) / (half * 2);
      return ratio >= 0.5;
    },

    captureBottomDockDragBoundaries(dragId) {
      var key = String(dragId || '').trim();
      if (!key || typeof document === 'undefined') {
        this._bottomDockDragBoundaries = [];
        this._bottomDockLastInsertionIndex = -1;
        return [];
      }
      var dock = null;
      try {
        dock = document.querySelector('.bottom-dock');
      } catch(_) {
        dock = null;
      }
      if (!dock) {
        this._bottomDockDragBoundaries = [];
        this._bottomDockLastInsertionIndex = -1;
        return [];
      }
      var centers = [];
      var basis = this.bottomDockAxisBasis();
      try {
        var nodes = dock.querySelectorAll('.bottom-dock-btn[data-dock-id]');
        for (var i = 0; i < nodes.length; i += 1) {
          var node = nodes[i];
          if (!node || typeof node.getAttribute !== 'function') continue;
          var id = String(node.getAttribute('data-dock-id') || '').trim();
          if (!id || id === key || typeof node.getBoundingClientRect !== 'function') continue;
          var rect = node.getBoundingClientRect();
          var width = Number(rect.width || 0);
          var height = Number(rect.height || 0);
          if (!Number.isFinite(width) || width <= 0) continue;
          if (!Number.isFinite(height) || height <= 0) continue;
          var centerX = Number(rect.left || 0) + (width / 2);
          var centerY = Number(rect.top || 0) + (height / 2);
          centers.push(this.bottomDockProjectPointToAxis(centerX, centerY, basis).primary);
        }
      } catch(_) {}
      centers.sort(function(a, b) { return a - b; });
      this._bottomDockDragBoundaries = centers;
      this._bottomDockLastInsertionIndex = -1;
      return centers;
    },

    bottomDockAppendTargetId(dragId) {
      var key = String(dragId || '').trim();
      if (!key) return '';
      var order = this.normalizeBottomDockOrder(this.bottomDockOrder);
      var filtered = [];
      for (var i = 0; i < order.length; i += 1) {
        var id = String(order[i] || '').trim();
        if (!id || id === key) continue;
        filtered.push(id);
      }
      if (!filtered.length) return '';
      return String(filtered[filtered.length - 1] || '').trim();
    },

    bottomDockShouldAppendFromPointer(dragId, ev) {
      var key = String(dragId || '').trim();
      if (!key || !ev || typeof document === 'undefined') return false;
      var clientX = Number(ev.clientX || 0);
      var clientY = Number(ev.clientY || 0);
      if (!Number.isFinite(clientX) || !Number.isFinite(clientY)) return false;
      var appendTargetId = this.bottomDockAppendTargetId(key);
      if (!appendTargetId) return false;
      var node = null;
      try {
        node = document.querySelector('.bottom-dock-btn[data-dock-id="' + appendTargetId + '"]');
      } catch(_) {
        node = null;
      }
      if (!node || typeof node.getBoundingClientRect !== 'function') return false;
      var rect = node.getBoundingClientRect();
      var width = Number(rect.width || 0);
      var height = Number(rect.height || 0);
      if (!Number.isFinite(width) || width <= 0) return false;
      if (!Number.isFinite(height) || height <= 0) return false;
      var basis = this.bottomDockAxisBasis();
      var centerX = Number(rect.left || 0) + (width / 2);
      var centerY = Number(rect.top || 0) + (height / 2);
      var centerProj = this.bottomDockProjectPointToAxis(centerX, centerY, basis);
      var pointerProj = this.bottomDockProjectPointToAxis(clientX, clientY, basis);
      var extent = this.bottomDockAxisHalfExtent(width, height, basis);
      var halfPrimary = Number(extent.primary || 0);
      var halfSecondary = Number(extent.secondary || 0);
      if (!Number.isFinite(halfPrimary) || halfPrimary <= 0) halfPrimary = Math.max(width, height) / 2;
      if (!Number.isFinite(halfSecondary) || halfSecondary <= 0) halfSecondary = Math.min(width, height) / 2;
      var secondaryPad = Math.max(18, halfSecondary * 0.75);
      if (Math.abs(pointerProj.secondary - centerProj.secondary) > (halfSecondary + secondaryPad)) return false;
      var threshold = centerProj.primary + halfPrimary - Math.min(18, halfPrimary * 0.7);
      return pointerProj.primary >= threshold;
    },

    bottomDockInsertionIndexFromCoords(dragId, clientXRaw, clientYRaw) {
      var key = String(dragId || '').trim();
      if (!key || typeof document === 'undefined') return null;
      var clientX = Number(clientXRaw || 0);
      var clientY = Number(clientYRaw || 0);
      if (!Number.isFinite(clientX) || !Number.isFinite(clientY)) return null;
      var dock = null;
      try {
        dock = document.querySelector('.bottom-dock');
      } catch(_) {
        dock = null;
      }
      if (!dock || typeof dock.getBoundingClientRect !== 'function') return null;
      var dockRect = dock.getBoundingClientRect();
      var basis = this.bottomDockAxisBasis();
      var pointerProj = this.bottomDockProjectPointToAxis(clientX, clientY, basis);
      var dockBounds = this.bottomDockProjectedRectBounds(dockRect, basis);
      if (!dockBounds) return null;
      if (
        pointerProj.secondary < (Number(dockBounds.secondaryMin || 0) - 24) ||
        pointerProj.secondary > (Number(dockBounds.secondaryMax || 0) + 24)
      ) return null;
      var centers = this.captureBottomDockDragBoundaries(key);
      if (centers.length === 0) return null;
      var insertionIndex = 0;
      for (var c = 0; c < centers.length; c += 1) {
        if (pointerProj.primary >= centers[c]) insertionIndex += 1;
      }
      insertionIndex = Math.max(0, Math.min(centers.length, insertionIndex));
      return insertionIndex;
    },

    bottomDockGhostCenterPoint() {
      var x = Number(this._bottomDockGhostTargetX || this._bottomDockGhostCurrentX || 0);
      var y = Number(this._bottomDockGhostTargetY || this._bottomDockGhostCurrentY || 0);
      var width = Number(this._bottomDockDragGhostWidth || 0);
      var height = Number(this._bottomDockDragGhostHeight || 0);
      if (!Number.isFinite(width) || width <= 0) width = 32;
      if (!Number.isFinite(height) || height <= 0) height = 32;
      return {
        x: x + (width / 2),
        y: y + (height / 2)
      };
    },

    bottomDockInsertionIndexFromPointer(dragId, ev) {
      var key = String(dragId || '').trim();
      if (!key || !ev) return null;
      var center = this.bottomDockGhostCenterPoint();
      return this.bottomDockInsertionIndexFromCoords(key, center.x, center.y);
    },

    applyBottomDockReorderByIndex(dragId, insertionIndex, animate) {
      var key = String(dragId || '').trim();
      if (!key) return false;
      var current = this.normalizeBottomDockOrder(this.bottomDockOrder);
      var fromIndex = current.indexOf(key);
      if (fromIndex < 0) return false;
      var next = current.slice();
      next.splice(fromIndex, 1);
      var idx = Number(insertionIndex);
      if (!Number.isFinite(idx)) return false;
      idx = Math.max(0, Math.min(next.length, Math.round(idx)));
      next.splice(idx, 0, key);
      if (JSON.stringify(next) === JSON.stringify(current)) return false;
      var doAnimate = Boolean(animate);
      var beforeRects = doAnimate ? this.bottomDockButtonRects() : null;
      this.bottomDockOrder = next;
      if (doAnimate && beforeRects) this.animateBottomDockFromRects(beforeRects);
      return true;
    },
    persistBottomDockOrderIfChangedFromDragStart() {
      var current = this.normalizeBottomDockOrder(this.bottomDockOrder);
      var start = this.normalizeBottomDockOrder(this.bottomDockDragStartOrder);
      if (JSON.stringify(current) !== JSON.stringify(start)) {
        this.bottomDockOrder = current;
        this.persistBottomDockOrder();
        this.bottomDockDragCommitted = true;
      }
    },
    completeBottomDockDropCleanup(ev) {
      this.bottomDockDragId = '';
      this.bottomDockDragStartOrder = [];
      this._bottomDockSuppressClickUntil = Date.now() + 220;
      this.cleanupBottomDockDragGhost();
      this.reviveBottomDockHoverFromPoint(
        Number(ev && ev.clientX || 0),
        Number(ev && ev.clientY || 0)
      );
      if (ev && typeof ev.preventDefault === 'function') ev.preventDefault();
    },

    handleBottomDockContainerDragOver(ev) {
      if (ev && ev.dataTransfer) {
        try { ev.dataTransfer.dropEffect = 'move'; } catch(_) {}
      }
      var dragId = String(this.bottomDockDragId || '').trim();
      if (!dragId) return;
      var targetId = '';
      var targetEl = null;
      try {
        targetEl = ev && ev.target && typeof ev.target.closest === 'function'
          ? ev.target.closest('.bottom-dock-btn[data-dock-id]')
          : null;
        targetId = targetEl ? String(targetEl.getAttribute('data-dock-id') || '').trim() : '';
      } catch(_) {}
      if (targetId && targetId !== dragId) {
        this._bottomDockLastInsertionIndex = -1;
        var preferAfter = this.bottomDockShouldInsertAfter(targetId, ev, targetEl);
        this.handleBottomDockDragOver(targetId, ev, preferAfter);
        return;
      }
      if (!this.bottomDockShouldAppendFromPointer(dragId, ev)) return;
      var appendTargetId = this.bottomDockAppendTargetId(dragId);
      if (!appendTargetId) return;
      this._bottomDockLastInsertionIndex = -1;
      this.handleBottomDockDragOver(appendTargetId, ev, true);
    },

    handleBottomDockContainerDrop(ev) {
      var dragId = String(this.bottomDockDragId || '').trim();
      if (!dragId) return;
      var targetId = '';
      var targetEl = null;
      try {
        targetEl = ev && ev.target && typeof ev.target.closest === 'function'
          ? ev.target.closest('.bottom-dock-btn[data-dock-id]')
          : null;
        targetId = targetEl ? String(targetEl.getAttribute('data-dock-id') || '').trim() : '';
      } catch(_) {}
      if (targetId) {
        var preferAfter = this.bottomDockShouldInsertAfter(targetId, ev, targetEl);
        this.handleBottomDockDrop(targetId, ev, preferAfter);
        return;
      }
      if (this.bottomDockShouldAppendFromPointer(dragId, ev)) {
        var appendTargetId = this.bottomDockAppendTargetId(dragId);
        if (appendTargetId) {
          this.handleBottomDockDrop(appendTargetId, ev, true);
          return;
        }
      }
      this.persistBottomDockOrderIfChangedFromDragStart();
      this.completeBottomDockDropCleanup(ev);
    },

    handleBottomDockDragOver(id, ev, preferAfter) {
      var targetId = String(id || '').trim();
      var dragId = String(this.bottomDockDragId || '').trim();
      if (!targetId || !dragId || targetId === dragId) return;
      var nowMs = Date.now();
      var lockUntil = Number(this._bottomDockReorderLockUntil || 0);
      if (Number.isFinite(lockUntil) && lockUntil > nowMs) return;
      if (ev && ev.dataTransfer) {
        try { ev.dataTransfer.dropEffect = 'move'; } catch(_) {}
      }
      var placeAfter = Boolean(preferAfter);
      var current = this.normalizeBottomDockOrder(this.bottomDockOrder);
      var next = current.slice();
      var fromIndex = next.indexOf(dragId);
      var toIndex = next.indexOf(targetId);
      if (fromIndex < 0 || toIndex < 0 || fromIndex === toIndex) return;
      next.splice(fromIndex, 1);
      if (fromIndex < toIndex) toIndex -= 1;
      if (placeAfter) toIndex += 1;
      if (toIndex < 0) toIndex = 0;
      if (toIndex > next.length) toIndex = next.length;
      next.splice(toIndex, 0, dragId);
      if (JSON.stringify(next) === JSON.stringify(current)) return;
      var beforeRects = this.bottomDockButtonRects();
      this.bottomDockOrder = next;
      this.animateBottomDockFromRects(beforeRects);
      var moveDuration = this.bottomDockMoveDurationMs();
      var lockMs = Math.max(320, Math.min(520, Math.round(moveDuration + 60)));
      this._bottomDockReorderLockUntil = nowMs + lockMs;
    },

    handleBottomDockDrop(id, ev, preferAfter) {
      var targetId = String(id || '').trim();
      var dragId = String(this.bottomDockDragId || '').trim();
      if (!targetId || !dragId) {
        this._bottomDockSuppressClickUntil = Date.now() + 220;
        this.cleanupBottomDockDragGhost();
        this.bottomDockDragId = '';
        this.bottomDockDragStartOrder = [];
        this.bottomDockDragCommitted = false;
        this.reviveBottomDockHoverFromPoint(
          Number(ev && ev.clientX || 0),
          Number(ev && ev.clientY || 0)
        );
        return;
      }
      if (targetId === dragId) {
        this.persistBottomDockOrderIfChangedFromDragStart();
        this.completeBottomDockDropCleanup(ev);
        return;
      }
      var next = this.normalizeBottomDockOrder(this.bottomDockOrder);
      var fromIndex = next.indexOf(dragId);
      var toIndex = next.indexOf(targetId);
      var placeAfter = Boolean(preferAfter);
      if (fromIndex < 0 || toIndex < 0) {
        this.bottomDockDragId = '';
        this.bottomDockDragStartOrder = [];
        this.bottomDockDragCommitted = false;
        this.reviveBottomDockHoverFromPoint(
          Number(ev && ev.clientX || 0),
          Number(ev && ev.clientY || 0)
        );
        return;
      }
      next.splice(fromIndex, 1);
      if (fromIndex < toIndex) toIndex -= 1;
      if (placeAfter) toIndex += 1;
      if (toIndex < 0) toIndex = 0;
      if (toIndex > next.length) toIndex = next.length;
      next.splice(toIndex, 0, dragId);
      this.bottomDockOrder = next;
      this.persistBottomDockOrder();
      this.bottomDockDragCommitted = true;
      this.completeBottomDockDropCleanup(ev);
    },

    endBottomDockDrag() {
      if (!this.bottomDockDragCommitted && Array.isArray(this.bottomDockDragStartOrder) && this.bottomDockDragStartOrder.length) {
        var current = this.normalizeBottomDockOrder(this.bottomDockOrder);
        var start = this.normalizeBottomDockOrder(this.bottomDockDragStartOrder);
        if (JSON.stringify(current) !== JSON.stringify(start)) {
          this.bottomDockOrder = current;
          this.persistBottomDockOrder();
          this.bottomDockDragCommitted = true;
        } else {
          var beforeRects = this.bottomDockButtonRects();
          this.bottomDockOrder = start;
          this.animateBottomDockFromRects(beforeRects);
        }
      }
      this.bottomDockDragId = '';
      this.bottomDockHoverId = '';
      this.bottomDockDragStartOrder = [];
      this.bottomDockDragCommitted = false;
      this._bottomDockSuppressClickUntil = Date.now() + 220;
      this.cleanupBottomDockDragGhost();
    },

    dashboardPopupOrigin(overrides) {
      var service = this.dashboardPopupService();
      if (service && typeof service.origin === 'function') {
        return service.origin(overrides);
      }
      return Object.assign({
        source: '',
        active: false,
        ready: false,
        side: 'top',
        inline_away: 'right',
        block_away: 'bottom',
        left: 0,
        top: 0,
        compact: false,
        title: '',
        body: '',
        meta_origin: '',
        meta_time: '',
        unread: false
      }, overrides || {});
    },

    bottomDockPopupOrigin() {
      var label = String(this.bottomDockPreviewText || '').trim();
      var left = Math.round(Number(this.bottomDockPreviewX || 0));
      var top = Math.round(Number(this.bottomDockPreviewY || 0));
      if (!this.bottomDockPreviewVisible || !label) return this.dashboardPopupOrigin();
      return this.dashboardPopupOrigin({
        source: 'bottom_dock',
        active: true,
        ready: left > 0 && top > 0,
        side: this.bottomDockOpenSide(),
        inline_away: 'center',
        block_away: 'center',
        left: left,
        top: top,
        compact: false,
        title: label
      });
    },

    dashboardPopupStateOrigin() {
      var service = this.dashboardPopupService();
      if (service && typeof service.stateOrigin === 'function') {
        return service.stateOrigin(this.dashboardPopup);
      }
      var popup = this.dashboardPopup || {};
      var title = String(popup.title || '').trim();
      var body = String(popup.body || '').trim();
      var left = Math.round(Number(popup.left || 0));
      var top = Math.round(Number(popup.top || 0));
      var side = String(popup.side || 'bottom').trim().toLowerCase();
      var inlineAway = String(popup.inline_away || 'right').trim().toLowerCase();
      var blockAway = String(popup.block_away || 'bottom').trim().toLowerCase();
      if (side !== 'top' && side !== 'left' && side !== 'right') side = 'bottom';
      if (inlineAway !== 'left' && inlineAway !== 'right') inlineAway = 'center';
      if (blockAway !== 'top' && blockAway !== 'bottom') blockAway = 'center';
      if (!popup.active || !title) return this.dashboardPopupOrigin();
      return this.dashboardPopupOrigin({
        source: String(popup.source || 'ui').trim(),
        active: true,
        ready: left > 0 && top > 0,
        side: side,
        inline_away: inlineAway,
        block_away: blockAway,
        left: left,
        top: top,
        compact: false,
        title: title,
        body: body,
        meta_origin: String(popup.meta_origin || '').trim(),
        meta_time: String(popup.meta_time || '').trim(),
        unread: !!popup.unread
      });
    },

    activeDashboardPopupOrigin() {
      var sharedPopup = this.dashboardPopupStateOrigin();
      if (sharedPopup.active && sharedPopup.ready) return sharedPopup;
      var dockPopup = this.bottomDockPopupOrigin();
      if (dockPopup.active && dockPopup.ready) return dockPopup;
      return this.dashboardPopupOrigin();
    },

    isDashboardPopupVisible() {
      var popup = this.activeDashboardPopupOrigin();
      return !!(popup.active && popup.ready && popup.title);
    },

    dashboardPopupOverlayClass() {
      var popup = this.activeDashboardPopupOrigin();
      var service = this.dashboardPopupService();
      if (service && typeof service.overlayClass === 'function') {
        return service.overlayClass(popup, 'fogged-glass');
      }
      return {
        'is-visible': !!(popup.active && popup.ready && popup.title),
        'is-side-top': popup.side === 'top',
        'is-side-bottom': popup.side === 'bottom',
        'is-side-left': popup.side === 'left',
        'is-side-right': popup.side === 'right',
        'is-inline-away-left': popup.inline_away === 'left',
        'is-inline-away-right': popup.inline_away === 'right',
        'is-inline-away-center': popup.inline_away !== 'left' && popup.inline_away !== 'right',
        'is-block-away-top': popup.block_away === 'top',
        'is-block-away-bottom': popup.block_away === 'bottom',
        'is-block-away-center': popup.block_away !== 'top' && popup.block_away !== 'bottom',
        'is-unread': !!popup.unread
      };
    },

    dashboardPopupOverlayStyle() {
      var popup = this.activeDashboardPopupOrigin();
      var service = this.dashboardPopupService();
      if (service && typeof service.overlayStyle === 'function') {
        return service.overlayStyle(popup);
      }
      if (!popup.active || !popup.ready) return 'left:-9999px;top:-9999px;';
      return 'left:' + Math.round(Number(popup.left || 0)) + 'px;top:' + Math.round(Number(popup.top || 0)) + 'px;';
    },

    updateSidebarScrollIndicators() {
      var refs = this.$refs || {};
      var navState = this._computeScrollHintState(refs.sidebarNav);
      this.sidebarHasOverflowAbove = !!navState.above;
      this.sidebarHasOverflowBelow = !!navState.below;
      var chatState = this._computeScrollHintState(refs.chatSidebarList);
      this.chatSidebarHasOverflowAbove = !!chatState.above;
      this.chatSidebarHasOverflowBelow = !!chatState.below;
    },
    scheduleSidebarScrollIndicators() {
      if (this._sidebarScrollIndicatorRaf) return;
      var self = this;
      this._sidebarScrollIndicatorRaf = requestAnimationFrame(function() {
        self._sidebarScrollIndicatorRaf = 0;
        self.updateSidebarScrollIndicators();
        if (typeof self.maybeAnimateChatSidebarRows === 'function') {
          self.maybeAnimateChatSidebarRows();
        }
      });
    },
    getAppStore() {
      try {
        var store = Alpine && typeof Alpine.store === 'function' ? Alpine.store('app') : null;
        return (store && typeof store === 'object') ? store : null;
      } catch(_) {
        return null;
      }
    },
    get agents() {
      var store = this.getAppStore();
      return store && Array.isArray(store.agents) ? store.agents : [];
    },
    isSystemSidebarThread(agent) {
      if (!agent || typeof agent !== 'object') return false;
      if (agent.is_system_thread === true) return true;
      var id = String(agent.id || '').trim().toLowerCase();
      if (id === 'system') return true;
      var role = String(agent.role || '').trim().toLowerCase();
      return role === 'system';
    },
    isSidebarArchivedAgent(agent) {
      if (!agent || typeof agent !== 'object') return false;
      var store = this.getAppStore();
      if (store && typeof store.isArchivedLikeAgent === 'function') return store.isArchivedLikeAgent(agent);
      if (Object.prototype.hasOwnProperty.call(agent, 'sidebar_archived')) return !!agent.sidebar_archived;
      return !!agent.archived;
    },
    isReservedSystemEmoji(rawEmoji) {
      var normalized = String(rawEmoji || '').replace(/\uFE0F/g, '').trim();
      return normalized === '⚙';
    },
    sanitizeSidebarAgentRow(agent) {
      if (!agent || typeof agent !== 'object') return agent;
      var row = Object.assign({}, agent);
      var identity = Object.assign({}, (row.identity && typeof row.identity === 'object') ? row.identity : {});
      if (this.isSystemSidebarThread(row)) {
        row.id = 'system';
        row.name = 'System';
        row.is_system_thread = true;
        row.role = 'system';
        identity.emoji = '\u2699\ufe0f';
        row.identity = identity;
        return row;
      }
      if (this.isReservedSystemEmoji(identity.emoji)) {
        identity.emoji = '';
      }
      row.identity = identity;
      return row;
    },
    persistChatSidebarTopologyOrder() {
      var seen = {};
      var out = [];
      (this.chatSidebarTopologyOrder || []).forEach(function(id) {
        var key = String(id || '').trim();
        if (!key || seen[key]) return;
        seen[key] = true;
        out.push(key);
      });
      this.chatSidebarTopologyOrder = out;
      try {
        localStorage.setItem('infring-chat-sidebar-topology-order', JSON.stringify(out));
      } catch(_) {}
    },
    chatSidebarCanReorderTopology() {
      return String(this.chatSidebarSortMode || '').toLowerCase() === 'topology';
    },
    startChatSidebarTopologyDrag(agent, ev) {
      if (!this.chatSidebarCanReorderTopology() || !agent || !agent.id) return;
      this.syncChatSidebarTopologyOrderFromAgents();
      this.chatSidebarDragAgentId = String(agent.id);
      this.chatSidebarDropTargetId = '';
      this.chatSidebarDropAfter = false;
      if (ev && ev.dataTransfer) {
        ev.dataTransfer.effectAllowed = 'move';
        ev.dataTransfer.setData('text/plain', this.chatSidebarDragAgentId);
      }
    },
    handleChatSidebarTopologyDragOver(agent, ev) {
      if (!this.chatSidebarCanReorderTopology() || !this.chatSidebarDragAgentId || !agent || !agent.id) return;
      if (ev) {
        ev.preventDefault();
        if (ev.dataTransfer) ev.dataTransfer.dropEffect = 'move';
      }
      var targetId = String(agent.id);
      var dropAfter = false;
      if (ev && ev.currentTarget && typeof ev.clientY === 'number' && typeof ev.currentTarget.getBoundingClientRect === 'function') {
        var rect = ev.currentTarget.getBoundingClientRect();
        dropAfter = ev.clientY > (rect.top + (rect.height / 2));
      }
      this.chatSidebarDropAfter = !!dropAfter;
      this.chatSidebarDropTargetId = targetId === this.chatSidebarDragAgentId ? '' : targetId;
    },
    handleChatSidebarTopologyDrop(agent, ev) {
      if (ev) ev.preventDefault();
      if (!this.chatSidebarCanReorderTopology() || !agent || !agent.id) return this.endChatSidebarTopologyDrag();
      var dragId = String(this.chatSidebarDragAgentId || '').trim();
      if (!dragId && ev && ev.dataTransfer) dragId = String(ev.dataTransfer.getData('text/plain') || '').trim();
      var targetId = String(agent.id).trim();
      if (!dragId || !targetId || dragId === targetId) return this.endChatSidebarTopologyDrag();
      this.syncChatSidebarTopologyOrderFromAgents();
      var order = (this.chatSidebarTopologyOrder || []).slice();
      var fromIndex = order.indexOf(dragId);
      var targetIndex = order.indexOf(targetId);
      if (fromIndex < 0 || targetIndex < 0) return this.endChatSidebarTopologyDrag();
      var dropAfter = false;
      if (ev && ev.currentTarget && typeof ev.clientY === 'number' && typeof ev.currentTarget.getBoundingClientRect === 'function') {
        var rect = ev.currentTarget.getBoundingClientRect();
        dropAfter = ev.clientY > (rect.top + (rect.height / 2));
      }
      order.splice(fromIndex, 1);
      if (fromIndex < targetIndex) targetIndex -= 1;
      if (dropAfter) targetIndex += 1;
      if (targetIndex < 0) targetIndex = 0;
      if (targetIndex > order.length) targetIndex = order.length;
      order.splice(targetIndex, 0, dragId);
      this.chatSidebarTopologyOrder = order;
      this.persistChatSidebarTopologyOrder();
      this.endChatSidebarTopologyDrag();
      this.scheduleSidebarScrollIndicators();
    },
    endChatSidebarTopologyDrag() {
      this.chatSidebarDragAgentId = '';
      this.chatSidebarDropTargetId = '';
      this.chatSidebarDropAfter = false;
    },
    get chatSidebarAgents() {
      var list = (this.agents || []).slice();
      var self = this;
      var pendingFreshId = String((this.getAppStore() && this.getAppStore().pendingFreshAgentId) || '').trim();
      list = list.filter(function(agent) {
        if (!agent || !agent.id) return false;
        if (pendingFreshId && String(agent.id || '') === pendingFreshId) return false;
        if (self.isSidebarArchivedAgent(agent)) return false;
        return true;
      });
      list.sort(function(a, b) {
        return self.chatSidebarSortComparator(a, b);
      });
      if (this.chatSidebarCanReorderTopology() && Array.isArray(this.chatSidebarTopologyOrder) && this.chatSidebarTopologyOrder.length) {
        var rank = {};
        this.chatSidebarTopologyOrder.forEach(function(id, idx) {
          var key = String(id || '').trim();
          if (!key || rank[key] != null) return;
          rank[key] = idx;
        });
        list.sort(function(a, b) {
          var aId = String((a && a.id) || '');
          var bId = String((b && b.id) || '');
          var hasA = Object.prototype.hasOwnProperty.call(rank, aId);
          var hasB = Object.prototype.hasOwnProperty.call(rank, bId);
          if (hasA && hasB && rank[aId] !== rank[bId]) return rank[aId] - rank[bId];
          if (hasA && !hasB) return -1;
          if (!hasA && hasB) return 1;
          return self.chatSidebarSortComparator(a, b);
        });
      }
      return list.map(function(agent) {
        return self.sanitizeSidebarAgentRow(agent);
      });
    },
    get chatSidebarRows() {
      if (this.chatSidebarDragActive && Array.isArray(this._chatSidebarDragRowsCache)) {
        return this._chatSidebarDragRowsCache;
      }
      var query = String(this.chatSidebarQuery || '').trim();
      var rows;
      if (!query) rows = this.chatSidebarAgents || [];
      else if (Array.isArray(this.chatSidebarSearchResults) && this.chatSidebarSearchResults.length) rows = this.chatSidebarSearchResults;
      else rows = [];
      if (this.chatSidebarDragActive) {
        this._chatSidebarDragRowsCache = Array.isArray(rows) ? rows.slice() : [];
      } else {
        this._chatSidebarDragRowsCache = null;
      }
      return rows;
    },
    chatSidebarDragRenderWindow(rows) {
      var sourceRows = Array.isArray(rows) ? rows : [];
      var total = sourceRows.length;
      var maxRows = Math.max(1, Math.floor(Number(this._chatSidebarDragRenderMaxRows || 10)));
      if (!this.chatSidebarDragActive || total <= maxRows) {
        return { virtualized: false, start: 0, end: total, padTop: 0, padBottom: 0 };
      }
      var refs = this.$refs || {};
      var nav = refs.sidebarNav || null;
      var rowHeight = Math.max(1, Math.floor(Number(this._chatSidebarDragRenderRowHeight || 56)));
      var scrollTop = nav ? Math.max(0, Number(nav.scrollTop || 0)) : 0;
      var start = Math.max(0, Math.floor(scrollTop / rowHeight));
      if (start > (total - maxRows)) start = Math.max(0, total - maxRows);
      var end = Math.min(total, start + maxRows);
      return {
        virtualized: true,
        start: start,
        end: end,
        padTop: start * rowHeight,
        padBottom: Math.max(0, (total - end) * rowHeight)
      };
    },
    get chatSidebarVirtualized() {
      var rows = Array.isArray(this.chatSidebarRows) ? this.chatSidebarRows : [];
      return this.chatSidebarDragRenderWindow(rows).virtualized;
    },
    get chatSidebarVirtualPadTop() {
      var rows = Array.isArray(this.chatSidebarRows) ? this.chatSidebarRows : [];
      return this.chatSidebarDragRenderWindow(rows).padTop;
    },
    get chatSidebarVirtualPadBottom() {
      var rows = Array.isArray(this.chatSidebarRows) ? this.chatSidebarRows : [];
      return this.chatSidebarDragRenderWindow(rows).padBottom;
    },
    get chatSidebarVisibleRows() {
      var rows = Array.isArray(this.chatSidebarRows) ? this.chatSidebarRows : [];
      var window = this.chatSidebarDragRenderWindow(rows);
      if (!window.virtualized) return rows;
      return rows.slice(window.start, window.end);
    },
    chatSidebarHasMoreRows() { return false; },
    showMoreChatSidebarRows() { this.scheduleSidebarScrollIndicators(); },
    init() {
      var self = this;
      this._bootSplashStartedAt = Date.now();
      this.bootSplashVisible = true;
      this.applyOverlayGlassTemplate('simple-glass', true);
      if (typeof this.resetBootProgress === 'function') this.resetBootProgress();
      if (typeof this.setBootProgressEvent === 'function') this.setBootProgressEvent('splash_visible');
      if (typeof this.hideDashboardPopupBySource === 'function') this.hideDashboardPopupBySource('sidebar');
      if (this._bootSplashMaxTimer) {
        clearTimeout(this._bootSplashMaxTimer);
        this._bootSplashMaxTimer = 0;
      }
      this._bootSplashMaxTimer = window.setTimeout(function() {
        self.releaseBootSplash(true);
      }, Number(this._bootSplashMaxMs || 5000));
      window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', function(e) {
        if (self.themeMode === 'system') {
          self.beginInstantThemeFlip();
          self.theme = e.matches ? 'dark' : 'light';
        }
      });
      var validPages = ['chat','agents','sessions','approvals','comms','workflows','scheduler','channels','eyes','skills','hands','overview','analytics','logs','runtime','settings','wizard'];
      var pageRedirects = {
        'automation': 'scheduler',
        'templates': 'agents',
        'triggers': 'workflows',
        'cron': 'scheduler',
        'schedules': 'scheduler',
        'memory': 'sessions',
        'audit': 'logs',
        'security': 'settings',
        'peers': 'settings',
        'migration': 'settings',
        'usage': 'analytics',
        'approval': 'approvals'
      };
      this.syncAgentChatsSectionForPage = function() {
        this.agentChatsSectionCollapsed = false;
      };
      this.toggleAgentChatsSection = function() {
        this.agentChatsSectionCollapsed = false;
      };
      var searchParams = new URLSearchParams(window.location.search || '');
      var embeddedDashboardMode = searchParams.get('embed') === '1';
      var embeddedPage = String(searchParams.get('page') || '').trim().toLowerCase();
      var pathnamePage = '';
      try {
        var pathname = String(window.location.pathname || '').trim();
        if (pathname.indexOf('/dashboard/') === 0) {
          pathnamePage = pathname.slice('/dashboard/'.length).split('/')[0].trim().toLowerCase();
        }
      } catch (_) {}
      if (embeddedDashboardMode && document && document.body && document.body.classList) {
        document.body.classList.add('dashboard-embedded-shell');
      }
      function handleHash() {
        var hash = window.location.hash.replace('#', '') || embeddedPage || pathnamePage || 'chat';
        if (pageRedirects[hash]) {
          hash = pageRedirects[hash];
          window.location.hash = hash;
        }
        if (validPages.indexOf(hash) >= 0) {
          self.page = hash;
          self.syncAgentChatsSectionForPage(hash);
          if (typeof self.syncPageHistory === 'function') self.syncPageHistory(hash);
        }
      }
      window.addEventListener('hashchange', handleHash);
      handleHash();

      document.addEventListener('keydown', function(e) {
        if ((e.ctrlKey || e.metaKey) && e.key === 'k') {
          e.preventDefault();
          self.navigate('agents');
        }
        if ((e.ctrlKey || e.metaKey) && e.key === 'n' && !e.shiftKey) {
          e.preventDefault();
          self.createSidebarAgentChat();
        }
        if ((e.ctrlKey || e.metaKey) && e.shiftKey && e.key === 'F') {
          e.preventDefault();
          var keyStore = self.getAppStore();
          if (keyStore && typeof keyStore.toggleFocusMode === 'function') {
            keyStore.toggleFocusMode();
          }
        }
        if (e.key === 'Escape') {
          self.mobileMenuOpen = false;
        }
      });

      InfringAPI.onConnectionChange(function(state) {
        var connStore = self.getAppStore();
        if (connStore) connStore.connectionState = state;
        self.connectionState = state;
        self.queueConnectionIndicatorState(state);
      });

      if (!window.__infringToastCaptureInstalled) {
        window.addEventListener('infring:toast', function(ev) {
          var detail = (ev && ev.detail) ? ev.detail : {};
          var store = self.getAppStore();
          if (store && typeof store.addNotification === 'function') {
            store.addNotification(detail);
          }
        });
        window.__infringToastCaptureInstalled = true;
      }

      this.pollStatus();
      var initStore = this.getAppStore();
      if (initStore && typeof initStore.checkOnboarding === 'function') initStore.checkOnboarding();
      if (initStore && typeof initStore.checkAuth === 'function') initStore.checkAuth();
      if (!this._dashboardClockTimer) this._dashboardClockTimer = setInterval(function() { self.clockTick = Date.now(); }, 1000);
      if (!this._dashboardStatusTimer) this._dashboardStatusTimer = setInterval(function() {
        if (document && document.hidden) return;
        self.pollStatus();
      }, 10000);
      if (!this._dashboardVisibilityHandler && document) {
        this._dashboardVisibilityHandler = function() { if (!document.hidden) self.pollStatus(); };
        document.addEventListener('visibilitychange', this._dashboardVisibilityHandler);
      }
      window.addEventListener('resize', function() {
        self.scheduleSidebarScrollIndicators();
      });
      this.$nextTick(function() {
        self.scheduleSidebarScrollIndicators();
      });
    },
    releaseBootSplash(force) {
      if (!this.bootSplashVisible) return;
      var now = Date.now();
      var elapsed = Math.max(0, now - Number(this._bootSplashStartedAt || now));
      var minRemain = Math.max(0, Number(this._bootSplashMinMs || 0) - elapsed);
      var store = this.getAppStore();
      var ready = !!force || !store || store.booting === false;
      if (!ready) return;
      if (typeof this.setBootProgressEvent === 'function') this.setBootProgressEvent('releasing', { bootStage: store && store.bootStage });
      if (this._bootSplashHideTimer) {
        clearTimeout(this._bootSplashHideTimer);
        this._bootSplashHideTimer = 0;
      }
      var self = this;
      var progressNow = typeof this.bootProgressClamped === 'function'
        ? this.bootProgressClamped(this.bootProgressPercent)
        : Math.max(0, Math.min(100, Number(this.bootProgressPercent || 0)));
      var completionAnimationDelayMs = progressNow < 100 ? 500 : 0;
      var hideDelayMs = Math.max(minRemain, completionAnimationDelayMs);
      if (typeof this.setBootProgressEvent === 'function') this.setBootProgressEvent('complete', { bootStage: store && store.bootStage });
      if (hideDelayMs <= 0) {
        this.bootSplashVisible = false;
        if (this._bootSplashMaxTimer) {
          clearTimeout(this._bootSplashMaxTimer);
          this._bootSplashMaxTimer = 0;
        }
        return;
      }
      this._bootSplashHideTimer = window.setTimeout(function() {
        self.bootSplashVisible = false;
        self._bootSplashHideTimer = 0;
        if (self._bootSplashMaxTimer) {
          clearTimeout(self._bootSplashMaxTimer);
          self._bootSplashMaxTimer = 0;
        }
      }, hideDelayMs);
    },
    normalizeNavigablePage(pageId) {
      var raw = String(pageId || '').trim().toLowerCase();
      if (!raw) return 'chat';
      var aliases = {
        'automation': 'scheduler',
        'templates': 'agents',
        'triggers': 'workflows',
        'cron': 'scheduler',
        'schedules': 'scheduler',
        'memory': 'sessions',
        'audit': 'logs',
        'security': 'settings',
        'peers': 'settings',
        'migration': 'settings',
        'usage': 'analytics',
        'approval': 'approvals'
      };
      return aliases[raw] || raw;
    },
    isKnownNavigablePage(pageId) {
      var normalized = this.normalizeNavigablePage(pageId);
      return ['chat','agents','sessions','approvals','comms','workflows','scheduler','channels','eyes','skills','hands','overview','analytics','logs','runtime','settings','wizard']
        .indexOf(normalized) >= 0;
    },
    syncPageHistory(nextPage) {
      var next = this.normalizeNavigablePage(nextPage);
      if (!this.isKnownNavigablePage(next)) return;
      var current = this.normalizeNavigablePage(this._navCurrentPage || this.page || '');
      var action = String(this._navHistoryAction || '').trim().toLowerCase();
      var back = Array.isArray(this.navBackStack) ? this.navBackStack.slice() : [];
      var forward = Array.isArray(this.navForwardStack) ? this.navForwardStack.slice() : [];
      var cap = Number(this._navHistoryCap || 48);
      if (!Number.isFinite(cap) || cap < 8) cap = 48;
      var trim = function(list) {
        return list.length > cap ? list.slice(list.length - cap) : list;
      };
      if (!current || !this.isKnownNavigablePage(current)) {
        this._navCurrentPage = next;
        this._navHistoryAction = '';
        return;
      }
      if (next === current) {
        this._navCurrentPage = next;
        this._navHistoryAction = '';
        return;
      }
      if (action === 'back') {
        if (forward.length === 0 || forward[forward.length - 1] !== current) forward.push(current);
      } else if (action === 'forward') {
        if (back.length === 0 || back[back.length - 1] !== current) back.push(current);
      } else if (back.length > 0 && back[back.length - 1] === next) {
        back.pop();
        if (forward.length === 0 || forward[forward.length - 1] !== current) forward.push(current);
      } else if (forward.length > 0 && forward[forward.length - 1] === next) {
        forward.pop();
        if (back.length === 0 || back[back.length - 1] !== current) back.push(current);
      } else {
        if (back.length === 0 || back[back.length - 1] !== current) back.push(current);
        forward = [];
      }
      this.navBackStack = trim(back);
      this.navForwardStack = trim(forward);
      this._navCurrentPage = next;
      this._navHistoryAction = '';
    },
    canNavigateBack() {
      return Array.isArray(this.navBackStack) && this.navBackStack.length > 0;
    },
    canNavigateForward() {
      return Array.isArray(this.navForwardStack) && this.navForwardStack.length > 0;
    },
    navigateBackPage() {
      if (!this.canNavigateBack()) return;
      var back = this.navBackStack.slice();
      var target = this.normalizeNavigablePage(back.pop());
      this.navBackStack = back;
      this._navHistoryAction = 'back';
      if (!target || target === this.normalizeNavigablePage(this.page)) {
        this._navHistoryAction = '';
        return;
      }
      this.navigate(target);
    },
    navigateForwardPage() {
      if (!this.canNavigateForward()) return;
      var forward = this.navForwardStack.slice();
      var target = this.normalizeNavigablePage(forward.pop());
      this.navForwardStack = forward;
      this._navHistoryAction = 'forward';
      if (!target || target === this.normalizeNavigablePage(this.page)) {
        this._navHistoryAction = '';
        return;
      }
      this.navigate(target);
    },
    navigate(p) {
      if (typeof this.hideDashboardPopupBySource === 'function') this.hideDashboardPopupBySource('sidebar');
      if (String(p || '') !== 'chat') {
        var store = this.getAppStore();
        var pendingId = String((store && store.pendingFreshAgentId) || '').trim();
        var activeId = String((store && store.activeAgentId) || '').trim();
        if (pendingId) {
          if (store) {
            store.pendingFreshAgentId = null;
            store.pendingAgent = null;
            if (pendingId === activeId) {
              if (typeof store.setActiveAgentId === 'function') store.setActiveAgentId(null);
              else store.activeAgentId = null;
            }
          }
          this.chatSidebarTopologyOrder = (this.chatSidebarTopologyOrder || []).filter(function(id) {
            return String(id || '').trim() !== pendingId;
          });
          this.persistChatSidebarTopologyOrder();
          InfringAPI.del('/api/agents/' + encodeURIComponent(pendingId)).catch(function() {});
          if (store && typeof store.refreshAgents === 'function') setTimeout(function() { store.refreshAgents({ force: true }).catch(function() {}); }, 0);
        }
      }
      this.page = p;
      if (typeof this.syncAgentChatsSectionForPage === 'function') {
        this.syncAgentChatsSectionForPage(p);
      }
      window.location.hash = p;

      this.mobileMenuOpen = false;
    },
    setTheme(mode) {
      this.beginInstantThemeFlip();
      this.themeMode = mode;
      localStorage.setItem('infring-theme-mode', mode);
      if (mode === 'system') {
        this.theme = window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
      } else {
        this.theme = mode;
      }
    },
    isChatSidebarSearchActive() {
      return String(this.chatSidebarQuery || '').trim().length > 0;
    },
    clearChatSidebarSearch() {
      if (this._chatSidebarSearchTimer) { clearTimeout(this._chatSidebarSearchTimer); this._chatSidebarSearchTimer = 0; }
      this.chatSidebarSearchSeq = Number(this.chatSidebarSearchSeq || 0) + 1;
      this.chatSidebarSearchLoading = false;
      this.chatSidebarSearchError = '';
      this.chatSidebarSearchResults = [];
      this.scheduleSidebarScrollIndicators();
    },
    onChatSidebarQueryInput(value) {
      this.chatSidebarQuery = String(value || '');
      this.chatSidebarVisibleCount = Math.max(1, Math.floor(Number(this.chatSidebarVisibleBase || 7)));
      var query = String(this.chatSidebarQuery || '').trim();
      if (!query) {
        this.clearChatSidebarSearch();
        return;
      }
      this.scheduleChatSidebarSearch();
    },
    scheduleChatSidebarSearch() {
      var query = String(this.chatSidebarQuery || '').trim();
      if (!query) { this.clearChatSidebarSearch(); return; }
      if (this._chatSidebarSearchTimer) { clearTimeout(this._chatSidebarSearchTimer); this._chatSidebarSearchTimer = 0; }
      var self = this;
      var seq = Number(this.chatSidebarSearchSeq || 0) + 1;
      this.chatSidebarSearchSeq = seq;
      this.chatSidebarSearchLoading = true;
      this.chatSidebarSearchError = '';
      this._chatSidebarSearchTimer = setTimeout(function() { self._chatSidebarSearchTimer = 0; self.runChatSidebarSearch(seq); }, 140);
    },
    async runChatSidebarSearch(seq) {
      var token = Number(seq || 0);
      var currentToken = Number(this.chatSidebarSearchSeq || 0);
      if (token !== currentToken) return;
      var query = String(this.chatSidebarQuery || '').trim();
      if (!query) {
        this.clearChatSidebarSearch();
        return;
      }
      try {
        var path = '/api/search/conversations?q=' + encodeURIComponent(query) + '&limit=80';
        var payload = await InfringAPI.get(path);
        if (token !== Number(this.chatSidebarSearchSeq || 0)) return;
        var self = this;
        var serverRows = payload && Array.isArray(payload.sidebar_rows) ? payload.sidebar_rows : null;
        if (serverRows && serverRows.length) {
          this.chatSidebarSearchResults = serverRows.filter(function(agent) {
            return !self.isSidebarArchivedAgent(agent);
          }).map(function(agent) {
            return self.sanitizeSidebarAgentRow(agent);
          });
          this.chatSidebarSearchError = '';
          return;
        }
        var quickRows = payload && Array.isArray(payload.quick_actions) ? payload.quick_actions : [];
        this.chatSidebarSearchResults = quickRows.filter(function(agent) {
          return !self.isSidebarArchivedAgent(agent);
        }).map(function(agent) {
          return self.sanitizeSidebarAgentRow(agent);
        });
        this.chatSidebarSearchError = '';
      } catch (e) {
        if (token !== Number(this.chatSidebarSearchSeq || 0)) return;
        this.chatSidebarSearchResults = [];
        this.chatSidebarSearchError = String(e && e.message ? e.message : 'search_failed');
      } finally {
        if (token === Number(this.chatSidebarSearchSeq || 0)) {
          this.chatSidebarSearchLoading = false;
        }
        this.scheduleSidebarScrollIndicators();
      }
    },
    overlayGlassTemplateNormalized(modeRaw) {
      var mode = String(modeRaw || '').trim().toLowerCase();
      if (mode === 'simple-glass') return 'simple-glass';
      if (mode === 'fogged-glass') return 'fogged-glass';
      if (mode === 'warped-glass' || mode === 'magnified-glass') return 'warped-glass';
      if (mode === 'liquid-glass') return 'fogged-glass';
      return 'simple-glass';
    },
    applyOverlayGlassTemplate(modeRaw, persistRaw) {
      var mode = this.overlayGlassTemplateNormalized(modeRaw);
      this.overlayGlassTemplate = mode;
      var persist = persistRaw !== false;
      if (document && document.documentElement) {
        try {
          document.documentElement.setAttribute('data-overlay-glass-template', mode);
        } catch (_) {}
      }
      if (persist) {
        try { localStorage.setItem('infring-overlay-glass-template', mode); } catch (_) {}
      }
      return mode;
    },
    uiBackgroundTemplateNormalized(modeRaw) {
      var service = this.taskbarDockService ? this.taskbarDockService() : infringTaskbarDockService();
      if (service && typeof service.normalizeBackgroundTemplate === 'function') return service.normalizeBackgroundTemplate(modeRaw);
      var mode = String(modeRaw || '').trim().toLowerCase();
      if (mode === 'unsplash-paper') return 'light-wood';
      if (mode === 'default-grid') return 'default-grid';
      if (mode === 'light-wood') return 'light-wood';
      if (mode === 'sand') return 'sand';
      return 'sand';
    },
    applyUiBackgroundTemplate(modeRaw, persistRaw) {
      var mode = this.uiBackgroundTemplateNormalized(modeRaw);
      this.uiBackgroundTemplate = mode;
      var persist = persistRaw !== false;
      if (document && document.documentElement) {
        try {
          document.documentElement.setAttribute('data-ui-background-template', mode);
        } catch (_) {}
      }
      if (persist) {
        try {
          var service = this.taskbarDockService ? this.taskbarDockService() : infringTaskbarDockService();
          if (service && typeof service.writeDisplayBackground === 'function') service.writeDisplayBackground(mode);
          else {
            var rawDisplaySettings = localStorage.getItem('infring-display-settings') || '';
            var displaySettings = rawDisplaySettings ? JSON.parse(rawDisplaySettings) : {};
            displaySettings = displaySettings && typeof displaySettings === 'object' ? displaySettings : {};
            displaySettings.background = mode;
            localStorage.setItem('infring-display-settings', JSON.stringify(displaySettings));
          }
        } catch (_) {}
      }
      return mode;
    },
    beginInstantThemeFlip() {
      var self = this;
      var body = document && document.body ? document.body : null;
      if (!body) return;
      body.classList.add('theme-switching');
      // Force style flush so no-transition styles are applied before theme variables swap.
      void body.offsetHeight;
      if (this._themeSwitchReset) {
        clearTimeout(this._themeSwitchReset);
      }
      this._themeSwitchReset = window.setTimeout(function() {
        body.classList.remove('theme-switching');
        self._themeSwitchReset = 0;
      }, 260);
    },
    toggleTheme() {
      var modes = ['light', 'system', 'dark'];
      var next = modes[(modes.indexOf(this.themeMode) + 1) % modes.length];
      this.setTheme(next);
    },
    toggleSidebar() {
      if (typeof this.shouldSuppressSidebarToggle === 'function' && this.shouldSuppressSidebarToggle()) return;
      var nextCollapsed = !this.sidebarCollapsed;
      var resolveMessagesHost = function() {
        var nodes = document.querySelectorAll('#messages');
        for (var ni = 0; ni < nodes.length; ni++) if (nodes[ni] && nodes[ni].offsetParent !== null) return nodes[ni];
        return nodes && nodes.length ? nodes[0] : null;
      };
      var captureMessageBottomAnchor = function() {
        var host = resolveMessagesHost();
        if (!host || host.offsetParent === null) return null;
        var hostRect = host.getBoundingClientRect();
        var input = document.getElementById('msg-input');
        var alignY = hostRect.bottom;
        if (input && input.offsetParent !== null) {
          var inputRect = input.getBoundingClientRect();
          if (inputRect.top > hostRect.top && inputRect.top < (hostRect.bottom + 140)) alignY = inputRect.top;
        }
        var rows = host.querySelectorAll('.chat-message-block .message[id]');
        var best = null;
        var bestDiff = Number.POSITIVE_INFINITY;
        for (var i = 0; i < rows.length; i++) {
          var row = rows[i];
          if (!row || row.offsetParent === null) continue;
          var rect = row.getBoundingClientRect();
          if (rect.bottom < (hostRect.top - 40) || rect.top > (hostRect.bottom + 40)) continue;
          var diff = Math.abs(rect.bottom - alignY);
          if (diff < bestDiff) { bestDiff = diff; best = row; }
        }
        return best && best.id ? { id: String(best.id) } : null;
      };
      if (nextCollapsed) this._sidebarChatAnchorForExpand = captureMessageBottomAnchor();
      this.sidebarCollapsed = nextCollapsed;
      localStorage.setItem('infring-sidebar', this.sidebarCollapsed ? 'collapsed' : 'expanded');
      // Always clear stale sidebar popup when toggling sidebar state.
      this.hideDashboardPopupBySource('sidebar');
      if (!nextCollapsed) {
        var anchor = (this._sidebarChatAnchorForExpand && this._sidebarChatAnchorForExpand.id)
          ? this._sidebarChatAnchorForExpand
          : captureMessageBottomAnchor();
        this._sidebarChatAnchorForExpand = null;
        var passes = 4;
        var restoreAnchor = function() {
          var host = resolveMessagesHost();
          if (!host || host.offsetParent === null || !anchor || !anchor.id) return;
          var row = document.getElementById(anchor.id);
          if (!row || !host.contains(row) || row.offsetParent === null) return;
          var hostRect = host.getBoundingClientRect();
          var input = document.getElementById('msg-input');
          var alignY = hostRect.bottom;
          if (input && input.offsetParent !== null) {
            var inputRect = input.getBoundingClientRect();
            if (inputRect.top > hostRect.top && inputRect.top < (hostRect.bottom + 140)) alignY = inputRect.top;
          }
          var alignOffset = Math.max(0, Math.min(Math.max(0, Number(host.clientHeight || 0)), Math.round(alignY - hostRect.top)));
          var rowBottom = Number(row.offsetTop || 0) + Math.max(0, Number(row.offsetHeight || 0));
          var maxTop = Math.max(0, Number(host.scrollHeight || 0) - Math.max(0, Number(host.clientHeight || 0)));
          var nextTop = Math.max(0, Math.min(maxTop, Math.round(rowBottom - alignOffset)));
          host.scrollTop = nextTop;
          if (passes-- > 1 && typeof requestAnimationFrame === 'function') requestAnimationFrame(restoreAnchor);
          try { host.dispatchEvent(new Event('scroll')); } catch (_) {}
        };
        if (typeof requestAnimationFrame === 'function') requestAnimationFrame(restoreAnchor);
        else setTimeout(restoreAnchor, 0);
      }
      this.scheduleSidebarScrollIndicators();
    },
    runtimeFacadeHealthSummary() {
      var summary = this.healthSummary && typeof this.healthSummary === 'object' ? this.healthSummary : null;
      if (!summary) return null;
      var loadedAt = Number(this._healthSummaryLoadedAt || 0);
      if (loadedAt > 0 && (Date.now() - loadedAt) > 60000) return null;
      return summary;
    },
    runtimeFacadeState() {
      var store = this.getAppStore();
      var conn = this.normalizeConnectionIndicatorState(
        this.connectionIndicatorState ||
        ((store && store.connectionState) || this.connectionState || '')
      );
      if (conn === 'connecting') return 'connecting';
      if (conn === 'disconnected') return this.runtimeFacadeHealthSummary() ? 'connecting' : 'down';
      if (this.runtimeEtaSeconds() > 0) return 'active';
      return 'connected';
    },
    runtimeFacadeClass() {
      var state = this.runtimeFacadeState();
      if (state === 'connected' || state === 'active') return 'health-ok';
      if (state === 'connecting') return 'health-connecting';
      return 'health-down';
    },
    runtimeFacadeLabel() {
      var state = this.runtimeFacadeState();
      if (state === 'active') return 'Active';
      if (state === 'connected') {
        var store = this.getAppStore();
        var health = this.runtimeFacadeHealthSummary();
        var agents = ((store && store.agents && store.agents.length) || (store && store.agentCount) || this.agentCount || Number(health && health.agent_count || 0) || Number(health && health.agents && health.agents.length || 0));
        return String(agents) + ' agents';
      }
      if (state === 'connecting' && this.runtimeFacadeHealthSummary()) return 'Reconnecting...';
      if (state === 'connecting') return 'Connecting...';
      return 'Disconnected';
    },
    runtimeFacadeDisplayLabel() {
      var label = String(this.runtimeFacadeLabel() || '').trim();
      if (!label) return '';
      return label.replace(/\s+agents?$/i, '');
    },
    runtimeResponseP95Ms() {
      var store = this.getAppStore();
      var runtime = store && store.runtimeSync && typeof store.runtimeSync === 'object'
        ? store.runtimeSync
        : null;
      if (!runtime) {
        var health = this.runtimeFacadeHealthSummary();
        var durationMs = Number(health && health.durationMs);
        return Number.isFinite(durationMs) && durationMs >= 0 ? Math.round(durationMs) : null;
      }
      var facadeP95 = Number(runtime.facade_response_p95_ms);
      if (Number.isFinite(facadeP95) && facadeP95 > 0) return Math.round(facadeP95);
      var p95 = Number(runtime.receipt_latency_p95_ms);
      if (Number.isFinite(p95) && p95 > 0) return Math.round(p95);
      var p99 = Number(runtime.receipt_latency_p99_ms);
      if (Number.isFinite(p99) && p99 > 0) return Math.round(p99);
      return null;
    },
    runtimeConfidencePercent() {
      var store = this.getAppStore();
      var runtime = store && store.runtimeSync && typeof store.runtimeSync === 'object'
        ? store.runtimeSync
        : null;
      if (!runtime) return this.runtimeFacadeHealthSummary() ? 92 : 80;
      var facadeConfidence = Number(runtime.facade_confidence_percent);
      if (Number.isFinite(facadeConfidence) && facadeConfidence > 0) {
        return Math.max(10, Math.min(100, Math.round(facadeConfidence)));
      }

      var score = 100;
      var queueDepth = Number(runtime.queue_depth || 0);
      var stale = Number(runtime.cockpit_stale_blocks || 0);
      var gaps = Number(runtime.health_coverage_gap_count || 0);
      var conduitSignals = Number(runtime.conduit_signals || 0);
      var targetSignals = Math.max(1, Number(runtime.target_conduit_signals || 4));
      var benchmark = String(runtime.benchmark_sanity_cockpit_status || runtime.benchmark_sanity_status || 'unknown').toLowerCase();
      var spine = Number(runtime.spine_success_rate);

      if (queueDepth > 20) score -= Math.min(20, Math.floor((queueDepth - 20) / 2));
      if (stale > 0) score -= Math.min(20, stale * 2);
      if (gaps > 0) score -= Math.min(20, gaps * 6);
      if (conduitSignals < Math.max(3, Math.floor(targetSignals * 0.5))) score -= 12;
      if (benchmark === 'warn') score -= 8;
      if (benchmark === 'fail' || benchmark === 'error') score -= 20;
      if (Number.isFinite(spine)) {
        if (spine < 0.9) score -= 15;
        if (spine < 0.6) score -= 10;
      }

      score = Math.max(10, Math.min(100, Math.round(score)));
      return score;
    },
    runtimeEtaSeconds() {
      var store = this.getAppStore();
      var runtime = store && store.runtimeSync && typeof store.runtimeSync === 'object'
        ? store.runtimeSync
        : null;
      if (!runtime) return 0;
      var facadeEta = Number(runtime.facade_eta_seconds);
      if (Number.isFinite(facadeEta) && facadeEta >= 0) {
        return Math.max(0, Math.min(300, Math.round(facadeEta)));
      }
      var queueDepth = Math.max(0, Number(runtime.queue_depth || 0));
      if (queueDepth <= 0) return 0;
      // Conservative client-side estimate for "Active" mode only.
      return Math.max(1, Math.min(300, Math.ceil(queueDepth / 8)));
    },
    runtimeFacadeDetail() {
      var state = this.runtimeFacadeState();
      var store = this.getAppStore();
      var bootStage = String((store && store.bootStage) || '').trim();
      var stageSuffix = bootStage ? (' · ' + bootStage.replace(/_/g, ' ')) : '';
      if (state === 'connecting' && this.runtimeFacadeHealthSummary()) return 'HTTP health OK · reconnecting live runtime' + stageSuffix;
      if (state === 'connecting') return 'Establishing runtime link' + stageSuffix;
      if (state === 'down') return 'Runtime unavailable' + stageSuffix;
      var response = this.runtimeResponseP95Ms();
      var confidence = this.runtimeConfidencePercent();
      var health = this.runtimeFacadeHealthSummary();
      var agents = ((store && store.agents && store.agents.length) || (store && store.agentCount) || Number(health && health.agent_count || 0) || Number(health && health.agents && health.agents.length || 0));
      var base = 'Response ' + (response != null ? (response + 'ms') : '—') + ' · Confidence ' + confidence + '%';
      if (store && store.statusDegraded) {
        return base + ' · Status degraded' + stageSuffix;
      }
      if (state === 'active') {
        var eta = this.runtimeEtaSeconds();
        return (eta > 0 ? ('ETA ~' + eta + 's · ') : '') + base;
      }
      return base + ' · ' + agents + ' agent(s)';
    },
    runtimeFacadeTitle() {
      return this.runtimeFacadeLabel();
    },
    taskbarClockParts() {
      var tick = Number(this.clockTick || Date.now());
      var dt = new Date(tick);
      if (!Number.isFinite(dt.getTime())) dt = new Date();
      var dayNames = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'];
      var monthNames = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];
      var dayName = dayNames[dt.getDay()] || '';
      var monthName = monthNames[dt.getMonth()] || '';
      var day = dt.getDate();
      var hours24 = dt.getHours();
      var minutes = dt.getMinutes();
      var suffix = hours24 >= 12 ? 'PM' : 'AM';
      var hours12 = hours24 % 12;
      if (hours12 === 0) hours12 = 12;
      var minuteText = minutes < 10 ? ('0' + minutes) : String(minutes);
      return {
        main: dayName + ' ' + monthName + ' ' + day + ' ' + hours12 + ':' + minuteText,
        meridiem: suffix
      };
    },
    taskbarClockMainLabel() {
      return this.taskbarClockParts().main;
    },
    taskbarClockMeridiemLabel() {
      return this.taskbarClockParts().meridiem;
    },
    taskbarClockLabel() {
      var parts = this.taskbarClockParts();
      return parts.main + ' ' + parts.meridiem;
    },
    toggleAgentChatsSidebar() {
      if (this.sidebarCollapsed) {
        this.sidebarCollapsed = false;
        localStorage.setItem('infring-sidebar', 'expanded');
      }
      this.hideDashboardPopupBySource('sidebar');
      this.scheduleSidebarScrollIndicators();
    },
    closeAgentChatsSidebar() {
      if (this.chatSidebarMode !== 'default') {
        this.chatSidebarMode = 'default';
        this.chatSidebarQuery = '';
        this.clearChatSidebarSearch();
      }
      this.confirmArchiveAgentId = '';
      this.scheduleSidebarScrollIndicators();
    },
    async applyBootChatSelection() {
      if (this.bootSelectionApplied) return;
      var store = this.getAppStore();
      if (!store || store.agentsLoading || !store.agentsHydrated) {
        return;
      }
      var rows = Array.isArray(store.agents) ? store.agents.slice() : [];
      if (!rows.length) {
        this.bootSelectionApplied = true;
        if (typeof store.setActiveAgentId === 'function') store.setActiveAgentId(null);
        else store.activeAgentId = null;
        this.navigate('chat');
        this.chatSidebarQuery = '';
        this.clearChatSidebarSearch();
        return;
      }
      var target = null;
      if (store.activeAgentId) {
        var saved = String(store.activeAgentId);
        target = rows.find(function(agent) { return agent && String(agent.id) === saved; }) || null;
      }
      if (!target) {
        rows.sort(function(a, b) {

// Layer ownership: client/runtime/systems/ui (dashboard static UX surface only; no runtime authority).
          return this.chatSidebarSortComparator(a, b);
        }.bind(this));
        target = rows.length ? rows[0] : null;
      }
      if (target && target.id) {
        if (typeof store.setActiveAgentId === 'function') store.setActiveAgentId(target.id);
        else store.activeAgentId = target.id;
      }
      this.bootSelectionApplied = true;
      this.navigate('chat');
      this.closeAgentChatsSidebar();
    },
    sidebarAgentSortTs(agent) {
      if (!agent) return 0;
      var serverTs = Number(agent.sidebar_sort_ts);
      if (Number.isFinite(serverTs) && serverTs > 0) return Math.round(serverTs);
      return 0;
    },
    chatSidebarTopologyKey(agent) {
      if (!agent || !agent.id) return 'z|~~~~|';
      var serverKey = String(agent.sidebar_topology_key || '').trim().toLowerCase();
      if (serverKey) return serverKey;
      return 'z|' + String(agent.id || '').trim().toLowerCase();
    },
    chatSidebarSortComparator(a, b) {
      var mode = String(this.chatSidebarSortMode || '').toLowerCase();
      if (mode === 'topology') {
        var topoA = this.chatSidebarTopologyKey(a);
        var topoB = this.chatSidebarTopologyKey(b);
        if (topoA < topoB) return -1;
        if (topoA > topoB) return 1;
      }
      var byTs = this.sidebarAgentSortTs(b) - this.sidebarAgentSortTs(a);
      if (byTs !== 0) return byTs;
      var aName = String((a && (a.name || a.id)) || '').toLowerCase();
      var bName = String((b && (b.name || b.id)) || '').toLowerCase();
      if (aName < bName) return -1;
      if (aName > bName) return 1;
      return 0;
    },
    syncChatSidebarTopologyOrderFromAgents() {
      var self = this;
      var pool = (this.agents || []).filter(function(agent) {
        if (!agent || !agent.id) return false;
        return !(typeof self.isSidebarArchivedAgent === 'function' && self.isSidebarArchivedAgent(agent));
      });
      pool.sort(function(a, b) {
        return self.chatSidebarSortComparator(a, b);
      });
      var liveIds = pool.map(function(agent) { return String(agent.id); });
      var liveSet = new Set(liveIds);
      var seen = {};
      var prior = Array.isArray(this.chatSidebarTopologyOrder) ? this.chatSidebarTopologyOrder : [];
      var next = [];
      prior.forEach(function(id) {
        var key = String(id || '').trim();
        if (!key || seen[key] || !liveSet.has(key)) return;
        seen[key] = true;
        next.push(key);
      });
      liveIds.forEach(function(id) {
        if (seen[id]) return;
        seen[id] = true;
        next.push(id);
      });
      var changed = next.length !== prior.length;
      if (!changed) changed = next.some(function(id, idx) { return id !== String(prior[idx] || ''); });
      if (changed) {
        this.chatSidebarTopologyOrder = next;
        this.persistChatSidebarTopologyOrder();
      }
    },
    setChatSidebarSortMode(mode) {
      var normalized = String(mode || '').trim().toLowerCase() === 'topology' ? 'topology' : 'age';
      this.chatSidebarSortMode = normalized;
      if (normalized === 'topology' && typeof this.syncChatSidebarTopologyOrderFromAgents === 'function') {
        this.syncChatSidebarTopologyOrderFromAgents();
      } else if (typeof this.endChatSidebarTopologyDrag === 'function') {
        this.endChatSidebarTopologyDrag();
      }
      try {
        localStorage.setItem('infring-chat-sidebar-sort-mode', normalized);
      } catch(_) {}
      this.scheduleSidebarScrollIndicators();
    },
    chatSidebarPreview(agent) {
      if (!agent) return { text: 'No messages yet', ts: 0, role: 'agent', has_tools: false, tool_state: '', tool_label: '', unread_response: false };
      if (agent.revive_recommended === true) {
        return {
          text: 'Open chat to revive',
          ts: this.sidebarAgentSortTs(agent),
          role: 'agent',
          has_tools: false,
          tool_state: '',
          tool_label: '',
          unread_response: false
        };
      }
      var isSystemThread = agent.is_system_thread === true || String(agent.id || '').toLowerCase() === 'system';
      var fallbackText = isSystemThread ? '' : 'No messages yet'; if (typeof this._isCollapsedHoverStatePlaceholderText === 'function' && this._isCollapsedHoverStatePlaceholderText(fallbackText)) fallbackText = '';
      var store = this.getAppStore();
      var preview = store && typeof store.getAgentChatPreview === 'function' ? store.getAgentChatPreview(agent.id) : null;
      var serverPreview = agent && agent.sidebar_preview && typeof agent.sidebar_preview === 'object' ? agent.sidebar_preview : null;
      if (serverPreview && typeof serverPreview === 'object') {
        var serverText = String(serverPreview.text || '').trim();
        return {
          text: serverText || fallbackText,
          ts: Number(serverPreview.ts || this.sidebarAgentSortTs(agent)) || this.sidebarAgentSortTs(agent),
          role: String(serverPreview.role || 'assistant'),
          has_tools: !!serverPreview.has_tools,
          tool_state: String(serverPreview.tool_state || ''),
          tool_label: String(serverPreview.tool_label || ''),
          unread_response: !!(preview && preview.unread_response)
        };
      }
      if (isSystemThread) {
        return {
          text: '',
          ts: preview && preview.ts ? preview.ts : this.sidebarAgentSortTs(agent),
          role: 'agent',
          has_tools: !!(preview && preview.has_tools),
          tool_state: preview && preview.tool_state ? preview.tool_state : '',
          tool_label: preview && preview.tool_label ? preview.tool_label : '',
          unread_response: !!(preview && preview.unread_response)
        };
      }
      return { text: fallbackText, ts: this.sidebarAgentSortTs(agent), role: 'agent', has_tools: false, tool_state: '', tool_label: '', unread_response: false };
    },
    sidebarDisplayEmoji(agent) {
      if (!agent) return '';
      var isSystem = this.isSystemSidebarThread && this.isSystemSidebarThread(agent);
      if (isSystem) return '\u2699\ufe0f';
      var emoji = String((agent.identity && agent.identity.emoji) || '').trim();
      if (this.isReservedSystemEmoji && this.isReservedSystemEmoji(emoji)) return '';
      return emoji;
    },
    async archiveAgentFromSidebar(agent) {
      if (!agent || !agent.id) return;
      var agentId = String(agent.id);
      if (typeof this.isSidebarArchivedAgent === 'function' && this.isSidebarArchivedAgent(agent)) return;
      this.confirmArchiveAgentId = '';
      var missingPurged = false;
      try {
        await InfringAPI.del('/api/agents/' + encodeURIComponent(agentId));
      } catch(e) {
        var msg = String(e && e.message ? e.message : '');
        if (msg.indexOf('agent_not_found') >= 0) {
          missingPurged = true;
        } else {
          InfringToast.error('Failed to archive agent: ' + (e && e.message ? e.message : 'unknown error'));
          return;
        }
      }
      this.syncChatSidebarTopologyOrderFromAgents();
      var store = this.getAppStore();
      if (store.activeAgentId === agent.id) {
        var next = this.chatSidebarAgents.length ? this.chatSidebarAgents[0] : null;
        if (next && next.id) {
          if (typeof store.setActiveAgentId === 'function') store.setActiveAgentId(next.id);
          else store.activeAgentId = next.id;
        } else {
          if (typeof store.setActiveAgentId === 'function') store.setActiveAgentId(null);
          else store.activeAgentId = null;
        }
      }
      await store.refreshAgents();
      if (missingPurged) {
        InfringToast.success('Removed stale agent "' + (agent.name || agent.id) + '"');
      } else {
        InfringToast.success('Archived "' + (agent.name || agent.id) + '"');
      }
      this.scheduleSidebarScrollIndicators();
    },
    async createSidebarAgentChat() {
      if (this.sidebarSpawningAgent) return;
      this.confirmArchiveAgentId = '';
      this.sidebarSpawningAgent = true;
      try {
        var res = await InfringAPI.post('/api/agents', {
          role: 'analyst'
        });
        var createdId = String((res && (res.id || res.agent_id)) || '').trim();
        if (!createdId) throw new Error('spawn_failed');
        var store = this.getAppStore();
        if (!store || typeof store.refreshAgents !== 'function') throw new Error('app_store_unavailable');
        await store.refreshAgents({ force: true });
        var authoritative = null;
        if (Array.isArray(store.agents)) {
          for (var ai = 0; ai < store.agents.length; ai++) {
            var row = store.agents[ai];
            if (row && String((row && row.id) || '') === createdId) {
              authoritative = row;
              break;
            }
          }
        }
        if (!authoritative) {
          try {
            authoritative = await InfringAPI.get('/api/agents/' + encodeURIComponent(createdId));
          } catch(_) {}
        }
        var createdSource = authoritative && typeof authoritative === 'object'
          ? Object.assign({}, res || {}, authoritative)
          : (res && typeof res === 'object' ? Object.assign({}, res) : {});
        var createdStatusState = String((createdSource && createdSource.sidebar_status_state) || '').trim().toLowerCase();
        if (createdStatusState !== 'active' && createdStatusState !== 'idle' && createdStatusState !== 'offline') {
          createdStatusState = '';
        }
        var createdStatusLabel = String((createdSource && createdSource.sidebar_status_label) || '').trim().toLowerCase();
        if (createdStatusLabel !== 'active' && createdStatusLabel !== 'idle' && createdStatusLabel !== 'offline') {
          createdStatusLabel = createdStatusState;
        }
        var createdFreshness = {
          source: String((createdSource && createdSource.sidebar_status_source) || ''),
          source_sequence: String((createdSource && createdSource.sidebar_status_source_sequence) || ''),
          age_seconds: Number((createdSource && createdSource.sidebar_status_age_seconds) || 0),
          stale: !!(createdSource && createdSource.sidebar_status_stale === true)
        };
        var created = Object.assign({}, createdSource, {
          id: createdId,
          agent_id: createdId,
          name: String((createdSource && createdSource.name) || createdId),
          role: String((createdSource && createdSource.role) || 'analyst'),
          identity: (createdSource && createdSource.identity && typeof createdSource.identity === 'object') ? createdSource.identity : {},
          avatar_url: String((createdSource && createdSource.avatar_url) || ''),
          state: String((createdSource && createdSource.state) || createdStatusLabel || createdStatusState || 'Running'),
          sidebar_status_state: createdStatusState || 'active',
          sidebar_status_label: createdStatusLabel || createdStatusState || 'active',
          sidebar_status_source: createdFreshness.source,
          sidebar_status_source_sequence: createdFreshness.source_sequence,
          sidebar_status_age_seconds: createdFreshness.age_seconds,
          sidebar_status_stale: createdFreshness.stale,
          sidebar_status_freshness: createdFreshness,
          model_name: String((createdSource && (createdSource.model_name || createdSource.runtime_model || '')) || ''),
          model_provider: String((createdSource && createdSource.model_provider) || ''),
          runtime_model: String((createdSource && createdSource.runtime_model) || ''),
          created_at: String((createdSource && createdSource.created_at) || new Date().toISOString())
        });
        this.syncChatSidebarTopologyOrderFromAgents();
        store.pendingAgent = created;
        store.pendingFreshAgentId = created.id;
        if (typeof store.setActiveAgentId === 'function') store.setActiveAgentId(created.id);
        else store.activeAgentId = created.id;
        this.navigate('chat');
        this.closeAgentChatsSidebar();
        InfringToast.success('Agent draft created. Complete initialization to launch.');
        this.scheduleSidebarScrollIndicators();
        // Keep draft agent hidden from rosters until launch completes.
      } catch(e) {
        InfringToast.error('Failed to create agent: ' + (e && e.message ? e.message : 'unknown error'));
      }
      this.sidebarSpawningAgent = false;
    },
    selectAgentChatFromSidebar(agent) {
      if (!agent || !agent.id) return;
      if (typeof this.hideDashboardPopupBySource === 'function') this.hideDashboardPopupBySource('sidebar');
      this.confirmArchiveAgentId = '';
      var quickAction = agent && agent._sidebar_quick_action && typeof agent._sidebar_quick_action === 'object' ? agent._sidebar_quick_action : null;
      if (quickAction) {
        var actionType = String(quickAction.type || '').trim().toLowerCase();
        if (actionType === 'copy_connect') {
          var checklist = 'Gateway connect checklist: open Settings, verify pairing or API token setup, and use HTTPS or localhost when device identity is required.';
          try { if (navigator && navigator.clipboard && typeof navigator.clipboard.writeText === 'function') navigator.clipboard.writeText(checklist).catch(function() {}); } catch(_) {}
          InfringToast.success('Copied connection checklist');
        }
        this.navigate(quickAction.page || 'chat');
        this.clearChatSidebarSearch();
        this.closeAgentChatsSidebar();
        this.scheduleSidebarScrollIndicators();
        return;
      }
      var store = this.getAppStore();
      var archived = typeof this.isSidebarArchivedAgent === 'function' && this.isSidebarArchivedAgent(agent);
      if (store && archived) {
        var pendingState = '';
        var rawSidebarStatusState = (typeof agent.sidebar_status_state === 'string')
          ? agent.sidebar_status_state
          : '';
        var rawSidebarStatusLabel = (typeof agent.sidebar_status_label === 'string')
          ? agent.sidebar_status_label
          : '';
        if (typeof this.agentStatusLabel === 'function') {
          pendingState = String(this.agentStatusLabel(agent) || '').trim().toLowerCase();
        }
        if (!pendingState) pendingState = 'offline';
        var pending = {
          id: String(agent.id),
          name: String(agent.name || agent.id),
          state: pendingState,
          archived: true,
          avatar_url: String(agent.avatar_url || '').trim(),
          sidebar_status_state: String(rawSidebarStatusState).trim().toLowerCase(),
          sidebar_status_label: String(rawSidebarStatusLabel).trim().toLowerCase(),
          sidebar_status_source: String(agent.sidebar_status_source || ''),
          sidebar_status_source_sequence: String(agent.sidebar_status_source_sequence || ''),
          sidebar_status_age_seconds: Number(agent.sidebar_status_age_seconds || 0),
          sidebar_status_stale: !!(agent.sidebar_status_stale === true),
          sidebar_status_freshness: agent.sidebar_status_freshness && typeof agent.sidebar_status_freshness === 'object'
            ? agent.sidebar_status_freshness
            : {
                source: String(agent.sidebar_status_source || ''),
                source_sequence: String(agent.sidebar_status_source_sequence || ''),
                age_seconds: Number(agent.sidebar_status_age_seconds || 0),
                stale: !!(agent.sidebar_status_stale === true)
              },
          identity: { emoji: String((agent.identity && agent.identity.emoji) || '') },
          role: String(agent.role || 'analyst')
        };
        store.pendingAgent = pending;
        store.pendingFreshAgentId = null;
      }
      if (store && typeof store.setActiveAgentId === 'function') store.setActiveAgentId(agent.id);
      else if (store) store.activeAgentId = agent.id;
      this.navigate('chat');
      this.closeAgentChatsSidebar();
      this.scheduleSidebarScrollIndicators();
      if (agent.revive_recommended === true) {
        var reviveId = String(agent.id || '').trim();
        if (reviveId) {
          InfringAPI.post('/api/agents/' + encodeURIComponent(reviveId) + '/revive', {
            reason: 'sidebar_contract_revival'
          }).then(function() {
            if (store && typeof store.refreshAgents === 'function') {
              store.refreshAgents({ force: true }).catch(function() {});
            }
          }).catch(function() {});
        }
      }
    },
    formatChatSidebarTime(ts) {
      if (!ts) return '';
      var d = new Date(ts);
      if (Number.isNaN(d.getTime())) return '';
      var now = new Date();
      var sameDay = d.getFullYear() === now.getFullYear() && d.getMonth() === now.getMonth() && d.getDate() === now.getDate();
      if (sameDay) return d.toLocaleTimeString([], { hour: 'numeric', minute: '2-digit' });
      var y = new Date(now.getFullYear(), now.getMonth(), now.getDate() - 1);
      var isYesterday = d.getFullYear() === y.getFullYear() && d.getMonth() === y.getMonth() && d.getDate() === y.getDate();
      if (isYesterday) return 'Yesterday';
      return d.toLocaleDateString([], { month: 'short', day: 'numeric' });
    },
    agentAutoTerminateEnabled(agent) {
      if (!agent || typeof agent !== 'object') return false;
      if (typeof agent.auto_terminate_allowed === 'boolean') {
        return agent.auto_terminate_allowed;
      }
      // Server contract should provide explicit policy; default fail-closed.
      return false;
    },
    agentContractRemainingMs(agent) {
      // Force recompute every second for live countdown updates.
      var _tick = Number(this.clockTick || 0);
      void _tick;
      if (!this.agentAutoTerminateEnabled(agent)) return null;
      var store = this.getAppStore();
      var lastRefreshAt = Number((store && store._lastAgentsRefreshAt) || 0);
      var ageDriftMs =
        Number.isFinite(lastRefreshAt) && lastRefreshAt > 0
          ? Math.max(0, Date.now() - lastRefreshAt)
          : 0;
      if (!agent || typeof agent !== 'object') return null;
      var directRemaining = Number(agent.contract_remaining_ms);
      if (Number.isFinite(directRemaining) && directRemaining >= 0) {
        return Math.max(0, Math.floor(directRemaining - ageDriftMs));
      }
      return null;
    },
    agentContractHasFiniteExpiry(agent) {
      if (!agent || typeof agent !== 'object') return false;
      if (agent.revive_recommended === true) return true;
      if (typeof agent.contract_finite_expiry === 'boolean') {
        return agent.contract_finite_expiry;
      }
      var directRemaining = Number(agent.contract_remaining_ms);
      if (Number.isFinite(directRemaining) && directRemaining >= 0) return true;
      var totalMs = Number(agent.contract_total_ms);
      return Number.isFinite(totalMs) && totalMs > 0;
    },
    agentContractTerminationGraceMs() {
      return 10000;
    },
    isAgentPendingTermination(agent) {
      if (!this.agentAutoTerminateEnabled(agent)) return false;
      if (!this.agentContractHasFiniteExpiry(agent)) return false;
      var remainingMs = this.agentContractRemainingMs(agent);
      if (remainingMs == null || remainingMs > 0) return false;
      var store = this.getAppStore();
      var lastRefreshAt = Number((store && store._lastAgentsRefreshAt) || 0);
      if (!Number.isFinite(lastRefreshAt) || lastRefreshAt <= 0) return true;
      var refreshAgeMs = Math.max(0, Date.now() - lastRefreshAt);
      return refreshAgeMs < this.agentContractTerminationGraceMs();
    },
    shouldShowInfinityLifespan(agent) {
      if (!agent || typeof agent !== 'object') return false;
      if (agent.revive_recommended === true) return false;
      if (typeof agent.contract_finite_expiry === 'boolean') {
        if (agent.contract_finite_expiry) return false;
        return !this.agentAutoTerminateEnabled(agent);
      }
      if (!this.agentAutoTerminateEnabled(agent)) return true;
      // Unknown contract timing should not be rendered as explicit infinity.
      return false;
    },
    shouldShowExpiryCountdown(agent) {
      if (agent && agent.revive_recommended === true) return true;
      if (!this.agentAutoTerminateEnabled(agent)) return false;
      if (!this.agentContractHasFiniteExpiry(agent)) return false;
      var remainingMs = this.agentContractRemainingMs(agent);
      if (remainingMs == null) return false;
      if (remainingMs <= 0) return this.isAgentPendingTermination(agent);
      return true;
    },
    expiryCountdownLabel(agent) {
      if (agent && agent.revive_recommended === true) return 'timed out';
      var remainingMs = this.agentContractRemainingMs(agent);
      if (remainingMs == null) return '';

      if (remainingMs <= 0) return this.isAgentPendingTermination(agent) ? '0m' : '';
      var totalMin = Math.max(1, Math.ceil(remainingMs / 60000));
      var monthMin = 30 * 24 * 60;
      if (totalMin >= monthMin) {
        return Math.max(1, Math.ceil(totalMin / monthMin)) + 'm';
      }
      if (totalMin >= 1440) {
        return Math.max(1, Math.ceil(totalMin / 1440)) + 'd';
      }
      if (totalMin >= 60) {
        return Math.max(1, Math.ceil(totalMin / 60)) + 'h';
      }
      return totalMin + 'm';
    },

    expiryCountdownCritical(agent) {
      if (agent && agent.revive_recommended === true) return false;
      if (this.isAgentPendingTermination(agent)) return true;
      var remainingMs = this.agentContractRemainingMs(agent);
      if (remainingMs == null) return false;
      var totalMs = this.agentContractTotalMs(agent);
      if (!Number.isFinite(totalMs) || totalMs <= 0) return false;
      var thresholdMs = Math.min(3600000, Math.max(1, Math.floor(totalMs * 0.2)));
      return remainingMs > 0 && remainingMs <= thresholdMs;
    },

    agentContractTotalMs(agent) {
      if (!agent || typeof agent !== 'object') return null;
      var durationMs = Number(agent.contract_total_ms);
      if (Number.isFinite(durationMs) && durationMs > 0) return Math.floor(durationMs);
      return null;
    },

    agentHeartStates(agent) {
      var totalHearts = 5;
      var hearts = [true, true, true, true, true];
      if (!agent || typeof agent !== 'object') return hearts;
      if (agent.is_system_thread) return hearts;
      if (agent.revive_recommended === true) return [false, false, false, false, false];
      if (!this.agentAutoTerminateEnabled(agent) || !this.agentContractHasFiniteExpiry(agent)) return [true];
      var remainingMs = this.agentContractRemainingMs(agent);
      if (remainingMs == null) return [true];
      if (remainingMs <= 0 && this.isAgentPendingTermination(agent)) return [false, false, false, false, false];
      var totalMs = this.agentContractTotalMs(agent);
      if (!Number.isFinite(totalMs) || totalMs <= 0) return [true];
      var ratio = Math.max(0, Math.min(1, remainingMs / totalMs));
      var filled = Math.ceil(ratio * totalHearts);
      if (remainingMs <= 0 && this.isAgentPendingTermination(agent)) filled = 0;
      if (filled < 0) filled = 0;
      if (filled > totalHearts) filled = totalHearts;
      for (var i = 0; i < totalHearts; i++) {
        hearts[i] = i < filled;
      }
      return hearts;
    },

    agentHeartShowsInfinity(agent) {
      if (!agent || typeof agent !== 'object') return false;
      if (agent.is_system_thread) return false;
      if (agent.revive_recommended === true) return false;
      return !this.agentAutoTerminateEnabled(agent) || !this.agentContractHasFiniteExpiry(agent);
    },

    agentHeartMeterLabel(agent) {
      if (!agent || typeof agent !== 'object' || agent.is_system_thread) return '';
      if (agent.revive_recommended === true) return 'Time limit: timed out';
      if (!this.agentAutoTerminateEnabled(agent) || !this.agentContractHasFiniteExpiry(agent)) {
        return 'Time limit: unlimited';
      }
      var label = this.expiryCountdownLabel(agent);
      if (label) return 'Time remaining: ' + label;
      return 'Time limit active';
    },

    showDashboardPopup(id, label, ev, overrides) {
      var popupId = String(id || '').trim();
      var title = String(label || '').trim();
      if (!popupId || !title) {
        if (typeof this.hideDashboardPopup === 'function') this.hideDashboardPopup();
        return;
      }
      var eventType = String((ev && ev.type) || '').toLowerCase();
      if (
        eventType === 'mouseleave' ||
        eventType === 'pointerleave' ||
        eventType === 'blur' ||
        eventType === 'focusout'
      ) {
        if (typeof this.hideDashboardPopup === 'function') this.hideDashboardPopup(popupId);
        return;
      }
      if (ev && ev.isTrusted === false) return;
      var config = overrides && typeof overrides === 'object' ? overrides : {};
      var anchor = typeof this.dashboardPopupAnchorPoint === 'function'
        ? this.dashboardPopupAnchorPoint(ev, config.side)
        : { left: 0, top: 0, side: String(config.side || 'bottom'), inline_away: 'right', block_away: 'bottom' };
      var service = typeof this.dashboardPopupService === 'function' ? this.dashboardPopupService() : null;
      this.dashboardPopup = service && typeof service.openState === 'function'
        ? service.openState(popupId, title, config, anchor)
        : {
          id: popupId,
          active: true,
          source: String(config.source || '').trim(),
          title: title,
          body: String(config.body || '').trim(),
          meta_origin: String(config.meta_origin || 'Taskbar').trim(),
          meta_time: String(config.meta_time || '').trim(),
          unread: !!config.unread,
          left: anchor.left,
          top: anchor.top,
          side: anchor.side,
          inline_away: anchor.inline_away === 'left' ? 'left' : 'right',
          block_away: anchor.block_away === 'top' ? 'top' : 'bottom',
          compact: false
        };
    },

    hideDashboardPopup(rawId) {
      var service = typeof this.dashboardPopupService === 'function' ? this.dashboardPopupService() : null;
      if (service && typeof service.closeState === 'function') {
        this.dashboardPopup = service.closeState(this.dashboardPopup, rawId);
        return;
      }
      var popupId = String(rawId || '').trim();
      var currentId = String((this.dashboardPopup && this.dashboardPopup.id) || '').trim();
      if (popupId && currentId && popupId !== currentId) return;
      if (typeof this.clearDashboardPopupState === 'function') {
        this.clearDashboardPopupState();
        return;
      }
      this.dashboardPopup = {
        id: '',
        active: false,
        source: '',
        title: '',
        body: '',
        meta_origin: '',
        meta_time: '',
        unread: false,
        left: 0,
        top: 0,
        side: 'bottom',
        inline_away: 'right',
        block_away: 'bottom',
        compact: false
      };
    },

    hideDashboardPopupBySource(source) {
      var popupSource = String(source || '').trim();
      if (!popupSource) return;
      var popup = this.dashboardPopup || {};
      if (String(popup.source || '').trim() !== popupSource) return;
      this.hideDashboardPopup(String(popup.id || '').trim());
    },

    closeTaskbarHeroMenu() {
      this.taskbarHeroMenuOpen = false;
    },

    closeTaskbarTextMenu() {
      this.taskbarTextMenuOpen = '';
    },

    taskbarTextMenuIsOpen(menuName) {
      var key = String(menuName || '').trim().toLowerCase();
      if (!key) return false;
      return String(this.taskbarTextMenuOpen || '').trim().toLowerCase() === key;
    },

    toggleTaskbarTextMenu(menuName) {
      var key = String(menuName || '').trim().toLowerCase();
      if (!key) {
        this.closeTaskbarTextMenu();
        return;
      }
      this.closeTaskbarHeroMenu();
      this.taskbarTextMenuOpen = this.taskbarTextMenuIsOpen(key) ? '' : key;
    },

    handleTaskbarHelpManual() {
      this.closeTaskbarTextMenu();
      this.openPopupWindow('manual');
    },
    handleTaskbarHelpReportIssue() {
      this.closeTaskbarTextMenu();
      this.openPopupWindow('report');
    },
    async submitReportIssueDraft() {
      var draft = String(this.reportIssueDraft || '').trim();
      if (!draft) {
        InfringToast.error('Please add issue details before submitting.');
        return;
      }
      var entry = {
        id: 'issue-' + String(Date.now()),
        ts: Date.now(),
        text: draft,
        page: String(this.page || '').trim(),
        agent_id: String((this.currentAgent && this.currentAgent.id) || '').trim()
      };
      try {
        var raw = localStorage.getItem('infring-issue-report-drafts');
        var list = raw ? JSON.parse(raw) : [];
        if (!Array.isArray(list)) list = [];
        list.unshift(entry);
        localStorage.setItem('infring-issue-report-drafts', JSON.stringify(list.slice(0, 25)));
      } catch(_) {}
      var title = ((draft.split(/\r?\n/).find(function(line) { return String(line || '').trim(); }) || draft).replace(/\s+/g, ' ').trim().slice(0, 120) || 'Dashboard issue report');
      var issueBody = '## User Report\n\n' + draft + '\n\n## Runtime Context\n- page: ' + (entry.page || 'unknown') + '\n- agent_id: ' + (entry.agent_id || 'none') + '\n- reported_at: ' + new Date(entry.ts || Date.now()).toISOString() + '\n- client_version: ' + String(this.version || 'unknown');
      try {
        var result = await InfringAPI.post('/api/dashboard/action', {
          action: 'dashboard.github.issue.create',
          payload: { title: title, body: issueBody, source: 'dashboard_report_popup' }
        });
        var actionResult = result && typeof result === 'object' ? (result.lane || result.payload || result) : {};
        if ((result && result.ok === false) || (actionResult && actionResult.ok === false)) {
          throw new Error(String((actionResult && (actionResult.error || actionResult.message)) || (result && (result.error || result.message)) || 'issue_submit_failed'));
        }
        var issueUrl = String((actionResult && (actionResult.html_url || actionResult.issue_url)) || '').trim();
        this.reportIssueDraft = ''; this.closePopupWindow('report');
        InfringToast.success(issueUrl ? ('Issue submitted: ' + issueUrl) : 'Issue submitted.');
      } catch (e) {
        InfringToast.error('Issue submit failed (saved locally): ' + String(e && e.message ? e.message : 'unknown error'));
      }
    },
    manualDocumentMarkdown() {
      // Canonical source: docs/workspace/manuals/infring_manual_help_tab.md
      var encoded = 'IyBJbmZyaW5nIE1hbnVhbAoKX09wZXJhdG9yLWZhY2luZyBndWlkZSBmb3IgdGhlIEhlbHAgdGFiXwoKIyMgVGFibGUgb2YgQ29udGVudHMKLSBbV2hhdCBJbmZyaW5nIElzXSgjd2hhdC1pbmZyaW5nLWlzKQotIFtJbnN0YWxsICsgU3RhcnRdKCNpbnN0YWxsLS1zdGFydCkKLSBbQ0xJIEd1aWRlXSgjY2xpLWd1aWRlKQotIFtVSSBHdWlkZV0oI3VpLWd1aWRlKQotIFtUb29scyArIEV2aWRlbmNlXSgjdG9vbHMtLWV2aWRlbmNlKQotIFtNZW1vcnkgKyBTZXNzaW9uc10oI21lbW9yeS0tc2Vzc2lvbnMpCi0gW1NhZmV0eSBNb2RlbF0oI3NhZmV0eS1tb2RlbCkKLSBbVHJvdWJsZXNob290aW5nXSgjdHJvdWJsZXNob290aW5nKQotIFtSZXBvcnRpbmcgSXNzdWVzXSgjcmVwb3J0aW5nLWlzc3VlcykKCi0tLQoKIyMgV2hhdCBJbmZyaW5nIElzCgpJbmZyaW5nIGlzIGEgbG9jYWwsIGRldGVybWluaXN0aWMsIHJlY2VpcHQtZmlyc3QgYXV0b21hdGlvbiBhbmQgb3JjaGVzdHJhdGlvbiBydW50aW1lLgoKSW4gcHJhY3RpY2FsIHRlcm1zLCB0aGF0IG1lYW5zOgotICoqQ29yZSB0cnV0aCBsaXZlcyBpbiB0aGUgUnVzdCBjb3JlLioqIENyaXRpY2FsIHBvbGljeSwgcmVjZWlwdHMsIGV4ZWN1dGlvbiwgYW5kIHNhZmV0eSBkZWNpc2lvbnMgYXJlIGF1dGhvcml0YXRpdmUgaW4gY29yZSBsYW5lcy4KLSAqKlRoZSBvcmNoZXN0cmF0aW9uIGxheWVyIGNvb3JkaW5hdGVzIHdvcmsuKiogSXQgc2hhcGVzIHJlcXVlc3RzLCBwbGFucyB3b3JrLCBoYW5kbGVzIGNsYXJpZmljYXRpb24sIGFuZCBwYWNrYWdlcyByZXN1bHRzLgotICoqVGhlIGNsaWVudC9kYXNoYm9hcmQgaXMgYSBwcmVzZW50YXRpb24gc3VyZmFjZS4qKiBJdCBpcyB0aGVyZSB0byBoZWxwIHlvdSBvcGVyYXRlIHRoZSBzeXN0ZW0sIG5vdCB0byBiZSB0aGUgc291cmNlIG9mIHRydXRoLgotICoqT3BlcmF0aW9ucyBhcmUgZXZpZGVuY2UtYmFja2VkLioqIEltcG9ydGFudCBhY3Rpb25zIGFuZCBvdXRjb21lcyBhcmUgZGVzaWduZWQgdG8gYmUgdHJhY2VhYmxlLgotICoqRmFpbHVyZSBpcyBkZXNpZ25lZCB0byBiZSBmYWlsLWNsb3NlZC4qKiBJZiBJbmZyaW5nIGlzIHVuc3VyZSBvciBhIHJlcXVpcmVkIGxhbmUgaXMgdW5hdmFpbGFibGUsIHRoZSBjb3JyZWN0IHJlc3VsdCBpcyBvZnRlbiB0byBzdG9wLCBkZWdyYWRlIHNhZmVseSwgb3IgYXNrIGZvciBjbGFyaWZpY2F0aW9uIGluc3RlYWQgb2YgZ3Vlc3NpbmcuCgojIyMgUnVudGltZSBQcm9maWxlcwoKSW5mcmluZyBzdXBwb3J0cyBtdWx0aXBsZSBydW50aW1lIHByb2ZpbGVzOgotICoqcmljaCoqIOKAlCBmdWxsIG9wZXJhdG9yIGV4cGVyaWVuY2UsIGluY2x1ZGluZyB0aGUgZ2F0ZXdheS9kYXNoYm9hcmQgc3VyZmFjZS4KLSAqKnB1cmUqKiDigJQgUnVzdC1vbmx5IHByb2ZpbGUgd2l0aCBubyByaWNoIGdhdGV3YXkgVUkgc3VyZmFjZS4KLSAqKnRpbnktbWF4Kiog4oCUIHNtYWxsZXN0IHB1cmUgcHJvZmlsZSBmb3IgY29uc3RyYWluZWQgZW52aXJvbm1lbnRzLgoKIyMjIEV4cGVyaW1lbnRhbCBTdXJmYWNlcwoKU29tZSBsYW5lcyBhcmUgZXhwbGljaXRseSBleHBlcmltZW50YWwuIEluIHBhcnRpY3VsYXIsIHRoZSBgYXNzaW1pbGF0ZWAgcnVudGltZSBzdXJmYWNlIGlzIGd1YXJkZWQgYW5kIG5vdCBwYXJ0IG9mIHRoZSBub3JtYWwgcHVibGljIHByb2R1Y3Rpb24gc3VyZmFjZS4KCiMjIyBXaGVuIHRvIHVzZSBJbmZyaW5nCgpVc2UgSW5mcmluZyB3aGVuIHlvdSB3YW50OgotIGEgbG9jYWwgb3BlcmF0b3IgcnVudGltZQotIGRldGVybWluaXN0aWMsIHBvbGljeS1nb3Zlcm5lZCBleGVjdXRpb24KLSBhIGRhc2hib2FyZCBmb3IgaW50ZXJhY3RpdmUgb3BlcmF0aW9uCi0gYSBDTEkgZm9yIHNjcmlwdGluZywgdmVyaWZpY2F0aW9uLCBhbmQgY29udHJvbGxlZCB3b3JrZmxvd3MKCi0tLQoKIyMgSW5zdGFsbCArIFN0YXJ0CgojIyMgUXVpY2sgaW5zdGFsbAoKIyMjIG1hY09TIC8gTGludXgKYGBgYmFzaApjdXJsIC1mc1NMIGh0dHBzOi8vcmF3LmdpdGh1YnVzZXJjb250ZW50LmNvbS9wcm90aGV1c2xhYnMvSW5mUmluZy9tYWluL2luc3RhbGwuc2ggfCBzaCAtcyAtLSAtLWZ1bGwgaW5mcmluZyBnYXRld2F5CmBgYAoKIyMjIFdpbmRvd3MgKFBvd2VyU2hlbGwpCmBgYHBvd2Vyc2hlbGwKU2V0LUV4ZWN1dGlvblBvbGljeSAtU2NvcGUgUHJvY2VzcyAtRXhlY3V0aW9uUG9saWN5IEJ5cGFzcyAtRm9yY2UKJHRtcCA9IEpvaW4tUGF0aCAkZW52OlRFTVAgImluZnJpbmctaW5zdGFsbC5wczEiCmlybSBodHRwczovL3Jhdy5naXRodWJ1c2VyY29udGVudC5jb20vcHJvdGhldXNsYWJzL0luZlJpbmcvbWFpbi9pbnN0YWxsLnBzMSAtT3V0RmlsZSAkdG1wCiYgJHRtcCAtUmVwYWlyIC1GdWxsClJlbW92ZS1JdGVtICR0bXAgLUZvcmNlCkdldC1Db21tYW5kIGluZnJpbmcgLUVycm9yQWN0aW9uIFNpbGVudGx5Q29udGludWUKaW5mcmluZyBnYXRld2F5CmBgYAoKIyMjIFZlcmlmeSB0aGUgQ0xJCmBgYGJhc2gKaW5mcmluZyAtLWhlbHAKaW5mcmluZyBsaXN0CmluZnJpbmcgZ2F0ZXdheSBzdGF0dXMKYGBgCgpJZiB5b3VyIHNoZWxsIGhhcyBub3QgcmVmcmVzaGVkIGBQQVRIYCB5ZXQ6CmBgYGJhc2gKLiAiJEhPTUUvLmluZnJpbmcvZW52LnNoIgpoYXNoIC1yIDI+L2Rldi9udWxsIHx8IHRydWUKaW5mcmluZyAtLWhlbHAKYGBgCgpEaXJlY3QtcGF0aCBmYWxsYmFjazoKYGBgYmFzaAoiJEhPTUUvLmluZnJpbmcvYmluL2luZnJpbmciIC0taGVscApgYGAKClBvd2VyU2hlbGwgZmFsbGJhY2s6CmBgYHBvd2Vyc2hlbGwKJGVudjpQYXRoID0gIiRIT01FLy5pbmZyaW5nL2JpbjskZW52OlBhdGgiCmluZnJpbmcgLS1oZWxwCmBgYAoKIyMjIFN0YXJ0IHRoZSBvcGVyYXRvciBzdXJmYWNlCmBgYGJhc2gKaW5mcmluZyBnYXRld2F5CmBgYAoKVGhpcyBzdGFydHMgdGhlIHJ1bnRpbWUgYW5kIGRhc2hib2FyZC4KClByaW1hcnkgZGFzaGJvYXJkIFVSTDoKYGBgdGV4dApodHRwOi8vMTI3LjAuMC4xOjQxNzMvZGFzaGJvYXJkI2NoYXQKYGBgCgpIZWFsdGggZW5kcG9pbnQ6CmBgYHRleHQKaHR0cDovLzEyNy4wLjAuMTo0MTczL2hlYWx0aHoKYGBgCgojIyMgQ29tbW9uIGxpZmVjeWNsZSBjb21tYW5kcwpgYGBiYXNoCmluZnJpbmcgZ2F0ZXdheSBzdGF0dXMKaW5mcmluZyBnYXRld2F5IHN0b3AKaW5mcmluZyBnYXRld2F5IHJlc3RhcnQKYGBgCgojIyMgSW5zdGFsbCBtb2RlcwotIGAtLW1pbmltYWxgIOKAlCBDTEkgKyBkYWVtb24gd3JhcHBlcnMKLSBgLS1mdWxsYCDigJQgZnVsbCBydW50aW1lIGJvb3RzdHJhcAotIGAtLXB1cmVgIOKAlCBSdXN0LW9ubHkgcnVudGltZSBzdXJmYWNlCi0gYC0tdGlueS1tYXhgIOKAlCBzbWFsbGVzdCBwdXJlIHByb2ZpbGUKLSBgLS1yZXBhaXJgIOKAlCBjbGVhbiByZWluc3RhbGwgLyBzdGFsZS1hcnRpZmFjdCBjbGVhbnVwCgpFeGFtcGxlczoKYGBgYmFzaAojIHB1cmUgcHJvZmlsZQpjdXJsIC1mc1NMIGh0dHBzOi8vcmF3LmdpdGh1YnVzZXJjb250ZW50LmNvbS9wcm90aGV1c2xhYnMvSW5mUmluZy9tYWluL2luc3RhbGwuc2ggfCBzaCAtcyAtLSAtLXB1cmUKCiMgdGlueS1tYXggcHJvZmlsZQpjdXJsIC1mc1NMIGh0dHBzOi8vcmF3LmdpdGh1YnVzZXJjb250ZW50LmNvbS9wcm90aGV1c2xhYnMvSW5mUmluZy9tYWluL2luc3RhbGwuc2ggfCBzaCAtcyAtLSAtLXRpbnktbWF4CgojIHJlcGFpciArIGZ1bGwKY3VybCAtZnNTTCBodHRwczovL3Jhdy5naXRodWJ1c2VyY29udGVudC5jb20vcHJvdGhldXNsYWJzL0luZlJpbmcvbWFpbi9pbnN0YWxsLnNoIHwgc2ggLXMgLS0gLS1yZXBhaXIgLS1mdWxsCgojIGluLXBsYWNlIHVwZGF0ZQppbmZyaW5nIHVwZGF0ZSAtLXJlcGFpciAtLWZ1bGwKYGBgCgotLS0KCiMjIENMSSBHdWlkZQoKIyMjIFByaW1hcnkgZW50cnlwb2ludHMKLSBgaW5mcmluZ2Ag4oCUIG1haW4gb3BlcmF0b3IgZW50cnlwb2ludAotIGBpbmZyaW5nY3RsYCDigJQgd3JhcHBlci9jb250cm9sIHN1cmZhY2UKLSBgaW5mcmluZ2RgIOKAlCBkYWVtb24tb3JpZW50ZWQgd3JhcHBlcgoKIyMjIEV2ZXJ5ZGF5IGNvbW1hbmRzCmBgYGJhc2gKaW5mcmluZyBoZWxwCmluZnJpbmcgbGlzdAppbmZyaW5nIHZlcnNpb24KaW5mcmluZyBnYXRld2F5CmluZnJpbmcgZ2F0ZXdheSBzdGF0dXMKaW5mcmluZyBnYXRld2F5IHN0b3AKaW5mcmluZyBnYXRld2F5IHJlc3RhcnQKYGBgCgojIyMgT3BlcmF0aW9uYWwgZmFsbGJhY2sgc3VyZmFjZQpXaGVuIE5vZGUuanMgaXMgdW5hdmFpbGFibGUsIEluZnJpbmcgc3RpbGwgZXhwb3NlcyBhIHJlZHVjZWQgUnVzdC1iYWNrZWQgc3VyZmFjZS4KCkF2YWlsYWJsZSBmYWxsYmFjayBmYW1pbGllcyBpbmNsdWRlOgotIGBnYXRld2F5IFtzdGFydHxzdG9wfHJlc3RhcnR8c3RhdHVzXWAKLSBgdXBkYXRlYAotIGB2ZXJpZnktZ2F0ZXdheWAKLSBgc3RhcnRgLCBgc3RvcGAsIGByZXN0YXJ0YAotIGBkYXNoYm9hcmRgLCBgc3RhdHVzYAotIGBzZXNzaW9uYAotIGByYWdgCi0gYG1lbW9yeWAKLSBgYWRhcHRpdmVgCi0gYGVudGVycHJpc2UtaGFyZGVuaW5nYAotIGBiZW5jaG1hcmtgCi0gYGFscGhhLWNoZWNrYAotIGByZXNlYXJjaGAKLSBgaGVscGAsIGBsaXN0YCwgYHZlcnNpb25gCgpOb3QgYXZhaWxhYmxlIGluIE5vZGUtZnJlZSBmYWxsYmFjazoKLSBgYXNzaW1pbGF0ZWAKCiMjIyBGdWxsIC8gZXhwZXJpbWVudGFsIHN1cmZhY2UKYGFzc2ltaWxhdGVgIHJlcXVpcmVzIHRoZSBmdWxsIE5vZGUuanMtYXNzaXN0ZWQgc3VyZmFjZSBhbmQgc2hvdWxkIGJlIHRyZWF0ZWQgYXMgZXhwZXJpbWVudGFsLgoKRXhhbXBsZToKYGBgYmFzaAppbmZyaW5nIGFzc2ltaWxhdGUgdGFyZ2V0LW5hbWUgLS1wbGFuLW9ubHk9MSAtLWpzb249MQpgYGAKClVzZWZ1bCBmbGFnczoKLSBgLS1wbGFuLW9ubHk9MWAg4oCUIGVtaXQgdGhlIHBsYW5uaW5nIGNoYWluIHdpdGhvdXQgZXhlY3V0aW5nIG11dGF0aW9ucwotIGAtLWpzb249MWAg4oCUIHN0cnVjdHVyZWQgb3V0cHV0Ci0gYC0tc3RyaWN0PTFgIOKAlCB0aWdodGVyIGVuZm9yY2VtZW50Ci0gYC0tYWxsb3ctbG9jYWwtc2ltdWxhdGlvbj0xYCDigJQgdGVzdC1vbmx5IGxvY2FsIHNpbXVsYXRpb24gcGF0aAoKIyMjIENvbnRyaWJ1dG9yIC8gcmVwb3NpdG9yeSB3b3JrZmxvd3MKSWYgeW91IGFyZSB3b3JraW5nIGZyb20gdGhlIHJlcG9zaXRvcnkgZGlyZWN0bHksIHRoZXNlIGFyZSB0aGUgY2Fub25pY2FsIHdvcmtzcGFjZSBlbnRyeXBvaW50czoKYGBgYmFzaApucG0gcnVuIC1zIHdvcmtzcGFjZTpjb21tYW5kcwpucG0gcnVuIC1zIHRvb2xpbmc6bGlzdApucG0gcnVuIC1zIHdvcmtzcGFjZTpkZXYKbnBtIHJ1biAtcyB3b3Jrc3BhY2U6dmVyaWZ5Cm5wbSBydW4gLXMgbGFuZTpsaXN0IC0tIC0tanNvbj0xCmBgYAoKLS0tCgojIyBVSSBHdWlkZQoKIyMjIFdoYXQgdGhlIGRhc2hib2FyZCBpcyBmb3IKVGhlIGRhc2hib2FyZCBpcyB0aGUgcHJpbWFyeSBpbnRlcmFjdGl2ZSBvcGVyYXRvciBzdXJmYWNlIGluIHRoZSAqKnJpY2gqKiBwcm9maWxlLiBJdCBpcyB0aGUgcmlnaHQgcGxhY2UgdG86Ci0gd29yayBpbnRlcmFjdGl2ZWx5Ci0gaW5zcGVjdCBzdGF0dXMgYW5kIG91dHB1dHMKLSB1c2UgdGhlIGNoYXQvb3BlcmF0b3Igc3VyZmFjZQotIHJlYWQgYnVpbHQtaW4gaGVscAotIHZhbGlkYXRlIHRoYXQgdGhlIHJ1bnRpbWUgaXMgdXAgYmVmb3JlIHlvdSBtb3ZlIGludG8gZGVlcGVyIENMSS9vcHMgd29yawoKIyMjIFdoYXQgdGhlIGRhc2hib2FyZCBpcyBub3QKVGhlIGRhc2hib2FyZCBpcyAqKm5vdCoqIHRoZSBzeXN0ZW3igJlzIHNvdXJjZSBvZiB0cnV0aC4gSWYgdGhlIFVJIGFuZCB0aGUgcnVudGltZSBkaXNhZ3JlZSwgdHJ1c3QgdGhlIHJ1bnRpbWXigJlzIHJlY2VpcHRzLCBzdGF0dXMgY29tbWFuZHMsIGFuZCBzdXBwb3J0IGFydGlmYWN0cy4KCiMjIyBSZWNvbW1lbmRlZCBvcGVyYXRvciB3b3JrZmxvdwoxLiBTdGFydCB0aGUgc3lzdGVtIHdpdGggYGluZnJpbmcgZ2F0ZXdheWAuCjIuIE9wZW4gdGhlIGRhc2hib2FyZC4KMy4gVXNlIHRoZSBjaGF0L29wZXJhdG9yIHN1cmZhY2UgZm9yIGludGVyYWN0aXZlIHdvcmsuCjQuIFVzZSBDTEkgc3RhdHVzIGNvbW1hbmRzIGZvciB2ZXJpZmljYXRpb24gd2hlbiBuZWVkZWQuCjUuIFVzZSBzdXBwb3J0L2V4cG9ydCB0b29saW5nIHdoZW4gZGlhZ25vc2luZyBpbmNpZGVudHMgb3IgZmlsaW5nIGlzc3Vlcy4KCiMjIyBSaWNoIHZzIHB1cmUgcHJvZmlsZXMKLSAqKnJpY2gqKjogZGFzaGJvYXJkIGF2YWlsYWJsZQotICoqcHVyZSAvIHRpbnktbWF4Kio6IGludGVudGlvbmFsbHkgbm8gcmljaCBnYXRld2F5IFVJIHN1cmZhY2UKCklmIHlvdSBhcmUgb24gYC0tcHVyZWAgb3IgYC0tdGlueS1tYXhgLCB1c2UgdGhlIENMSSBpbnN0ZWFkIG9mIGV4cGVjdGluZyB0aGUgZGFzaGJvYXJkLgoKIyMjIEFjY2Vzc2liaWxpdHkgZXhwZWN0YXRpb25zClRoZSBVSSBjb250cmFjdCBleHBlY3RzOgotIGtleWJvYXJkIG5hdmlnYXRpb24gZm9yIHByaW1hcnkgYWN0aW9ucwotIHZpc2libGUgZm9jdXMgaW5kaWNhdG9ycwotIHN1ZmZpY2llbnQgY29udHJhc3QgZm9yIGNyaXRpY2FsIHRleHQKLSBkb2N1bWVudGVkIGRpc2NvdmVyYWJpbGl0eSBmb3IgdGhlIGNvbW1hbmQgcGFsZXR0ZSAvIHByaW1hcnkgYWN0aW9ucwoKLS0tCgojIyBUb29scyArIEV2aWRlbmNlCgojIyMgV2hhdCB0b29scyBtZWFuIGluIEluZnJpbmcKQSB0b29sIGlzIGFuIG9wZXJhdG9yLXVzYWJsZSBsYW5lIHRoYXQgcGVyZm9ybXMgYSBnb3Zlcm5lZCBhY3Rpb24gdGhyb3VnaCB0aGUgcnVudGltZS4gSW5mcmluZyBpcyBkZXNpZ25lZCBzbyBpbXBvcnRhbnQgYWN0aW9ucyBhcmUgcG9saWN5LWdvdmVybmVkIGFuZCBldmlkZW5jZS1iYWNrZWQgaW5zdGVhZCBvZiBiZWluZyBvcGFxdWUgc2lkZSBlZmZlY3RzLgoKIyMjIFdoYXQgZXZpZGVuY2UgbWVhbnMKRXZpZGVuY2UgaXMgdGhlIHN1cHBvcnRpbmcgcmVjb3JkIGZvciBhIGNsYWltLCByZXN1bHQsIG9yIGFjdGlvbi4gSW5mcmluZ+KAmXMgZG9jdW1lbnRhdGlvbiBwb2xpY3kgaXMgZXhwbGljaXQ6IG1lYXN1cmFibGUsIGNvbXBhcmF0aXZlLCBzZWN1cml0eS1zZW5zaXRpdmUsIG9yIGN1c3RvbWVyLWltcGFjdGluZyBjbGFpbXMgbXVzdCBoYXZlIGxpbmtlZCBldmlkZW5jZS4KCkV4YW1wbGVzIG9mIGV2aWRlbmNlIGluY2x1ZGU6Ci0gcmVjZWlwdHMKLSBiZW5jaG1hcmsgYXJ0aWZhY3RzCi0gdmVyaWZpY2F0aW9uIG91dHB1dHMKLSBkcmlsbCAvIHJlY292ZXJ5IGFydGlmYWN0cwotIHN1cHBvcnQgYnVuZGxlcwotIGxvZ3MgYW5kIHN0YXRlIGFydGlmYWN0cyB3aGVuIHNoYXJlYWJsZSBhbmQgYXBwcm9wcmlhdGUKCiMjIyBIb3cgdG8gaW50ZXJwcmV0IG91dHB1dHMKV2hlbiByZWFkaW5nIGEgcmVzdWx0LCBhc2s6Ci0gV2hhdCBoYXBwZW5lZD8KLSBXaGF0IGV2aWRlbmNlIHN1cHBvcnRzIGl0PwotIFdhcyB0aGUgYWN0aW9uIHN1Y2Nlc3NmdWwsIGRlZ3JhZGVkLCBibG9ja2VkLCBvciBmYWlsLWNsb3NlZD8KLSBJcyB0aGVyZSBhIHJlY2VpcHQsIGFydGlmYWN0LCBvciBzdGF0dXMgcmVjb3JkIEkgY2FuIGluc3BlY3Q/CgojIyMgUHJhY3RpY2FsIHJ1bGUKSWYgeW91IHdhbnQgdG8gbWFrZSBhIHB1YmxpYyBjbGFpbSBhYm91dCBwZXJmb3JtYW5jZSwgcmVsaWFiaWxpdHksIG9yIHNlY3VyaXR5LCBkbyBub3QgcmVseSBvbiBVSSB0ZXh0IGFsb25lLiBMaW5rIHRoZSBzdXBwb3J0aW5nIGFydGlmYWN0LgoKIyMjIFVzZWZ1bCBldmlkZW5jZS9vcHMgY29tbWFuZHMKYGBgYmFzaApucG0gcnVuIC1zIG9wczpwcm9kdWN0aW9uLXRvcG9sb2d5OnN0YXR1cwpucG0gcnVuIC1zIG9wczp0cmFuc3BvcnQ6c3Bhd24tYXVkaXQKbnBtIHJ1biAtcyBvcHM6c3VwcG9ydC1idW5kbGU6ZXhwb3J0Cm5wbSBydW4gLXMgb3BzOnJlbGVhc2U6dmVyZGljdApgYGAKCi0tLQoKIyMgTWVtb3J5ICsgU2Vzc2lvbnMKCiMjIyBTZXNzaW9ucwpVc2Ugc2Vzc2lvbnMgZm9yIGFjdGl2ZSBvcGVyYXRvciB3b3JrIGFuZCBsaXZlIHJ1bnRpbWUgY29udGV4dC4KCiMjIyBNZW1vcnkKVXNlIG1lbW9yeSBzdXJmYWNlcyBmb3IgcGVyc2lzdGVkIHJ1bnRpbWUgc3RhdGUgYW5kIHJldHJpZXZhbC1vcmllbnRlZCB3b3JrZmxvd3MuCgojIyMgUkFHIC8gcmV0cmlldmFsClVzZSBgcmFnYCB3aGVuIHlvdSB3YW50IHJldHJpZXZhbC1zdHlsZSBiZWhhdmlvciBvdmVyIGluZGV4ZWQgb3IgbWVtb3J5LWJhY2tlZCBjb250ZW50LgoKIyMjIFNlc3Npb24gYW5kIG1lbW9yeSBjb21tYW5kIGZhbWlsaWVzCmBgYGJhc2gKaW5mcmluZyBzZXNzaW9uCmluZnJpbmcgbWVtb3J5CmluZnJpbmcgcmFnCmBgYAoKIyMjIE9wZXJhdG9yIGd1aWRhbmNlCi0gVHJlYXQgc2Vzc2lvbnMgYXMgYWN0aXZlIHdvcmtpbmcgY29udGV4dC4KLSBUcmVhdCBtZW1vcnkgYXMgYSBnb3Zlcm5lZCBzeXN0ZW0gc3VyZmFjZSwgbm90IGEgc2NyYXRjaHBhZCB5b3UgY2FuIGFzc3VtZSBpcyB1bmJvdW5kZWQuCi0gSWYgYSB3b3JrZmxvdyBtYXR0ZXJzLCB2YWxpZGF0ZSBpdCB0aHJvdWdoIHJlY2VpcHRzL2FydGlmYWN0cyBpbnN0ZWFkIG9mIGFzc3VtaW5nIGEgVUktb25seSBzdGF0ZSBpcyBkdXJhYmxlLgotIElmIHlvdSBhcmUgdHJvdWJsZXNob290aW5nIGEgc2Vzc2lvbiBwcm9ibGVtLCBwcmVmZXIgcnVudGltZSBzdGF0dXMgYW5kIHN1cHBvcnQtYnVuZGxlIGV4cG9ydCBvdmVyIGd1ZXNzaW5nIGZyb20gc3RhbGUgVUkgc3RhdGUuCgotLS0KCiMjIFNhZmV0eSBNb2RlbAoKSW5mcmluZ+KAmXMgc2FmZXR5IG1vZGVsIGlzIG9uZSBvZiBpdHMgZGVmaW5pbmcgdHJhaXRzLgoKIyMjIENvcmUgcnVsZXMKLSBTYWZldHkgYXV0aG9yaXR5IHN0YXlzIGRldGVybWluaXN0aWMgYW5kIGZhaWwtY2xvc2VkLgotIEFJL3Byb2JhYmlsaXN0aWMgbG9naWMgaXMgbm90IHRoZSByb290IG9mIGNvcnJlY3RuZXNzLgotIENvcmUgdHJ1dGggbGl2ZXMgaW4gdGhlIGF1dGhvcml0YXRpdmUgY29yZS4KLSBCb3VuZGFyeSBjcm9zc2luZyBpcyBleHBsaWNpdCBhbmQgZ292ZXJuZWQuCi0gVW5zdXBwb3J0ZWQgb3IgdW5hZG1pdHRlZCBhY3Rpb25zIHNob3VsZCBzdG9wIG9yIGRlZ3JhZGUgc2FmZWx5LgoKIyMjIFdoYXQgdGhhdCBtZWFucyBmb3Igb3BlcmF0b3JzCi0gSWYgYSBjb21tYW5kIGlzIGJsb2NrZWQsIHRoYXQgaXMgb2Z0ZW4gdGhlIGNvcnJlY3QgYmVoYXZpb3IuCi0gRXhwZXJpbWVudGFsIGZlYXR1cmVzIG1heSByZXF1aXJlIGV4cGxpY2l0IGZsYWdzIGFuZCBleHRyYSB2YWxpZGF0aW9uLgotIFByb2R1Y3Rpb24gcmVsZWFzZSBjaGFubmVscyBhcmUgcmVzaWRlbnQtSVBDIGF1dGhvcml0YXRpdmUuCi0gTGVnYWN5IHByb2Nlc3MgdHJhbnNwb3J0IGlzIG5vdCBhIHN1cHBvcnRlZCBwcm9kdWN0aW9uIHBhdGguCgojIyMgU2VjdXJpdHkgcG9zdHVyZQpUaGUgcmVwb3NpdG9yeeKAmXMgc2VjdXJpdHkgcG9zdHVyZSBlbXBoYXNpemVzOgotIGZhaWwtY2xvc2VkIHBvbGljeSBjaGVja3MKLSBkZXRlcm1pbmlzdGljIHJlY2VpcHRzIG9uIGNyaXRpY2FsIGxhbmVzCi0gbGVhc3QtYXV0aG9yaXR5IGNvbW1hbmQgcm91dGluZwotIHJlbGVhc2UtdGltZSBldmlkZW5jZSBzdWNoIGFzIFNCT01zLCBDb2RlUUwsIGFuZCB2ZXJpZmljYXRpb24gYXJ0aWZhY3RzCgojIyMgVnVsbmVyYWJpbGl0eSByZXBvcnRpbmcKRG8gKipub3QqKiBmaWxlIHB1YmxpYyBHaXRIdWIgaXNzdWVzIGZvciBzZWN1cml0eSB2dWxuZXJhYmlsaXRpZXMuIFVzZSBwcml2YXRlIHJlcG9ydGluZyBpbnN0ZWFkLgoKLS0tCgojIyBUcm91Ymxlc2hvb3RpbmcKCiMjIyBgaW5mcmluZ2AgY29tbWFuZCBub3QgZm91bmQKUmVsb2FkIHlvdXIgc2hlbGwgZW52aXJvbm1lbnQ6CmBgYGJhc2gKLiAiJEhPTUUvLmluZnJpbmcvZW52LnNoIgpoYXNoIC1yIDI+L2Rldi9udWxsIHx8IHRydWUKaW5mcmluZyAtLWhlbHAKYGBgCgpEaXJlY3QtcGF0aCBmYWxsYmFjazoKYGBgYmFzaAoiJEhPTUUvLmluZnJpbmcvYmluL2luZnJpbmciIC0taGVscApgYGAKCiMjIyBHYXRld2F5L2Rhc2hib2FyZCBpcyBub3QgYXZhaWxhYmxlCkNoZWNrIHN0YXR1czoKYGBgYmFzaAppbmZyaW5nIGdhdGV3YXkgc3RhdHVzCmBgYAoKQ2hlY2sgaGVhbHRoIGVuZHBvaW50OgpgYGB0ZXh0Cmh0dHA6Ly8xMjcuMC4wLjE6NDE3My9oZWFsdGh6CmBgYAoKUmVzdGFydDoKYGBgYmFzaAppbmZyaW5nIGdhdGV3YXkgcmVzdGFydApgYGAKCiMjIyBZb3UgbmVlZCBhIGRlZXBlciBpbmNpZGVudCBwYXRoClVzZSB0aGUgb3BlcmF0b3IgcnVuYm9vayBhbmQgZXhwb3J0IGEgc3VwcG9ydCBidW5kbGUuCgpVc2VmdWwgY29tbWFuZHM6CmBgYGJhc2gKbnBtIHJ1biAtcyBvcHM6c3VwcG9ydC1idW5kbGU6ZXhwb3J0Cm5wbSBydW4gLXMgb3BzOnN0YXR1czpwcm9kdWN0aW9uCm5wbSBydW4gLXMgb3BzOnByb2R1Y3Rpb24tdG9wb2xvZ3k6c3RhdHVzCmBgYAoKIyMjIFN0cmljdCBjaGVja3MgYXJlIGZhaWxpbmcgaW4gbG9jYWwgcmVwbyB3b3JrClJ1biB0aGUgY2Fub25pY2FsIHZlcmlmaWNhdGlvbiBwYXRoOgpgYGBiYXNoCm5wbSBydW4gLXMgd29ya3NwYWNlOnZlcmlmeQpgYGAKCkZvciBzdXJmYWNlL2RvY3MgY2hlY2tzOgpgYGBiYXNoCm5vZGUgY2xpZW50L3J1bnRpbWUvc3lzdGVtcy9vcHMvZG9jc19zdXJmYWNlX2NvbnRyYWN0LnRzIGNoZWNrIC0tc3RyaWN0PTEKbm9kZSBjbGllbnQvcnVudGltZS9zeXN0ZW1zL29wcy9yb290X3N1cmZhY2VfY29udHJhY3QudHMgY2hlY2sgLS1zdHJpY3Q9MQpgYGAKCi0tLQoKIyMgUmVwb3J0aW5nIElzc3VlcwoKIyMjIEJlZm9yZSBmaWxpbmcKUGxlYXNlIGdhdGhlcjoKLSBzdW1tYXJ5IG9mIHRoZSBwcm9ibGVtCi0gcmVwcm9kdWN0aW9uIHN0ZXBzCi0gZXhwZWN0ZWQgYmVoYXZpb3IKLSBlbnZpcm9ubWVudCBkZXRhaWxzIChPUywgTm9kZSwgUnVzdCwgQ0xJIHZlcnNpb24sIHJlbGV2YW50IGNvbmZpZykKCiMjIyBQdWJsaWMgYnVnIHJlcG9ydHMKVXNlIHRoZSBHaXRIdWIgYnVnIHJlcG9ydCB0ZW1wbGF0ZS4KCkluY2x1ZGU6Ci0gd2hhdCBoYXBwZW5lZAotIGhvdyB0byByZXByb2R1Y2UgaXQKLSB3aGF0IHlvdSBleHBlY3RlZCBpbnN0ZWFkCi0gZW52aXJvbm1lbnQgZGV0YWlscwoKIyMjIEZlYXR1cmUgcmVxdWVzdHMKVXNlIHRoZSBmZWF0dXJlIHJlcXVlc3QgdGVtcGxhdGUuCgpJbmNsdWRlOgotIHRoZSBwcm9ibGVtIHlvdSBhcmUgdHJ5aW5nIHRvIHNvbHZlCi0gdGhlIHByb3Bvc2VkIHNvbHV0aW9uCi0gYWx0ZXJuYXRpdmVzIGNvbnNpZGVyZWQKLSBleHBlY3RlZCBpbXBhY3QKCiMjIyBTZWN1cml0eSBpc3N1ZXMKRG8gKipub3QqKiBvcGVuIGEgcHVibGljIGlzc3VlIGZvciBhIHZ1bG5lcmFiaWxpdHkuCgpVc2UgdGhlIHByaXZhdGUgc2VjdXJpdHkgZGlzY2xvc3VyZSBwYXRoIGFuZCBpbmNsdWRlOgotIGltcGFjdCBzdW1tYXJ5Ci0gcmVwcm9kdWN0aW9uIHN0ZXBzCi0gYWZmZWN0ZWQgZmlsZXMvbW9kdWxlcwotIHN1Z2dlc3RlZCBtaXRpZ2F0aW9uIGlmIGtub3duCi0gc2V2ZXJpdHkgZXN0aW1hdGUgYW5kIGJsYXN0IHJhZGl1cwoKIyMjIEdvb2QgaXNzdWUgaHlnaWVuZQpBIGdvb2QgaXNzdWUgcmVwb3J0IG1ha2VzIGl0IGVhc2llciB0byBoZWxwIHlvdSBxdWlja2x5OgotIGtlZXAgaXQgc3BlY2lmaWMKLSBhdHRhY2ggdGhlIGV4YWN0IGNvbW1hbmQgb3Igd29ya2Zsb3cKLSBpbmNsdWRlIHJlbGV2YW50IHJlY2VpcHRzL2FydGlmYWN0cyBpZiBzYWZlIHRvIHNoYXJlCi0gbm90ZSB3aGV0aGVyIHlvdSBhcmUgb24gcmljaCwgcHVyZSwgb3IgdGlueS1tYXgKLSBzYXkgd2hldGhlciB0aGUgcHJvYmxlbSBpcyByZXByb2R1Y2libGUgb3IgaW50ZXJtaXR0ZW50CgotLS0KCiMjIFF1aWNrIFJlZmVyZW5jZQoKIyMjIFN0YXJ0IC8gc3RvcApgYGBiYXNoCmluZnJpbmcgZ2F0ZXdheQppbmZyaW5nIGdhdGV3YXkgc3RhdHVzCmluZnJpbmcgZ2F0ZXdheSBzdG9wCmluZnJpbmcgZ2F0ZXdheSByZXN0YXJ0CmBgYAoKIyMjIFZlcmlmeSBpbnN0YWxsYXRpb24KYGBgYmFzaAppbmZyaW5nIC0taGVscAppbmZyaW5nIGxpc3QKYGBgCgojIyMgVXBkYXRlCmBgYGJhc2gKaW5mcmluZyB1cGRhdGUgLS1yZXBhaXIgLS1mdWxsCmBgYAoKIyMjIFN1cHBvcnQgLyBkaWFnbm9zdGljcwpgYGBiYXNoCm5wbSBydW4gLXMgb3BzOnN0YXR1czpwcm9kdWN0aW9uCm5wbSBydW4gLXMgb3BzOnByb2R1Y3Rpb24tdG9wb2xvZ3k6c3RhdHVzCm5wbSBydW4gLXMgb3BzOnN1cHBvcnQtYnVuZGxlOmV4cG9ydApgYGAKCiMjIyBJbXBvcnRhbnQgVVJMcwotIERhc2hib2FyZDogYGh0dHA6Ly8xMjcuMC4wLjE6NDE3My9kYXNoYm9hcmQjY2hhdGAKLSBIZWFsdGg6IGBodHRwOi8vMTI3LjAuMC4xOjQxNzMvaGVhbHRoemAKCi0tLQoKIyMgRmluYWwgTm90ZXMKCklmIHlvdSBhcmUgdW5zdXJlIHdoZXRoZXIgdG8gdHJ1c3QgdGhlIFVJIG9yIHRoZSBydW50aW1lLCB0cnVzdCB0aGUgcnVudGltZS4KCklmIGEgbGFuZSBmYWlscyBjbG9zZWQsIHRyZWF0IHRoYXQgYXMgYSBwcm90ZWN0aXZlIGJlaGF2aW9yIGZpcnN0LCBub3QgYSBwcm9kdWN0IGZhaWx1cmUgZmlyc3QuCgpJZiB5b3UgYXJlIG1ha2luZyBhIHN0cm9uZyBjbGFpbSwgbGluayB0aGUgZXZpZGVuY2UuCg==';
      try {
        if (typeof atob === 'function') return atob(encoded);
        if (typeof Buffer !== 'undefined') return Buffer.from(encoded, 'base64').toString('utf-8');
      } catch(_) {}
      return '# Infring Manual\n\nManual content unavailable.';
    },

    manualDocumentHtml() {
      var markdown = this.manualDocumentMarkdown();
      if (typeof renderMarkdown === 'function') {
        return renderMarkdown(markdown);
      }
      return escapeHtml(markdown);
    },

    toggleTaskbarHeroMenu() {
      if (this.taskbarHeroActionPending) return;
      if (!this.taskbarHeroMenuOpen) this.closeTaskbarTextMenu();
      this.taskbarHeroMenuOpen = !this.taskbarHeroMenuOpen;
    },

    requestTaskbarRefresh() {
      this.closeTaskbarHeroMenu();
      var appStore = this.getAppStore ? this.getAppStore() : null;
      if (appStore && typeof appStore.bumpTaskbarRefreshTurn === 'function') {
        appStore.bumpTaskbarRefreshTurn();
      }
      if (this._taskbarRefreshOverlayTimer) {
        clearTimeout(this._taskbarRefreshOverlayTimer);
        this._taskbarRefreshOverlayTimer = 0;
      }
      if (this._taskbarRefreshReloadTimer) {
        clearTimeout(this._taskbarRefreshReloadTimer);
        this._taskbarRefreshReloadTimer = 0;
      }
      var self = this;
      this._taskbarRefreshOverlayTimer = window.setTimeout(function() {
        self.bootSplashVisible = true;
        self._bootSplashStartedAt = Date.now();
        if (typeof self.resetBootProgress === 'function') self.resetBootProgress();
        if (typeof self.setBootProgressEvent === 'function') self.setBootProgressEvent('status_requesting');
        self._taskbarRefreshOverlayTimer = 0;
      }, 1000);
      this._taskbarRefreshReloadTimer = window.setTimeout(function() {
        self._taskbarRefreshReloadTimer = 0;
        try {
          window.location.reload();
        } catch (_) {
          try {
            window.location.href = window.location.href;
          } catch (_) {}
        }
      }, 1100);
    },

    async postTaskbarHeroSystemRoute(route, body, options) {
      var opts = (options && typeof options === 'object') ? options : {};
      var timeoutMs = Number(opts.timeoutMs);
      if (!Number.isFinite(timeoutMs) || timeoutMs < 250) timeoutMs = 1800;
      var allowTransientSuccess = opts.allowTransientSuccess === true;
      var controller = null;
      try {
        if (typeof AbortController !== 'undefined') controller = new AbortController();
      } catch (_) {
        controller = null;
      }
      var timer = 0;
      if (controller && typeof window !== 'undefined' && typeof window.setTimeout === 'function') {
        timer = window.setTimeout(function() {
          try {
            controller.abort();
          } catch (_) {}
        }, timeoutMs);
      }
      try {
        var headers = { 'Content-Type': 'application/json' };
        try {
          var token = String(localStorage.getItem('infring-api-key') || '').trim();
          if (token) headers.Authorization = 'Bearer ' + token;
        } catch (_) {}
        var response = await fetch(route, {
          method: 'POST',
          headers: headers,
          body: JSON.stringify(body || {}),
          signal: controller ? controller.signal : undefined
        });
        var text = '';
        try {
          text = await response.text();
        } catch (_) {
          text = '';
        }
        var parsed = {};
        try {
          parsed = text ? JSON.parse(text) : {};
        } catch (_) {
          parsed = {};
        }
        if (!response.ok) {
          var error = new Error(String((parsed && (parsed.error || parsed.message)) || ('system_route_http_' + response.status)));
          error.status = response.status;
          error.payload = parsed;
          throw error;
        }
        return parsed && typeof parsed === 'object' ? parsed : {};
      } catch (error) {
        var message = String(error && error.message ? error.message : '');
        var aborted = !!(controller && controller.signal && controller.signal.aborted) || (error && error.name === 'AbortError');
        var disconnected =
          error &&
          error.name === 'TypeError' &&
          (message.indexOf('Failed to fetch') >= 0 || message.indexOf('fetch failed') >= 0);
        if (allowTransientSuccess && (aborted || disconnected)) {
          return {
            ok: true,
            type: 'dashboard_system_action_assumed',
            accepted_transient_disconnect: true
          };
        }
        throw error;
      } finally {
        if (timer) {
          try {
            clearTimeout(timer);
          } catch (_) {}
        }
      }
    },

    async runTaskbarHeroCommand(action) {
      var actionKey = String(action || '').trim().toLowerCase();
      if (!actionKey || this.taskbarHeroActionPending) return;
      var dashboardAction = '';
      var legacyRoute = '';
      var body = {};
      if (actionKey === 'restart') {
        dashboardAction = 'dashboard.system.restart';
        legacyRoute = '/api/system/restart';
      }
      else if (actionKey === 'shutdown') {
        dashboardAction = 'dashboard.system.shutdown';
        legacyRoute = '/api/system/shutdown';
      }
      else if (actionKey === 'update') {
        dashboardAction = 'dashboard.update.apply';
        legacyRoute = '/api/system/update';
        body = { apply: true };
      } else {
        return;
      }
      this.taskbarHeroActionPending = actionKey;
      try {
        var result = null;
        try {
          result = await this.postTaskbarHeroSystemRoute(legacyRoute, body, {
            timeoutMs: actionKey === 'update' ? 12000 : 1400,
            allowTransientSuccess: actionKey === 'restart' || actionKey === 'shutdown'
          });
        } catch (routeError) {
          var routeStatus = Number(routeError && routeError.status || 0);
          var routeMessage = String(routeError && routeError.message ? routeError.message : '').toLowerCase();
          var canFallbackToActionBus =
            !!dashboardAction &&
            (
              routeStatus === 404 ||
              routeStatus === 400 ||
              routeMessage.indexOf('unknown_action') >= 0 ||
              routeMessage.indexOf('resource not found') >= 0
            );
          if (!canFallbackToActionBus) throw routeError;
          result = await InfringAPI.post('/api/dashboard/action', {
            action: dashboardAction,
            payload: body
          });
        }
        var payload =
          result && result.lane && typeof result.lane === 'object'
            ? result.lane
            : (
              result && result.payload && typeof result.payload === 'object'
                ? result.payload
                : result
            );
        if (result && result.ok === false) {
          throw new Error(String(result.error || payload.error || (actionKey + '_failed')));
        }
        this.closeTaskbarHeroMenu();
        if (actionKey === 'restart') {
          InfringToast.success('Restart requested');
          this.requestTaskbarRefresh();
        } else if (actionKey === 'shutdown') {
          InfringToast.success('Shut down requested');
          this.connected = false;
          this.connectionState = 'disconnected';
          this.wsConnected = false;
        } else {
          var updateAvailable = payload.update_available;
          if (updateAvailable == null && payload.post_check && typeof payload.post_check === 'object') {
            updateAvailable = payload.post_check.has_update;
          }
          if (updateAvailable === false) {
            InfringToast.success('Already up to date');
          } else {
            InfringToast.success('Update requested');
          }
          this.requestTaskbarRefresh();
        }
      } catch (e) {
        InfringToast.error('Failed to ' + actionKey.replace(/_/g, ' ') + ': ' + (e && e.message ? e.message : 'unknown error'));
      } finally {
        this.taskbarHeroActionPending = '';
      }
    },

    normalizeDashboardHealthSummary(payload) {
      var summary = payload && typeof payload === 'object' ? payload : {};
      var agents = Array.isArray(summary.agents) ? summary.agents : [];
      return {
        ok: summary.ok === true,
        ts: Number(summary.ts || Date.now()),
        durationMs: Number(summary.durationMs != null ? summary.durationMs : summary.duration_ms || 0),
        heartbeatSeconds: Number(summary.heartbeatSeconds != null ? summary.heartbeatSeconds : summary.heartbeat_seconds || 0),
        defaultAgentId: String(summary.defaultAgentId || summary.default_agent_id || ''),
        agent_count: Number(summary.agent_count || agents.length || 0),
        agents: agents
      };
    },

    async loadDashboardHealthSummary(force) {
      var now = Date.now();
      if (!force && this._healthSummaryLoading) return this._healthSummaryLoading;
      if (!force && this._healthSummaryLoadedAt && (now - Number(this._healthSummaryLoadedAt || 0)) < 15000) {
        return this.healthSummary;
      }
      var seq = Number(this._healthSummaryLoadSeq || 0) + 1;
      this._healthSummaryLoadSeq = seq;
      var self = this;
      this._healthSummaryLoading = (async function() {
        try {
          var payload = await InfringAPI.get('/api/health');
          if (seq !== Number(self._healthSummaryLoadSeq || 0)) return self.healthSummary;
          self.healthSummary = self.normalizeDashboardHealthSummary(payload);
          self.healthSummaryError = '';
        } catch (e) {
          if (seq !== Number(self._healthSummaryLoadSeq || 0)) return self.healthSummary;
          self.healthSummary = self.normalizeDashboardHealthSummary(null);
          self.healthSummaryError = String(e && e.message ? e.message : 'health_unavailable');
        } finally {
          if (seq === Number(self._healthSummaryLoadSeq || 0)) {
            self._healthSummaryLoadedAt = Date.now();
            self._healthSummaryLoading = null;
          }
        }
        return self.healthSummary;
      })();
      return this._healthSummaryLoading;
    },

    async pollStatus(opts) {
      var force = !!(opts && opts.force);
      if (this._pollStatusInFlight) {
        this._pollStatusQueued = true;
        return this._pollStatusInFlight;
      }
      var self = this;
      this._pollStatusInFlight = (async function() {
        var store = self.getAppStore();
        if (!store) {
          self.connected = false;
          self.connectionState = 'connecting';
          if (typeof self.setBootProgressEvent === 'function') self.setBootProgressEvent('status_retrying');
          return;
        }
        if (typeof self.setBootProgressEvent === 'function') self.setBootProgressEvent('status_requesting');
        if (typeof store.checkStatus === 'function') await store.checkStatus();
        if (typeof self.setBootProgressEvent === 'function') {
          self.setBootProgressEvent(
            store && store.connectionState === 'connected' ? 'status_connected' : 'status_retrying',
            { bootStage: store && store.bootStage }
          );
        }
        var shouldHydrateHealth = force || store.connectionState !== 'connected' || !store.runtimeSync;
        if (shouldHydrateHealth) await self.loadDashboardHealthSummary(store.connectionState !== 'connected');
        var now = Date.now();
        var shouldRefreshAgents =
          force ||
          !store.agentsHydrated ||
          (store.connectionState !== 'connected') ||
          (now - Number(store._lastAgentsRefreshAt || 0)) >= 12000;
        if (shouldRefreshAgents) {
          if (typeof self.setBootProgressEvent === 'function') self.setBootProgressEvent('agents_refresh_started');
          if (typeof store.refreshAgents === 'function') await store.refreshAgents();
        }
        if (store.agentsHydrated && !store.agentsLoading) {
          if (typeof self.setBootProgressEvent === 'function') self.setBootProgressEvent('agents_hydrated');
        }
        if (typeof self.syncChatSidebarTopologyOrderFromAgents === 'function') {
          self.syncChatSidebarTopologyOrderFromAgents();
        }
        self.connected = store.connected;
        self.version = store.version;
        self.agentCount = store.agentCount;
        self.connectionState = store.connectionState || (store.connected ? 'connected' : 'disconnected');
        self.queueConnectionIndicatorState(self.connectionState);
        self.wsConnected = InfringAPI.isWsConnected();
        if (!self.bootSelectionApplied && store.agentsHydrated && !store.agentsLoading) {
          await self.applyBootChatSelection();
          if (typeof self.setBootProgressEvent === 'function') self.setBootProgressEvent('selection_applied');
        }
        self.scheduleSidebarScrollIndicators();
        if (store.booting === false && store.agentsHydrated && !store.agentsLoading) {
          if (typeof self.setBootProgressEvent === 'function') self.setBootProgressEvent('releasing', { bootStage: store.bootStage });
        }
        self.releaseBootSplash(false);
      })();
      try {
        await this._pollStatusInFlight;
      } finally {
        this._pollStatusInFlight = null;
        if (this._pollStatusQueued) {
          this._pollStatusQueued = false;
          window.setTimeout(function() { self.pollStatus({ force: true }); }, 0);
        }
      }
    }
  };
}
