'use strict';

function infringChatPointerFxMethods() {
  return {
    pointerFxThemeMode() {
      try {
        var bodyTheme = '';
        var rootTheme = '';
        if (document && document.body && document.body.dataset) {
          bodyTheme = String(document.body.dataset.theme || '').toLowerCase().trim();
        }
        if (document && document.documentElement) {
          rootTheme = String(
            (document.documentElement.dataset && document.documentElement.dataset.theme) ||
            document.documentElement.getAttribute('data-theme') ||
            ''
          ).toLowerCase().trim();
        }
        var resolved = bodyTheme || rootTheme;
        if (!resolved) {
          try {
            resolved = window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches
              ? 'dark'
              : 'light';
          } catch(_) {
            resolved = 'light';
          }
        }
        if (document && document.body && document.body.dataset) {
          if (!bodyTheme || bodyTheme !== resolved) {
            document.body.dataset.theme = resolved;
          }
        }
        return resolved === 'dark' ? 'dark' : 'light';
      } catch(_) {
        return 'light';
      }
    },

    pointerTrailProfile() {
      if (
        typeof window !== 'undefined' &&
        window.__INFRING_POINTER_TRAIL_PROFILE_V1 &&
        typeof window.__INFRING_POINTER_TRAIL_PROFILE_V1 === 'object'
      ) {
        return window.__INFRING_POINTER_TRAIL_PROFILE_V1;
      }
      return {
        spacing: 0.13,
        max_steps: 52,
        head_interval_ms: 28,
        segment_thickness_base: 2.05,
        segment_thickness_gain: 1.85,
        segment_opacity_base: 0.32,
        segment_opacity_gain: 0.45,
        segment_hue_base: -4,
        segment_hue_gain: 8,
        head_particles: [
          { back: 0.0, lateral: 0.0, size: 3.9, opacity: 0.58, hue: 0 },
          { back: 1.55, lateral: 0.64, size: 3.4, opacity: 0.5, hue: 2 },
          { back: 2.45, lateral: -0.58, size: 3.0, opacity: 0.44, hue: -2 },
          { back: 3.15, lateral: 0.0, size: 2.7, opacity: 0.38, hue: 1 }
        ]
      };
    },

    pointerTrailFadeDurationMs(kind, slow) {
      var base = String(kind || '') === 'segment' ? 760 : 860;
      return slow ? (base * 10) : base;
    },

    clearPointerFxCleanupTimer(node) {
      if (!node) return;
      if (node._pointerFxCleanupTimer) {
        try { clearTimeout(node._pointerFxCleanupTimer); } catch(_) {}
        node._pointerFxCleanupTimer = 0;
      }
    },

    schedulePointerFxCleanup(node, kind, slow) {
      if (!node) return;
      this.clearPointerFxCleanupTimer(node);
      var delay = this.pointerTrailFadeDurationMs(kind, !!slow);
      node._pointerFxCleanupTimer = setTimeout(function() {
        try { node.remove(); } catch(_) {}
      }, Math.max(120, delay + 120));
    },

    updatePointerTrailHoldState(container, releaseSlow) {
      var host = this.resolveMessagesScroller(container || this._pointerTrailHoldHost || null) || this.resolveMessagesScroller();
      if (!host) return;
      var layer = this.resolvePointerFxLayer(host) || host;
      var nodes = layer.querySelectorAll('.chat-pointer-trail-dot:not(.chat-pointer-agent), .chat-pointer-trail-segment:not(.chat-pointer-agent)');
      for (var i = 0; i < nodes.length; i++) {
        var node = nodes[i];
        var isSegment = !!(node.classList && node.classList.contains('chat-pointer-trail-segment'));
        var kind = isSegment ? 'segment' : 'dot';
        this.clearPointerFxCleanupTimer(node);
        if (!node.classList) continue;
        if (releaseSlow) {
          node.classList.remove('chat-pointer-held');
          node.classList.remove('chat-pointer-release-slow');
          try { void node.offsetWidth; } catch(_) {}
          node.classList.add('chat-pointer-release-slow');
          this.schedulePointerFxCleanup(node, kind, true);
          continue;
        }
        node.classList.remove('chat-pointer-release-slow');
        node.classList.add('chat-pointer-held');
      }
    },

    ensurePointerTrailReleaseListener() {
      if (this._pointerTrailMouseUpHandler) return;
      var self = this;
      this._pointerTrailMouseUpHandler = function(ev) {
        self.handleMessagesPointerUp(ev || null);
      };
      document.addEventListener('mouseup', this._pointerTrailMouseUpHandler, true);
      document.addEventListener('pointerup', this._pointerTrailMouseUpHandler, true);
      window.addEventListener('blur', this._pointerTrailMouseUpHandler, true);
    },

    removePointerTrailReleaseListener() {
      if (!this._pointerTrailMouseUpHandler) return;
      try { document.removeEventListener('mouseup', this._pointerTrailMouseUpHandler, true); } catch(_) {}
      try { document.removeEventListener('pointerup', this._pointerTrailMouseUpHandler, true); } catch(_) {}
      try { window.removeEventListener('blur', this._pointerTrailMouseUpHandler, true); } catch(_) {}
      this._pointerTrailMouseUpHandler = null;
    },

// Layer ownership: client/runtime/systems/ui (dashboard static UX surface only; no runtime authority).

    spawnPointerTrail(container, x, y, opts) {
      var options = opts || {};
      if (!options.agentTrail && this.shouldSuspendPointerFx && this.shouldSuspendPointerFx()) return;
      var layer = options.agentTrail ? this.resolveAgentFxLayer(container) : this.resolvePointerFxLayer(container);
      if (!layer) return;
      var marker = document.createElement('span');
      marker.className = options.agentTrail ? 'chat-pointer-trail-dot chat-pointer-agent' : 'chat-pointer-trail-dot';
      if (options.agentTrail && marker.dataset) {
        var ownerId = String(
          options.fairyOwnerId ||
          (this.currentFairyOwnerId ? this.currentFairyOwnerId() : '')
        ).trim();
        if (ownerId) marker.dataset.fairyOwner = ownerId;
      }
      marker.style.left = x + 'px';
      marker.style.top = y + 'px';
      if (Number.isFinite(Number(options.size))) marker.style.setProperty('--trail-size', String(Number(options.size)));
      if (Number.isFinite(Number(options.opacity))) marker.style.setProperty('--trail-opacity', String(Number(options.opacity)));
      if (Number.isFinite(Number(options.scale))) marker.style.setProperty('--trail-scale', String(Number(options.scale)));
      if (Number.isFinite(Number(options.hueShift))) marker.style.setProperty('--trail-hue-shift', String(Number(options.hueShift)) + 'deg');
      var holdMouseTrail = !options.agentTrail && !!this._pointerTrailMouseHeld;
      if (holdMouseTrail) marker.classList.add('chat-pointer-held');
      layer.appendChild(marker);
      if (!holdMouseTrail) this.schedulePointerFxCleanup(marker, 'dot', false);
    },

    spawnPointerTrailSegment(container, x0, y0, x1, y1, opts) {
      var options = opts || {};
      if (!options.agentTrail && this.shouldSuspendPointerFx && this.shouldSuspendPointerFx()) return;
      var layer = options.agentTrail ? this.resolveAgentFxLayer(container) : this.resolvePointerFxLayer(container);
      if (!layer) return;
      var dx = Number(x1 || 0) - Number(x0 || 0);
      var dy = Number(y1 || 0) - Number(y0 || 0);
      var dist = Math.sqrt(dx * dx + dy * dy);
      if (!Number.isFinite(dist) || dist < 0.75) return;
      var seg = document.createElement('span');
      seg.className = options.agentTrail ? 'chat-pointer-trail-segment chat-pointer-agent' : 'chat-pointer-trail-segment';
      if (options.agentTrail && seg.dataset) {
        var ownerId = String(
          options.fairyOwnerId ||
          (this.currentFairyOwnerId ? this.currentFairyOwnerId() : '')
        ).trim();
        if (ownerId) seg.dataset.fairyOwner = ownerId;
      }
      var mx = Number(x0 || 0) + (dx * 0.5);
      var my = Number(y0 || 0) + (dy * 0.5);
      var angle = Math.atan2(dy, dx) * (180 / Math.PI);
      seg.style.left = mx + 'px';
      seg.style.top = my + 'px';
      seg.style.width = Math.max(2, dist + 1) + 'px';
      seg.style.transform = 'translate(-50%, -50%) rotate(' + angle + 'deg)';
      if (Number.isFinite(Number(options.thickness))) seg.style.setProperty('--trail-seg-thickness', String(Number(options.thickness)));
      if (Number.isFinite(Number(options.opacity))) seg.style.setProperty('--trail-seg-opacity', String(Number(options.opacity)));
      if (Number.isFinite(Number(options.hueShift))) seg.style.setProperty('--trail-seg-hue-shift', String(Number(options.hueShift)) + 'deg');
      var holdMouseTrail = !options.agentTrail && !!this._pointerTrailMouseHeld;
      if (holdMouseTrail) seg.classList.add('chat-pointer-held');
      layer.appendChild(seg);
      if (!holdMouseTrail) this.schedulePointerFxCleanup(seg, 'segment', false);
    },

    spawnPointerRipple(container, x, y) {
      if (this.shouldSuspendPointerFx && this.shouldSuspendPointerFx()) return;
      var layer = this.resolvePointerFxLayer(container);
      if (!layer) return;
      var ripple = document.createElement('span');
      ripple.className = 'chat-pointer-ripple';
      ripple.style.left = x + 'px';
      ripple.style.top = y + 'px';
      layer.appendChild(ripple);
      setTimeout(function() {
        try { ripple.remove(); } catch(_) {}
      }, 820);
    },
    shouldSuspendPointerFx() {
      return !!this.recording;
    },

    resolvePointerFxLayer(container) {
      if (!container || typeof container.querySelector !== 'function') return null;
      return container.querySelector('.chat-grid-overlay') || container;
    },
    resolveAgentFxLayer(container) {
      if (!container || typeof container.querySelector !== 'function') return null;
      var layer = container.querySelector('.chat-agent-overlay');
      if (layer) return layer;
      layer = document.createElement('div');
      layer.className = 'chat-agent-overlay';
      container.appendChild(layer);
      return layer;
    },

    ensurePointerOrb(container, x, y) {
      if (this.shouldSuspendPointerFx && this.shouldSuspendPointerFx()) return null;
      var layer = this.resolvePointerFxLayer(container);
      if (!layer) return null;
      var orb = this._pointerOrbEl;
      if (!orb || !orb.isConnected || orb.parentNode !== layer) {
        if (orb) {
          try { orb.remove(); } catch(_) {}
        }
        orb = document.createElement('span');
        orb.className = 'chat-pointer-orb';
        layer.appendChild(orb);
        this._pointerOrbEl = orb;
      }
      orb.style.left = x + 'px';
      orb.style.top = y + 'px';
      return orb;
    },

    removePointerOrb() {
      var orb = this._pointerOrbEl;
      if (!orb) return;
      try { orb.remove(); } catch(_) {}
      this._pointerOrbEl = null;
    },

    handleMessagesPointerMove(event) {
      if (!event || !event.currentTarget) return;
      var host = event.currentTarget;
      this.startAgentTrailLoop(host);
      this.syncDirectHoverFromPointer(event);
      if (this.shouldSuspendPointerFx && this.shouldSuspendPointerFx()) {
        this.removePointerOrb();
        return;
      }
      if (this.pointerFxThemeMode() !== 'dark') {
        this.removePointerOrb();
        return;
      }
      var now = Date.now();
      if ((now - Number(this._pointerTrailLastAt || 0)) < 8) return;
      this._pointerTrailLastAt = now;
      var rect = host.getBoundingClientRect();
      // Keep pointer FX in viewport coordinates so the mask remains visible
      // while reading scrolled chat history.
      var x = event.clientX - rect.left;
      var y = event.clientY - rect.top;
      host.style.setProperty('--chat-grid-x', Math.round(x) + 'px');
      host.style.setProperty('--chat-grid-y', Math.round(y) + 'px');
      host.style.setProperty('--chat-grid-active', '1');
      this.ensurePointerOrb(host, x, y);
      if (!this._pointerTrailSeeded) {
        this._pointerTrailLastX = x;
        this._pointerTrailLastY = y;
        this._pointerTrailSeeded = true;
      }
      var dx = x - Number(this._pointerTrailLastX || x);
      var dy = y - Number(this._pointerTrailLastY || y);
      var dist = Math.sqrt(dx * dx + dy * dy);
      var profile = typeof this.pointerTrailProfile === 'function'
        ? this.pointerTrailProfile()
        : null;
      var spacing = Number(profile && profile.spacing);
      if (!Number.isFinite(spacing) || spacing <= 0) spacing = 0.13;
      var maxSteps = Number(profile && profile.max_steps);
      if (!Number.isFinite(maxSteps) || maxSteps < 1) maxSteps = 52;
      var steps = Math.max(1, Math.min(maxSteps, Math.ceil(dist / spacing)));
      for (var i = 1; i <= steps; i++) {
        var t0 = (i - 1) / steps;
        var t1 = i / steps;
        var sx0 = this._pointerTrailLastX + (dx * t0);
        var sy0 = this._pointerTrailLastY + (dy * t0);
        var sx1 = this._pointerTrailLastX + (dx * t1);
        var sy1 = this._pointerTrailLastY + (dy * t1);
        var progress = t1;
        var thickness = Number(profile && profile.segment_thickness_base);
        if (!Number.isFinite(thickness)) thickness = 2.05;
        var thicknessGain = Number(profile && profile.segment_thickness_gain);
        if (!Number.isFinite(thicknessGain)) thicknessGain = 1.85;
        thickness += (progress * thicknessGain);
        var alpha = Number(profile && profile.segment_opacity_base);
        if (!Number.isFinite(alpha)) alpha = 0.32;
        var alphaGain = Number(profile && profile.segment_opacity_gain);
        if (!Number.isFinite(alphaGain)) alphaGain = 0.45;
        alpha += (progress * alphaGain);
        var hueShift = Number(profile && profile.segment_hue_base);
        if (!Number.isFinite(hueShift)) hueShift = -4;
        var hueGain = Number(profile && profile.segment_hue_gain);
        if (!Number.isFinite(hueGain)) hueGain = 8;
        hueShift += (progress * hueGain);
        this.spawnPointerTrailSegment(host, sx0, sy0, sx1, sy1, {

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
  };
}
