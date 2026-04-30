// Chat source-chip and tool-trace presentation helpers.
'use strict';

function infringChatSourceTraceMethods() {
  return {
    _collectSourceCandidatesFromValue: function(value, out, seen, depth) {
      if (!value || !out || !seen) return;
      var nextDepth = Number(depth || 0);
      if (!Number.isFinite(nextDepth) || nextDepth < 0) nextDepth = 0;
      if (nextDepth > 4 || out.length >= 24) return;
      if (typeof value === 'string') {
        var text = String(value || '').trim();
        if (/^https?:\/\//i.test(text)) {
          if (!seen[text]) {
            seen[text] = true;
            out.push({ url: text, label: '', source: '' });
          }
        }
        return;
      }
      if (Array.isArray(value)) {
        for (var ai = 0; ai < value.length && out.length < 24; ai += 1) {
          this._collectSourceCandidatesFromValue(value[ai], out, seen, nextDepth + 1);
        }
        return;
      }
      if (typeof value !== 'object') return;
      var url = String(
        value.url ||
        value.href ||
        value.link ||
        value.source_url ||
        value.final_url ||
        value.resolved_url ||
        ''
      ).trim();
      if (url && /^https?:\/\//i.test(url) && !seen[url]) {
        seen[url] = true;
        out.push({
          url: url,
          label: String(value.title || value.name || value.label || '').trim(),
          source: String(value.source || value.provider || value.domain || '').trim()
        });
      }
      var keys = Object.keys(value);
      for (var ki = 0; ki < keys.length && out.length < 24; ki += 1) {
        var key = keys[ki];
        if (!Object.prototype.hasOwnProperty.call(value, key)) continue;
        if (key === 'url' || key === 'href' || key === 'link' || key === 'source_url' || key === 'final_url' || key === 'resolved_url') continue;
        if (key === 'content' || key === 'result' || key === 'output' || key === 'payload' || key === 'data') {
          this._collectSourceCandidatesFromValue(value[key], out, seen, nextDepth + 1);
          continue;
        }
        if (nextDepth <= 2 && typeof value[key] === 'object') {
          this._collectSourceCandidatesFromValue(value[key], out, seen, nextDepth + 1);
        }
      }
    },

    _normalizeMessageSourceChip: function(row, idx) {
      var entry = row && typeof row === 'object' ? row : {};
      var url = String(entry.url || entry.href || entry.link || '').trim();
      if (!url || !/^https?:\/\//i.test(url)) return null;
      var label = String(entry.label || entry.title || entry.name || '').trim();
      var host = '';
      try {
        host = new URL(url).hostname.replace(/^www\./i, '');
      } catch (_) {
        host = '';
      }
      var source = String(entry.source || '').trim();
      if (!label) label = source || host || ('Source ' + (Number(idx || 0) + 1));
      if (label.length > 64) label = label.slice(0, 61).trim() + '...';
      return {
        id: 'src-' + (idx + 1) + '-' + label.toLowerCase().replace(/[^a-z0-9]+/g, '-'),
        label: label,
        host: host,
        source: source,
        url: url
      };
    },

    assistantTurnMetadataFromPayload: function(payload, tools) {
      var data = payload && typeof payload === 'object' ? payload : {};
      var out = {};
      if (data.response_workflow && typeof data.response_workflow === 'object') out.response_workflow = data.response_workflow;
      var finalization = typeof this.responseFinalizationFromPayload === 'function'
        ? this.responseFinalizationFromPayload(data)
        : (data.response_finalization && typeof data.response_finalization === 'object' ? data.response_finalization : null);
      if (finalization) out.response_finalization = finalization;
      if (data.turn_transaction && typeof data.turn_transaction === 'object') out.turn_transaction = data.turn_transaction;
      if (Array.isArray(data.terminal_transcript) && data.terminal_transcript.length) out.terminal_transcript = data.terminal_transcript.slice(0, 48);
      if (data.attention_queue && typeof data.attention_queue === 'object') out.attention_queue = data.attention_queue;
      if (Array.isArray(data.sources) && data.sources.length) out.sources = data.sources.slice(0, 16);
      if (Array.isArray(data.citations) && data.citations.length) out.citations = data.citations.slice(0, 24);
      if (Array.isArray(data.reference_links) && data.reference_links.length) out.reference_links = data.reference_links.slice(0, 24);
      var failureSummary = typeof this.readableToolFailureSummary === 'function'
        ? this.readableToolFailureSummary(data, tools)
        : '';
      if (failureSummary) out.tool_failure_summary = failureSummary;
      return out;
    },

    messageSourceChips: function(msg) {
      var row = msg && typeof msg === 'object' ? msg : {};
      var signature = [
        String(row.id || ''),
        String(row.text || '').length,
        Array.isArray(row.tools) ? row.tools.length : 0,
        row.response_workflow ? 'wf1' : 'wf0',
        row.response_finalization ? 'rf1' : 'rf0',
        row.turn_transaction ? 'tx1' : 'tx0'
      ].join('|');
      if (row._source_chip_signature === signature && Array.isArray(row._source_chips_cached)) {
        return row._source_chips_cached;
      }
      var candidates = [];
      var seenUrls = {};
      this._collectSourceCandidatesFromValue(row.sources, candidates, seenUrls, 0);
      this._collectSourceCandidatesFromValue(row.citations, candidates, seenUrls, 0);
      this._collectSourceCandidatesFromValue(row.response_workflow && row.response_workflow.citations, candidates, seenUrls, 0);
      this._collectSourceCandidatesFromValue(row.response_workflow && row.response_workflow.sources, candidates, seenUrls, 0);
      this._collectSourceCandidatesFromValue(row.response_finalization && row.response_finalization.citations, candidates, seenUrls, 0);
      this._collectSourceCandidatesFromValue(row.response_finalization && row.response_finalization.sources, candidates, seenUrls, 0);
      this._collectSourceCandidatesFromValue(row.turn_transaction && row.turn_transaction.citations, candidates, seenUrls, 0);
      this._collectSourceCandidatesFromValue(row.turn_transaction && row.turn_transaction.evidence, candidates, seenUrls, 0);
      if (Array.isArray(row.tools)) {
        for (var i = 0; i < row.tools.length && candidates.length < 24; i += 1) {
          var tool = row.tools[i] || {};
          this._collectSourceCandidatesFromValue(tool.citations || tool.sources || tool.source_refs || tool.evidence || null, candidates, seenUrls, 0);
          if (Array.isArray(tool._imageUrls)) {
            for (var ui = 0; ui < tool._imageUrls.length && candidates.length < 24; ui += 1) {
              this._collectSourceCandidatesFromValue(tool._imageUrls[ui], candidates, seenUrls, 0);
            }
          }
        }
      }
      var chips = [];
      for (var ci = 0; ci < candidates.length && chips.length < 8; ci += 1) {
        var normalized = this._normalizeMessageSourceChip(candidates[ci], ci);
        if (!normalized) continue;
        chips.push(normalized);
      }
      row._source_chip_signature = signature;
      row._source_chips_cached = chips;
      return chips;
    },

    messageHasSourceChips: function(msg) {
      return this.messageSourceChips(msg).length > 0;
    },

    messageToolTraceSummary: function(msg) {
      var rows = this.resolveMessageToolRows(msg);
      var summary = {
        visible: false,
        running: false,
        total: 0,
        done: 0,
        blocked: 0,
        errored: 0,
        label: '',
        detail: ''
      };
      if (!rows.length) return summary;
      summary.visible = true;
      summary.total = rows.length;
      for (var i = 0; i < rows.length; i += 1) {
        var tool = rows[i];
        if (!tool) continue;
        if (tool.running) {
          summary.running = true;
          continue;
        }
        if (this.isBlockedTool(tool)) {
          summary.blocked += 1;
          continue;
        }
        if (tool.is_error) {
          summary.errored += 1;
          continue;
        }
        summary.done += 1;
      }
      summary.label = summary.running ? 'Tool trace running' : 'Tool trace complete';
      var bits = [];
      if (summary.done > 0) bits.push(summary.done + ' done');
      if (summary.errored > 0) bits.push(summary.errored + ' error');
      if (summary.blocked > 0) bits.push(summary.blocked + ' blocked');
      if (summary.running) bits.push((summary.total - (summary.done + summary.errored + summary.blocked)) + ' in progress');
      if (!bits.length) bits.push(summary.total + ' recorded');
      summary.detail = bits.join(' · ');
      return summary;
    },

    messageToolTraceRows: function(msg) {
      var rows = this.resolveMessageToolRows(msg);
      var out = [];
      for (var i = 0; i < rows.length && out.length < 6; i += 1) {
        var tool = rows[i] || {};
        var label = this.toolDisplayName(tool);
        var state = tool.running
          ? 'running'
          : (this.isBlockedTool(tool) ? 'blocked' : (tool.is_error ? 'error' : 'done'));
        out.push({
          id: String(tool.id || tool.attempt_id || (label + '-' + i)).trim(),
          label: label,
          state: state,
          detail: String(tool.status || '').trim()
        });
      }
      return out;
    },
  };
}
