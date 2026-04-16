    bottomDockShouldInsertAfter(targetId, ev, targetEl) {
      var key = String(targetId || '').trim();
      if (!key) return false;
      if (!ev) return false;
      var clientX = Number(ev.clientX || 0);
      var clientY = Number(ev.clientY || 0);
      if (!Number.isFinite(clientX) || !Number.isFinite(clientY)) return false;
      var node = targetEl || null;
      if (!node && typeof document !== 'undefined') {
        try {
          node = document.querySelector('.bottom-dock-btn[data-dock-id="' + key + '"]');
        } catch(_) {
          node = null;
        }
      }
      if (!node || typeof node.getBoundingClientRect !== 'function') return false;
      var rect = node.getBoundingClientRect();
      var width = Number(rect.width || 0);
      var height = Number(rect.height || 0);
      if (!Number.isFinite(width) || width <= 0) return false;
      if (!Number.isFinite(height) || height <= 0) return false;
      var basis = this.bottomDockAxisBasis();
      var centerX = Number(rect.left || 0) + (width / 2);
      var centerY = Number(rect.top || 0) + (height / 2);
      var centerProj = this.bottomDockProjectPointToAxis(centerX, centerY, basis);
      var pointerProj = this.bottomDockProjectPointToAxis(clientX, clientY, basis);
      var half = this.bottomDockAxisHalfExtent(width, height, basis).primary;
      if (!Number.isFinite(half) || half <= 0) half = Math.max(width, height) / 2;
      if (!Number.isFinite(half) || half <= 0) return false;
      var ratio = (pointerProj.primary - (centerProj.primary - half)) / (half * 2);
      return ratio >= 0.5;
    },

    captureBottomDockDragBoundaries(dragId) {
      var key = String(dragId || '').trim();
      if (!key || typeof document === 'undefined') {
        this._bottomDockDragBoundaries = [];
        this._bottomDockLastInsertionIndex = -1;
        return [];
      }
      var dock = null;
      try {
        dock = document.querySelector('.bottom-dock');
      } catch(_) {
        dock = null;
      }
      if (!dock) {
        this._bottomDockDragBoundaries = [];
        this._bottomDockLastInsertionIndex = -1;
        return [];
      }
      var centers = [];
      var basis = this.bottomDockAxisBasis();
      try {
        var nodes = dock.querySelectorAll('.bottom-dock-btn[data-dock-id]');
        for (var i = 0; i < nodes.length; i += 1) {
          var node = nodes[i];
          if (!node || typeof node.getAttribute !== 'function') continue;
          var id = String(node.getAttribute('data-dock-id') || '').trim();
          if (!id || id === key || typeof node.getBoundingClientRect !== 'function') continue;
          var rect = node.getBoundingClientRect();
          var width = Number(rect.width || 0);
          var height = Number(rect.height || 0);
          if (!Number.isFinite(width) || width <= 0) continue;
          if (!Number.isFinite(height) || height <= 0) continue;
          var centerX = Number(rect.left || 0) + (width / 2);
          var centerY = Number(rect.top || 0) + (height / 2);
          centers.push(this.bottomDockProjectPointToAxis(centerX, centerY, basis).primary);
        }
      } catch(_) {}
      centers.sort(function(a, b) { return a - b; });
      this._bottomDockDragBoundaries = centers;
      this._bottomDockLastInsertionIndex = -1;
      return centers;
    },

    bottomDockAppendTargetId(dragId) {
      var key = String(dragId || '').trim();
      if (!key) return '';
      var order = this.normalizeBottomDockOrder(this.bottomDockOrder);
      var filtered = [];
      for (var i = 0; i < order.length; i += 1) {
        var id = String(order[i] || '').trim();
        if (!id || id === key) continue;
        filtered.push(id);
      }
      if (!filtered.length) return '';
      return String(filtered[filtered.length - 1] || '').trim();
    },

    bottomDockShouldAppendFromPointer(dragId, ev) {
      var key = String(dragId || '').trim();
      if (!key || !ev || typeof document === 'undefined') return false;
      var clientX = Number(ev.clientX || 0);
      var clientY = Number(ev.clientY || 0);
      if (!Number.isFinite(clientX) || !Number.isFinite(clientY)) return false;
      var appendTargetId = this.bottomDockAppendTargetId(key);
      if (!appendTargetId) return false;
      var node = null;
      try {
        node = document.querySelector('.bottom-dock-btn[data-dock-id="' + appendTargetId + '"]');
      } catch(_) {
        node = null;
      }
      if (!node || typeof node.getBoundingClientRect !== 'function') return false;
      var rect = node.getBoundingClientRect();
      var width = Number(rect.width || 0);
      var height = Number(rect.height || 0);
      if (!Number.isFinite(width) || width <= 0) return false;
      if (!Number.isFinite(height) || height <= 0) return false;
      var basis = this.bottomDockAxisBasis();
      var centerX = Number(rect.left || 0) + (width / 2);
      var centerY = Number(rect.top || 0) + (height / 2);
      var centerProj = this.bottomDockProjectPointToAxis(centerX, centerY, basis);
      var pointerProj = this.bottomDockProjectPointToAxis(clientX, clientY, basis);
      var extent = this.bottomDockAxisHalfExtent(width, height, basis);
      var halfPrimary = Number(extent.primary || 0);
      var halfSecondary = Number(extent.secondary || 0);
      if (!Number.isFinite(halfPrimary) || halfPrimary <= 0) halfPrimary = Math.max(width, height) / 2;
      if (!Number.isFinite(halfSecondary) || halfSecondary <= 0) halfSecondary = Math.min(width, height) / 2;
      var secondaryPad = Math.max(18, halfSecondary * 0.75);
      if (Math.abs(pointerProj.secondary - centerProj.secondary) > (halfSecondary + secondaryPad)) return false;
      var threshold = centerProj.primary + halfPrimary - Math.min(18, halfPrimary * 0.7);
      return pointerProj.primary >= threshold;
    },

    bottomDockInsertionIndexFromCoords(dragId, clientXRaw, clientYRaw) {
      var key = String(dragId || '').trim();
      if (!key || typeof document === 'undefined') return null;
      var clientX = Number(clientXRaw || 0);
      var clientY = Number(clientYRaw || 0);
      if (!Number.isFinite(clientX) || !Number.isFinite(clientY)) return null;
      var dock = null;
      try {
        dock = document.querySelector('.bottom-dock');
      } catch(_) {
        dock = null;
      }
      if (!dock || typeof dock.getBoundingClientRect !== 'function') return null;
      var dockRect = dock.getBoundingClientRect();
      var basis = this.bottomDockAxisBasis();
      var pointerProj = this.bottomDockProjectPointToAxis(clientX, clientY, basis);
      var dockBounds = this.bottomDockProjectedRectBounds(dockRect, basis);
      if (!dockBounds) return null;
      if (
        pointerProj.secondary < (Number(dockBounds.secondaryMin || 0) - 24) ||
        pointerProj.secondary > (Number(dockBounds.secondaryMax || 0) + 24)
      ) return null;
      var centers = this.captureBottomDockDragBoundaries(key);
      if (centers.length === 0) return null;
      var insertionIndex = 0;
      for (var c = 0; c < centers.length; c += 1) {
        if (pointerProj.primary >= centers[c]) insertionIndex += 1;
      }
      insertionIndex = Math.max(0, Math.min(centers.length, insertionIndex));
      return insertionIndex;
    },

    bottomDockGhostCenterPoint() {
      var x = Number(this._bottomDockGhostTargetX || this._bottomDockGhostCurrentX || 0);
      var y = Number(this._bottomDockGhostTargetY || this._bottomDockGhostCurrentY || 0);
      var width = Number(this._bottomDockDragGhostWidth || 0);
      var height = Number(this._bottomDockDragGhostHeight || 0);
      if (!Number.isFinite(width) || width <= 0) width = 32;
      if (!Number.isFinite(height) || height <= 0) height = 32;
      return {
        x: x + (width / 2),
        y: y + (height / 2)
      };
    },

    bottomDockInsertionIndexFromPointer(dragId, ev) {
      var key = String(dragId || '').trim();
      if (!key || !ev) return null;
      var center = this.bottomDockGhostCenterPoint();
      return this.bottomDockInsertionIndexFromCoords(key, center.x, center.y);
    },

    applyBottomDockReorderByIndex(dragId, insertionIndex, animate) {
      var key = String(dragId || '').trim();
      if (!key) return false;
      var current = this.normalizeBottomDockOrder(this.bottomDockOrder);
      var fromIndex = current.indexOf(key);
      if (fromIndex < 0) return false;
      var next = current.slice();
      next.splice(fromIndex, 1);
      var idx = Number(insertionIndex);
      if (!Number.isFinite(idx)) return false;
      idx = Math.max(0, Math.min(next.length, Math.round(idx)));
      next.splice(idx, 0, key);
      if (JSON.stringify(next) === JSON.stringify(current)) return false;
      var doAnimate = Boolean(animate);
      var beforeRects = doAnimate ? this.bottomDockButtonRects() : null;
      this.bottomDockOrder = next;
      if (doAnimate && beforeRects) this.animateBottomDockFromRects(beforeRects);
      return true;
    },
    persistBottomDockOrderIfChangedFromDragStart() {
      var current = this.normalizeBottomDockOrder(this.bottomDockOrder);
      var start = this.normalizeBottomDockOrder(this.bottomDockDragStartOrder);
      if (JSON.stringify(current) !== JSON.stringify(start)) {
        this.bottomDockOrder = current;
        this.persistBottomDockOrder();
        this.bottomDockDragCommitted = true;
      }
    },
    completeBottomDockDropCleanup(ev) {
      this.bottomDockDragId = '';
      this.bottomDockDragStartOrder = [];
      this._bottomDockSuppressClickUntil = Date.now() + 220;
      this.cleanupBottomDockDragGhost();
      this.reviveBottomDockHoverFromPoint(
        Number(ev && ev.clientX || 0),
        Number(ev && ev.clientY || 0)
      );
      if (ev && typeof ev.preventDefault === 'function') ev.preventDefault();
    },

    handleBottomDockContainerDragOver(ev) {
      if (ev && ev.dataTransfer) {
        try { ev.dataTransfer.dropEffect = 'move'; } catch(_) {}
      }
      var dragId = String(this.bottomDockDragId || '').trim();
      if (!dragId) return;
      var targetId = '';
      var targetEl = null;
      try {
        targetEl = ev && ev.target && typeof ev.target.closest === 'function'
          ? ev.target.closest('.bottom-dock-btn[data-dock-id]')
          : null;
        targetId = targetEl ? String(targetEl.getAttribute('data-dock-id') || '').trim() : '';
      } catch(_) {}
      if (targetId && targetId !== dragId) {
        this._bottomDockLastInsertionIndex = -1;
        var preferAfter = this.bottomDockShouldInsertAfter(targetId, ev, targetEl);
        this.handleBottomDockDragOver(targetId, ev, preferAfter);
        return;
      }
      if (!this.bottomDockShouldAppendFromPointer(dragId, ev)) return;
      var appendTargetId = this.bottomDockAppendTargetId(dragId);
      if (!appendTargetId) return;
      this._bottomDockLastInsertionIndex = -1;
      this.handleBottomDockDragOver(appendTargetId, ev, true);
    },

    handleBottomDockContainerDrop(ev) {
      var dragId = String(this.bottomDockDragId || '').trim();
      if (!dragId) return;
      var targetId = '';
      var targetEl = null;
      try {
        targetEl = ev && ev.target && typeof ev.target.closest === 'function'
          ? ev.target.closest('.bottom-dock-btn[data-dock-id]')
          : null;
        targetId = targetEl ? String(targetEl.getAttribute('data-dock-id') || '').trim() : '';
      } catch(_) {}
      if (targetId) {
        var preferAfter = this.bottomDockShouldInsertAfter(targetId, ev, targetEl);
        this.handleBottomDockDrop(targetId, ev, preferAfter);
        return;
      }
      if (this.bottomDockShouldAppendFromPointer(dragId, ev)) {
        var appendTargetId = this.bottomDockAppendTargetId(dragId);
        if (appendTargetId) {
          this.handleBottomDockDrop(appendTargetId, ev, true);
          return;
        }
      }
      this.persistBottomDockOrderIfChangedFromDragStart();
      this.completeBottomDockDropCleanup(ev);
    },

    handleBottomDockDragOver(id, ev, preferAfter) {
      var targetId = String(id || '').trim();
      var dragId = String(this.bottomDockDragId || '').trim();
      if (!targetId || !dragId || targetId === dragId) return;
      var nowMs = Date.now();
      var lockUntil = Number(this._bottomDockReorderLockUntil || 0);
      if (Number.isFinite(lockUntil) && lockUntil > nowMs) return;
      if (ev && ev.dataTransfer) {
        try { ev.dataTransfer.dropEffect = 'move'; } catch(_) {}
      }
      var placeAfter = Boolean(preferAfter);
      var current = this.normalizeBottomDockOrder(this.bottomDockOrder);
      var next = current.slice();
      var fromIndex = next.indexOf(dragId);
      var toIndex = next.indexOf(targetId);
      if (fromIndex < 0 || toIndex < 0 || fromIndex === toIndex) return;
      next.splice(fromIndex, 1);
      if (fromIndex < toIndex) toIndex -= 1;
      if (placeAfter) toIndex += 1;
      if (toIndex < 0) toIndex = 0;
      if (toIndex > next.length) toIndex = next.length;
      next.splice(toIndex, 0, dragId);
      if (JSON.stringify(next) === JSON.stringify(current)) return;
      var beforeRects = this.bottomDockButtonRects();
      this.bottomDockOrder = next;
      this.animateBottomDockFromRects(beforeRects);
      var moveDuration = this.bottomDockMoveDurationMs();
      var lockMs = Math.max(320, Math.min(520, Math.round(moveDuration + 60)));
      this._bottomDockReorderLockUntil = nowMs + lockMs;
    },

    handleBottomDockDrop(id, ev, preferAfter) {
      var targetId = String(id || '').trim();
      var dragId = String(this.bottomDockDragId || '').trim();
      if (!targetId || !dragId) {
        this._bottomDockSuppressClickUntil = Date.now() + 220;
        this.cleanupBottomDockDragGhost();
        this.bottomDockDragId = '';
        this.bottomDockDragStartOrder = [];
        this.bottomDockDragCommitted = false;
        this.reviveBottomDockHoverFromPoint(
          Number(ev && ev.clientX || 0),
          Number(ev && ev.clientY || 0)
        );
        return;
      }
      if (targetId === dragId) {
        this.persistBottomDockOrderIfChangedFromDragStart();
        this.completeBottomDockDropCleanup(ev);
        return;
      }
      var next = this.normalizeBottomDockOrder(this.bottomDockOrder);
      var fromIndex = next.indexOf(dragId);
      var toIndex = next.indexOf(targetId);
      var placeAfter = Boolean(preferAfter);
      if (fromIndex < 0 || toIndex < 0) {
        this.bottomDockDragId = '';
        this.bottomDockDragStartOrder = [];
        this.bottomDockDragCommitted = false;
        this.reviveBottomDockHoverFromPoint(
          Number(ev && ev.clientX || 0),
          Number(ev && ev.clientY || 0)
        );
        return;
      }
      next.splice(fromIndex, 1);
      if (fromIndex < toIndex) toIndex -= 1;
      if (placeAfter) toIndex += 1;
      if (toIndex < 0) toIndex = 0;
      if (toIndex > next.length) toIndex = next.length;
      next.splice(toIndex, 0, dragId);
      this.bottomDockOrder = next;
      this.persistBottomDockOrder();
      this.bottomDockDragCommitted = true;
      this.completeBottomDockDropCleanup(ev);
    },

    endBottomDockDrag() {
      if (!this.bottomDockDragCommitted && Array.isArray(this.bottomDockDragStartOrder) && this.bottomDockDragStartOrder.length) {
        var current = this.normalizeBottomDockOrder(this.bottomDockOrder);
        var start = this.normalizeBottomDockOrder(this.bottomDockDragStartOrder);
        if (JSON.stringify(current) !== JSON.stringify(start)) {
          this.bottomDockOrder = current;
          this.persistBottomDockOrder();
          this.bottomDockDragCommitted = true;
        } else {
          var beforeRects = this.bottomDockButtonRects();
          this.bottomDockOrder = start;
          this.animateBottomDockFromRects(beforeRects);
        }
      }
      this.bottomDockDragId = '';
      this.bottomDockHoverId = '';
      this.bottomDockDragStartOrder = [];
      this.bottomDockDragCommitted = false;
      this._bottomDockSuppressClickUntil = Date.now() + 220;
      this.cleanupBottomDockDragGhost();
    },

    dashboardPopupOrigin(overrides) {
      return Object.assign({
        source: '',
        active: false,
        ready: false,
        side: 'top',
        left: 0,
        top: 0,
        compact: false,
        title: '',
        body: '',
        meta_origin: '',
        meta_time: '',
        unread: false
      }, overrides || {});
    },

    bottomDockPopupOrigin() {
      var label = String(this.bottomDockPreviewText || '').trim();
      var left = Math.round(Number(this.bottomDockPreviewX || 0));
      var top = Math.round(Number(this.bottomDockPreviewY || 0));
      if (!this.bottomDockPreviewVisible || !label) return this.dashboardPopupOrigin();
      return this.dashboardPopupOrigin({
        source: 'bottom_dock',
        active: true,
        ready: left > 0 && top > 0,
        side: this.bottomDockOpenSide(),
        left: left,
        top: top,
        compact: false,
        title: label
      });
    },

    dashboardPopupStateOrigin() {
      var popup = this.dashboardPopup || {};
      var title = String(popup.title || '').trim();
      var body = String(popup.body || '').trim();
      var left = Math.round(Number(popup.left || 0));
      var top = Math.round(Number(popup.top || 0));
      var side = String(popup.side || 'bottom').trim().toLowerCase();
      if (side !== 'top' && side !== 'left' && side !== 'right') side = 'bottom';
      if (!popup.active || !title) return this.dashboardPopupOrigin();
      return this.dashboardPopupOrigin({
        source: String(popup.source || 'ui').trim(),
        active: true,
        ready: left > 0 && top > 0,
        side: side,
        left: left,
        top: top,
        compact: false,
        title: title,
        body: body,
        meta_origin: String(popup.meta_origin || '').trim(),
        meta_time: String(popup.meta_time || '').trim(),
        unread: !!popup.unread
      });
    },

    activeDashboardPopupOrigin() {
      var sharedPopup = this.dashboardPopupStateOrigin();
      if (sharedPopup.active && sharedPopup.ready) return sharedPopup;
      var dockPopup = this.bottomDockPopupOrigin();
      if (dockPopup.active && dockPopup.ready) return dockPopup;
      return this.dashboardPopupOrigin();
    },

    isDashboardPopupVisible() {
      var popup = this.activeDashboardPopupOrigin();
      return !!(popup.active && popup.ready && popup.title);
    },

    dashboardPopupOverlayClass() {
      var popup = this.activeDashboardPopupOrigin();
      return {
        'is-visible': !!(popup.active && popup.ready && popup.title),
        'is-side-top': popup.side === 'top',
        'is-side-bottom': popup.side === 'bottom',
        'is-side-left': popup.side === 'left',
        'is-side-right': popup.side === 'right',
        'is-unread': !!popup.unread
      };
    },

    dashboardPopupOverlayStyle() {
      var popup = this.activeDashboardPopupOrigin();
      if (!popup.active || !popup.ready) return 'left:-9999px;top:-9999px;';
      return 'left:' + Math.round(Number(popup.left || 0)) + 'px;top:' + Math.round(Number(popup.top || 0)) + 'px;';
    },
