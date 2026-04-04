        }
        streamedTools = streamedTools.concat(Array.isArray(row.tools) ? row.tools : []);
      }
      return {
        text: streamedText,
        tools: streamedTools,
        thought: String(streamedThought || '').trim()
      };
    },
    extractThinkingLeak: function(text) {
      if (!text) return { thought: '', content: '' };
      var raw = String(text).replace(/\r\n?/g, '\n');
      var trimmed = raw.replace(/^\s+/, '');
      if (!trimmed) return { thought: '', content: '' };
      var thinkingPrefix = /^(thinking(?:\s+out\s+loud)?(?:\.\.\.|:)?|analysis(?:\.\.\.|:)?|reasoning(?:\.\.\.|:)?)/i;
      var explicitPrefix = thinkingPrefix.test(trimmed);
      if (!explicitPrefix && !this.looksLikeThoughtLeak(trimmed)) return { thought: '', content: raw };
      var splitAt = this.findThinkingBoundary(trimmed);
      if (splitAt < 0) return { thought: trimmed.trim(), content: '' };
      return {
        thought: trimmed.slice(0, splitAt).trim(),
        content: trimmed.slice(splitAt).trim()
      };
    },

    looksLikeThoughtLeak: function(text) {
      var value = String(text || '').replace(/\s+/g, ' ').trim();
      if (!value) return false;
      if (value.length < 80) return false;
      var lead = /^(alright|okay|ok|hmm|let me|i need to|to answer this|first[, ]|i should|i will|i'm going to)\b/i;
      if (!lead.test(value)) return false;
      var markers = [
        /\b(user'?s request|the user asked|address the user|step by step)\b/i,
        /\blet me think\b/i,
        /\bi need to\b/i,
        /\bfirst[, ]/i,
        /\bcheck\b/i,
        /\bconsider\b/i
      ];
      var hits = 0;
      for (var i = 0; i < markers.length; i++) {
        if (markers[i].test(value)) hits += 1;
      }
      return hits >= 2;
    },
    findThinkingBoundary: function(text) {
      if (!text) return -1;
      var boundaries = [];
      var markers = [
        /\n\s*final answer\s*:/i,
        /\n\s*answer\s*:/i,
        /\n\s*response\s*:/i,
        /\n\s*output\s*:/i,
        /\n\s*```/i,
        /\n\s*\n(?=\s*[\{\[])/,
      ];
      markers.forEach(function(rx) {
        var match = text.match(rx);
        if (match && Number.isFinite(match.index)) {
          boundaries.push(match.index + 1);
        }
      });
      if (!boundaries.length) return -1;
      boundaries.sort(function(a, b) { return a - b; });
      return boundaries[0];
    },

    makeThoughtToolCard: function(thoughtText, durationMs) {
      var ms = Number(durationMs || 0);
      if (!Number.isFinite(ms) || ms < 0) ms = 0;
      return {
        id: 'thought-' + Date.now() + '-' + Math.floor(Math.random() * 10000),
        name: 'thought_process',
        running: false,
        expanded: false,
        input: String(thoughtText || '').trim(),
        result: '',
        is_error: false,
        duration_ms: ms
      };
    },

    appendThoughtChunk: function(base, chunk) {
      var prior = String(base || '').trim();
      var next = String(chunk || '').trim();
      if (!next) return prior;
      if (!prior) return next;
      if (prior.slice(-next.length) === next) return prior;
      var merged = prior + '\n' + next;
      if (merged.length > 8000) {
        merged = merged.slice(merged.length - 8000);
      }
      return merged;
    },
    latestCompleteSentence: function(inputText) {
      var raw = String(inputText || '')
        .replace(/<[^>]*>/g, ' ')
        .replace(/^\*+|\*+$/g, '')
        .replace(/\r/g, '')
        .trim();
      if (!raw) return '';
      var value = raw.replace(/[ \t]+/g, ' ').trim();
      if (!value) return '';
      var sentenceMatches = value.match(/[^.!?…。！？;:]+[.!?…。！？;:]+(?:["')\]]+)?/g);
      if (sentenceMatches && sentenceMatches.length) {
        var latest = String(sentenceMatches[sentenceMatches.length - 1] || '').trim();
        return latest || '';
      }
      var lines = raw.split('\n').map(function(line) {
        return String(line || '').replace(/\s+/g, ' ').trim();
      }).filter(function(line) { return !!line; });
      if (lines.length < 2) return '';
      var finalLine = String(lines[lines.length - 1] || '').trim();
      if (/[.!?…]$/.test(finalLine)) return finalLine;
      return String(lines[lines.length - 2] || '').trim();
    },
    thoughtSentenceFrames: function(inputText) {
      var value = String(inputText || '')
        .replace(/<[^>]*>/g, ' ')
        .replace(/\r/g, '')
        .trim();
      if (!value) return [];
      var matches = value.match(/[^.!?…。！？;:]+[.!?…。！？;:]+(?:["')\]]+)?/g) || [];
      return matches
        .map(function(part) { return String(part || '').replace(/\s+/g, ' ').trim(); })
        .filter(function(part) { return !!part; });
    },
    nextThoughtSentenceFrame: function(msg, thoughtText) {
      var frames = this.thoughtSentenceFrames(thoughtText);
      if (!frames.length) return '';
      if (!msg || typeof msg !== 'object') {
        return frames[frames.length - 1];
      }
      var nextIndex = Number(msg._thought_frame_index || 0);
      if (!Number.isFinite(nextIndex) || nextIndex < 0) nextIndex = 0;
      var seenCount = Number(msg._thought_frame_seen_count || 0);
      if (!Number.isFinite(seenCount) || seenCount < 0) seenCount = 0;
      // Advance the shown thought line only when an additional complete sentence
      // appears (punctuation-delimited), not on every text delta token.
      if (seenCount <= 0) {
        nextIndex = 0;
      } else if (frames.length > seenCount) {
        nextIndex = Math.min(nextIndex + (frames.length - seenCount), Math.max(0, frames.length - 1));
      } else {
        nextIndex = Math.max(0, Math.min(nextIndex, frames.length - 1));
      }
      msg._thought_frame_seen_count = frames.length;
      msg._thought_frame_index = nextIndex;
      msg._thought_frame_signature = frames.length + '|' + frames[frames.length - 1];
      var frame = String(frames[Math.max(0, Math.min(frames.length - 1, nextIndex))] || '').trim();
      if (frame) msg._thought_last_complete_sentence = frame;
      return frame;
    },
    renderLiveThoughtHtml: function(thoughtText, msg) {
      var text = this.nextThoughtSentenceFrame(msg, thoughtText) || this.latestCompleteSentence(thoughtText) || '';
      return '<span class="thinking-live-inline"><em>' + escapeHtml(text) + '</em></span>';
    },

    defaultAssistantFallback: function(thoughtText, tools) {
      var thought = String(thoughtText || '').trim();
      var hasToolError = Array.isArray(tools) && tools.some(function(tool) {
        return !!(tool && tool.is_error);
      });
      if (hasToolError) {
        return 'I could not finish the request because a required step failed. Please clarify the goal or try again.';
      }
      var successfulToolSummary = '';
      if (Array.isArray(tools) && tools.length) {
        var successful = tools.filter(function(tool) {
          if (!tool || tool.running || tool.is_error) return false;
          return !!String(tool.result || '').trim();
        });
        if (successful.length) {
          var parts = successful.slice(0, 2).map(function(tool) {
            var toolName = String(tool.name || 'tool').replace(/_/g, ' ').trim();
            var result = String(tool.result || '').replace(/\s+/g, ' ').trim();
            if (result.length > 120) result = result.slice(0, 117) + '...';
            return toolName ? (toolName + ': ' + result) : result;
          }).filter(function(part) { return !!part; });
          if (parts.length) successfulToolSummary = parts.join(' | ');
        }
      }
