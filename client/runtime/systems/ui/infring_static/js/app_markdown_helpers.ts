// Canonical Shell helper source: dashboard markdown rendering utilities.
// Loaded before app.ts by the dashboard asset router.

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
