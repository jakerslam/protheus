    applyTaskbarReorder(group, dragItem, targetItem, preferAfter, animate) {
      var key = String(group || '').trim().toLowerCase();
      if (key !== 'right') key = 'left';
      var dragId = String(dragItem || '').trim();
      var targetId = String(targetItem || '').trim();
      if (!dragId || !targetId || dragId === targetId) return false;
      var current = this.normalizeTaskbarReorder(key, this.taskbarReorderOrderForGroup(key));
      var fromIndex = current.indexOf(dragId);
      var toIndex = current.indexOf(targetId);
      if (fromIndex < 0 || toIndex < 0) return false;
      var next = current.slice();
      next.splice(fromIndex, 1);
      if (fromIndex < toIndex) toIndex -= 1;
      if (Boolean(preferAfter)) toIndex += 1;
      if (toIndex < 0) toIndex = 0;
      if (toIndex > next.length) toIndex = next.length;
      next.splice(toIndex, 0, dragId);
      if (JSON.stringify(next) === JSON.stringify(current)) return false;
      var beforeRects = Boolean(animate) ? this.taskbarReorderItemRects(key) : null;
      this.setTaskbarReorderOrderForGroup(key, next);
      if (beforeRects) this.animateTaskbarReorderFromRects(key, beforeRects);
      return true;
    },
    handleTaskbarReorderPointerDown(group, ev) {
      if (String(this.taskbarDragGroup || '').trim()) return;
      if (!ev || Number(ev.button) !== 0) return;
      var key = String(group || '').trim().toLowerCase();
      if (key !== 'right') key = 'left';
      var target = ev && ev.target && typeof ev.target.closest === 'function'
        ? ev.target.closest('.taskbar-reorder-item[data-taskbar-item]')
        : null;
      var item = target ? String(target.getAttribute('data-taskbar-item') || '').trim() : '';
      if (!item) return;
      this.cancelTaskbarDragHold();
      this._taskbarDragHoldGroup = key;
      this._taskbarDragHoldItem = item;
      var self = this;
      if (typeof window !== 'undefined' && typeof window.setTimeout === 'function') {
        this._taskbarDragHoldTimer = window.setTimeout(function() {
          self._taskbarDragHoldTimer = 0;
          self._taskbarDragArmedGroup = key;
          self._taskbarDragArmedItem = item;
        }, 180);
      }
    },
    cancelTaskbarDragHold() {
      if (this._taskbarDragHoldTimer) {
        try { clearTimeout(this._taskbarDragHoldTimer); } catch(_) {}
      }
      this._taskbarDragHoldTimer = 0;
      this._taskbarDragHoldGroup = '';
      this._taskbarDragHoldItem = '';
      if (!String(this.taskbarDragGroup || '').trim()) {
        this._taskbarDragArmedGroup = '';
        this._taskbarDragArmedItem = '';
      }
    },
    forceTaskbarMoveDragEffect(ev) {
      if (!ev || !ev.dataTransfer) return;
      try { ev.dataTransfer.effectAllowed = 'move'; } catch(_) {}
      try { ev.dataTransfer.dropEffect = 'move'; } catch(_) {}
    },
    setTaskbarDragBodyActive(active) {
      if (typeof document === 'undefined' || !document.body || !document.body.classList) return;
      if (active) {
        document.body.classList.add('taskbar-drag-active');
      } else {
        document.body.classList.remove('taskbar-drag-active');
      }
    },
    handleTaskbarReorderDragStart(group, ev) {
      var key = String(group || '').trim().toLowerCase();
      if (key !== 'right') key = 'left';
      var target = ev && ev.target && typeof ev.target.closest === 'function'
        ? ev.target.closest('.taskbar-reorder-item[data-taskbar-item]')
        : null;
      var item = target ? String(target.getAttribute('data-taskbar-item') || '').trim() : '';
      if (!item || this._taskbarDragArmedGroup !== key || this._taskbarDragArmedItem !== item) {
        if (ev && typeof ev.preventDefault === 'function') ev.preventDefault();
        return;
      }
      this.taskbarDragGroup = key;
      this.taskbarDragItem = item;
      this.taskbarDragStartOrder = this.normalizeTaskbarReorder(key, this.taskbarReorderOrderForGroup(key));
      this._taskbarDragArmedGroup = '';
      this._taskbarDragArmedItem = '';
      this.cancelTaskbarDragHold();
      if (ev && ev.dataTransfer) {
        this.forceTaskbarMoveDragEffect(ev);
        try { ev.dataTransfer.setData('application/x-infring-taskbar', key + ':' + item); } catch(_) {}
        try { ev.dataTransfer.setData('text/plain', key + ':' + item); } catch(_) {}
        try {
          if (
            typeof document !== 'undefined' &&
            document.body &&
            typeof ev.dataTransfer.setDragImage === 'function'
          ) {
            var ghost = target && typeof target.cloneNode === 'function'
              ? target.cloneNode(true)
              : document.createElement('span');
            ghost.style.position = 'fixed';
            ghost.style.left = '-9999px';
            ghost.style.top = '-9999px';
            ghost.style.margin = '0';
            ghost.style.pointerEvents = 'none';
            ghost.style.transform = 'none';
            ghost.style.opacity = '1';
            if (ghost.classList && ghost.classList.contains('dragging')) {
              ghost.classList.remove('dragging');
            }
            var rect = target && typeof target.getBoundingClientRect === 'function'
              ? target.getBoundingClientRect()
              : null;
            var offsetX = 0;
            var offsetY = 0;
            if (rect) {
              var width = Math.max(1, Math.round(Number(rect.width || 0)));
              var height = Math.max(1, Math.round(Number(rect.height || 0)));
              ghost.style.width = width + 'px';
              ghost.style.height = height + 'px';
              ghost.style.boxSizing = 'border-box';
              if (typeof ev.clientX === 'number') {
                offsetX = Math.round(Math.max(0, Math.min(width, ev.clientX - rect.left)));
              }
              if (typeof ev.clientY === 'number') {
                offsetY = Math.round(Math.max(0, Math.min(height, ev.clientY - rect.top)));
              }
            } else {
              ghost.style.width = '1px';
              ghost.style.height = '1px';
            }
            document.body.appendChild(ghost);
            ev.dataTransfer.setDragImage(ghost, offsetX, offsetY);
            window.setTimeout(function() {
              if (ghost.parentNode) ghost.parentNode.removeChild(ghost);
            }, 0);
          }
        } catch(_) {}
      }
      if (target && target.classList) target.classList.add('dragging');
      this.setTaskbarDragBodyActive(true);
    },
    handleTaskbarReorderDragMove(ev) {
      this.forceTaskbarMoveDragEffect(ev);
    },
    handleTaskbarReorderDragEnter(group, ev) {
      var key = String(group || '').trim().toLowerCase();
      if (key !== 'right') key = 'left';
      if (String(this.taskbarDragGroup || '').trim() !== key) return;
      if (ev && typeof ev.preventDefault === 'function') ev.preventDefault();
      this.forceTaskbarMoveDragEffect(ev);
    },
    handleTaskbarReorderDragOver(group, ev) {
      var key = String(group || '').trim().toLowerCase();
      if (key !== 'right') key = 'left';
      if (String(this.taskbarDragGroup || '').trim() !== key) return;
      if (ev && typeof ev.preventDefault === 'function') ev.preventDefault();
      this.forceTaskbarMoveDragEffect(ev);
      var dragItem = String(this.taskbarDragItem || '').trim();
      if (!dragItem) return;
      var target = ev && ev.target && typeof ev.target.closest === 'function'
        ? ev.target.closest('.taskbar-reorder-item[data-taskbar-item]')
        : null;
      var targetItem = target ? String(target.getAttribute('data-taskbar-item') || '').trim() : '';
      if (!targetItem || targetItem === dragItem) return;
      var preferAfter = false;
      if (target && typeof target.getBoundingClientRect === 'function') {
        var rect = target.getBoundingClientRect();
        var midX = Number(rect.left || 0) + (Number(rect.width || 0) / 2);
        preferAfter = Number(ev && ev.clientX || 0) >= midX;
      }
      this.applyTaskbarReorder(key, dragItem, targetItem, preferAfter, true);
    },
    clearTaskbarReorderDraggingClass() {
      if (typeof document === 'undefined') return;
      try {
        var draggingNodes = document.querySelectorAll('.taskbar-reorder-item.dragging');
        for (var i = 0; i < draggingNodes.length; i += 1) {
          draggingNodes[i].classList.remove('dragging');
        }
      } catch(_) {}
    },
    handleTaskbarReorderDrop(group, ev) {
      var key = String(group || '').trim().toLowerCase();
      if (key !== 'right') key = 'left';
      if (String(this.taskbarDragGroup || '').trim() !== key) return;
      if (ev && typeof ev.preventDefault === 'function') ev.preventDefault();
      this.persistTaskbarReorder(key);
      this.taskbarDragGroup = '';
      this.taskbarDragItem = '';
      this.taskbarDragStartOrder = [];
      this.cancelTaskbarDragHold();
      this.setTaskbarDragBodyActive(false);
      this.clearTaskbarReorderDraggingClass();
    },
    handleTaskbarDragEnd() {
      var key = String(this.taskbarDragGroup || '').trim();
      if (key) this.persistTaskbarReorder(key);
      this.taskbarDragGroup = '';
      this.taskbarDragItem = '';
      this.taskbarDragStartOrder = [];
      this.cancelTaskbarDragHold();
      this.setTaskbarDragBodyActive(false);
      this.clearTaskbarReorderDraggingClass();
    },
    chatSidebarSnapDefinitions() {
      return [
        { id: 'left-top', x: 0, y: 0 },
        { id: 'left-middle', x: 0, y: 0.5 },
        { id: 'left-bottom', x: 0, y: 1 }
      ];
    },
    chatSidebarSnapDefinitionById(id) {
      var key = String(id || '').trim().toLowerCase();
      var defs = this.chatSidebarSnapDefinitions();
      for (var i = 0; i < defs.length; i += 1) {
        var row = defs[i];
        if (!row || row.id !== key) continue;
        return row;
      }
      return defs[1] || defs[0] || { id: 'left-middle', x: 0, y: 0.5 };
    },
    chatSidebarAnchorForSnapId(id) {
      var snap = this.chatSidebarSnapDefinitionById(id);
      var wallGap = this.overlayWallGapPx();
      var minLeft = wallGap;
      var maxLeft = this.chatOverlayViewportWidth() - wallGap - this.readChatSidebarWidth() - this.readChatSidebarPulltabWidth();
      if (!Number.isFinite(maxLeft) || maxLeft < minLeft) maxLeft = minLeft;
      var bounds = this.chatOverlayVerticalBounds();
      var height = this.readChatSidebarHeight();
      var minTop = Number(bounds.minTop || 0);
      var maxTop = Number(bounds.maxBottom || 0) - height;
      if (!Number.isFinite(maxTop) || maxTop < minTop) maxTop = minTop;
      var nx = Number(snap && snap.x);
      var ny = Number(snap && snap.y);
      if (!Number.isFinite(nx)) nx = 0;
      if (!Number.isFinite(ny)) ny = 0.5;
      nx = Math.max(0, Math.min(1, nx));
      ny = Math.max(0, Math.min(1, ny));
      return {
        id: String(snap && snap.id || 'left-middle'),
        left: this.chatSidebarClampLeft(minLeft + ((maxLeft - minLeft) * nx)),
        top: this.chatSidebarClampTop(minTop + ((maxTop - minTop) * ny))
      };
    },
    chatSidebarNearestSnapId(leftRaw, topRaw) {
      var defs = this.chatSidebarSnapDefinitions();
      if (!defs.length) return 'left-middle';
      var left = this.chatSidebarClampLeft(leftRaw);
      var top = this.chatSidebarClampTop(topRaw);
      var bestId = String(defs[0].id || 'left-middle');
      var bestDist = Number.POSITIVE_INFINITY;
      for (var i = 0; i < defs.length; i += 1) {
        var row = defs[i];
        if (!row) continue;
        var anchor = this.chatSidebarAnchorForSnapId(row.id);
        var dx = Number(left || 0) - Number(anchor.left || 0);
        var dy = Number(top || 0) - Number(anchor.top || 0);
        var dist = (dx * dx) + (dy * dy);
        if (!Number.isFinite(dist) || dist >= bestDist) continue;
        bestDist = dist;
        bestId = String(row.id || bestId);
      }
      return bestId || 'left-middle';
    },
    chatSidebarResolvedLeftFromRatio() {
      var ratio = 0;
      try {
        var raw = Number(localStorage.getItem('infring-chat-sidebar-placement-x'));
        if (Number.isFinite(raw)) ratio = Math.max(0, Math.min(1, raw));
      } catch(_) {}
      if (Number.isFinite(this.chatSidebarPlacementX)) ratio = Math.max(0, Math.min(1, Number(this.chatSidebarPlacementX)));
      var wallGap = this.overlayWallGapPx();
      var minLeft = wallGap;
      var maxLeft = this.chatOverlayViewportWidth() - wallGap - this.readChatSidebarWidth() - this.readChatSidebarPulltabWidth();
      if (!Number.isFinite(maxLeft) || maxLeft < minLeft) maxLeft = minLeft;
      return this.chatSidebarClampLeft(minLeft + ((maxLeft - minLeft) * ratio));
    },
    chatSidebarResolvedTopFromRatio() {
      var topPx = Number(this.chatSidebarPlacementTopPx);
      if (!Number.isFinite(topPx)) {
        try {
          var rawTop = Number(localStorage.getItem('infring-chat-sidebar-placement-top-px'));
          if (Number.isFinite(rawTop)) topPx = rawTop;
        } catch(_) {}
      }
      if (Number.isFinite(topPx)) {
        return this.chatSidebarClampTop(topPx);
      }
      var bounds = this.chatOverlayVerticalBounds();
      var height = this.readChatSidebarHeight();
      var minTop = Number(bounds.minTop || 0);
      var maxTop = Number(bounds.maxBottom || 0) - height;
      if (!Number.isFinite(maxTop) || maxTop < minTop) maxTop = minTop;
      var ratio = Number(this.chatSidebarPlacementY);
      if (!Number.isFinite(ratio)) ratio = 0.5;
      ratio = Math.max(0, Math.min(1, ratio));
      return this.chatSidebarClampTop(minTop + ((maxTop - minTop) * ratio));
    },
    chatSidebarActiveSnapId() {
      if (this.chatSidebarDragActive) {
        return this.chatSidebarNearestSnapId(this.chatSidebarDragLeft, this.chatSidebarDragTop);
      }
      var storedId = String(this.chatSidebarPlacementAnchorId || '').trim().toLowerCase();
      if (!storedId) {
        try {
          var raw = String(localStorage.getItem('infring-chat-sidebar-placement-anchor') || '').trim().toLowerCase();
          if (raw) storedId = raw;
        } catch(_) {}
      }
      if (storedId) return this.chatSidebarSnapDefinitionById(storedId).id;
      var fallbackLeft = this.chatSidebarClampLeft(this.chatSidebarResolvedLeftFromRatio());
      var fallbackTop = this.chatSidebarClampTop(this.chatSidebarResolvedTopFromRatio());
      return this.chatSidebarNearestSnapId(fallbackLeft, fallbackTop);
    },
    chatSidebarPersistSnapId(id) {
      var snap = this.chatSidebarSnapDefinitionById(id);
      this.chatSidebarPlacementAnchorId = String(snap && snap.id || 'left-middle');
      try {
        localStorage.setItem('infring-chat-sidebar-placement-anchor', this.chatSidebarPlacementAnchorId);
      } catch(_) {}
    },
    readChatMapWidth() {
      var lockedWall = this.chatMapWallLockNormalized();
      if (lockedWall) {
        var surface = null;
        if (typeof document !== 'undefined' && typeof document.querySelector === 'function') {
          try { surface = document.querySelector('.chat-map .chat-map-surface'); } catch(_) {}
        }
        var lockedWidth = Number(surface && surface.offsetWidth || 0);
        if (Number.isFinite(lockedWidth) && lockedWidth > 0) return lockedWidth;
        return 60;
      }
      var node = this.readChatMapElement();
      var width = Number(node && node.offsetWidth || 0);
      if (Number.isFinite(width) && width > 0) return width;
      return 76;
    },
    chatMapSnapDefinitions() {
      return [
        { id: 'right-top', x: 1, y: 0 },
        { id: 'right-middle', x: 1, y: 0.5 },
        { id: 'right-bottom', x: 1, y: 1 }
      ];
    },
    chatMapSnapDefinitionById(id) {
      var key = String(id || '').trim().toLowerCase();
      var defs = this.chatMapSnapDefinitions();
      for (var i = 0; i < defs.length; i += 1) {
        var row = defs[i];
        if (!row || row.id !== key) continue;
        return row;
      }
      return defs[1] || defs[0] || { id: 'right-middle', x: 1, y: 0.5 };
    },
    chatMapAnchorForSnapId(id) {
      var snap = this.chatMapSnapDefinitionById(id);
      var wallGap = this.overlayWallGapPx();
      var minLeft = wallGap;
      var maxLeft = this.chatOverlayViewportWidth() - wallGap - this.readChatMapWidth();
      if (!Number.isFinite(maxLeft) || maxLeft < minLeft) maxLeft = minLeft;
      var bounds = this.chatOverlayVerticalBounds();
      var height = this.readChatMapHeight();
      var minTop = Number(bounds.minTop || 0);
      var maxTop = Number(bounds.maxBottom || 0) - height;
      if (!Number.isFinite(maxTop) || maxTop < minTop) maxTop = minTop;
      var nx = Number(snap && snap.x);
      var ny = Number(snap && snap.y);
      if (!Number.isFinite(nx)) nx = 1;
      if (!Number.isFinite(ny)) ny = 0.5;
      nx = Math.max(0, Math.min(1, nx));
      ny = Math.max(0, Math.min(1, ny));
      return {
        id: String(snap && snap.id || 'right-middle'),
        left: this.chatMapClampLeft(minLeft + ((maxLeft - minLeft) * nx)),
        top: this.chatMapClampTop(minTop + ((maxTop - minTop) * ny))
      };
    },
    chatMapNearestSnapId(leftRaw, topRaw) {
      var defs = this.chatMapSnapDefinitions();
      if (!defs.length) return 'right-middle';
      var left = this.chatMapClampLeft(leftRaw);
      var top = this.chatMapClampTop(topRaw);
      var bestId = String(defs[0].id || 'right-middle');
      var bestDist = Number.POSITIVE_INFINITY;
      for (var i = 0; i < defs.length; i += 1) {
        var row = defs[i];
        if (!row) continue;
        var anchor = this.chatMapAnchorForSnapId(row.id);
        var dx = Number(left || 0) - Number(anchor.left || 0);
        var dy = Number(top || 0) - Number(anchor.top || 0);
        var dist = (dx * dx) + (dy * dy);
        if (!Number.isFinite(dist) || dist >= bestDist) continue;
        bestDist = dist;
        bestId = String(row.id || bestId);
      }
      return bestId || 'right-middle';
    },
    chatMapResolvedLeftFromRatio() {
      var ratio = 1;
      try {
        var raw = Number(localStorage.getItem('infring-chat-map-placement-x'));
        if (Number.isFinite(raw)) ratio = Math.max(0, Math.min(1, raw));
      } catch(_) {}
      if (Number.isFinite(this.chatMapPlacementX)) ratio = Math.max(0, Math.min(1, Number(this.chatMapPlacementX)));
      var wallGap = this.overlayWallGapPx();
      var minLeft = wallGap;
      var maxLeft = this.chatOverlayViewportWidth() - wallGap - this.readChatMapWidth();
      if (!Number.isFinite(maxLeft) || maxLeft < minLeft) maxLeft = minLeft;
      return this.chatMapClampLeft(minLeft + ((maxLeft - minLeft) * ratio));
    },
    chatMapResolvedTopFromRatio() {
      var bounds = this.chatOverlayVerticalBounds();
      var height = this.readChatMapHeight();
      var minTop = Number(bounds.minTop || 0);
      var maxTop = Number(bounds.maxBottom || 0) - height;
      if (!Number.isFinite(maxTop) || maxTop < minTop) maxTop = minTop;
      var ratio = Number(this.chatMapPlacementY);
      if (!Number.isFinite(ratio)) ratio = 0.38;
      ratio = Math.max(0, Math.min(1, ratio));
      return this.chatMapClampTop(minTop + ((maxTop - minTop) * ratio));
    },
    chatMapActiveSnapId() {
      if (this.chatMapDragActive) {
        return this.chatMapNearestSnapId(this.chatMapDragLeft, this.chatMapDragTop);
      }
      var storedId = String(this.chatMapPlacementAnchorId || '').trim().toLowerCase();
      if (!storedId) {
        try {
          var raw = String(localStorage.getItem('infring-chat-map-placement-anchor') || '').trim().toLowerCase();
          if (raw) storedId = raw;
        } catch(_) {}
      }
      if (storedId) return this.chatMapSnapDefinitionById(storedId).id;
      var fallbackLeft = this.chatMapClampLeft(this.chatMapResolvedLeftFromRatio());
      var fallbackTop = this.chatMapClampTop(this.chatMapResolvedTopFromRatio());
      return this.chatMapNearestSnapId(fallbackLeft, fallbackTop);
    },
    chatMapPersistSnapId(id) {
      var snap = this.chatMapSnapDefinitionById(id);
      this.chatMapPlacementAnchorId = String(snap && snap.id || 'right-middle');
      try {
        localStorage.setItem('infring-chat-map-placement-anchor', this.chatMapPlacementAnchorId);
      } catch(_) {}
    },
    chatMapClampLeft(leftRaw) {
      var wallGap = this.overlayWallGapPx();
      var minLeft = wallGap;
      var maxLeft = this.chatOverlayViewportWidth() - wallGap - this.readChatMapWidth();
      if (!Number.isFinite(maxLeft) || maxLeft < minLeft) maxLeft = minLeft;
      var left = Number(leftRaw);
      if (!Number.isFinite(left)) left = maxLeft;
      return Math.max(minLeft, Math.min(maxLeft, left));
    },
    chatMapHardBounds() {
      return this.dragSurfaceHardBounds(this.readChatMapWidth(), this.readChatMapHeight());
    },
    chatMapWallLockNormalized() {
      var wall = this.dragSurfaceNormalizeWall(this.chatMapWallLock);
      return wall === 'left' || wall === 'right' ? wall : '';
    },
    chatMapSetWallLock(wallRaw) {
      var wall = this.dragSurfaceNormalizeWall(wallRaw);
      if (wall !== 'left' && wall !== 'right') wall = '';
      this.chatMapWallLock = wall;
      try {
        if (wall) localStorage.setItem('infring-chat-map-wall-lock', wall);
        else localStorage.removeItem('infring-chat-map-wall-lock');
        localStorage.removeItem('infring-chat-map-smash-wall');
      } catch(_) {}
      infringUpdateShellLayoutConfig(function(config) {
        config.chatMap.wallLock = wall;
      });
      return wall;
    },
    chatMapResolvedLeft() {
      if (this.chatMapDragActive) return Number(this.chatMapDragLeft || 0);
      var left = this.chatMapClampLeft(this.chatMapResolvedLeftFromRatio());
      var top = this.chatMapClampTop(this.chatMapResolvedTopFromRatio());
      var wall = this.chatMapWallLockNormalized();
      if (!wall) return left;
      return this.dragSurfaceApplyWallLock(this.chatMapHardBounds(), left, top, wall).left;
    },
    chatMapResolvedTop() {
      if (this.chatMapDragActive) return Number(this.chatMapDragTop || 0);
      var left = this.chatMapClampLeft(this.chatMapResolvedLeftFromRatio());
      var top = this.chatMapClampTop(this.chatMapResolvedTopFromRatio());
      var wall = this.chatMapWallLockNormalized();
      if (!wall) return top;
      return this.dragSurfaceApplyWallLock(this.chatMapHardBounds(), left, top, wall).top;
    },
    chatMapPersistPlacementFromLeft(leftRaw) {
      var wallGap = this.overlayWallGapPx();
      var minLeft = wallGap;
      var maxLeft = this.chatOverlayViewportWidth() - wallGap - this.readChatMapWidth();
      if (!Number.isFinite(maxLeft) || maxLeft < minLeft) maxLeft = minLeft;
      var left = this.chatMapClampLeft(leftRaw);
      var ratio = maxLeft > minLeft ? (left - minLeft) / (maxLeft - minLeft) : 1;
      ratio = Math.max(0, Math.min(1, ratio));
      this.chatMapPlacementX = ratio;
      try {
        localStorage.setItem('infring-chat-map-placement-x', String(ratio));
      } catch(_) {}
      infringUpdateShellLayoutConfig(function(config) {
        config.chatMap.placementX = ratio;
      });
    },
    chatMapContainerStyle() {
      if (!this.chatMapPlacementEnabled()) return '';
      var top = this.chatMapResolvedTop();
      var left = this.chatMapResolvedLeft();
      var height = this.readChatMapHeight();
      var durationMs = this.chatMapDragActive ? 0 : this.dragSurfaceMoveDurationMs(this._chatMapMoveDurationMs, 280);
      var wall = this.chatMapWallLockNormalized();
      var lockCss = this.dragSurfaceLockVisualCssVars('chat-map', wall, {
        transformMs: this._dragSurfaceLockTransformMs,
        shellPaddingInline: '8px',
        shellPaddingInlineLocked: '0px',
        shellPaddingBlock: '2px',
        shellPaddingBlockLocked: '0px',
        shellAlignItems: 'flex-end',
        shellAlignItemsLeft: 'flex-start',
        shellAlignItemsRight: 'flex-end',
        surfaceMarginInline: 'auto',
        surfaceMarginInlineLocked: '0'
      });
      return (
        'left:' + Math.round(left) + 'px;' +
        'top:' + Math.round(top) + 'px;' +
        'right:auto;' +
        'bottom:auto;' +
        'height:' + Math.round(height) + 'px;' +
        lockCss +
        'transition:top ' + Math.max(0, Math.round(durationMs)) + 'ms var(--ease-smooth), left ' + Math.max(0, Math.round(durationMs)) + 'ms var(--ease-smooth);'
      );
    },
    startChatMapPointerDrag(ev) {
      if (!ev || !this.chatMapPlacementEnabled()) return;
      var button = Number(ev.button);
      if (Number.isFinite(button) && button > 0) return;
      var target = ev && ev.target ? ev.target : null;
      if (this.shouldIgnoreChatMapDragTarget(target)) return;
      this._chatMapPointerActive = true;
      this._chatMapPointerMoved = false;
      this._chatMapPointerStartX = Number(ev.clientX || 0);
      this._chatMapPointerStartY = Number(ev.clientY || 0);
      this._chatMapPointerOriginLeft = this.chatMapResolvedLeft();
      this._chatMapPointerOriginTop = this.chatMapResolvedTop();
      this._chatMapPointerLastX = this._chatMapPointerStartX;
      this._chatMapPointerLastY = this._chatMapPointerStartY;
      this._chatMapPointerLastAt = Date.now();
      this._chatMapPointerVelocity = 0;
      this.chatMapDragLeft = this._chatMapPointerOriginLeft;
      this.chatMapDragTop = this._chatMapPointerOriginTop;
      this.bindChatMapPointerListeners();
      try {
        if (ev.currentTarget && typeof ev.currentTarget.setPointerCapture === 'function' && Number.isFinite(ev.pointerId)) {
          ev.currentTarget.setPointerCapture(ev.pointerId);
        }
      } catch(_) {}
    },
    handleChatMapPointerMove(ev) {
      if (!this._chatMapPointerActive || !this.chatMapPlacementEnabled()) return;
      var nextX = Number(ev.clientX || 0);
      var nextY = Number(ev.clientY || 0);
      var now = Date.now();
      var prevX = Number(this._chatMapPointerLastX || nextX);
      var prevY = Number(this._chatMapPointerLastY || nextY);
      var prevAt = Number(this._chatMapPointerLastAt || now);
      var dt = Math.max(1, now - prevAt);
      var stepDx = nextX - prevX;
      var stepDy = nextY - prevY;
      this._chatMapPointerVelocity = Math.sqrt((stepDx * stepDx) + (stepDy * stepDy)) / dt;
      this._chatMapPointerLastX = nextX;
      this._chatMapPointerLastY = nextY;
      this._chatMapPointerLastAt = now;
      var movedX = Math.abs(nextX - Number(this._chatMapPointerStartX || 0));
      var movedY = Math.abs(nextY - Number(this._chatMapPointerStartY || 0));
      if (!this._chatMapPointerMoved) {
        if (movedX < 4 && movedY < 4) return;
        this._chatMapPointerMoved = true;
        this.chatMapDragActive = true;
        this.hideDashboardPopupBySource('chat-map');
      }
      var dragDx = nextX - Number(this._chatMapPointerStartX || 0);
      var dragDy = nextY - Number(this._chatMapPointerStartY || 0);
      var candidateLeft = Number(this._chatMapPointerOriginLeft || 0) + dragDx;
      var candidateTop = Number(this._chatMapPointerOriginTop || 0) + dragDy;
      var hardBounds = this.chatMapHardBounds();
      var lockedWall = this.chatMapWallLockNormalized();
      if (lockedWall) {
        var unlockDistance = this.dragSurfaceDistanceFromWall(hardBounds, candidateLeft, candidateTop, lockedWall);
        if (unlockDistance >= this.dragSurfaceWallUnlockDistanceThreshold()) {
          lockedWall = this.chatMapSetWallLock('');
        } else {
          var holdLeft = Number.isFinite(Number(this.chatMapDragLeft))
            ? Number(this.chatMapDragLeft)
            : Number(this._chatMapPointerOriginLeft || 0);
          var holdTop = Number.isFinite(Number(this.chatMapDragTop))
            ? Number(this.chatMapDragTop)
            : Number(this._chatMapPointerOriginTop || 0);
          var stayLocked = this.dragSurfaceApplyWallLock(hardBounds, holdLeft, holdTop, lockedWall);
          this.chatMapDragLeft = stayLocked.left;
          this.chatMapDragTop = stayLocked.top;
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
        var persistedLockWall = this.chatMapSetWallLock(lockWall);
        var snapped = this.dragSurfaceApplyWallLock(hardBounds, clamped.left, clamped.top, persistedLockWall);
        this.chatMapDragLeft = snapped.left;
        this.chatMapDragTop = snapped.top;
      } else {
        this.chatMapDragLeft = clamped.left;
        this.chatMapDragTop = clamped.top;
      }
      if (ev.cancelable && typeof ev.preventDefault === 'function') ev.preventDefault();
    },
    endChatMapPointerDrag() {
      if (!this._chatMapPointerActive) return;
      this._chatMapPointerActive = false;
      this.unbindChatMapPointerListeners();
      if (!this._chatMapPointerMoved) {
        this.chatMapDragActive = false;
        return;
      }
      this._chatMapPointerMoved = false;
      var hardBounds = this.chatMapHardBounds();
      var lockedWall = this.chatMapWallLockNormalized();
      var final;
      if (lockedWall) {
        final = this.dragSurfaceApplyWallLock(hardBounds, this.chatMapDragLeft, this.chatMapDragTop, lockedWall);
        this.chatMapPlacementAnchorId = '';
        try { localStorage.removeItem('infring-chat-map-placement-anchor'); } catch(_) {}
      } else {
        var clamped = this.dragSurfaceClampWithBounds(hardBounds, this.chatMapDragLeft, this.chatMapDragTop);
        var snapId = this.chatMapNearestSnapId(clamped.left, clamped.top);
        var snap = this.chatMapAnchorForSnapId(snapId);
        final = this.dragSurfaceClampWithBounds(hardBounds, snap.left, snap.top);
        this.chatMapPersistSnapId(snapId);
      }
      this.chatMapDragLeft = final.left;
      this.chatMapDragTop = final.top;
      this.chatMapPersistPlacementFromLeft(this.chatMapDragLeft);
      this.chatMapPersistPlacementFromTop(this.chatMapDragTop);
      this.chatMapDragActive = false;
    },
