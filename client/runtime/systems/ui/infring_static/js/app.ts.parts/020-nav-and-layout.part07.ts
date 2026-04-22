    readChatSidebarElement() {
      if (typeof document === 'undefined' || typeof document.querySelector !== 'function') return null;
      try { return document.querySelector('.sidebar'); } catch(_) {}
      return null;
    },
    readChatSidebarHeight() {
      var node = this.readChatSidebarElement();
      var height = Number(node && node.offsetHeight || 0);
      if (!Number.isFinite(height) || height <= 0) {
        height = Math.max(180, Math.round(this.taskbarReadViewportHeight() * 0.52));
      }
      return height;
    },
    readChatSidebarWidth() {
      var node = this.readChatSidebarElement();
      var width = Number(node && node.offsetWidth || 0);
      if (Number.isFinite(width) && width > 0) return width;
      var fallback = this.sidebarCollapsed ? 72 : 248;
      if (typeof window !== 'undefined' && typeof window.getComputedStyle === 'function' && document && document.documentElement) {
        try {
          var key = this.sidebarCollapsed ? '--sidebar-collapsed' : '--sidebar-width';
          var raw = String(window.getComputedStyle(document.documentElement).getPropertyValue(key) || '').trim();
          var parsed = parseFloat(raw);
          if (Number.isFinite(parsed) && parsed > 0) fallback = parsed;
        } catch(_) {}
      }
      return Math.max(1, Math.round(fallback));
    },
    readChatSidebarPulltabWidth() {
      var fallback = 22;
      if (typeof window !== 'undefined' && typeof window.getComputedStyle === 'function' && document && document.documentElement) {
        try {
          var raw = String(window.getComputedStyle(document.documentElement).getPropertyValue('--sidebar-pulltab-width') || '').trim();
          var parsed = parseFloat(raw);
          if (Number.isFinite(parsed) && parsed > 0) fallback = parsed;
        } catch(_) {}
      }
      return Math.max(1, Math.round(fallback));
    },
    chatSidebarClampLeft(leftRaw) {
      var wallGap = this.overlayWallGapPx();
      var minLeft = wallGap;
      var maxLeft = this.chatOverlayViewportWidth() - wallGap - this.readChatSidebarWidth() - this.readChatSidebarPulltabWidth();
      if (!Number.isFinite(maxLeft) || maxLeft < minLeft) maxLeft = minLeft;
      var left = Number(leftRaw);
      if (!Number.isFinite(left)) left = minLeft;
      return Math.max(minLeft, Math.min(maxLeft, left));
    },
    chatSidebarHardBounds() {
      return this.dragSurfaceHardBounds(this.readChatSidebarWidth(), this.readChatSidebarHeight());
    },
    chatSidebarWallLockNormalized() {
      var wall = this.dragSurfaceNormalizeWall(this.chatSidebarWallLock);
      return wall === 'left' || wall === 'right' ? wall : '';
    },
    chatSidebarSetWallLock(wallRaw) {
      var wall = this.dragSurfaceNormalizeWall(wallRaw);
      if (wall !== 'left' && wall !== 'right') wall = '';
      this.chatSidebarWallLock = wall;
      try {
        if (wall) localStorage.setItem('infring-chat-sidebar-wall-lock', wall);
        else localStorage.removeItem('infring-chat-sidebar-wall-lock');
        localStorage.removeItem('infring-chat-sidebar-smash-wall');
      } catch(_) {}
      infringUpdateShellLayoutConfig(function(config) {
        config.chatBar.wallLock = wall;
      });
      return wall;
    },
    chatSidebarResolvedLeft() {
      if (this.chatSidebarDragActive) return Number(this.chatSidebarDragLeft || 0);
      var left = this.chatSidebarClampLeft(this.chatSidebarResolvedLeftFromRatio());
      var top = this.chatSidebarClampTop(this.chatSidebarResolvedTopFromRatio());
      var wall = this.chatSidebarWallLockNormalized();
      if (!wall) return left;
      return this.dragSurfaceApplyWallLock(this.chatSidebarHardBounds(), left, top, wall).left;
    },
    chatSidebarPersistPlacementFromLeft(leftRaw) {
      var wallGap = this.overlayWallGapPx();
      var minLeft = wallGap;
      var maxLeft = this.chatOverlayViewportWidth() - wallGap - this.readChatSidebarWidth() - this.readChatSidebarPulltabWidth();
      if (!Number.isFinite(maxLeft) || maxLeft < minLeft) maxLeft = minLeft;
      var left = this.chatSidebarClampLeft(leftRaw);
      var ratio = maxLeft > minLeft ? (left - minLeft) / (maxLeft - minLeft) : 0;
      ratio = Math.max(0, Math.min(1, ratio));
      this.chatSidebarPlacementX = ratio;
      try {
        localStorage.setItem('infring-chat-sidebar-placement-x', String(ratio));
      } catch(_) {}
      infringUpdateShellLayoutConfig(function(config) {
        config.chatBar.placementX = ratio;
      });
    },
    chatSidebarClampTop(topRaw) {
      var bounds = this.chatOverlayVerticalBounds();
      var height = this.readChatSidebarHeight();
      var minTop = Number(bounds.minTop || 0);
      var maxTop = Number(bounds.maxBottom || 0) - height;
      if (!Number.isFinite(maxTop) || maxTop < minTop) maxTop = minTop;
      var top = Number(topRaw);
      if (!Number.isFinite(top)) top = minTop + ((maxTop - minTop) * 0.5);
      return Math.max(minTop, Math.min(maxTop, top));
    },
    chatSidebarResolvedTop() {
      if (this.chatSidebarDragActive) return Number(this.chatSidebarDragTop || 0);
      var left = this.chatSidebarClampLeft(this.chatSidebarResolvedLeftFromRatio());
      var top = this.chatSidebarClampTop(this.chatSidebarResolvedTopFromRatio());
      var wall = this.chatSidebarWallLockNormalized();
      if (!wall) return top;
      return this.dragSurfaceApplyWallLock(this.chatSidebarHardBounds(), left, top, wall).top;
    },
    chatSidebarPersistPlacementFromTop(topRaw) {
      var bounds = this.chatOverlayVerticalBounds();
      var height = this.readChatSidebarHeight();
      var minTop = Number(bounds.minTop || 0);
      var maxTop = Number(bounds.maxBottom || 0) - height;
      if (!Number.isFinite(maxTop) || maxTop < minTop) maxTop = minTop;
      var top = this.chatSidebarClampTop(topRaw);
      this.chatSidebarPlacementTopPx = top;
      try {
        localStorage.setItem('infring-chat-sidebar-placement-top-px', String(top));
      } catch(_) {}
      var ratio = maxTop > minTop ? (top - minTop) / (maxTop - minTop) : 0.5;
      ratio = Math.max(0, Math.min(1, ratio));
      this.chatSidebarPlacementY = ratio;
      try {
        localStorage.setItem('infring-chat-sidebar-placement-y', String(ratio));
      } catch(_) {}
      infringUpdateShellLayoutConfig(function(config) {
        config.chatBar.placementTopPx = top;
        config.chatBar.placementY = ratio;
      });
    },
    chatSidebarContainerStyle() {
      if (this.page !== 'chat') return '';
      var top = this.chatSidebarResolvedTop();
      var left = this.chatSidebarResolvedLeft();
      var durationMs = this.chatSidebarDragActive ? 0 : this.dragSurfaceMoveDurationMs(this._chatSidebarMoveDurationMs, 280);
      var wall = this.chatSidebarWallLockNormalized();
      var lockCss = this.dragSurfaceLockVisualCssVars('chat-sidebar', wall, {
        transformMs: this._dragSurfaceLockTransformMs
      });
      return (
        'position:fixed;' +
        'left:' + Math.round(left) + 'px;' +
        'top:' + Math.round(top) + 'px;' +
        'bottom:auto;' +
        'height:fit-content;' +
        'min-height:calc(56px * 3);' +
        'max-height:80vh;' +
        'transform:none;' +
        '--sidebar-position-transition:' + Math.max(0, Math.round(durationMs)) + 'ms;' +
        lockCss
      );
    },
    chatSidebarNavShellStyle() {
      return this.page === 'chat'
        ? 'flex:0 1 auto;min-height:0;max-height:calc(80vh - 16px);'
        : '';
    },
    chatSidebarNavStyle() {
      return this.page === 'chat'
        ? 'height:auto;flex:0 1 auto;max-height:calc(80vh - 16px);'
        : '';
    },
    chatSidebarPulltabStyle() {
      if (this.page !== 'chat') return '';
      var durationMs = this.chatSidebarDragActive ? 0 : this.dragSurfaceMoveDurationMs(this._chatSidebarMoveDurationMs, 280);
      var wall = this.chatSidebarWallLockNormalized();
      var dockRight = wall === 'right';
      return [
        'position:absolute;',
        'left:' + (dockRight ? 'auto' : '100%') + ';',
        'right:' + (dockRight ? '100%' : 'auto') + ';',
        'top:50%;',
        'transform:translateY(-50%);',
        '--sidebar-position-transition:' + Math.max(0, Math.round(durationMs)) + 'ms;'
      ].join('');
    },
    shouldIgnoreChatSidebarDragTarget(target) {
      var node = target;
      if (node && typeof node.closest !== 'function' && node.parentElement) {
        node = node.parentElement;
      }
      if (!node || typeof node.closest !== 'function') return false;
      if (node.closest('.sidebar-pulltab')) return true;
      return Boolean(
        node.closest(
          'input,textarea,select,[contenteditable="true"],button,a,[role="button"],.nav-item,.nav-agent-row,[data-agent-id]'
        )
      );
    },

    bindChatSidebarPointerListeners() {
      if (this._chatSidebarPointerMoveHandler || this._chatSidebarPointerUpHandler) return;
      var self = this;
      this._chatSidebarPointerMoveHandler = function(ev) { self.handleChatSidebarPointerMove(ev); };
      this._chatSidebarPointerUpHandler = function() { self.endChatSidebarPointerDrag(); };
      var supportsPointer = typeof window !== 'undefined' && ('PointerEvent' in window);
      if (supportsPointer) {
        window.addEventListener('pointermove', this._chatSidebarPointerMoveHandler, true);
        window.addEventListener('pointerup', this._chatSidebarPointerUpHandler, true);
        window.addEventListener('pointercancel', this._chatSidebarPointerUpHandler, true);
      } else {
        window.addEventListener('mousemove', this._chatSidebarPointerMoveHandler, true);
        window.addEventListener('mouseup', this._chatSidebarPointerUpHandler, true);
      }
    },

    unbindChatSidebarPointerListeners() {
      if (this._chatSidebarPointerMoveHandler) {
        try { window.removeEventListener('pointermove', this._chatSidebarPointerMoveHandler, true); } catch(_) {}
        try { window.removeEventListener('mousemove', this._chatSidebarPointerMoveHandler, true); } catch(_) {}
      }
      if (this._chatSidebarPointerUpHandler) {
        try { window.removeEventListener('pointerup', this._chatSidebarPointerUpHandler, true); } catch(_) {}
        try { window.removeEventListener('pointercancel', this._chatSidebarPointerUpHandler, true); } catch(_) {}
        try { window.removeEventListener('mouseup', this._chatSidebarPointerUpHandler, true); } catch(_) {}
      }
      this._chatSidebarPointerMoveHandler = null;
      this._chatSidebarPointerUpHandler = null;
    },

    startChatSidebarPointerDrag(ev) {
      if (!ev || this.page !== 'chat') return;
      if (this._chatSidebarPointerActive) return;
      var button = Number(ev.button);
      if (Number.isFinite(button) && button !== 0) return;
      var target = ev && ev.target ? ev.target : null;
      if (this.shouldIgnoreChatSidebarDragTarget(target)) return;
      this._chatSidebarPointerActive = true;
      this._chatSidebarPointerMoved = false;
      this._chatSidebarPointerStartX = Number(ev.clientX || 0);
      this._chatSidebarPointerStartY = Number(ev.clientY || 0);
      this._chatSidebarPointerOriginLeft = this.chatSidebarResolvedLeft();
      this._chatSidebarPointerOriginTop = this.chatSidebarResolvedTop();
      this._chatSidebarPointerLastX = this._chatSidebarPointerStartX;
      this._chatSidebarPointerLastY = this._chatSidebarPointerStartY;
      this._chatSidebarPointerLastAt = Date.now();
      this._chatSidebarPointerVelocity = 0;
      this.chatSidebarDragLeft = this._chatSidebarPointerOriginLeft;
      this.chatSidebarDragTop = this._chatSidebarPointerOriginTop;
      this.bindChatSidebarPointerListeners();
      try {
        if (ev.currentTarget && typeof ev.currentTarget.setPointerCapture === 'function' && Number.isFinite(ev.pointerId)) {
          ev.currentTarget.setPointerCapture(ev.pointerId);
        }
      } catch(_) {}
    },

    handleChatSidebarPointerMove(ev) {
      if (!this._chatSidebarPointerActive || this.page !== 'chat') return;
      var nextX = Number(ev.clientX || 0);
      var nextY = Number(ev.clientY || 0);
      var now = Date.now();
      var prevX = Number(this._chatSidebarPointerLastX || nextX);
      var prevY = Number(this._chatSidebarPointerLastY || nextY);
      var prevAt = Number(this._chatSidebarPointerLastAt || now);
      var dt = Math.max(1, now - prevAt);
      var stepDx = nextX - prevX;
      var stepDy = nextY - prevY;
      this._chatSidebarPointerVelocity = Math.sqrt((stepDx * stepDx) + (stepDy * stepDy)) / dt;
      this._chatSidebarPointerLastX = nextX;
      this._chatSidebarPointerLastY = nextY;
      this._chatSidebarPointerLastAt = now;
      var movedX = Math.abs(nextX - Number(this._chatSidebarPointerStartX || 0));
      var movedY = Math.abs(nextY - Number(this._chatSidebarPointerStartY || 0));
      if (!this._chatSidebarPointerMoved) {
        if (movedX < 4 && movedY < 4) return;
        this._chatSidebarPointerMoved = true;
        this.chatSidebarDragActive = true;
        this.hideDashboardPopupBySource('sidebar');
      }
      var dragDx = nextX - Number(this._chatSidebarPointerStartX || 0);
      var dragDy = nextY - Number(this._chatSidebarPointerStartY || 0);
      var candidateLeft = Number(this._chatSidebarPointerOriginLeft || 0) + dragDx;
      var candidateTop = Number(this._chatSidebarPointerOriginTop || 0) + dragDy;
      var hardBounds = this.chatSidebarHardBounds();
      var lockedWall = this.chatSidebarWallLockNormalized();
      if (lockedWall) {
        var unlockDistance = this.dragSurfaceDistanceFromWall(hardBounds, candidateLeft, candidateTop, lockedWall);
        if (unlockDistance >= this.dragSurfaceWallUnlockDistanceThreshold()) {
          lockedWall = this.chatSidebarSetWallLock('');
        } else {
          var holdLeft = Number.isFinite(Number(this.chatSidebarDragLeft))
            ? Number(this.chatSidebarDragLeft)
            : Number(this._chatSidebarPointerOriginLeft || 0);
          var holdTop = Number.isFinite(Number(this.chatSidebarDragTop))
            ? Number(this.chatSidebarDragTop)
            : Number(this._chatSidebarPointerOriginTop || 0);
          var stayLocked = this.dragSurfaceApplyWallLock(hardBounds, holdLeft, holdTop, lockedWall);
          this.chatSidebarDragLeft = stayLocked.left;
          this.chatSidebarDragTop = stayLocked.top;
          if (ev.cancelable && typeof ev.preventDefault === 'function') ev.preventDefault();
          return;
        }
      }
      var clamped = this.dragSurfaceClampWithBounds(hardBounds, candidateLeft, candidateTop);
      var nearest = this.dragSurfaceNearestWall(hardBounds, clamped.left, clamped.top);
      var lockWall = this.dragSurfaceResolveWallLock(
        hardBounds,
        candidateLeft,
        candidateTop,
        nearest,
        dragDx,
        dragDy
      );
      if (lockWall) {
        var persistedLockWall = this.chatSidebarSetWallLock(lockWall);
        var snapped = this.dragSurfaceApplyWallLock(hardBounds, clamped.left, clamped.top, persistedLockWall);
        this.chatSidebarDragLeft = snapped.left;
        this.chatSidebarDragTop = snapped.top;
      } else {
        this.chatSidebarDragLeft = clamped.left;
        this.chatSidebarDragTop = clamped.top;
      }
      if (ev.cancelable && typeof ev.preventDefault === 'function') ev.preventDefault();
    },

    endChatSidebarPointerDrag() {
      if (!this._chatSidebarPointerActive) return;
      this._chatSidebarPointerActive = false;
      this.unbindChatSidebarPointerListeners();
      if (!this._chatSidebarPointerMoved) {
        this.chatSidebarDragActive = false;
        this._chatSidebarDragRowsCache = null;
        return;
      }
      this._chatSidebarPointerMoved = false;
      var hardBounds = this.chatSidebarHardBounds();
      var lockedWall = this.chatSidebarWallLockNormalized();
      var final;
      if (lockedWall) {
        final = this.dragSurfaceApplyWallLock(hardBounds, this.chatSidebarDragLeft, this.chatSidebarDragTop, lockedWall);
      } else {
        final = this.dragSurfaceClampWithBounds(hardBounds, this.chatSidebarDragLeft, this.chatSidebarDragTop);
      }
      this.chatSidebarPlacementAnchorId = '';
      try { localStorage.removeItem('infring-chat-sidebar-placement-anchor'); } catch(_) {}
      this.chatSidebarDragLeft = final.left;
      this.chatSidebarDragTop = final.top;
      this.chatSidebarPersistPlacementFromLeft(this.chatSidebarDragLeft);
      this.chatSidebarPersistPlacementFromTop(this.chatSidebarDragTop);
      this.chatSidebarDragActive = false;
      this._chatSidebarDragRowsCache = null;
      this._sidebarToggleSuppressUntil = Date.now() + 260;
    },

    shouldSuppressSidebarToggle() {
      var until = Number(this._sidebarToggleSuppressUntil || 0);
      return Number.isFinite(until) && until > Date.now();
    },

    popupWindowStorageKey(kind, axis) {
      var key = String(kind || '').trim().toLowerCase();
      var lane = String(axis || '').trim().toLowerCase() === 'top' ? 'top' : 'left';
      return 'infring-popup-window-' + (key || 'manual') + '-' + lane;
    },
    popupWindowWallLockStorageKey(kind) {
      var key = String(kind || '').trim().toLowerCase() || 'manual';
      return 'infring-popup-window-' + key + '-wall-lock';
    },
    popupWindowWallLock(kind) {
      void kind;
      return '';
    },
    popupWindowSetWallLock(kind, wallRaw) {
      var key = String(kind || '').trim().toLowerCase();
      void wallRaw;
      if (!key) return '';
      if (!this.popupWindowWallLocks || typeof this.popupWindowWallLocks !== 'object') {
        this.popupWindowWallLocks = {};
      }
      this.popupWindowWallLocks[key] = '';
      try {
        localStorage.removeItem(this.popupWindowWallLockStorageKey(key));
        localStorage.removeItem('infring-popup-window-' + key + '-smash-wall');
      } catch(_) {}
      return '';
    },

    popupWindowOpenState(kind) {
      var key = String(kind || '').trim().toLowerCase();
      if (key === 'report') return !!this.reportIssueWindowOpen;
      return !!this.helpManualWindowOpen;
    },

    popupWindowSetOpenState(kind, open) {
      var key = String(kind || '').trim().toLowerCase();
      var nextOpen = open !== false;
      if (key === 'report') {
        this.reportIssueWindowOpen = nextOpen;
        return;
      }
      this.helpManualWindowOpen = nextOpen;
    },

    readPopupWindowElement(kind) {
      if (typeof document === 'undefined' || typeof document.querySelector !== 'function') return null;
      var key = String(kind || '').trim().toLowerCase();
      if (!key) return null;
      try {
        return document.querySelector('.popup-window[data-popup-window-kind="' + key + '"]');
      } catch(_) {}
      return null;
    },

    popupWindowDefaultSize(kind) {
      var key = String(kind || '').trim().toLowerCase();
      if (key === 'report') return { width: 540, height: 360 };
      return { width: 760, height: 560 };
    },

    readPopupWindowSize(kind) {
      var node = this.readPopupWindowElement(kind);
      var fallback = this.popupWindowDefaultSize(kind);
      var width = Number(node && node.offsetWidth || 0);
      var height = Number(node && node.offsetHeight || 0);
      if (!Number.isFinite(width) || width <= 0) width = Number(fallback.width || 640);
      if (!Number.isFinite(height) || height <= 0) height = Number(fallback.height || 420);
      return {
        width: Math.max(280, Math.round(width)),
        height: Math.max(180, Math.round(height))
      };
    },

    popupWindowBounds(kind, widthRaw, heightRaw) {
      void kind;
      var wallGap = this.overlayWallGapPx();
      var width = Number(widthRaw || 0);
      var height = Number(heightRaw || 0);
      if (!Number.isFinite(width) || width <= 0) width = 640;
      if (!Number.isFinite(height) || height <= 0) height = 420;
      var minLeft = wallGap;
      var maxLeft = this.chatOverlayViewportWidth() - wallGap - width;
      if (!Number.isFinite(maxLeft) || maxLeft < minLeft) maxLeft = minLeft;
      var vertical = this.chatOverlayVerticalBounds();
      var minTop = Number(vertical && vertical.minTop || wallGap) + 2;
      var maxTop = Number(vertical && vertical.maxBottom || this.taskbarReadViewportHeight()) - wallGap - height;
      if (!Number.isFinite(maxTop) || maxTop < minTop) maxTop = minTop;
      return {
        minLeft: minLeft,
        maxLeft: maxLeft,
        minTop: minTop,
        maxTop: maxTop
      };
    },

    popupWindowClampPlacement(kind, leftRaw, topRaw) {
      var size = this.readPopupWindowSize(kind);
      var bounds = this.popupWindowBounds(kind, size.width, size.height);
      var left = Number(leftRaw);
      var top = Number(topRaw);
      if (!Number.isFinite(left)) left = bounds.minLeft + ((bounds.maxLeft - bounds.minLeft) * 0.5);
      if (!Number.isFinite(top)) top = bounds.minTop + ((bounds.maxTop - bounds.minTop) * 0.48);
      return {
        left: Math.max(bounds.minLeft, Math.min(bounds.maxLeft, left)),
        top: Math.max(bounds.minTop, Math.min(bounds.maxTop, top))
      };
    },
    popupWindowHardBounds(kind) {
      var size = this.readPopupWindowSize(kind);
      return this.dragSurfaceHardBounds(size.width, size.height);
    },

    popupWindowEnsurePlacement(kind, forceCenter) {
      var key = String(kind || '').trim().toLowerCase();
      if (!key) return { left: 0, top: 0 };
      if (forceCenter) {
        var centerSize = this.readPopupWindowSize(key);
        var centerBounds = this.popupWindowBounds(key, centerSize.width, centerSize.height);
        var centerPoint = this.dragSurfaceCenteredPoint(centerBounds);
        var centered = this.popupWindowClampPlacement(key, centerPoint.left, centerPoint.top);
        if (!this.popupWindowPlacements || typeof this.popupWindowPlacements !== 'object') {
          this.popupWindowPlacements = {};
        }
        this.popupWindowPlacements[key] = { left: centered.left, top: centered.top };
        return centered;
      }
      var map = (this.popupWindowPlacements && typeof this.popupWindowPlacements === 'object')
        ? this.popupWindowPlacements
        : {};
      var row = map[key] && typeof map[key] === 'object' ? map[key] : { left: null, top: null };
      var left = Number(row.left);
      var top = Number(row.top);
      var hasStored = Number.isFinite(left) && Number.isFinite(top);
      if (!hasStored) {
        try {
          left = Number(localStorage.getItem(this.popupWindowStorageKey(key, 'left')));
          top = Number(localStorage.getItem(this.popupWindowStorageKey(key, 'top')));
        } catch(_) {}
      }
      if (!Number.isFinite(left) || !Number.isFinite(top)) {
        var size = this.readPopupWindowSize(key);
        var bounds = this.popupWindowBounds(key, size.width, size.height);
        left = bounds.minLeft + ((bounds.maxLeft - bounds.minLeft) * 0.5);
        top = bounds.minTop + ((bounds.maxTop - bounds.minTop) * (key === 'report' ? 0.56 : 0.44));
      }
      var clamped = this.popupWindowClampPlacement(key, left, top);
      if (!this.popupWindowPlacements || typeof this.popupWindowPlacements !== 'object') {
        this.popupWindowPlacements = {};
      }
      this.popupWindowPlacements[key] = { left: clamped.left, top: clamped.top };
      return clamped;
    },

    popupWindowPersistPlacement(kind, leftRaw, topRaw) {
      var key = String(kind || '').trim().toLowerCase();
      if (!key) return;
      var clamped = this.popupWindowClampPlacement(key, leftRaw, topRaw);
      if (!this.popupWindowPlacements || typeof this.popupWindowPlacements !== 'object') {
        this.popupWindowPlacements = {};
      }
      this.popupWindowPlacements[key] = { left: clamped.left, top: clamped.top };
      try {
        localStorage.setItem(this.popupWindowStorageKey(key, 'left'), String(clamped.left));
        localStorage.setItem(this.popupWindowStorageKey(key, 'top'), String(clamped.top));
      } catch(_) {}
    },

    popupWindowResolvedLeft(kind) {
      var key = String(kind || '').trim().toLowerCase();
      if (!key) return this.overlayWallGapPx();
      if (this.popupWindowDragActive && this.popupWindowDragKind === key) {
        return Number(this.popupWindowDragLeft || 0);
      }
      var base = this.popupWindowEnsurePlacement(key);
      return this.popupWindowClampPlacement(key, base.left, base.top).left;
    },

    popupWindowResolvedTop(kind) {
      var key = String(kind || '').trim().toLowerCase();
      if (!key) return this.overlayWallGapPx();
      if (this.popupWindowDragActive && this.popupWindowDragKind === key) {
        return Number(this.popupWindowDragTop || 0);
      }
      var base = this.popupWindowEnsurePlacement(key);
      return this.popupWindowClampPlacement(key, base.left, base.top).top;
    },

    popupWindowStyle(kind) {
      var key = String(kind || '').trim().toLowerCase();
      if (!key || !this.popupWindowOpenState(key)) return 'display:none;';
      var left = this.popupWindowResolvedLeft(key);
      var top = this.popupWindowResolvedTop(key);
      var durationMs = (this.popupWindowDragActive && this.popupWindowDragKind === key)
        ? 0
        : this.dragSurfaceMoveDurationMs(this._popupWindowMoveDurationMs, 260);
      return (
        'left:' + Math.round(left) + 'px;' +
        'top:' + Math.round(top) + 'px;' +
        'transition:left ' + Math.max(0, Math.round(durationMs)) + 'ms var(--ease-smooth), top ' + Math.max(0, Math.round(durationMs)) + 'ms var(--ease-smooth);'
      );
    },

    openPopupWindow(kind) {
      var key = String(kind || '').trim().toLowerCase();
      if (!key) return;
      this.popupWindowSetOpenState(key, true);
      this.popupWindowSetWallLock(key, '');
      this.popupWindowEnsurePlacement(key, true);
      var self = this;
      this.$nextTick(function() {
        self.popupWindowEnsurePlacement(key, true);
      });
    },

    closePopupWindow(kind) {
      var key = String(kind || '').trim().toLowerCase();
      if (!key) return;
      if (this._popupWindowPointerActive && this.popupWindowDragKind === key) {
        this.endPopupWindowPointerDrag();
      }
      this.popupWindowSetOpenState(key, false);
    },

    bindPopupWindowPointerListeners() {
      if (this._popupWindowPointerMoveHandler || this._popupWindowPointerUpHandler) return;
      var self = this;
      this._popupWindowPointerMoveHandler = function(ev) { self.handlePopupWindowPointerMove(ev); };
      this._popupWindowPointerUpHandler = function() { self.endPopupWindowPointerDrag(); };
      window.addEventListener('pointermove', this._popupWindowPointerMoveHandler, true);
      window.addEventListener('pointerup', this._popupWindowPointerUpHandler, true);
      window.addEventListener('pointercancel', this._popupWindowPointerUpHandler, true);
      window.addEventListener('mousemove', this._popupWindowPointerMoveHandler, true);
      window.addEventListener('mouseup', this._popupWindowPointerUpHandler, true);
    },

    unbindPopupWindowPointerListeners() {
      if (this._popupWindowPointerMoveHandler) {
        try { window.removeEventListener('pointermove', this._popupWindowPointerMoveHandler, true); } catch(_) {}
        try { window.removeEventListener('mousemove', this._popupWindowPointerMoveHandler, true); } catch(_) {}
      }
      if (this._popupWindowPointerUpHandler) {
        try { window.removeEventListener('pointerup', this._popupWindowPointerUpHandler, true); } catch(_) {}
        try { window.removeEventListener('pointercancel', this._popupWindowPointerUpHandler, true); } catch(_) {}
        try { window.removeEventListener('mouseup', this._popupWindowPointerUpHandler, true); } catch(_) {}
      }
      this._popupWindowPointerMoveHandler = null;
      this._popupWindowPointerUpHandler = null;
    },

    startPopupWindowPointerDrag(kind, ev) {
      var key = String(kind || '').trim().toLowerCase();
      if (!ev || !key || !this.popupWindowOpenState(key)) return;
      var button = Number(ev.button);
      if (Number.isFinite(button) && button !== 0) return;
      var target = ev && ev.target ? ev.target : null;
      if (target && typeof target.closest === 'function') {
        if (target.closest('button, input, textarea, select, a, [contenteditable="true"]')) return;
      }
      this._popupWindowPointerActive = true;
      this._popupWindowPointerMoved = false;
      this.popupWindowDragKind = key;
      this._popupWindowPointerStartX = Number(ev.clientX || 0);
      this._popupWindowPointerStartY = Number(ev.clientY || 0);
      this._popupWindowPointerOriginLeft = this.popupWindowResolvedLeft(key);
      this._popupWindowPointerOriginTop = this.popupWindowResolvedTop(key);
      this._popupWindowPointerLastX = this._popupWindowPointerStartX;
      this._popupWindowPointerLastY = this._popupWindowPointerStartY;
      this._popupWindowPointerLastAt = Date.now();
      this._popupWindowPointerVelocity = 0;
      this.popupWindowDragLeft = this._popupWindowPointerOriginLeft;
      this.popupWindowDragTop = this._popupWindowPointerOriginTop;
      this.popupWindowDragWallLock = '';
      this.bindPopupWindowPointerListeners();
      try {
        if (ev.currentTarget && typeof ev.currentTarget.setPointerCapture === 'function' && Number.isFinite(ev.pointerId)) {
          ev.currentTarget.setPointerCapture(ev.pointerId);
        }
      } catch(_) {}
    },

    handlePopupWindowPointerMove(ev) {
      if (!this._popupWindowPointerActive) return;
      var key = String(this.popupWindowDragKind || '').trim().toLowerCase();
      if (!key || !this.popupWindowOpenState(key)) return;
      var nextX = Number(ev.clientX || 0);
      var nextY = Number(ev.clientY || 0);
      var now = Date.now();
      var prevX = Number(this._popupWindowPointerLastX || nextX);
      var prevY = Number(this._popupWindowPointerLastY || nextY);
      var prevAt = Number(this._popupWindowPointerLastAt || now);
      var dt = Math.max(1, now - prevAt);
      var stepDx = nextX - prevX;
      var stepDy = nextY - prevY;
      this._popupWindowPointerVelocity = Math.sqrt((stepDx * stepDx) + (stepDy * stepDy)) / dt;
      this._popupWindowPointerLastX = nextX;
      this._popupWindowPointerLastY = nextY;
      this._popupWindowPointerLastAt = now;
      var movedX = Math.abs(nextX - Number(this._popupWindowPointerStartX || 0));
      var movedY = Math.abs(nextY - Number(this._popupWindowPointerStartY || 0));
      if (!this._popupWindowPointerMoved) {
        if (movedX < 4 && movedY < 4) return;
        this._popupWindowPointerMoved = true;
        this.popupWindowDragActive = true;
      }
      var dragDx = nextX - Number(this._popupWindowPointerStartX || 0);
      var dragDy = nextY - Number(this._popupWindowPointerStartY || 0);
      var candidateLeft = Number(this._popupWindowPointerOriginLeft || 0) + dragDx;
      var candidateTop = Number(this._popupWindowPointerOriginTop || 0) + dragDy;
      var hardBounds = this.popupWindowHardBounds(key);
      var clamped = this.dragSurfaceClampWithBounds(hardBounds, candidateLeft, candidateTop);
      this.popupWindowDragLeft = clamped.left;
      this.popupWindowDragTop = clamped.top;
      if (ev.cancelable && typeof ev.preventDefault === 'function') ev.preventDefault();
    },

    endPopupWindowPointerDrag() {
      if (!this._popupWindowPointerActive) return;
      var key = String(this.popupWindowDragKind || '').trim().toLowerCase();
      var moved = !!this._popupWindowPointerMoved;
      this._popupWindowPointerActive = false;
      this._popupWindowPointerMoved = false;
      this.unbindPopupWindowPointerListeners();
      if (key && moved) {
        var hardBounds = this.popupWindowHardBounds(key);
        var finalPlacement = this.dragSurfaceClampWithBounds(hardBounds, this.popupWindowDragLeft, this.popupWindowDragTop);
        this.popupWindowDragLeft = finalPlacement.left;
        this.popupWindowDragTop = finalPlacement.top;
        this.popupWindowPersistPlacement(key, this.popupWindowDragLeft, this.popupWindowDragTop);
      }
      this.popupWindowDragActive = false;
      this.popupWindowDragWallLock = '';
      this.popupWindowDragKind = '';
    },

    bottomDockDefaultOrder() {
      var registry = (this.bottomDockTileConfig && typeof this.bottomDockTileConfig === 'object')
        ? this.bottomDockTileConfig
        : null;
      if (registry) {
        var ids = Object.keys(registry);
        if (ids.length) return ids;
      }
      return ['chat', 'overview', 'agents', 'scheduler', 'skills', 'runtime', 'settings'];
    },

    bottomDockTileConfigById(id) {
      var key = String(id || '').trim();
      if (!key) return null;
      var registry = (this.bottomDockTileConfig && typeof this.bottomDockTileConfig === 'object')
        ? this.bottomDockTileConfig
        : null;
      var tile = registry && Object.prototype.hasOwnProperty.call(registry, key) ? registry[key] : null;
      return tile && typeof tile === 'object' ? tile : null;
    },

    bottomDockTileData(id, field, fallback) {
      var key = String(field || '').trim();
      var tile = this.bottomDockTileConfigById(id);
      var value = (key && tile && Object.prototype.hasOwnProperty.call(tile, key)) ? tile[key] : fallback;
      return (value === undefined || value === null) ? String(fallback || '') : String(value);
    },

    bottomDockTileAnimationName(id) {
      var tile = this.bottomDockTileConfigById(id);
      var animation = tile && Array.isArray(tile.animation) ? tile.animation : null;
      var name = animation ? String(animation[0] || '').trim() : '';
      return name || 'none';
    },

    bottomDockTileAnimationDurationAttr(id) {
      var tile = this.bottomDockTileConfigById(id);
      var animation = tile && Array.isArray(tile.animation) ? tile.animation : null;
      if (!animation) return null;
      var durationMs = Number(animation[1]);
      if (!Number.isFinite(durationMs) || durationMs < 120) return null;
      return String(Math.round(durationMs));
    },

    bottomDockSlotStyle(id) {
      var key = String(id || '').trim();
      var order = key ? this.bottomDockOrderIndex(key) : 999;
      var weight = this.bottomDockHoverWeight(key);
      if (!Number.isFinite(weight) || weight < 0) weight = 0;
      if (weight > 1) weight = 1;
      return 'order:' + order + ';--bottom-dock-hover-weight:' + weight.toFixed(4);
    },

    bottomDockTileStyle(id) {
      var key = String(id || '').trim();
      var tile = this.bottomDockTileConfigById(key);
      var style = tile && typeof tile.style === 'string' ? String(tile.style || '').trim() : '';
      return style || '';
    },

    normalizeBottomDockOrder(rawOrder) {
      var defaults = this.bottomDockDefaultOrder();
      var source = Array.isArray(rawOrder) ? rawOrder : [];
      var seen = {};
      var ordered = [];
      for (var i = 0; i < source.length; i++) {
        var id = String(source[i] || '').trim();
        if (!id || seen[id] || defaults.indexOf(id) < 0) continue;
        seen[id] = true;
        ordered.push(id);
      }
      for (var j = 0; j < defaults.length; j++) {
        var fallbackId = defaults[j];
        if (seen[fallbackId]) continue;
        seen[fallbackId] = true;
        ordered.push(fallbackId);
      }
      return ordered;
    },

    persistBottomDockOrder() {
      this.bottomDockOrder = this.normalizeBottomDockOrder(this.bottomDockOrder);
      try {
        localStorage.setItem('infring-bottom-dock-order', JSON.stringify(this.bottomDockOrder));
      } catch(_) {}
      infringUpdateShellLayoutConfig(function(config) {
        config.dock.order = this.bottomDockOrder.slice();
      }.bind(this));
    },

    bottomDockOrderIndex(id) {
      var key = String(id || '').trim();
      if (!key) return 999;
      var order = this.normalizeBottomDockOrder(this.bottomDockOrder);
      var idx = order.indexOf(key);
      if (idx >= 0) return idx;
      var fallback = this.bottomDockDefaultOrder().indexOf(key);
      return fallback >= 0 ? fallback : 999;
    },

    bottomDockAxisBasis(sideHint) {
      var rotationDeg = this.bottomDockRotationDegResolved(sideHint);
      var theta = (Number(rotationDeg || 0) * Math.PI) / 180;
      var ux = Math.cos(theta);
      var uy = Math.sin(theta);
      if (Math.abs(ux) < 0.0001) ux = 0;
      if (Math.abs(uy) < 0.0001) uy = 0;
      return { ux: ux, uy: uy, vx: -uy, vy: ux };
    },

    bottomDockProjectPointToAxis(x, y, basis) {
      var axis = basis && typeof basis === 'object'
        ? basis
        : this.bottomDockAxisBasis();
      var ux = Number(axis.ux || 0);
      var uy = Number(axis.uy || 0);
      var vx = Number(axis.vx || (-uy));
      var vy = Number(axis.vy || ux);
      var px = Number(x || 0);
      var py = Number(y || 0);
      return {
        primary: (px * ux) + (py * uy),
        secondary: (px * vx) + (py * vy)
      };
    },

    bottomDockAxisHalfExtent(width, height, basis) {
      var axis = basis && typeof basis === 'object'
        ? basis
        : this.bottomDockAxisBasis();
      var w = Number(width || 0);
      var h = Number(height || 0);
      if (!Number.isFinite(w) || w < 0) w = 0;
      if (!Number.isFinite(h) || h < 0) h = 0;
      var ux = Math.abs(Number(axis.ux || 0));
      var uy = Math.abs(Number(axis.uy || 0));
      var vx = Math.abs(Number(axis.vx || 0));
      var vy = Math.abs(Number(axis.vy || 0));
      return {
        primary: ((ux * w) + (uy * h)) / 2,
        secondary: ((vx * w) + (vy * h)) / 2
      };
    },

    bottomDockProjectedRectBounds(rect, basis) {
      if (!rect) return null;
      var axis = basis && typeof basis === 'object'
        ? basis
        : this.bottomDockAxisBasis();
      var left = Number(rect.left || 0);
      var top = Number(rect.top || 0);
      var right = Number(rect.right || left);
      var bottom = Number(rect.bottom || top);
      var p1 = this.bottomDockProjectPointToAxis(left, top, axis);
      var p2 = this.bottomDockProjectPointToAxis(right, top, axis);
      var p3 = this.bottomDockProjectPointToAxis(left, bottom, axis);
      var p4 = this.bottomDockProjectPointToAxis(right, bottom, axis);
      var primaryMin = Math.min(p1.primary, p2.primary, p3.primary, p4.primary);
      var primaryMax = Math.max(p1.primary, p2.primary, p3.primary, p4.primary);
      var secondaryMin = Math.min(p1.secondary, p2.secondary, p3.secondary, p4.secondary);
      var secondaryMax = Math.max(p1.secondary, p2.secondary, p3.secondary, p4.secondary);
      return {
        primaryMin: primaryMin,
        primaryMax: primaryMax,
        secondaryMin: secondaryMin,
        secondaryMax: secondaryMax
      };
    },

    bottomDockButtonRects() {
      var out = {};
      var root = document.querySelector('.bottom-dock');
      if (!root) return out;
      var nodes = root.querySelectorAll('.bottom-dock-btn[data-dock-id]');
      for (var i = 0; i < nodes.length; i++) {
        var node = nodes[i];
        if (!node) continue;
        var id = String(node.getAttribute('data-dock-id') || '').trim();
        if (!id) continue;
        var rect = node.getBoundingClientRect();
        var width = Number(rect.width || 0);
        var height = Number(rect.height || 0);
        var left = Number(rect.left || 0);
        var top = Number(rect.top || 0);
        out[id] = {
          left: left,
          top: top,
          width: width,
          height: height,
          cx: left + (width / 2),
          cy: top + (height / 2)
        };
      }
      return out;
    },

    animateBottomDockFromRects(beforeRects) {
      if (!beforeRects || typeof beforeRects !== 'object') return;
      if (typeof requestAnimationFrame !== 'function') return;
      var durationMs = this.bottomDockMoveDurationMs();
      var self = this;
      requestAnimationFrame(function() {
        var root = document.querySelector('.bottom-dock');
        if (!root) return;
        var rootScale = self.readBottomDockScale(root);
        if (!Number.isFinite(rootScale) || rootScale <= 0.01) rootScale = 1;
        var side = self.bottomDockActiveSide();
        var nodes = root.querySelectorAll('.bottom-dock-btn[data-dock-id]');
        for (var i = 0; i < nodes.length; i++) {
          var node = nodes[i];
          if (!node || node.classList.contains('dragging')) continue;
          var id = String(node.getAttribute('data-dock-id') || '').trim();
          if (!id || !Object.prototype.hasOwnProperty.call(beforeRects, id)) continue;
          var from = beforeRects[id] || {};
          var rect = node.getBoundingClientRect();
          var fromCx = Number(from.cx);
          var fromCy = Number(from.cy);
          if (!Number.isFinite(fromCx)) fromCx = Number(from.left || 0) + (Number(from.width || 0) / 2);
          if (!Number.isFinite(fromCy)) fromCy = Number(from.top || 0) + (Number(from.height || 0) / 2);
          var toCx = Number(rect.left || 0) + (Number(rect.width || 0) / 2);
          var toCy = Number(rect.top || 0) + (Number(rect.height || 0) / 2);
          var screenDx = Number(fromCx || 0) - Number(toCx || 0);
          var screenDy = Number(fromCy || 0) - Number(toCy || 0);
          if (Math.abs(screenDx) < 0.5 && Math.abs(screenDy) < 0.5) continue;
          var localDelta = self.bottomDockScreenDeltaToLocal(screenDx, screenDy, side);
          var tx = Number(localDelta.x || 0) / rootScale;
          var ty = Number(localDelta.y || 0) / rootScale;
          if (Math.abs(tx) < 0.25 && Math.abs(ty) < 0.25) continue;
          node.style.setProperty('--dock-reorder-transition', '0ms');
          node.style.setProperty('--dock-reorder-translate-x', Math.round(tx) + 'px');
          node.style.setProperty('--dock-reorder-translate-y', Math.round(ty) + 'px');
          void node.offsetHeight;
          node.style.setProperty('--dock-reorder-transition', Math.max(0, Math.round(durationMs)) + 'ms');
          node.style.setProperty('--dock-reorder-translate-x', '0px');
          node.style.setProperty('--dock-reorder-translate-y', '0px');
          (function(el) {
            window.setTimeout(function() {
              if (
                !el.classList.contains('dragging') &&
                !el.classList.contains('hovered') &&
                !el.classList.contains('neighbor-hover') &&
                !el.classList.contains('second-neighbor-hover')
              ) {
                el.style.removeProperty('--dock-reorder-translate-x');
                el.style.removeProperty('--dock-reorder-translate-y');
              }
              el.style.removeProperty('--dock-reorder-transition');
            }, durationMs + 30);
          })(node);
        }
      });
    },
