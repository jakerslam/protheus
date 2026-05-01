'use strict';

function infringChatContextTelemetryMethods() {
  return {
    estimateTokenCountFromText(text) {
      return chatEstimateTokenCountFromText(text);
    },

    // Backward-compat shim for legacy callers during naming migration.
    estimateTokensFromText(text) {
      return this.estimateTokenCountFromText(text);
    },

    recomputeContextEstimate() {
      var rows = Array.isArray(this.messages) ? this.messages : [];
      var total = 0;
      for (var i = 0; i < rows.length; i++) {
        total += this.estimateTokenCountFromText(rows[i] && rows[i].text ? rows[i].text : '');
      }
      this.contextApproxTokens = total;
      this.refreshContextPressure();
    },

    applyContextTelemetry(data) {
      if (!data || typeof data !== 'object') return;
      var payloadAgentId = String(data.agent_id || '').trim();
      var selectedAgentId = String(this.currentAgent && this.currentAgent.id ? this.currentAgent.id : '').trim();
      if (payloadAgentId && selectedAgentId && payloadAgentId !== selectedAgentId) {
        return;
      }
      var pool = data.context_pool && typeof data.context_pool === 'object' ? data.context_pool : null;
      var hasApproxField =
        Object.prototype.hasOwnProperty.call(data, 'context_tokens') ||
        Object.prototype.hasOwnProperty.call(data, 'context_used_tokens') ||
        Object.prototype.hasOwnProperty.call(data, 'context_total_tokens') ||
        (pool && Object.prototype.hasOwnProperty.call(pool, 'active_tokens')) ||
        (pool && Object.prototype.hasOwnProperty.call(pool, 'pool_tokens'));
      var approx = Number(
        data.context_tokens != null ? data.context_tokens :
        (data.context_used_tokens != null ? data.context_used_tokens :
        (data.context_total_tokens != null ? data.context_total_tokens :
        (pool && pool.active_tokens != null ? pool.active_tokens :
        (pool && pool.pool_tokens != null ? pool.pool_tokens : 0))))
      );
      if (hasApproxField && Number.isFinite(approx) && approx >= 0) {
        this.contextApproxTokens = Math.max(0, Math.round(approx));
      } else if (typeof data.message === 'string') {
        var tokenMatch = data.message.match(/~?\s*([0-9,]+)\s+tokens/i);
        if (tokenMatch && tokenMatch[1]) {
          var parsed = Number(String(tokenMatch[1]).replace(/,/g, ''));
          if (Number.isFinite(parsed) && parsed > 0) this.contextApproxTokens = parsed;
        }
      }
      var windowSize = Number(
        data.context_window != null ? data.context_window :
        (data.context_window_tokens != null ? data.context_window_tokens :
        (pool && pool.context_window != null ? pool.context_window : 0))
      );
      if (Number.isFinite(windowSize) && windowSize > 0) {
        this.contextWindow = windowSize;
      }
      var ratio = Number(
        data.context_ratio != null ? data.context_ratio :
        (pool && pool.context_ratio != null ? pool.context_ratio : 0)
      );
      if ((!Number.isFinite(approx) || approx <= 0) && Number.isFinite(ratio) && ratio > 0 && this.contextWindow > 0) {
        this.contextApproxTokens = Math.round(this.contextWindow * ratio);
      }
      var pressure = String(
        data.context_pressure != null ? data.context_pressure :
        (pool && pool.context_pressure != null ? pool.context_pressure : '')
      ).trim();
      if (pressure) {
        this.contextPressure = pressure;
      } else {
        this.refreshContextPressure();
      }
    },
  };
}

function chatContextUsagePercent(vm) {
  var windowSize = Number(vm.contextWindow || 0);
  var used = Number(vm.contextApproxTokens || 0);
  if (windowSize > 0 && used >= 0) {
    var ratio = Math.round((used / windowSize) * 100);
    if (ratio < 0) return 0;
    if (ratio > 95) return 95;
    return ratio;
  }
  switch (vm.contextPressure) {
    case 'critical': return 95;
    case 'high': return 80;
    case 'medium': return 55;
    default: return 25;
  }
}

function chatContextRingArcLength(vm) {
  // 330deg sweep: starts at 1 o'clock and ends at 12 o'clock at 100%.
  var maxArc = 91.6667;
  var usage = chatContextUsagePercent(vm);
  if (!Number.isFinite(usage) || usage <= 0) return 0;
  if (usage >= 100) return maxArc;
  return Number(((usage / 100) * maxArc).toFixed(3));
}

function chatContextRingProgressStyle(vm) {
  return 'stroke-dasharray: ' + chatContextRingArcLength(vm) + ' 100; stroke-dashoffset: 0;';
}

function chatContextRingTooltip(vm) {
  return 'Context window\n' +
    chatContextUsagePercent(vm) + '% full\n' +
    ' ' + vm.formatTokenThousands(vm.contextApproxTokens) + ' / ' + vm.formatTokenThousands(vm.contextWindow) + ' tokens used\n\n' +
    ' Infring dynamically prunes its context';
}

function chatContextRingCompactLabel(vm) {
  return 'Context: ' + chatContextUsagePercent(vm) + '%, ' +
    vm.formatTokenThousands(vm.contextApproxTokens) + '/' + vm.formatTokenThousands(vm.contextWindow);
}
