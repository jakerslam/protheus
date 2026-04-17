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
