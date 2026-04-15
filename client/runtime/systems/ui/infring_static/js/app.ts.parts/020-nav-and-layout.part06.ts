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
      if (typeof document !== 'undefined') {
        try {
          var draggingNodes = document.querySelectorAll('.taskbar-reorder-item.dragging');
          for (var i = 0; i < draggingNodes.length; i += 1) {
            draggingNodes[i].classList.remove('dragging');
          }
        } catch(_) {}
      }
    },

    handleTaskbarDragEnd() {
      var key = String(this.taskbarDragGroup || '').trim();
      if (key) this.persistTaskbarReorder(key);
      this.taskbarDragGroup = '';
      this.taskbarDragItem = '';
      this.taskbarDragStartOrder = [];
      this.cancelTaskbarDragHold();
      this.setTaskbarDragBodyActive(false);
      if (typeof document !== 'undefined') {
        try {
          var draggingNodes = document.querySelectorAll('.taskbar-reorder-item.dragging');
          for (var i = 0; i < draggingNodes.length; i += 1) {
            draggingNodes[i].classList.remove('dragging');
          }
        } catch(_) {}
      }
    },

    readChatMapWidth() {
      var node = this.readChatMapElement();
      var width = Number(node && node.offsetWidth || 0);
      if (Number.isFinite(width) && width > 0) return width;
      return 76;
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

    chatMapResolvedLeft() {
      if (this.chatMapDragActive) return this.chatMapClampLeft(this.chatMapDragLeft);
      var ratio = 1;
      try {
        var raw = Number(localStorage.getItem('infring-chat-map-placement-x'));
        if (Number.isFinite(raw)) ratio = Math.max(0, Math.min(1, raw));
      } catch(_) {}
      var wallGap = this.overlayWallGapPx();
      var minLeft = wallGap;
      var maxLeft = this.chatOverlayViewportWidth() - wallGap - this.readChatMapWidth();
      if (!Number.isFinite(maxLeft) || maxLeft < minLeft) maxLeft = minLeft;
      return this.chatMapClampLeft(minLeft + ((maxLeft - minLeft) * ratio));
    },

    chatMapPersistPlacementFromLeft(leftRaw) {
      var wallGap = this.overlayWallGapPx();
      var minLeft = wallGap;
      var maxLeft = this.chatOverlayViewportWidth() - wallGap - this.readChatMapWidth();
      if (!Number.isFinite(maxLeft) || maxLeft < minLeft) maxLeft = minLeft;
      var left = this.chatMapClampLeft(leftRaw);
      var ratio = maxLeft > minLeft ? (left - minLeft) / (maxLeft - minLeft) : 1;
      ratio = Math.max(0, Math.min(1, ratio));
      try {
        localStorage.setItem('infring-chat-map-placement-x', String(ratio));
      } catch(_) {}
    },

    chatMapContainerStyle() {
      if (this.page !== 'chat') return '';
      var top = this.chatMapResolvedTop();
      var left = this.chatMapResolvedLeft();
      var height = this.readChatMapHeight();
      var durationMs = this.chatMapDragActive ? 0 : this.dragSurfaceMoveDurationMs(this._chatMapMoveDurationMs, 280);
      return (
        'left:' + Math.round(left) + 'px;' +
        'top:' + Math.round(top) + 'px;' +
        'right:auto;' +
        'bottom:auto;' +
        'height:' + Math.round(height) + 'px;' +
        'transition:top ' + Math.max(0, Math.round(durationMs)) + 'ms var(--ease-smooth), left ' + Math.max(0, Math.round(durationMs)) + 'ms var(--ease-smooth);'
      );
    },

    startChatMapPointerDrag(ev) {
      if (!ev) return;
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
      if (!this._chatMapPointerActive) return;
      var nextX = Number(ev.clientX || 0);
      var nextY = Number(ev.clientY || 0);
      var movedX = Math.abs(nextX - Number(this._chatMapPointerStartX || 0));
      var movedY = Math.abs(nextY - Number(this._chatMapPointerStartY || 0));
      if (!this._chatMapPointerMoved) {
        if (movedX < 4 && movedY < 4) return;
        this._chatMapPointerMoved = true;
        this.chatMapDragActive = true;
        this.hideDashboardPopupBySource('chat-map');
      }
      var candidateLeft = Number(this._chatMapPointerOriginLeft || 0) + (nextX - Number(this._chatMapPointerStartX || 0));
      var candidateTop = Number(this._chatMapPointerOriginTop || 0) + (nextY - Number(this._chatMapPointerStartY || 0));
      this.chatMapDragLeft = this.chatMapClampLeft(candidateLeft);
      this.chatMapDragTop = this.chatMapClampTop(candidateTop);
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
      var finalLeft = this.chatMapClampLeft(this.chatMapDragLeft);
      var finalTop = this.chatMapClampTop(this.chatMapDragTop);
      this.chatMapDragLeft = finalLeft;
      this.chatMapDragTop = finalTop;
      this.chatMapPersistPlacementFromLeft(finalLeft);
      this.chatMapPersistPlacementFromTop(finalTop);
      this.chatMapDragActive = false;
    },
