function infringChatMemprobeMethods() {
  return {
    runSlashMemprobe: function(cmdArgs) {
      var report = this.collectMemprobeReport(cmdArgs);
      try {
        var label = '[memprobe ' + report.captured_at_iso + ']';
        if (typeof console !== 'undefined' && console.group) {
          console.group(label);
          console.table(report.heap);
          console.table(report.dom_counts);
          console.table(report.custom_element_counts);
          console.table(report.storage_bytes);
          console.table(report.suspected_accumulators);
          console.log('full_report:', report);
          if (report.delta) console.log('delta_vs_previous:', report.delta);
          console.groupEnd();
        } else {
          console.log(label, report);
        }
      } catch (_) {}
      var heap = report.heap || {};
      var heapMb = (Number(heap.used_js_heap_mb) || 0).toFixed(1);
      var heapTotalMb = (Number(heap.total_js_heap_mb) || 0).toFixed(1);
      var domNodes = (report.dom_counts && report.dom_counts.total_nodes) || 0;
      var storageBytes = (report.storage_bytes && report.storage_bytes.total_storage_bytes) || 0;
      var bubbles = (report.custom_element_counts && report.custom_element_counts['infring-chat-bubble-render']) || 0;
      var placeholders = (report.custom_element_counts && report.custom_element_counts['infring-message-placeholder-shell']) || 0;
      var messageCount = Array.isArray(this.messages) ? this.messages.length : 0;
      var lines = [
        '**memprobe ' + report.capture_index + '**',
        '- heap_used: ' + heapMb + ' MB / total: ' + heapTotalMb + ' MB' + (heap.heap_unsupported ? ' (performance.memory unavailable)' : ''),
        '- dom_nodes: ' + domNodes,
        '- storage_bytes: ' + storageBytes,
        '- chat_bubble_render instances: ' + bubbles,
        '- message_placeholder_shell instances: ' + placeholders,
        '- messages: ' + messageCount,
      ];
      if (report.delta) {
        var d = report.delta;
        lines.push('- delta vs prev: heap ' + (d.used_js_heap_mb >= 0 ? '+' : '') + d.used_js_heap_mb.toFixed(1) + ' MB, dom ' + (d.total_nodes >= 0 ? '+' : '') + d.total_nodes + ' nodes, storage ' + (d.total_storage_bytes >= 0 ? '+' : '') + d.total_storage_bytes + ' bytes, bubbles ' + (d.bubble_count >= 0 ? '+' : '') + d.bubble_count + ', elapsed ' + d.elapsed_ms + ' ms');
      } else {
        lines.push('- (run /memprobe again after 30s to see delta)');
      }
      lines.push('- full report logged to DevTools console');
      try {
        console.log('[memprobe summary]', lines.join('\n'));
      } catch (_) {}
      InfringToast.info('memprobe captured; full report is in DevTools console.');
    },

    collectMemprobeReport: function(cmdArgs) {
      var now = Date.now();
      var capturedAt = new Date(now);
      var prev = this._lastMemprobeReport && typeof this._lastMemprobeReport === 'object'
        ? this._lastMemprobeReport
        : null;
      var perfMem = null;
      try {
        if (typeof performance !== 'undefined' && performance && performance.memory) {
          perfMem = performance.memory;
        }
      } catch (_) { perfMem = null; }
      var bytesToMb = function(n) { return Math.round((Number(n) || 0) / 1024 / 1024 * 100) / 100; };
      var heap = perfMem
        ? {
            heap_unsupported: false,
            used_js_heap_mb: bytesToMb(perfMem.usedJSHeapSize),
            total_js_heap_mb: bytesToMb(perfMem.totalJSHeapSize),
            jsheap_size_limit_mb: bytesToMb(perfMem.jsHeapSizeLimit),
            used_js_heap_bytes: Number(perfMem.usedJSHeapSize) || 0,
            total_js_heap_bytes: Number(perfMem.totalJSHeapSize) || 0,
          }
        : { heap_unsupported: true };
      var domCounts = { total_nodes: 0, scripts: 0, styles: 0, divs: 0 };
      try {
        domCounts.total_nodes = document.querySelectorAll('*').length;
        domCounts.scripts = document.querySelectorAll('script').length;
        domCounts.styles = document.querySelectorAll('style,link[rel="stylesheet"]').length;
        domCounts.divs = document.querySelectorAll('div').length;
      } catch (_) {}
      var customElementTags = [
        'infring-chat-bubble-render',
        'infring-message-placeholder-shell',
        'infring-message-context-shell',
        'infring-message-meta-shell',
        'infring-message-artifact-shell',
        'infring-message-media-shell',
        'infring-message-progress-shell',
        'infring-message-terminal-shell',
        'infring-chat-divider-shell',
        'infring-chat-thread-shell',
        'infring-chat-stream-shell',
        'infring-messages-surface-shell',
        'infring-chat-map-shell',
      ];
      var customElementCounts = {};
      try {
        for (var i = 0; i < customElementTags.length; i++) {
          var tag = customElementTags[i];
          customElementCounts[tag] = document.querySelectorAll(tag).length;
        }
      } catch (_) {}
      var jsonByteSize = function(value) {
        try {
          if (value == null) return 0;
          return JSON.stringify(value).length;
        } catch (_) { return -1; }
      };
      var storageByteSize = function(storage) {
        try {
          if (!storage || typeof storage.length !== 'number') return 0;
          var total = 0;
          for (var s = 0; s < storage.length; s++) {
            var key = String(storage.key(s) || '');
            total += key.length + String(storage.getItem(key) || '').length;
          }
          return total;
        } catch (_) { return -1; }
      };
      var localStorageBytes = typeof localStorage !== 'undefined' ? storageByteSize(localStorage) : 0;
      var sessionStorageBytes = typeof sessionStorage !== 'undefined' ? storageByteSize(sessionStorage) : 0;
      var storageBytesReport = {
        local_storage_bytes: localStorageBytes,
        session_storage_bytes: sessionStorageBytes,
        total_storage_bytes: Math.max(0, localStorageBytes) + Math.max(0, sessionStorageBytes),
      };
      var messages = Array.isArray(this.messages) ? this.messages : [];
      var totalMessageTextBytes = 0;
      var totalMessageStreamBufferBytes = 0;
      var maxMessageStreamBufferBytes = 0;
      try {
        for (var m = 0; m < messages.length; m++) {
          var msg = messages[m];
          if (!msg || typeof msg !== 'object') continue;
          totalMessageTextBytes += String(msg.text || '').length;
          var streamBuf = String(msg._streamRawText || '').length
            + String(msg._cleanText || '').length
            + String(msg._thoughtText || '').length
            + String(msg._typewriterFinalText || '').length
            + String(msg._typingVisualHtml || '').length;
          totalMessageStreamBufferBytes += streamBuf;
          if (streamBuf > maxMessageStreamBufferBytes) maxMessageStreamBufferBytes = streamBuf;
        }
      } catch (_) {}
      var suspectedAccumulators = {
        message_count: messages.length,
        message_text_total_bytes: totalMessageTextBytes,
        message_text_total_kb: Math.round(totalMessageTextBytes / 1024),
        message_stream_buffer_total_bytes: totalMessageStreamBufferBytes,
        message_stream_buffer_total_kb: Math.round(totalMessageStreamBufferBytes / 1024),
        message_stream_buffer_max_bytes: maxMessageStreamBufferBytes,
        telemetry_snapshot_bytes: jsonByteSize(this._telemetrySnapshot),
        continuity_snapshot_bytes: jsonByteSize(this._continuitySnapshot),
        message_hydration_keys: this.messageHydration && typeof this.messageHydration === 'object'
          ? Object.keys(this.messageHydration).length
          : 0,
        forced_hydrate_keys: this._forcedHydrateById && typeof this._forcedHydrateById === 'object'
          ? Object.keys(this._forcedHydrateById).length
          : 0,
        message_line_expand_state_keys: this.messageLineExpandState && typeof this.messageLineExpandState === 'object'
          ? Object.keys(this.messageLineExpandState).length
          : 0,
        sessions_last_loaded_keys: this._sessionsLastLoadedAtByAgent && typeof this._sessionsLastLoadedAtByAgent === 'object'
          ? Object.keys(this._sessionsLastLoadedAtByAgent).length
          : 0,
      };
      var captureIndex = Number((this._memprobeCaptureCount || 0) + 1) || 1;
      this._memprobeCaptureCount = captureIndex;
      var report = {
        type: 'chat_memprobe_report',
        capture_index: captureIndex,
        captured_at_ms: now,
        captured_at_iso: capturedAt.toISOString(),
        args: String(cmdArgs == null ? '' : cmdArgs),
        heap: heap,
        dom_counts: domCounts,
        custom_element_counts: customElementCounts,
        storage_bytes: storageBytesReport,
        suspected_accumulators: suspectedAccumulators,
        page_visible: typeof document !== 'undefined' && document && document.visibilityState ? document.visibilityState : 'unknown',
      };
      if (prev && prev.captured_at_ms) {
        var prevHeapMb = Number((prev.heap && prev.heap.used_js_heap_mb) || 0);
        var nextHeapMb = Number(heap.used_js_heap_mb || 0);
        var prevNodes = Number((prev.dom_counts && prev.dom_counts.total_nodes) || 0);
        var nextNodes = Number(domCounts.total_nodes || 0);
        var prevStorage = Number((prev.storage_bytes && prev.storage_bytes.total_storage_bytes) || 0);
        var nextStorage = Number(storageBytesReport.total_storage_bytes || 0);
        var prevBubbles = Number((prev.custom_element_counts && prev.custom_element_counts['infring-chat-bubble-render']) || 0);
        var nextBubbles = Number(customElementCounts['infring-chat-bubble-render'] || 0);
        report.delta = {
          elapsed_ms: now - Number(prev.captured_at_ms),
          used_js_heap_mb: Math.round((nextHeapMb - prevHeapMb) * 100) / 100,
          total_nodes: nextNodes - prevNodes,
          total_storage_bytes: nextStorage - prevStorage,
          bubble_count: nextBubbles - prevBubbles,
          message_count: messages.length - Number(prev.suspected_accumulators && prev.suspected_accumulators.message_count || 0),
        };
      }
      this._lastMemprobeReport = report;
      try { if (typeof window !== 'undefined') window.__infringMemprobe = report; } catch (_) {}
      return report;
    },

  };
}
