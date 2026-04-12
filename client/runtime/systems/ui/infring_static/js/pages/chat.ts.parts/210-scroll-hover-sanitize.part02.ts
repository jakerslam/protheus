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
    textLooksNoFindingsPlaceholder: function(text) {
      var lower = String(text || '').replace(/\s+/g, ' ').trim().toLowerCase();
      if (!lower) return false;
      return (
        lower.indexOf("don't have usable tool findings from this turn yet") >= 0 ||
        lower.indexOf("dont have usable tool findings from this turn yet") >= 0 ||
        lower.indexOf('no usable findings yet') >= 0 ||
        lower.indexOf("couldn't extract usable findings") >= 0 ||
        lower.indexOf('could not extract usable findings') >= 0 ||
        lower.indexOf("couldn't produce source-backed findings in this turn") >= 0 ||
        lower.indexOf('search returned no useful information') >= 0
      );
    },
    textLooksToolAckWithoutFindings: function(text) {
      var lower = String(text || '').replace(/\s+/g, ' ').trim().toLowerCase();
      if (!lower) return false;
      return (
        lower.indexOf('completed tool steps:') === 0 ||
        lower.indexOf('completed the tool call, but no synthesized response was available yet') >= 0 ||
        lower.indexOf('returned no usable findings yet') >= 0
      );
    },
    textMentionsContextGuard: function(text) {
      var lower = String(text || '').replace(/\s+/g, ' ').trim().toLowerCase();
      if (!lower) return false;
      return (
        lower.indexOf('context overflow: estimated context size exceeds safe threshold during tool loop') >= 0 ||
        lower.indexOf('more characters truncated') >= 0 ||
        lower.indexOf('middle content omitted') >= 0 ||
        lower.indexOf('safe context budget') >= 0
      );
    },
    isWebLikeToolName: function(toolName) {
      var lower = String(toolName || '').trim().toLowerCase();
      return (
        lower === 'web_search' ||
        lower === 'web_fetch' ||
        lower === 'batch_query' ||
        lower === 'search_web' ||
        lower === 'web_query' ||
        lower === 'browse'
      );
    },
    stripContextGuardMarkers: function(text) {
      var value = String(text || '');
      if (!value) return '';
      return value
        .replace(/\[\.\.\.\s+\d+\s+more characters truncated\]/gi, ' ')
        .replace(/context overflow:\s*estimated context size exceeds safe threshold during tool loop\.?/gi, ' ')
        .replace(/middle content omitted/gi, ' ')
        .replace(/\s+/g, ' ')
        .trim();
    },
    toolResultSummarySnippet: function(tool) {
      var text = this.stripContextGuardMarkers(String(tool && tool.result ? tool.result : ''));
      if (!text) return '';
      if (this.textLooksNoFindingsPlaceholder(text) || this.textLooksToolAckWithoutFindings(text)) return '';
      var sentence = this.latestCompleteSentence(text) || text;
      var out = String(sentence || '').replace(/\s+/g, ' ').trim();
      if (out.length > 160) out = out.slice(0, 157) + '...';
      return out;
    },
    lowSignalWebToolSummary: function(tool) {
      var toolName = String(tool && tool.name ? tool.name : 'web tool').replace(/_/g, ' ').trim();
      var aggregate = typeof this.formatToolAggregateMeta === 'function'
        ? String(this.formatToolAggregateMeta(tool || {}) || '').trim()
        : toolName;
      var suffix = aggregate && aggregate !== toolName ? ' (' + aggregate.replace(/^.*?:\s*/, '') + ')' : '';
      if (this.textMentionsContextGuard(tool && tool.result)) {
        return 'The ' + (toolName || 'web tool') + ' step' + suffix + ' returned more output than fit safely in context. Retry with a narrower query, one specific source URL, or ask me to continue from the partial result.';
      }
      return 'The ' + (toolName || 'web tool') + ' step' + suffix + ' ran, but only low-signal web output came back. Retry with a narrower query, one specific source URL, or ask me to continue from the recorded tool result.';
    },
    responseHasAuthoritativeToolCompletion: function(payload, tools) {
      var rows = Array.isArray(tools) ? tools : [];
      var finalization = payload && payload.response_finalization && typeof payload.response_finalization === 'object'
        ? payload.response_finalization
        : null;
      var completion = finalization && finalization.tool_completion && typeof finalization.tool_completion === 'object'
        ? finalization.tool_completion
        : null;
      var attempts = Array.isArray(completion && completion.tool_attempts) ? completion.tool_attempts : [];
      if (attempts.length) return true;
      if (finalization && finalization.findings_available === true) return true;
      return rows.some(function(tool) {
        if (!tool || tool.running) return false;
        if (tool.blocked || tool.is_error) return true;
        return !!String(tool.result || tool.status || '').trim();
      });
    },
    completedToolOnlySummary: function(tools) {
      var rows = Array.isArray(tools) ? tools.filter(function(tool) {
        return !!(tool && String(tool.name || '').toLowerCase() !== 'thought_process');
      }) : [];
      if (!rows.length) return '';
      var actionableWeb = rows.find(function(tool) {
        if (!tool || tool.running || !this.isWebLikeToolName(tool.name || '')) return false;
        return (
          this.textMentionsContextGuard(tool.result || '') ||
          this.textLooksNoFindingsPlaceholder(tool.result || '') ||
          this.textLooksToolAckWithoutFindings(tool.result || '')
        );
      }, this);
      if (actionableWeb) {
        return this.lowSignalWebToolSummary(actionableWeb);
      }
      var successful = rows.filter(function(tool) {
        if (!tool || tool.running || tool.is_error || tool.blocked) return false;
        return !!this.toolResultSummarySnippet(tool);
      }, this);
      if (successful.length) {
        var parts = successful.slice(0, 2).map(function(tool) {
          var toolName = String(tool.name || 'tool').replace(/_/g, ' ').trim();
          var result = this.toolResultSummarySnippet(tool);
          return toolName ? (toolName + ': ' + result) : result;
        }, this).filter(function(part) { return !!part; });
        if (parts.length) return parts.join(' | ');
      }
      var blocked = rows.filter(function(tool) { return !!(tool && tool.blocked); });
      if (blocked.length) {
        var blockedNames = blocked.slice(0, 2).map(function(tool) {
          return String(tool.name || 'tool').replace(/_/g, ' ').trim();
        }).filter(function(name) { return !!name; });
        return 'The tool run completed, but policy blocked ' + (blockedNames.join(' and ') || 'a required step') + ' before a final prose answer was composed.';
      }
      var failed = rows.filter(function(tool) { return !!(tool && tool.is_error); });
      if (failed.length) {
        var firstFailure = failed[0] || {};
        if (this.isWebLikeToolName(firstFailure.name || '') && this.textMentionsContextGuard(firstFailure.result || '')) {
          return this.lowSignalWebToolSummary(firstFailure);
        }
        var failureName = String(firstFailure.name || 'tool').replace(/_/g, ' ').trim();
        var failureDetail = this.toolResultSummarySnippet(firstFailure) || String(firstFailure.status || '').replace(/\s+/g, ' ').trim();
        if (failureDetail.length > 120) failureDetail = failureDetail.slice(0, 117) + '...';
        if (failureDetail) {
          return 'The tool run completed, but ' + (failureName || 'a required step') + ' failed before a final prose answer was composed: ' + failureDetail;
        }
        return 'The tool run completed, but a required step failed before a final prose answer was composed.';
      }
      var completedNames = rows.slice(0, 3).map(function(tool) {
        return String(tool && tool.name ? tool.name : 'tool').replace(/_/g, ' ').trim();
      }).filter(function(name, idx, list) {
        return !!name && list.indexOf(name) === idx;
      });
      if (completedNames.length) {
        return 'Completed tool steps: ' + completedNames.join(', ') + '. Ask me to continue from those recorded results.';
      }
      return '';
    },

    defaultAssistantFallback: function(thoughtText, tools) {
      var thought = String(thoughtText || '').trim();
      var hasToolError = Array.isArray(tools) && tools.some(function(tool) {
        return !!(tool && tool.is_error);
      });
      var toolCompletionSummary = this.completedToolOnlySummary(tools);
      var successfulToolSummary = '';
      if (Array.isArray(tools) && tools.length) {
        var successful = tools.filter(function(tool) {
          if (!tool || tool.running || tool.is_error) return false;
          return !!this.toolResultSummarySnippet(tool);
        }, this);
        if (successful.length) {
          var parts = successful.slice(0, 2).map(function(tool) {
            var toolName = String(tool.name || 'tool').replace(/_/g, ' ').trim();
            var result = this.toolResultSummarySnippet(tool);
            return toolName ? (toolName + ': ' + result) : result;
          }, this).filter(function(part) { return !!part; });
          if (parts.length) successfulToolSummary = parts.join(' | ');
        }
      }
      if (hasToolError && !successfulToolSummary && !toolCompletionSummary) {
        return 'I could not finish the request because a required step failed. Please clarify the goal or try again.';
      }
