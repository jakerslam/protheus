'use strict';

function infringChatAgentTrailMethods() {
  return {
    currentFairyOwnerId() {
      if (!this.currentAgent) return '';
      if (this.isSystemThreadAgent && this.isSystemThreadAgent(this.currentAgent)) return '';
      var value = String(this.currentAgent.id || '').trim();
      return value || '';
    },
    resolveFairyHost(container) {
      var host = this.resolveMessagesScroller(container || null);
      var scope = this.$el || null;
      if (host && scope && scope.contains(host)) return host;
      var scopedRef = this.$refs && this.$refs.messagesEl ? this.$refs.messagesEl : null;
      if (scopedRef && scope && scope.contains(scopedRef) && scopedRef.offsetParent !== null) return scopedRef;
      return null;
    },
    wireAgentTrailOrbBehavior(orb) {
      if (!orb) return null;
      var self = this;
      if (typeof orb.toggleIndex !== 'function') {
        orb.toggleIndex = function(forceTop) {
          return self.toggleAgentTrailOrbIndex(forceTop);
        };
      }
      this.applyAgentTrailOrbIndexState(orb);
      return orb;
    },
    resolveAgentTrailOverlay(orbRef) {
      var orb = orbRef || this._agentTrailOrbEl || null;
      if (!orb) return null;
      var layer = orb.parentElement || null;
      if (!layer || !layer.classList || !layer.classList.contains('chat-agent-overlay')) return null;
      return layer;
    },
    applyAgentTrailOrbIndexState(orbRef) {
      var orb = orbRef || this._agentTrailOrbEl || null;
      if (!orb || !orb.style) return;
      var top = !!this._agentTrailOrbElevated;
      if (top) {
        orb.style.zIndex = '2147483000';
        if (orb.classList) orb.classList.add('fairy-z-top');
      } else {
        orb.style.zIndex = '';
        if (orb.classList) orb.classList.remove('fairy-z-top');
      }
      var layer = this.resolveAgentTrailOverlay(orb);
      if (!layer) return;
      if (top) {
        layer.style.zIndex = '2147482999';
        layer.classList.add('fairy-z-top');
      } else {
        layer.style.zIndex = '';
        layer.classList.remove('fairy-z-top');
      }
    },
    toggleAgentTrailOrbIndex(forceTop) {
      var orb = this._agentTrailOrbEl;
      if (!orb || !orb.style) return false;
      var nextTop = false;
      if (forceTop === true) nextTop = true;
      else if (forceTop === false) nextTop = false;
      else nextTop = !this._agentTrailOrbElevated;
      this._agentTrailOrbElevated = !!nextTop;
      this.applyAgentTrailOrbIndexState(orb);
      return this._agentTrailOrbElevated;
    },
    teleportAgentTrailOrb(orbRef, x, y, toggleIndex, onMidpoint) {
      var orb = this.wireAgentTrailOrbBehavior(orbRef || this._agentTrailOrbEl);
      if (!orb || !orb.style) return;
      var shouldToggleIndex = toggleIndex !== false;
      var targetX = Number(x);
      var targetY = Number(y);
      var pendingX = Number(this._agentTrailTeleportTargetX);
      var pendingY = Number(this._agentTrailTeleportTargetY);
      var pendingToggle = this._agentTrailTeleportToggleIndex !== false;
      var hasPendingTeleport = !!this._agentTrailTeleportTimer;
      var samePendingTeleport = hasPendingTeleport &&
        Number.isFinite(targetX) &&
        Number.isFinite(targetY) &&
        Number.isFinite(pendingX) &&
        Number.isFinite(pendingY) &&
        Math.abs(targetX - pendingX) <= 0.5 &&
        Math.abs(targetY - pendingY) <= 0.5 &&
        pendingToggle === shouldToggleIndex;
      if (samePendingTeleport) return;
      if (hasPendingTeleport) {
        clearTimeout(this._agentTrailTeleportTimer);
        this._agentTrailTeleportTimer = 0;
        orb.style.opacity = '';
        orb.style.transition = '';
      }
      this._agentTrailTeleportTargetX = Number.isFinite(targetX) ? targetX : NaN;
      this._agentTrailTeleportTargetY = Number.isFinite(targetY) ? targetY : NaN;
      this._agentTrailTeleportToggleIndex = shouldToggleIndex;
      var self = this;
      orb.style.transition = 'opacity 95ms ease';
      orb.style.opacity = '0';
      this._agentTrailTeleportTimer = setTimeout(function() {
        self._agentTrailTeleportTimer = 0;
        self._agentTrailTeleportTargetX = NaN;
        self._agentTrailTeleportTargetY = NaN;
        self._agentTrailTeleportToggleIndex = true;
        if (!self._agentTrailOrbEl || self._agentTrailOrbEl !== orb) return;
        if (shouldToggleIndex && typeof orb.toggleIndex === 'function') orb.toggleIndex();
        if (Number.isFinite(targetX)) orb.style.left = targetX + 'px';
        if (Number.isFinite(targetY)) orb.style.top = targetY + 'px';
        if (typeof onMidpoint === 'function') {
          try { onMidpoint(); } catch(_) {}
        }
        requestAnimationFrame(function() {
          if (!self._agentTrailOrbEl || self._agentTrailOrbEl !== orb) return;
          orb.style.opacity = '';
          orb.style.transition = '';
        });
      }, 95);
    },
    setAgentTrailBlinkState(active, orbRef) {
      var orb = this.wireAgentTrailOrbBehavior(orbRef || this._agentTrailOrbEl);
      if (!orb || !orb.classList) return;
      if (active) {
        if (typeof orb.toggleIndex === 'function') orb.toggleIndex(true);
        orb.classList.add('agent-listening');
        return;
      }
      orb.classList.remove('agent-listening');
      if (typeof orb.toggleIndex === 'function') orb.toggleIndex(false);
    },
    ensureAgentTrailOrb(container, x, y) {
      var ownerId = this.currentFairyOwnerId();
      if (!ownerId) {
        this.removeAgentTrailOrb();
        return null;
      }
      var host = this.resolveFairyHost(container || null);
      var layer = this.resolveAgentFxLayer(host || container);
      if (!layer) return null;
      var orb = this._agentTrailOrbEl;
      if (!orb || !orb.isConnected || orb.parentNode !== layer) {
        if (orb) try { orb.remove(); } catch(_) {}
        orb = document.createElement('span');
        orb.className = 'chat-pointer-orb chat-pointer-agent';
        layer.appendChild(orb);
        this._agentTrailOrbEl = orb;
      }
      this.wireAgentTrailOrbBehavior(orb);
      if (ownerId && orb.dataset) {
        orb.dataset.fairyOwner = ownerId;
        this._agentFairyOwnerId = ownerId;
      }
      var currentX = Number(parseFloat(String(orb.style.left || 'NaN')));
      var currentY = Number(parseFloat(String(orb.style.top || 'NaN')));
      if (!Number.isFinite(currentX)) currentX = Number(orb.offsetLeft || NaN);
      if (!Number.isFinite(currentY)) currentY = Number(orb.offsetTop || NaN);
      var dx = Number.isFinite(currentX) ? Math.abs(Number(x) - currentX) : 0;
      var dy = Number.isFinite(currentY) ? Math.abs(Number(y) - currentY) : 0;
      var jumpDistance = Math.sqrt((dx * dx) + (dy * dy));
      if (orb.classList && orb.classList.contains('agent-listening') && jumpDistance >= 72) {
        this.teleportAgentTrailOrb(orb, x, y, !this._agentTrailOrbElevated);
      } else {
        orb.style.left = x + 'px';
        orb.style.top = y + 'px';
      }
      return orb;
    },
    pruneAgentTrailFx(container) {
      var host = this.resolveFairyHost(container || this._agentTrailHost || null);
      if (!host || typeof host.querySelectorAll !== 'function') return;
      var ownerId = this.currentFairyOwnerId();
      if (!ownerId) {
        var staleNodes = host.querySelectorAll('.chat-pointer-agent');
        for (var sn = 0; sn < staleNodes.length; sn++) {
          try { staleNodes[sn].remove(); } catch(_) {}
        }
        this.removeAgentTrailOrb();
        host.style.setProperty('--chat-agent-grid-active', '0');
        return;
      }
      var orbNodes = host.querySelectorAll('.chat-pointer-orb.chat-pointer-agent');
      var keepOrb = this._agentTrailOrbEl && this._agentTrailOrbEl.isConnected
        ? this._agentTrailOrbEl
        : null;
      for (var i = 0; i < orbNodes.length; i++) {
        var node = orbNodes[i];
        var nodeOwner = node && node.dataset && node.dataset.fairyOwner ? String(node.dataset.fairyOwner).trim() : '';
        if (ownerId && nodeOwner && nodeOwner !== ownerId) {
          try { node.remove(); } catch(_) {}
          continue;
        }
        if (ownerId && !nodeOwner && node && node.dataset) node.dataset.fairyOwner = ownerId;
        if (!keepOrb) {
          keepOrb = node;
          this._agentTrailOrbEl = node;
          this.wireAgentTrailOrbBehavior(node);
          continue;
        }
        if (keepOrb === node) continue;
        try { node.remove(); } catch(_) {}
      }
      var trailNodes = host.querySelectorAll('.chat-pointer-trail-dot.chat-pointer-agent, .chat-pointer-trail-segment.chat-pointer-agent');
      var ownedTrailNodes = [];
      for (var ti = 0; ti < trailNodes.length; ti++) {
        var trailNode = trailNodes[ti];
        var trailOwner = trailNode && trailNode.dataset && trailNode.dataset.fairyOwner ? String(trailNode.dataset.fairyOwner).trim() : '';
        if (ownerId && trailOwner && trailOwner !== ownerId) {
          try { trailNode.remove(); } catch(_) {}
          continue;
        }
        if (ownerId && !trailOwner && trailNode && trailNode.dataset) trailNode.dataset.fairyOwner = ownerId;
        ownedTrailNodes.push(trailNode);
      }
      var maxTrailNodes = 220;
      var extra = Number(ownedTrailNodes.length || 0) - maxTrailNodes;
      if (extra > 0) {
        for (var j = 0; j < extra; j++) {
          try { ownedTrailNodes[j].remove(); } catch(_) {}
        }
      }
    },
    removeAgentTrailOrb() {
      var orb = this._agentTrailOrbEl;
      if (this._agentTrailTeleportTimer) {
        clearTimeout(this._agentTrailTeleportTimer);
        this._agentTrailTeleportTimer = 0;
      }
      this._agentTrailTeleportTargetX = NaN;
      this._agentTrailTeleportTargetY = NaN;
      this._agentTrailTeleportToggleIndex = true;
      if (!orb) return;
      var layer = this.resolveAgentTrailOverlay(orb);
      if (layer) {
        layer.style.zIndex = '';
        layer.classList.remove('fairy-z-top');
      }
      try { orb.remove(); } catch(_) {}
      this._agentTrailOrbEl = null;
      this._agentFairyOwnerId = '';
      this._agentTrailOrbElevated = false;
    },
    clearAgentTrailFx(container) {
      var host = this.resolveFairyHost(container || this._agentTrailHost || null);

      if (!host) return;
      var nodes = host.querySelectorAll('.chat-pointer-agent');
      for (var i = 0; i < nodes.length; i++) {
        try { nodes[i].remove(); } catch(_) {}
      }
      this.removeAgentTrailOrb();
      host.style.setProperty('--chat-agent-grid-active', '0');
    },
    dedupeAgentTrailFx(activeContainer) {
      var activeHost = this.resolveFairyHost(activeContainer || this._agentTrailHost || null);
      var scope = this.$el || document;
      var ownerId = this.currentFairyOwnerId();
      if (!ownerId) {
        var stale = scope.querySelectorAll('.chat-pointer-agent, .chat-agent-overlay');
        for (var si = 0; si < stale.length; si++) {
          try { stale[si].remove(); } catch(_) {}
        }
        this._agentTrailOrbEl = null;
        this._agentFairyOwnerId = '';
        this._agentTrailOrbElevated = false;
        var staleHosts = scope.querySelectorAll('#messages');
        for (var shi = 0; shi < staleHosts.length; shi++) {
          try { staleHosts[shi].style.setProperty('--chat-agent-grid-active', '0'); } catch(_) {}
        }
        return;
      }
      var overlays = scope.querySelectorAll('.chat-agent-overlay');
      var keptActiveOverlay = null;
      for (var oi = 0; oi < overlays.length; oi++) {
        var overlay = overlays[oi];
        if (!overlay) continue;
        var owner = overlay.closest ? overlay.closest('#messages') : null;
        if (!activeHost || owner !== activeHost) {
          try { overlay.remove(); } catch(_) {}
          continue;
        }
        if (!keptActiveOverlay) {
          keptActiveOverlay = overlay;
          continue;
        }
        try { overlay.remove(); } catch(_) {}
      }
      var agentNodes = scope.querySelectorAll('.chat-pointer-agent');
      for (var ni = 0; ni < agentNodes.length; ni++) {
        var node = agentNodes[ni];
        if (!activeHost || !activeHost.contains(node)) {
          try { node.remove(); } catch(_) {}
          continue;
        }
        var nodeOwner = node && node.dataset && node.dataset.fairyOwner ? String(node.dataset.fairyOwner).trim() : '';
        if (ownerId && nodeOwner && nodeOwner !== ownerId) {
          try { node.remove(); } catch(_) {}
          continue;
        }
        if (ownerId && !nodeOwner && node && node.dataset) node.dataset.fairyOwner = ownerId;
      }
      if (activeHost && typeof activeHost.querySelectorAll === 'function') {
        var activeOrbs = activeHost.querySelectorAll('.chat-pointer-orb.chat-pointer-agent');
        var keepOrb = null;
        for (var ai = 0; ai < activeOrbs.length; ai++) {
          var orb = activeOrbs[ai];
          var orbOwner = orb && orb.dataset && orb.dataset.fairyOwner ? String(orb.dataset.fairyOwner).trim() : '';
          if (ownerId && orbOwner && orbOwner !== ownerId) {
            try { orb.remove(); } catch(_) {}
            continue;
          }
          if (ownerId && !orbOwner && orb && orb.dataset) orb.dataset.fairyOwner = ownerId;
          if (!keepOrb) {
            keepOrb = orb;
            continue;
          }
          try { orb.remove(); } catch(_) {}
        }
        this._agentTrailOrbEl = keepOrb || null;
        if (this._agentTrailOrbEl) this.wireAgentTrailOrbBehavior(this._agentTrailOrbEl);
        else {
          this._agentTrailOrbElevated = false;
          if (keptActiveOverlay) {
            keptActiveOverlay.style.zIndex = '';
            keptActiveOverlay.classList.remove('fairy-z-top');
          }
        }
      }
      var hosts = scope.querySelectorAll('#messages');
      for (var hi = 0; hi < hosts.length; hi++) {
        var host = hosts[hi];
        if (!host || (activeHost && host === activeHost)) continue;
        host.style.setProperty('--chat-agent-grid-active', '0');
      }
      if (this._agentTrailOrbEl && (!activeHost || !activeHost.contains(this._agentTrailOrbEl))) {
        this._agentTrailOrbEl = null;
        this._agentTrailOrbElevated = false;
        if (activeHost && typeof activeHost.querySelector === 'function') {
          var activeOverlay = activeHost.querySelector('.chat-agent-overlay');
          if (activeOverlay) {
            activeOverlay.style.zIndex = '';
            activeOverlay.classList.remove('fairy-z-top');
          }
        }
      }
    },
    startAgentTrailLoop(container) {
      if (this._agentTrailListening) return;
      if (!this.currentFairyOwnerId()) {
        this.stopAgentTrailLoop(true);
        return;
      }
      var host = this.resolveFairyHost(container || this._agentTrailHost || null);
      if (!host) return;
      var previousHost = this._agentTrailHost || null;
      if (host && previousHost && previousHost !== host) {
        this.clearAgentTrailFx(previousHost);
      }
      if (host) this._agentTrailHost = host;
      this.dedupeAgentTrailFx(this._agentTrailHost);
      if (this._agentTrailRaf) return;
      var self = this;
      var tick = function(ts) {
        self._agentTrailRaf = requestAnimationFrame(tick);
        self.stepAgentTrail(ts || performance.now());
      };
      this._agentTrailLastAt = 0;
      this._agentTrailRaf = requestAnimationFrame(tick);
    },
    stopAgentTrailLoop(clearVisuals) {
      if (this._agentTrailRaf) try { cancelAnimationFrame(this._agentTrailRaf); } catch(_) {}
      this._agentTrailRaf = 0;
      this._agentTrailLastAt = 0;
      this._agentTrailLastDotAt = 0;
      this._agentTrailSeeded = false;
      this._agentTrailState = null;
      if (clearVisuals) this.clearAgentTrailFx(this._agentTrailHost);
      this._agentTrailHost = null;
    },
    stepAgentTrail(now) {
      var host = this.resolveFairyHost(this._agentTrailHost || null);
      if (!host || host.offsetParent === null) {
        this.dedupeAgentTrailFx(null);
        return;
      }
      if (!this.currentFairyOwnerId()) {
        this.clearAgentTrailFx(host);
        return;
      }
      if ((now - Number(this._agentTrailSweepAt || 0)) >= 260) {
        this.dedupeAgentTrailFx(host);
        this._agentTrailSweepAt = now;
      }
      var agentTrailDarkMode = this.pointerFxThemeMode() === 'dark';
      if (!agentTrailDarkMode) {
        var lightModeTrailNodes = host.querySelectorAll('.chat-pointer-trail-dot.chat-pointer-agent, .chat-pointer-trail-segment.chat-pointer-agent');
        for (var ln = 0; ln < lightModeTrailNodes.length; ln++) {
          this.clearPointerFxCleanupTimer(lightModeTrailNodes[ln]);
          try { lightModeTrailNodes[ln].remove(); } catch(_) {}
        }
      }
      this.pruneAgentTrailFx(host);
      this.syncGridBackgroundOffset(host);
      var rect = host.getBoundingClientRect(), w = Number(rect.width || 0), h = Number(rect.height || 0);
      if (!(w > 24 && h > 24)) return;
      var pad = 7;
      var zoneRight = w / 3, zoneTop = h * (2 / 3), shadowBuffer = 42;
      var minX = pad + shadowBuffer, maxX = Math.max(minX + 2, zoneRight - pad);
      var minY = Math.max(pad, zoneTop + 2), maxY = Math.max(minY + 2, h - pad);
      // Thinking anchor has higher priority than init-panel anchor so the
      // fairy always hugs the active thinking bubble during initialization.
      if (this.anchorAgentTrailToThinking(host, rect, now, pad, w, h)) return;
      if (this.anchorAgentTrailToFreshInit(host, rect, now, pad, w, h)) return;
      var s = this._agentTrailState;
      if (!s) { s = { x: (minX + maxX) * 0.5, y: (minY + maxY) * 0.5, vx: 48, vy: -24, dir: Math.random() * Math.PI * 2, target: 0, turnAt: 0 }; s.target = s.dir; s.turnAt = now + 1000; this._agentTrailState = s; }
      var dt = this._agentTrailLastAt > 0 ? Math.min(0.05, Math.max(0.001, (now - this._agentTrailLastAt) / 1000)) : (1 / 60);
      this._agentTrailLastAt = now;
      if (now >= Number(s.turnAt || 0)) { s.target = Math.random() * Math.PI * 2; s.turnAt = now + 1000; }
      var turnDelta = Math.atan2(Math.sin(s.target - s.dir), Math.cos(s.target - s.dir));
      s.dir += turnDelta * Math.min(1, dt * 3.3);
      var ax = Math.cos(s.dir) * 110 + ((Math.random() - 0.5) * 30);
      var ay = Math.sin(s.dir) * 84 + ((Math.random() - 0.5) * 24);
      ay += (((minY + maxY) * 0.5) - s.y) * 1.8;
      var cx = Number(this._lastPointerClientX || 0), cy = Number(this._lastPointerClientY || 0);
      if (cx >= rect.left && cx <= rect.right && cy >= rect.top && cy <= rect.bottom) {
        var px = cx - rect.left, py = cy - rect.top;
        var rx = s.x - px, ry = s.y - py;
        var avoidR = 118, d2 = (rx * rx) + (ry * ry);
        if (d2 > 0.0001 && d2 < (avoidR * avoidR)) {
          var d = Math.sqrt(d2);
          var repel = 1 - (d / avoidR), f = 620 * repel * repel;
          ax += (rx / d) * f;
          ay += (ry / d) * f;
        }
      }
      s.vx = (s.vx + (ax * dt)) * 0.94;
      s.vy = (s.vy + (ay * dt)) * 0.94;
      var speed = Math.sqrt((s.vx * s.vx) + (s.vy * s.vy));
      if (speed > 624) { s.vx = (s.vx / speed) * 624; s.vy = (s.vy / speed) * 624; }
      s.x += s.vx * dt; s.y += s.vy * dt;
      if (s.x < minX || s.x > maxX) { s.vx = (s.x < minX ? Math.abs(s.vx) : -Math.abs(s.vx)) * 0.72; s.x = s.x < minX ? minX : maxX; }
      if (s.y < minY || s.y > maxY) { s.vy = (s.y < minY ? Math.abs(s.vy) : -Math.abs(s.vy)) * 0.72; s.y = s.y < minY ? minY : maxY; }
      var fairyOwnerId = this.currentFairyOwnerId();
      this.ensureAgentTrailOrb(host, s.x, s.y);
      host.style.setProperty('--chat-agent-grid-active', '1'); host.style.setProperty('--chat-agent-grid-x', Math.round(s.x) + 'px'); host.style.setProperty('--chat-agent-grid-y', Math.round(s.y) + 'px');
      if (!agentTrailDarkMode) {
        this._agentTrailSeeded = false;
        this._agentTrailLastDotAt = now;
        if (s) {
          s.trailX = s.x;
          s.trailY = s.y;
        }
        return;
      }
      var shouldSpawnTrail = (now - Number(this._agentTrailLastDotAt || 0)) >= 52;
      if (!this._agentTrailSeeded) {
        this.spawnPointerTrail(host, s.x, s.y, {
          agentTrail: true,
          fairyOwnerId: fairyOwnerId,
          size: 3.4,
          opacity: 0.58,
          scale: 1.02,
        });
        s.trailX = s.x;
        s.trailY = s.y;
        this._agentTrailSeeded = true;
        this._agentTrailLastDotAt = now;
      } else if (shouldSpawnTrail) {
        var fromX = Number(s.trailX);
        var fromY = Number(s.trailY);
        if (!Number.isFinite(fromX) || !Number.isFinite(fromY)) {
          fromX = s.x;
          fromY = s.y;
        }
        this.spawnPointerTrailSegment(host, fromX, fromY, s.x, s.y, {
          agentTrail: true,
          fairyOwnerId: fairyOwnerId,
          thickness: 2.4,
          opacity: 0.52,
        });
        this.spawnPointerTrail(host, s.x, s.y, {
          agentTrail: true,
          fairyOwnerId: fairyOwnerId,
          size: 3.1,
          opacity: 0.56,
          scale: 1.01,
        });
        s.trailX = s.x;
        s.trailY = s.y;
        this._agentTrailLastDotAt = now;
      }
    },
  };
}
