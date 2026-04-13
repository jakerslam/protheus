
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
