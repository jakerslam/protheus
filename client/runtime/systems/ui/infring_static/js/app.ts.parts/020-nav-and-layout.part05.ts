    taskbarDockEdgeNormalized(raw) {
      var key = String(raw || '').trim().toLowerCase();
      return key === 'bottom' ? 'bottom' : 'top';
    },

    taskbarPersistDockEdge() {
      this.taskbarDockEdge = this.taskbarDockEdgeNormalized(this.taskbarDockEdge);
      try {
        localStorage.setItem('infring-taskbar-dock-edge', this.taskbarDockEdge);
      } catch(_) {}
    },

    taskbarReadHeight() {
      if (typeof document === 'undefined') return 46;
      try {
        var node = document.querySelector('.global-taskbar');
        var height = Number(node && node.offsetHeight || 0);
        if (Number.isFinite(height) && height > 0) return height;
      } catch(_) {}
      return 46;
    },

    taskbarReadViewportHeight() {
      var h = 0;
      try { h = Number(window && window.innerHeight || 0); } catch(_) { h = 0; }
      if (!Number.isFinite(h) || h <= 0) {
        h = Number(document && document.documentElement && document.documentElement.clientHeight || 900);
      }
      if (!Number.isFinite(h) || h <= 0) h = 900;
      return h;
    },

    chatOverlayViewportWidth() {
      var w = 0;
      try { w = Number(window && window.innerWidth || 0); } catch(_) { w = 0; }
      if (!Number.isFinite(w) || w <= 0) {
        w = Number(document && document.documentElement && document.documentElement.clientWidth || 1440);
      }
      if (!Number.isFinite(w) || w <= 0) w = 1440;
      return w;
    },

    taskbarAnchorForDockEdge(edgeRaw) {
      var edge = this.taskbarDockEdgeNormalized(edgeRaw);
      if (edge === 'bottom') {
        return Math.max(0, this.taskbarReadViewportHeight() - this.taskbarReadHeight());
      }
      return 0;
    },

    taskbarClampDragY(yRaw) {
      var y = Number(yRaw);
      if (!Number.isFinite(y)) y = this.taskbarAnchorForDockEdge(this.taskbarDockEdge);
      var maxY = Math.max(0, this.taskbarReadViewportHeight() - this.taskbarReadHeight());
      return Math.max(0, Math.min(maxY, y));
    },

    taskbarNearestDockEdge(yRaw) {
      var y = this.taskbarClampDragY(yRaw);
      var topY = this.taskbarAnchorForDockEdge('top');
      var bottomY = this.taskbarAnchorForDockEdge('bottom');
      var topDist = Math.abs(y - topY);
      var bottomDist = Math.abs(y - bottomY);
      return bottomDist < topDist ? 'bottom' : 'top';
    },

    taskbarContainerStyle() {
      var styles = [];
      if (this.page !== 'chat') {
        styles.push('background:transparent;border-bottom:none;box-shadow:none;-webkit-backdrop-filter:none;backdrop-filter:none;');
      }
      var transitionMs = this.taskbarDockDragActive ? 0 : 220;
      styles.push('--taskbar-dock-transition:' + Math.max(0, Math.round(Number(transitionMs || 0))) + 'ms;');
      if (this.taskbarDockDragActive) {
        var y = this.taskbarClampDragY(this.taskbarDockDragY);
        styles.push('top:' + Math.round(Number(y || 0)) + 'px;bottom:auto;');
      } else if (this.taskbarDockEdgeNormalized(this.taskbarDockEdge) === 'bottom') {
        styles.push('top:auto;bottom:0;');
      } else {
        styles.push('top:0;bottom:auto;');
      }
      return styles.join('');
    },

    shouldIgnoreTaskbarDockDragTarget(target) {
      if (!target || typeof target.closest !== 'function') return false;
      return Boolean(
        target.closest(
          'button, a, input, textarea, select, [role="button"], [draggable="true"], .taskbar-reorder-item, .taskbar-hero-menu-anchor, .taskbar-hero-menu, .theme-switcher, .notif-wrap, .taskbar-search-popup, .taskbar-search-popup-anchor, .taskbar-clock'
        )
      );
    },

    bindTaskbarDockPointerListeners() {
      if (this._taskbarDockPointerMoveHandler || this._taskbarDockPointerUpHandler) return;
      var self = this;
      this._taskbarDockPointerMoveHandler = function(ev) { self.handleTaskbarDockPointerMove(ev); };
      this._taskbarDockPointerUpHandler = function(ev) { self.endTaskbarDockPointerDrag(ev); };
      window.addEventListener('pointermove', this._taskbarDockPointerMoveHandler, true);
      window.addEventListener('pointerup', this._taskbarDockPointerUpHandler, true);
      window.addEventListener('pointercancel', this._taskbarDockPointerUpHandler, true);
      window.addEventListener('mousemove', this._taskbarDockPointerMoveHandler, true);
      window.addEventListener('mouseup', this._taskbarDockPointerUpHandler, true);
    },

    unbindTaskbarDockPointerListeners() {
      if (this._taskbarDockPointerMoveHandler) {
        try { window.removeEventListener('pointermove', this._taskbarDockPointerMoveHandler, true); } catch(_) {}
        try { window.removeEventListener('mousemove', this._taskbarDockPointerMoveHandler, true); } catch(_) {}
      }
      if (this._taskbarDockPointerUpHandler) {
        try { window.removeEventListener('pointerup', this._taskbarDockPointerUpHandler, true); } catch(_) {}
        try { window.removeEventListener('pointercancel', this._taskbarDockPointerUpHandler, true); } catch(_) {}
        try { window.removeEventListener('mouseup', this._taskbarDockPointerUpHandler, true); } catch(_) {}
      }
      this._taskbarDockPointerMoveHandler = null;
      this._taskbarDockPointerUpHandler = null;
    },

    startTaskbarDockPointerDrag(ev) {
      if (!ev || Number(ev.button) !== 0) return;
      if (String(this.taskbarDragGroup || '').trim()) return;
      var target = ev && ev.target ? ev.target : null;
      if (this.shouldIgnoreTaskbarDockDragTarget(target)) return;
      this._taskbarDockPointerActive = true;
      this._taskbarDockPointerMoved = false;
      this._taskbarDockPointerStartX = Number(ev.clientX || 0);
      this._taskbarDockPointerStartY = Number(ev.clientY || 0);
      this._taskbarDockOriginY = this.taskbarAnchorForDockEdge(this.taskbarDockEdge);
      this.taskbarDockDragY = this._taskbarDockOriginY;
      this.bindTaskbarDockPointerListeners();
      try {
        if (ev.currentTarget && typeof ev.currentTarget.setPointerCapture === 'function' && Number.isFinite(ev.pointerId)) {
          ev.currentTarget.setPointerCapture(ev.pointerId);
        }
      } catch(_) {}
      if (ev.cancelable && typeof ev.preventDefault === 'function') ev.preventDefault();
    },

    handleTaskbarDockPointerMove(ev) {
      if (!this._taskbarDockPointerActive) return;
      var x = Number(ev.clientX || 0);
      var y = Number(ev.clientY || 0);
      var movedX = Math.abs(x - Number(this._taskbarDockPointerStartX || 0));
      var movedY = Math.abs(y - Number(this._taskbarDockPointerStartY || 0));
      if (!this._taskbarDockPointerMoved) {
        if (movedX < 4 && movedY < 4) return;
        this._taskbarDockPointerMoved = true;
        this.taskbarDockDragActive = true;
      }
      var candidateY = Number(this._taskbarDockOriginY || 0) + (y - Number(this._taskbarDockPointerStartY || 0));
      this.taskbarDockDragY = this.taskbarClampDragY(candidateY);
      if (ev.cancelable && typeof ev.preventDefault === 'function') ev.preventDefault();
    },

    endTaskbarDockPointerDrag() {
      if (!this._taskbarDockPointerActive) return;
      this._taskbarDockPointerActive = false;
      this.unbindTaskbarDockPointerListeners();
      if (!this._taskbarDockPointerMoved) {
        this.taskbarDockDragActive = false;
        return;
      }
      this._taskbarDockPointerMoved = false;
      this.taskbarDockEdge = this.taskbarNearestDockEdge(this.taskbarDockDragY);
      this.taskbarDockDragActive = false;
      this.taskbarPersistDockEdge();
    },

    overlayWallGapPx() {
      var fallback = 16;
      if (typeof window !== 'undefined' && typeof window.getComputedStyle === 'function' && document && document.documentElement) {
        try {
          var raw = String(window.getComputedStyle(document.documentElement).getPropertyValue('--overlay-wall-gap') || '').trim();
          var parsed = parseFloat(raw);
          if (Number.isFinite(parsed) && parsed >= 0) fallback = parsed;
        } catch(_) {}
      }
      return Math.max(0, Math.round(fallback));
    },

    chatOverlayVerticalBounds() {
      var viewportHeight = this.taskbarReadViewportHeight();
      var wallGap = this.overlayWallGapPx();
      var edge = this.taskbarDockEdgeNormalized(this.taskbarDockEdge);
      var taskbarH = this.taskbarReadHeight();
      var topInset = edge === 'top' ? taskbarH : 0;
      var bottomInset = edge === 'bottom' ? taskbarH : 0;
      return {
        minTop: topInset + wallGap,
        maxBottom: viewportHeight - bottomInset - wallGap,
        viewportHeight: viewportHeight,
        wallGap: wallGap
      };
    },

    readChatMapElement() {
      if (typeof document === 'undefined' || typeof document.querySelector !== 'function') return null;
      try { return document.querySelector('.chat-map'); } catch(_) {}
      return null;
    },

    readChatMapHeight() {
      var node = this.readChatMapElement();
      var height = Number(node && node.offsetHeight || 0);
      if (!Number.isFinite(height) || height <= 0) {
        height = Math.max(180, this.taskbarReadViewportHeight() - 276);
      }
      return height;
    },

    chatMapPlacementEnabled() {
      return this.page === 'chat' || (this.page === 'agents' && !!this.activeChatAgent);
    },

    chatMapClampTop(topRaw) {
      var bounds = this.chatOverlayVerticalBounds();
      var height = this.readChatMapHeight();
      var minTop = Number(bounds.minTop || 0);
      var maxTop = Number(bounds.maxBottom || 0) - height;
      if (!Number.isFinite(maxTop) || maxTop < minTop) maxTop = minTop;
      var top = Number(topRaw);
      if (!Number.isFinite(top)) top = minTop + ((maxTop - minTop) * 0.38);
      return Math.max(minTop, Math.min(maxTop, top));
    },

    chatMapPersistPlacementFromTop(topRaw) {
      var bounds = this.chatOverlayVerticalBounds();
      var height = this.readChatMapHeight();
      var minTop = Number(bounds.minTop || 0);
      var maxTop = Number(bounds.maxBottom || 0) - height;
      if (!Number.isFinite(maxTop) || maxTop < minTop) maxTop = minTop;
      var top = this.chatMapClampTop(topRaw);
      var ratio = maxTop > minTop ? (top - minTop) / (maxTop - minTop) : 0.38;
      ratio = Math.max(0, Math.min(1, ratio));
      this.chatMapPlacementY = ratio;
      try {
        localStorage.setItem('infring-chat-map-placement-y', String(ratio));
      } catch(_) {}
    },

    shouldIgnoreChatMapDragTarget(target) {
      var node = target;
      if (node && typeof node.closest !== 'function' && node.parentElement) {
        node = node.parentElement;
      }
      if (!node || typeof node.closest !== 'function') return false;
      return Boolean(
        node.closest(
          'button, a, input, textarea, select, [role="button"], [contenteditable="true"], .chat-map-item, .chat-map-day, .chat-map-jump'
        )
      );
    },

    bindChatMapPointerListeners() {
      if (this._chatMapPointerMoveHandler || this._chatMapPointerUpHandler) return;
      var self = this;
      this._chatMapPointerMoveHandler = function(ev) { self.handleChatMapPointerMove(ev); };
      this._chatMapPointerUpHandler = function() { self.endChatMapPointerDrag(); };
      window.addEventListener('pointermove', this._chatMapPointerMoveHandler, true);
      window.addEventListener('pointerup', this._chatMapPointerUpHandler, true);
      window.addEventListener('pointercancel', this._chatMapPointerUpHandler, true);
      window.addEventListener('mousemove', this._chatMapPointerMoveHandler, true);
      window.addEventListener('mouseup', this._chatMapPointerUpHandler, true);
    },

    unbindChatMapPointerListeners() {
      if (this._chatMapPointerMoveHandler) {
        try { window.removeEventListener('pointermove', this._chatMapPointerMoveHandler, true); } catch(_) {}
        try { window.removeEventListener('mousemove', this._chatMapPointerMoveHandler, true); } catch(_) {}
      }
      if (this._chatMapPointerUpHandler) {
        try { window.removeEventListener('pointerup', this._chatMapPointerUpHandler, true); } catch(_) {}
        try { window.removeEventListener('pointercancel', this._chatMapPointerUpHandler, true); } catch(_) {}
        try { window.removeEventListener('mouseup', this._chatMapPointerUpHandler, true); } catch(_) {}
      }
      this._chatMapPointerMoveHandler = null;
      this._chatMapPointerUpHandler = null;
    },

    taskbarReorderDefaults(group) {
      var key = String(group || '').trim().toLowerCase();
      if (key === 'right') return ['connectivity', 'theme', 'notifications', 'search', 'auth'];
      return ['nav_cluster'];
    },

    taskbarReorderStorageKey(group) {
      var key = String(group || '').trim().toLowerCase();
      return key === 'right' ? 'infring-taskbar-order-right' : 'infring-taskbar-order-left';
    },

    taskbarReorderOrderForGroup(group) {
      var key = String(group || '').trim().toLowerCase();
      return key === 'right' ? this.taskbarReorderRight : this.taskbarReorderLeft;
    },

    setTaskbarReorderOrderForGroup(group, nextOrder) {
      var key = String(group || '').trim().toLowerCase();
      if (key === 'right') {
        this.taskbarReorderRight = nextOrder;
        return;
      }
      this.taskbarReorderLeft = nextOrder;
    },

    normalizeTaskbarReorder(group, rawOrder) {
      var defaults = this.taskbarReorderDefaults(group);
      var source = Array.isArray(rawOrder) ? rawOrder : [];
      var seen = {};
      var ordered = [];
      for (var i = 0; i < source.length; i += 1) {
        var id = String(source[i] || '').trim();
        if (!id || seen[id] || defaults.indexOf(id) < 0) continue;
        seen[id] = true;
        ordered.push(id);
      }
      for (var j = 0; j < defaults.length; j += 1) {
        var fallbackId = defaults[j];
        if (seen[fallbackId]) continue;
        seen[fallbackId] = true;
        ordered.push(fallbackId);
      }
      return ordered;
    },

    persistTaskbarReorder(group) {
      var key = String(group || '').trim().toLowerCase();
      if (key !== 'right') key = 'left';
      var normalized = this.normalizeTaskbarReorder(key, this.taskbarReorderOrderForGroup(key));
      this.setTaskbarReorderOrderForGroup(key, normalized);
      try {
        localStorage.setItem(this.taskbarReorderStorageKey(key), JSON.stringify(normalized));
      } catch(_) {}
    },

    taskbarReorderOrderIndex(group, item) {
      var key = String(group || '').trim().toLowerCase();
      if (key !== 'right') key = 'left';
      var itemId = String(item || '').trim();
      if (!itemId) return 999;
      var order = this.normalizeTaskbarReorder(key, this.taskbarReorderOrderForGroup(key));
      var idx = order.indexOf(itemId);
      if (idx >= 0) return idx;
      var fallback = this.taskbarReorderDefaults(key).indexOf(itemId);
      return fallback >= 0 ? fallback : 999;
    },

    taskbarReorderItemStyle(group, item) {
      return 'order:' + this.taskbarReorderOrderIndex(group, item);
    },

    taskbarReorderItemRects(group) {
      if (typeof document === 'undefined') return {};
      var key = String(group || '').trim().toLowerCase();
      if (key !== 'right') key = 'left';
      var out = {};
      var box = null;
      try {
        box = document.querySelector('.taskbar-reorder-box-' + key);
      } catch(_) {
        box = null;
      }
      if (!box || typeof box.querySelectorAll !== 'function') return out;
      var nodes = box.querySelectorAll('.taskbar-reorder-item[data-taskbar-item]');
      for (var i = 0; i < nodes.length; i += 1) {
        var node = nodes[i];
        if (!node || typeof node.getBoundingClientRect !== 'function') continue;
        var id = String(node.getAttribute('data-taskbar-item') || '').trim();
        if (!id || Object.prototype.hasOwnProperty.call(out, id)) continue;
        var rect = node.getBoundingClientRect();
        out[id] = { left: Number(rect.left || 0), top: Number(rect.top || 0) };
      }
      return out;
    },

    animateTaskbarReorderFromRects(group, beforeRects) {
      if (!beforeRects || typeof beforeRects !== 'object') return;
      if (typeof requestAnimationFrame !== 'function' || typeof document === 'undefined') return;
      var key = String(group || '').trim().toLowerCase();
      if (key !== 'right') key = 'left';
      requestAnimationFrame(function() {
        var box = null;
        try {
          box = document.querySelector('.taskbar-reorder-box-' + key);
        } catch(_) {
          box = null;
        }
        if (!box || typeof box.querySelectorAll !== 'function') return;
        var nodes = box.querySelectorAll('.taskbar-reorder-item[data-taskbar-item]');
        for (var i = 0; i < nodes.length; i += 1) {
          var node = nodes[i];
          if (!node || node.classList.contains('dragging')) continue;
          var id = String(node.getAttribute('data-taskbar-item') || '').trim();
          if (!id || !Object.prototype.hasOwnProperty.call(beforeRects, id)) continue;
          var from = beforeRects[id] || {};
          var rect = node.getBoundingClientRect();
          var dx = Number(from.left || 0) - Number(rect.left || 0);
          var dy = Number(from.top || 0) - Number(rect.top || 0);
          if (Math.abs(dx) < 0.5 && Math.abs(dy) < 0.5) continue;
          node.style.transition = 'none';
          node.style.transform = 'translate(' + Math.round(dx) + 'px,' + Math.round(dy) + 'px)';
          void node.offsetHeight;
          node.style.transition = 'transform 220ms var(--ease-smooth)';
          node.style.transform = 'translate(0px, 0px)';
          (function(el) {
            window.setTimeout(function() {
              if (!el.classList.contains('dragging')) el.style.transform = '';
              el.style.transition = '';
            }, 250);
          })(node);
        }
      });
    },
