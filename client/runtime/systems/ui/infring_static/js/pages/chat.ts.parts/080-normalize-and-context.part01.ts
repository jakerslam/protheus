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
        if (
          node.matches &&
          node.matches(
            '.message-bubble-content, .message-bubble-content *, .chat-artifact-pre, .chat-artifact-pre *, .chat-artifact-path, .chat-artifact-path *, .chat-artifact-title, .chat-artifact-title *'
          )
        ) {
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
