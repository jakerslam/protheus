    applyTopbarReorder(group, dragItem, targetItem, preferAfter, animate) {
      var key = String(group || '').trim().toLowerCase();
      if (key !== 'right') key = 'left';
      var dragId = String(dragItem || '').trim();
      var targetId = String(targetItem || '').trim();
      if (!dragId || !targetId || dragId === targetId) return false;
      var current = this.normalizeTopbarReorder(key, this.topbarReorderOrderForGroup(key));
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
      var beforeRects = Boolean(animate) ? this.topbarReorderItemRects(key) : null;
      this.setTopbarReorderOrderForGroup(key, next);
      if (beforeRects) this.animateTopbarReorderFromRects(key, beforeRects);
      return true;
    },

    handleTopbarReorderPointerDown(group, ev) {
      if (String(this.topbarDragGroup || '').trim()) return;
      if (!ev || Number(ev.button) !== 0) return;
      var key = String(group || '').trim().toLowerCase();
      if (key !== 'right') key = 'left';
      var target = ev && ev.target && typeof ev.target.closest === 'function'
        ? ev.target.closest('.topbar-reorder-item[data-topbar-item]')
        : null;
      var item = target ? String(target.getAttribute('data-topbar-item') || '').trim() : '';
      if (!item) return;
      this.cancelTopbarDragHold();
      this._topbarDragHoldGroup = key;
      this._topbarDragHoldItem = item;
      var self = this;
      if (typeof window !== 'undefined' && typeof window.setTimeout === 'function') {
        this._topbarDragHoldTimer = window.setTimeout(function() {
          self._topbarDragHoldTimer = 0;
          self._topbarDragArmedGroup = key;
          self._topbarDragArmedItem = item;
        }, 180);
      }
    },

    cancelTopbarDragHold() {
      if (this._topbarDragHoldTimer) {
        try { clearTimeout(this._topbarDragHoldTimer); } catch(_) {}
      }
      this._topbarDragHoldTimer = 0;
      this._topbarDragHoldGroup = '';
      this._topbarDragHoldItem = '';
      if (!String(this.topbarDragGroup || '').trim()) {
        this._topbarDragArmedGroup = '';
        this._topbarDragArmedItem = '';
      }
    },

    forceTopbarMoveDragEffect(ev) {
      if (!ev || !ev.dataTransfer) return;
      try { ev.dataTransfer.effectAllowed = 'move'; } catch(_) {}
      try { ev.dataTransfer.dropEffect = 'move'; } catch(_) {}
    },

    setTopbarDragBodyActive(active) {
      if (typeof document === 'undefined' || !document.body || !document.body.classList) return;
      if (active) {
        document.body.classList.add('topbar-drag-active');
      } else {
        document.body.classList.remove('topbar-drag-active');
      }
    },

    handleTopbarReorderDragStart(group, ev) {
      var key = String(group || '').trim().toLowerCase();
      if (key !== 'right') key = 'left';
      var target = ev && ev.target && typeof ev.target.closest === 'function'
        ? ev.target.closest('.topbar-reorder-item[data-topbar-item]')
        : null;
      var item = target ? String(target.getAttribute('data-topbar-item') || '').trim() : '';
      if (!item || this._topbarDragArmedGroup !== key || this._topbarDragArmedItem !== item) {
        if (ev && typeof ev.preventDefault === 'function') ev.preventDefault();
        return;
      }
      this.topbarDragGroup = key;
      this.topbarDragItem = item;
      this.topbarDragStartOrder = this.normalizeTopbarReorder(key, this.topbarReorderOrderForGroup(key));
      this._topbarDragArmedGroup = '';
      this._topbarDragArmedItem = '';
      this.cancelTopbarDragHold();
      if (ev && ev.dataTransfer) {
        this.forceTopbarMoveDragEffect(ev);
        try { ev.dataTransfer.setData('application/x-infring-topbar', key + ':' + item); } catch(_) {}
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
      this.setTopbarDragBodyActive(true);
    },

    handleTopbarReorderDragMove(ev) {
      this.forceTopbarMoveDragEffect(ev);
    },

    handleTopbarReorderDragEnter(group, ev) {
      var key = String(group || '').trim().toLowerCase();
      if (key !== 'right') key = 'left';
      if (String(this.topbarDragGroup || '').trim() !== key) return;
      if (ev && typeof ev.preventDefault === 'function') ev.preventDefault();
      this.forceTopbarMoveDragEffect(ev);
    },

    handleTopbarReorderDragOver(group, ev) {
      var key = String(group || '').trim().toLowerCase();
      if (key !== 'right') key = 'left';
      if (String(this.topbarDragGroup || '').trim() !== key) return;
      if (ev && typeof ev.preventDefault === 'function') ev.preventDefault();
      this.forceTopbarMoveDragEffect(ev);
      var dragItem = String(this.topbarDragItem || '').trim();
      if (!dragItem) return;
      var target = ev && ev.target && typeof ev.target.closest === 'function'
        ? ev.target.closest('.topbar-reorder-item[data-topbar-item]')
        : null;
      var targetItem = target ? String(target.getAttribute('data-topbar-item') || '').trim() : '';
      if (!targetItem || targetItem === dragItem) return;
      var preferAfter = false;
      if (target && typeof target.getBoundingClientRect === 'function') {
        var rect = target.getBoundingClientRect();
        var midX = Number(rect.left || 0) + (Number(rect.width || 0) / 2);
        preferAfter = Number(ev && ev.clientX || 0) >= midX;
      }
      this.applyTopbarReorder(key, dragItem, targetItem, preferAfter, true);
    },

    handleTopbarReorderDrop(group, ev) {
      var key = String(group || '').trim().toLowerCase();
      if (key !== 'right') key = 'left';
      if (String(this.topbarDragGroup || '').trim() !== key) return;
      if (ev && typeof ev.preventDefault === 'function') ev.preventDefault();
      this.persistTopbarReorder(key);
      this.topbarDragGroup = '';
      this.topbarDragItem = '';
      this.topbarDragStartOrder = [];
      this.cancelTopbarDragHold();
      this.setTopbarDragBodyActive(false);
      if (typeof document !== 'undefined') {
        try {
          var draggingNodes = document.querySelectorAll('.topbar-reorder-item.dragging');
          for (var i = 0; i < draggingNodes.length; i += 1) {
            draggingNodes[i].classList.remove('dragging');
          }
        } catch(_) {}
      }
    },

    handleTopbarDragEnd() {
      var key = String(this.topbarDragGroup || '').trim();
      if (key) this.persistTopbarReorder(key);
      this.topbarDragGroup = '';
      this.topbarDragItem = '';
      this.topbarDragStartOrder = [];
      this.cancelTopbarDragHold();
      this.setTopbarDragBodyActive(false);
      if (typeof document !== 'undefined') {
        try {
          var draggingNodes = document.querySelectorAll('.topbar-reorder-item.dragging');
          for (var i = 0; i < draggingNodes.length; i += 1) {
            draggingNodes[i].classList.remove('dragging');
          }
        } catch(_) {}
      }
    },

