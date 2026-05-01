'use strict';

function infringChatPromptSuggestionMethods() {
  return {
    normalizePromptSuggestions(rows, contextText, disallowSamples) {
      var source = Array.isArray(rows) ? rows : [];
      var blocked = Array.isArray(disallowSamples) ? disallowSamples : [];
      var seen = {};
      var out = [];
      var contextKeywords = [];
      var wordCount = function(text) {
        return String(text == null ? '' : text).trim().split(/\s+/g).filter(Boolean).length;
      };
      var tokenize = function(value) {
        var stop = {
          can: true, could: true, would: true, should: true, what: true, why: true, how: true, when: true, where: true, who: true,
          the: true, this: true, that: true, with: true, from: true, into: true, your: true, you: true, and: true, for: true,
          then: true, now: true, again: true, please: true
        };
        return String(value == null ? '' : value)
          .toLowerCase()
          .replace(/[^a-z0-9_:-]+/g, ' ')
          .split(/\s+/g)
          .filter(function(token) { return !!(token && token.length >= 3 && !stop[token]); });
      };
      var tokenSimilarity = function(a, b) {
        var left = tokenize(a);
        var right = tokenize(b);
        if (!left.length && !right.length) return 1;
        if (!left.length || !right.length) return 0;
        var leftSet = {};
        var rightSet = {};
        var i;
        for (i = 0; i < left.length; i++) leftSet[left[i]] = true;
        for (i = 0; i < right.length; i++) rightSet[right[i]] = true;
        var overlap = 0;
        var union = {};
        Object.keys(leftSet).forEach(function(token) {
          union[token] = true;
          if (rightSet[token]) overlap += 1;
        });
        Object.keys(rightSet).forEach(function(token) { union[token] = true; });
        var unionSize = Object.keys(union).length || 1;
        return overlap / unionSize;
      };
      var isNearDuplicate = function(a, b) {
        var left = String(a == null ? '' : a).toLowerCase().trim();
        var right = String(b == null ? '' : b).toLowerCase().trim();
        if (!left || !right) return false;
        if (left === right) return true;
        if (left.indexOf(right) >= 0 || right.indexOf(left) >= 0) return true;
        return tokenSimilarity(left, right) >= 0.72;
      };
      var trimTrailingJoiners = function(text) {
        var words = String(text == null ? '' : text).trim().split(/\s+/g).filter(Boolean);
        while (words.length > 1) {
          var tail = String(words[words.length - 1] || '')
            .replace(/[^a-z0-9_-]+/gi, '')
            .toLowerCase();
          if (!tail || /^(and|or|to|with|for|from|via|then|than|versus|vs)$/i.test(tail)) {
            words.pop();
            continue;
          }
          break;
        }
        return words.join(' ');
      };
      var clampWords = function(text, maxWords) {
        var cap = Number(maxWords || 10);
        var words = String(text == null ? '' : text).trim().split(/\s+/g).filter(Boolean);
        if (!words.length) return '';
        if (!Number.isFinite(cap) || cap < 3) cap = 10;
        if (words.length > cap) words = words.slice(0, cap);
        return trimTrailingJoiners(words.join(' '));
      };
      var normalizeVoice = function(value) {
        var row = String(value == null ? '' : value)
          .replace(/\s+/g, ' ')
          .trim();
        if (!row) return '';
        row = row
          .replace(/^\s*[-*0-9.)\]]+\s*/, '')
          .replace(/^\s*\[[^\]\n]{2,96}\]\s*/, '')
          .replace(/^\s*(?:\*\*)?(?:agent|assistant|system|model|ai|jarvis|user|human)(?:\*\*)?\s*:\s*/i, '')
          .replace(/^ask\s+[^.?!]{0,140}?\s+to\s+/i, '')
          .replace(/^ask\s+[^.?!]{0,140}?\s+for\s+/i, '')
          .replace(/^ask\s+for\s+/i, '')
          .replace(/^request\s+/i, '')
          .replace(/^please\s+request\s+/i, '')
          .replace(/\s+/g, ' ')
          .trim();
        // Suggestions must read as user->agent prompts, not agent->user offers.
        row = row
          .replace(/^(?:do you want me to|would you like me to|do you want us to|would you like us to)\s+/i, '')
          .replace(/^(?:want me to|should i|should we)\s+/i, '')
          .replace(/^(?:can i|could i|can we|could we)\s+/i, '')
          .replace(/^(?:i can|i could|i will|i'll|we can|we could|we will|we'll)\s+/i, '')
          .replace(/^(?:let me|let us)\s+/i, '')
          .trim();
        row = clampWords(row, 10);
        row = row.replace(/[.!?]+$/g, '').trim();
        if (!row) return '';
        row = trimTrailingJoiners(row);
        if (!row) return '';
        row = row.replace(/\?+$/g, '').trim();
        if (row.length && /^[a-z]/.test(row.charAt(0))) {
          row = row.charAt(0).toUpperCase() + row.slice(1);
        }
        if (row.length > 180) row = row.substring(0, 177) + '...';
        return row;
      };
      var isLowValue = function(text) {
        var lowered = String(text || '').toLowerCase();
        if (!lowered) return true;
        if (lowered.indexOf('the infring runtime is currently') >= 0) return true;
        if (lowered.indexOf('if you need help') >= 0 || lowered.indexOf('feel free to ask') >= 0) return true;
        if (lowered.indexOf('the user wants exactly 3 actionable next user prompts') >= 0) return true;
        if (lowered.indexOf('json array of strings') >= 0) return true;
        if (lowered.indexOf('output only the') >= 0) return true;
        if (lowered.indexOf('do not include numbering') >= 0) return true;
        if (lowered.indexOf('highest-roi') >= 0) return true;
        if (lowered.indexOf('runbook') >= 0) return true;
        if (lowered.indexOf('reliability remediation') >= 0) return true;
        if (lowered.indexOf('rollback criteria') >= 0) return true;
        if (lowered.indexOf('3-step execution plan') >= 0) return true;
        if (/^(do you want me to|would you like me to|want me to|should i|should we)\b/i.test(lowered)) return true;
        if (lowered.indexOf('this task') >= 0) return true;
        if (lowered === 'thinking...' || lowered === 'thinking..' || lowered === 'thinking.') return true;
        var sentenceCount = (text.match(/[.!?]/g) || []).length;
        if (sentenceCount > 2) return true;
        if (/[\"“”]/.test(text) && text.length > 120) return true;
        if (/^(give me|request|ask for)\b/i.test(text)) return true;
        var words = wordCount(text);
        if (words < 3 || words > 10) return true;
        var actionableStart =
          /^(can|could|would|should|what|why|how|when|where|who|show|fix|check|run|retry|switch|clear|drain|scale|continue|compare|explain|validate|review|open|trace|summarize|draft|outline|tell|list)\b/i.test(text);
        if (!actionableStart && text.indexOf('?') < 0 && /^\s*(the|it|this|that)\b/i.test(text)) return true;
        return false;
      };
      var isGeneric = function(text) {
        var lowered = String(text || '').toLowerCase();
        if (!lowered) return true;
        return (
          lowered.indexOf('continue this and keep the same direction') >= 0 ||
          lowered.indexOf('best next move from here') >= 0 ||
          lowered.indexOf('summarize progress in three concrete bullets') >= 0 ||
          lowered.indexOf('show the first command to run now') >= 0 ||
          lowered.indexOf('turn that into a concrete checklist') >= 0 ||
          lowered.indexOf('take the next step on current task') >= 0 ||
          lowered.indexOf('respond to the latest update') >= 0
        );
      };
      var rawContext = String(contextText == null ? '' : contextText).toLowerCase();
      contextKeywords = rawContext
        .split(/[^a-z0-9_:-]+/g)
        .filter(function(token) {
          return !!(
            token &&
            token.length >= 4 &&
            ['this', 'that', 'with', 'from', 'into', 'your', 'have', 'will', 'when', 'where', 'what'].indexOf(token) === -1
          );
        })
        .slice(0, 10);
      for (var i = 0; i < source.length; i++) {
        var raw = normalizeVoice(source[i]);
        if (!raw || isLowValue(raw)) continue;
        var parrotsSample = blocked.some(function(sample) {
          var cleanSample = normalizeVoice(sample || '');
          if (!cleanSample) return false;
          return isNearDuplicate(raw, cleanSample);
        });
        if (parrotsSample) continue;
        if (isGeneric(raw) && contextKeywords.length) {
          var loweredRaw = String(raw || '').toLowerCase();

// Layer ownership: client/runtime/systems/ui (dashboard static UX surface only; no runtime authority).
          var hasContextOverlap = contextKeywords.some(function(keyword) {
            return loweredRaw.indexOf(keyword) >= 0;
          });
          if (!hasContextOverlap) continue;
        }
        var key = String(raw || '').toLowerCase();
        if (seen[key]) continue;
        var duplicate = out.some(function(existing) {
          return isNearDuplicate(existing, raw);
        });
        if (duplicate) continue;
        seen[key] = true;
        out.push(raw);
        if (out.length >= 3) break;
      }
      return out;
    },
  };
}
