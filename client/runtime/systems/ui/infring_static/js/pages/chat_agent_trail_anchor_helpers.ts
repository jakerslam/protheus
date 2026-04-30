function infringChatAgentTrailAnchorMethods() {
  return {
    anchorAgentTrailToThinking(host, hostRect, now, pad, w, h) {
      if (!host || typeof host.querySelectorAll !== 'function') return false;
      var self = this;
      var pinToLastThinkingAnchor = function() {
        var s = self._agentTrailState || null;
        if (!self.freshInitLaunching || !s || String(s.anchorMode || '') !== 'thinking') return false;
        var x = Number(s.anchorTargetX);
        var y = Number(s.anchorTargetY);
        if (!Number.isFinite(x) || !Number.isFinite(y)) {
          x = Number(s.x);
          y = Number(s.y);
        }
        if (!Number.isFinite(x) || !Number.isFinite(y)) return false;
        x = Math.max(pad + 1, Math.min(w - (pad + 1), x));
        y = Math.max(pad + 1, Math.min(h - (pad + 1), y));
        s.x = x; s.y = y; s.vx = 0; s.vy = 0; s.trailX = x; s.trailY = y; s.anchorLastAt = now;
        self._agentTrailState = s;
        self.ensureAgentTrailOrb(host, x, y);
        self.setAgentTrailBlinkState(true);
        host.style.setProperty('--chat-agent-grid-active', '1');
        host.style.setProperty('--chat-agent-grid-x', Math.round(x) + 'px');
        host.style.setProperty('--chat-agent-grid-y', Math.round(y) + 'px');
        return true;
      };
      var bubbles = host.querySelectorAll('.message.thinking .message-bubble.message-bubble-thinking');
      if (!bubbles || !bubbles.length) {
        if (pinToLastThinkingAnchor()) return true;
        if (!this._agentTrailListening) this.setAgentTrailBlinkState(false);
        return false;
      }
      var rect = hostRect && Number.isFinite(Number(hostRect.width || 0)) ? hostRect : host.getBoundingClientRect();
      var anchor = null;
      for (var i = bubbles.length - 1; i >= 0; i--) {
        var bubble = bubbles[i];
        if (!bubble || bubble.offsetParent === null) continue;
        var bubbleRect = bubble.getBoundingClientRect();
        if (!(Number(bubbleRect.width || 0) > 0 && Number(bubbleRect.height || 0) > 0)) continue;
        if (bubbleRect.bottom < rect.top || bubbleRect.top > rect.bottom || bubbleRect.right < rect.left || bubbleRect.left > rect.right) continue;
        // Pin the autonomous agent orb outside the bottom-left edge of
        // the active thinking dialog while the agent is working.
        // Keep a 1.5rem diagonal offset so the orb stays closer while thinking.
        var remPx = 16;
        try {
          var root = document && document.documentElement
            ? window.getComputedStyle(document.documentElement)
            : null;
          var rootFont = root ? parseFloat(String(root.fontSize || '16')) : 16;
          if (Number.isFinite(rootFont) && rootFont > 0) remPx = rootFont;
        } catch (_) {}
        var orbOffset = remPx * 1.5;
        anchor = { x: (bubbleRect.left - rect.left) - orbOffset, y: (bubbleRect.bottom - rect.top) + orbOffset };
        break;
      }
      if (!anchor) {
        if (pinToLastThinkingAnchor()) return true;
        if (!this._agentTrailListening) this.setAgentTrailBlinkState(false);
        return false;
      }
      var targetX = Math.max(pad + 1, Math.min(w - (pad + 1), Number(anchor.x || 0)));
      var targetY = Math.max(pad + 1, Math.min(h - (pad + 1), Number(anchor.y || 0)));
      var s = this._agentTrailState;
      var enteredThinking = !s || String(s.anchorMode || '') !== 'thinking';
      var x = NaN;
      var y = NaN;
      if (s && Number.isFinite(Number(s.x)) && Number.isFinite(Number(s.y))) {
        x = Number(s.x);
        y = Number(s.y);
      } else if (this._agentTrailOrbEl && this._agentTrailOrbEl.isConnected && this._agentTrailOrbEl.parentNode === host) {
        x = Number(parseFloat(String(this._agentTrailOrbEl.style.left || 'NaN')));
        y = Number(parseFloat(String(this._agentTrailOrbEl.style.top || 'NaN')));
        if (!Number.isFinite(x)) x = Number(this._agentTrailOrbEl.offsetLeft || NaN);
        if (!Number.isFinite(y)) y = Number(this._agentTrailOrbEl.offsetTop || NaN);
      }
      if (!Number.isFinite(x) || !Number.isFinite(y)) {
        x = targetX;
        y = targetY;
      }
      if (!s) {
        s = { x: x, y: y, vx: 0, vy: 0, dir: 0, target: 0, turnAt: now + 1000 };
      }
      var lastAnchorAt = Number(s.anchorLastAt || 0);
      var dt = lastAnchorAt > 0 ? Math.min(0.08, Math.max(0.001, (now - lastAnchorAt) / 1000)) : (1 / 60);
      var dx = targetX - x;
      var dy = targetY - y;
      var dist = Math.sqrt((dx * dx) + (dy * dy));
      if (enteredThinking) dist = 0;
      if (dist > 0.001) {
        // Move in a straight line into the thinking anchor, never teleport.
        var maxStep = 1480 * dt;
        if (dist <= maxStep) {
          x = targetX;
          y = targetY;
        } else {
          x += (dx / dist) * maxStep;
          y += (dy / dist) * maxStep;
        }

      } else {
        x = targetX;
        y = targetY;
      }
      s.x = x;
      s.y = y;
      s.vx = 0;
      s.vy = 0;
      s.trailX = x;
      s.trailY = y;
      s.anchorMode = 'thinking';
      s.anchorTargetX = targetX;
      s.anchorTargetY = targetY;
      s.anchorLastAt = now;
      this._agentTrailState = s;
      this._agentTrailSeeded = true;
      this._agentTrailLastDotAt = now;
      if (enteredThinking && this._agentTrailOrbEl) {
        // Promote + mark listening before reposition so ensureAgentTrailOrb
        // performs the teleport path instead of easing from the last spot.
        this.setAgentTrailBlinkState(true, this._agentTrailOrbEl);
      }
      var orb = this.ensureAgentTrailOrb(host, x, y);
      this.setAgentTrailBlinkState(true, orb);
      host.style.setProperty('--chat-agent-grid-active', '1');
      host.style.setProperty('--chat-agent-grid-x', Math.round(x) + 'px');
      host.style.setProperty('--chat-agent-grid-y', Math.round(y) + 'px');
      this._agentTrailLastAt = now;
      return true;
    },
    anchorAgentTrailToFreshInit(host, hostRect, now, pad, w, h) {
      if (!host || typeof host.querySelector !== 'function') return false;
      if (!this.showFreshArchetypeTiles || !this.freshInitRevealMenu) return false;
      // Never override active thinking positioning during init.
      var activeThinking = host.querySelector('.message.thinking .message-bubble.message-bubble-thinking');
      if (activeThinking && activeThinking.offsetParent !== null) return false;
      var panel = host.querySelector('.chat-init-panel');
      if (!panel || panel.offsetParent === null) return false;
      var rect = hostRect && Number.isFinite(Number(hostRect.width || 0)) ? hostRect : host.getBoundingClientRect();
      var panelRect = panel.getBoundingClientRect();
      if (!(Number(panelRect.width || 0) > 0 && Number(panelRect.height || 0) > 0)) return false;
      if (panelRect.bottom < rect.top || panelRect.top > rect.bottom || panelRect.right < rect.left || panelRect.left > rect.right) return false;
      // During agent initialization, pin the orb to the initial agent chat panel.
      // Keep it 1rem outside the panel's bottom-left corner.
      var anchor = {
        x: (panelRect.left - rect.left) - 16,
        y: (panelRect.bottom - rect.top) + 16,
      };
      var x = Math.max(pad + 1, Math.min(w - (pad + 1), Number(anchor.x || 0)));
      var y = Math.max(pad + 1, Math.min(h - (pad + 1), Number(anchor.y || 0)));
      var orb = this.ensureAgentTrailOrb(host, x, y);
      this.setAgentTrailBlinkState(true, orb);
      host.style.setProperty('--chat-agent-grid-active', '1');
      host.style.setProperty('--chat-agent-grid-x', Math.round(x) + 'px');
      host.style.setProperty('--chat-agent-grid-y', Math.round(y) + 'px');
      this._agentTrailState = { x: x, y: y, vx: 0, vy: 0, dir: 0, target: 0, turnAt: now + 1000 };
      this._agentTrailSeeded = false;
      this._agentTrailLastAt = now;
      return true;
    },

  };
}
