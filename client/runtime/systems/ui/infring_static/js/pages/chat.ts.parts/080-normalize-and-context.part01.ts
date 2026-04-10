// FILE_SIZE_EXCEPTION: reason=chat pointer fx split continuity owner=jay expires=2026-06-30
          thickness: thickness,
          opacity: alpha,
          hueShift: hueShift,
        });
      }
      var headInterval = Number(profile && profile.head_interval_ms);
      if (!Number.isFinite(headInterval) || headInterval < 1) headInterval = 28;
      var canSpawnHead = (now - Number(this._pointerTrailHeadLastAt || 0)) >= headInterval;
      if (canSpawnHead || dist < 1.5) {
        // Render several smaller head particles instead of one large dot.
        var invDist = dist > 0.0001 ? (1 / dist) : 0;
        var nx = dist > 0.0001 ? (dx * invDist) : 1;
        var ny = dist > 0.0001 ? (dy * invDist) : 0;
        var pxTrail = Array.isArray(profile && profile.head_particles) && profile.head_particles.length
          ? profile.head_particles
          : [
              { back: 0.0, lateral: 0.0, size: 3.9, opacity: 0.58, hue: 0 },
              { back: 1.55, lateral: 0.64, size: 3.4, opacity: 0.5, hue: 2 },
              { back: 2.45, lateral: -0.58, size: 3.0, opacity: 0.44, hue: -2 },
              { back: 3.15, lateral: 0.0, size: 2.7, opacity: 0.38, hue: 1 },
            ];
        for (var j = 0; j < pxTrail.length; j++) {
          var p = pxTrail[j];
          var px = x - (nx * p.back) + (-ny * p.lateral);
          var py = y - (ny * p.back) + (nx * p.lateral);
          this.spawnPointerTrail(host, px, py, {
            size: p.size,
            opacity: p.opacity,
            scale: 1.03,
            hueShift: p.hue,
          });
        }
        this._pointerTrailHeadLastAt = now;
      }
      this._pointerTrailLastX = x;
      this._pointerTrailLastY = y;
    },
    syncGridBackgroundOffset(container) {
      var host = this.resolveMessagesScroller(container || null);
      if (!host) return;
      var scrollX = Number(host.scrollLeft || 0);
      var scrollY = Number(host.scrollTop || 0);
      host.style.setProperty('--chat-grid-scroll-x', String(-Math.round(scrollX)) + 'px');
      host.style.setProperty('--chat-grid-scroll-y', String(-Math.round(scrollY)) + 'px');
    },

    normalizePointerTarget(target) {
      var node = target || null;
      if (!node) return null;
      if (node.nodeType === 3) return node.parentElement || null;
      return node.nodeType === 1 ? node : null;
    },

    isPointerInteractiveTarget(target, host) {
      var node = this.normalizePointerTarget(target);
      while (node && node !== host) {
        if (node.matches && node.matches('button,[role="button"],a[href],summary,details,input,textarea,select,option,label,[data-no-select-gate="true"]')) {
          return true;
        }
        node = node.parentElement;
      }
      return false;
    },

    canStartMessagesTextSelection(target, host) {
      var node = this.normalizePointerTarget(target);
      while (node && node !== host) {
        if (node.matches && node.matches('input,textarea,[contenteditable],[contenteditable=""],[contenteditable="true"],[contenteditable="plaintext-only"]')) {
          return true;
        }
        try {
          var style = window.getComputedStyle(node);
          var cursor = String(style && style.cursor ? style.cursor : '').toLowerCase();
          if (cursor.indexOf('text') !== -1) return true;
        } catch(_) {}
        node = node.parentElement;
      }
      return false;
    },

    handleMessagesSelectStart(event) {
      if (!event || !event.currentTarget) return;
      var host = event.currentTarget;
      if (!this.canStartMessagesTextSelection(event.target, host)) {
        event.preventDefault();
      }
    },

    handleMessagesPointerDown(event) {
      if (!event || !event.currentTarget) return;
      var host = event.currentTarget;
      var canSelectText = this.canStartMessagesTextSelection(event.target, host);
      var isInteractive = this.isPointerInteractiveTarget(event.target, host);
      if (!canSelectText && !isInteractive) {
        event.preventDefault();
      }
      if (!canSelectText) {
        this._pointerTrailMouseHeld = true;
        this._pointerTrailHoldHost = host;
        this.updatePointerTrailHoldState(host, false);
        this.ensurePointerTrailReleaseListener();
      }
      if (this.pointerFxThemeMode() !== 'light') return;
      var rect = host.getBoundingClientRect();
      var x = event.clientX - rect.left;
      var y = event.clientY - rect.top;
      this.spawnPointerRipple(host, x, y);
    },

    handleMessagesPointerUp(event) {
      if (!this._pointerTrailMouseHeld) {
        this.removePointerTrailReleaseListener();
        return;
      }
      var host = this.resolveMessagesScroller(this._pointerTrailHoldHost || (event && event.currentTarget ? event.currentTarget : null)) || this.resolveMessagesScroller();
      this._pointerTrailMouseHeld = false;
      this._pointerTrailHoldHost = null;
      this.updatePointerTrailHoldState(host, true);
      this.removePointerTrailReleaseListener();
    },

    clearPointerFx(event) {
      if (!event || !event.currentTarget) return;
      if (this._pointerTrailMouseHeld) return;
      var host = event.currentTarget, layer = this.resolvePointerFxLayer(host);
      if (this._pointerGridHideTimer) {
        clearTimeout(this._pointerGridHideTimer);
        this._pointerGridHideTimer = null;
      }
      var dots = (layer || host).querySelectorAll('.chat-pointer-trail-dot:not(.chat-pointer-agent),.chat-pointer-trail-segment:not(.chat-pointer-agent),.chat-pointer-ripple');
      for (var i = 0; i < dots.length; i++) {
        this.clearPointerFxCleanupTimer(dots[i]);
        try { dots[i].remove(); } catch(_) {}
      }
      this.removePointerOrb();
      this._pointerTrailSeeded = false;
      this._pointerTrailHeadLastAt = 0;
    },
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
      if (ownerId && orb.dataset) {
        orb.dataset.fairyOwner = ownerId;
        this._agentFairyOwnerId = ownerId;
      }
      orb.style.left = x + 'px';
      orb.style.top = y + 'px';
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
      if (!orb) return;
      try { orb.remove(); } catch(_) {}
      this._agentTrailOrbEl = null;
      this._agentFairyOwnerId = '';
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
      }
      var hosts = scope.querySelectorAll('#messages');
      for (var hi = 0; hi < hosts.length; hi++) {
        var host = hosts[hi];
        if (!host || (activeHost && host === activeHost)) continue;
        host.style.setProperty('--chat-agent-grid-active', '0');
      }
      if (this._agentTrailOrbEl && (!activeHost || !activeHost.contains(this._agentTrailOrbEl))) {
        this._agentTrailOrbEl = null;
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
    markAgentMessageComplete(msg) {
      if (!msg || msg.role !== 'agent') return;
