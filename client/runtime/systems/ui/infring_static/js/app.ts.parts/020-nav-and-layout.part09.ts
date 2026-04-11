    bindBottomDockPointerListeners() {
      if (this._bottomDockPointerMoveHandler || this._bottomDockPointerUpHandler) return;
      var self = this;
      this._bottomDockPointerMoveHandler = function(ev) { self.handleBottomDockPointerMove(ev); };
      this._bottomDockPointerUpHandler = function(ev) { self.endBottomDockPointerDrag(ev); };
      window.addEventListener('pointermove', this._bottomDockPointerMoveHandler, true);
      window.addEventListener('pointerup', this._bottomDockPointerUpHandler, true);
      window.addEventListener('pointercancel', this._bottomDockPointerUpHandler, true);
      window.addEventListener('mousemove', this._bottomDockPointerMoveHandler, true);
      window.addEventListener('mouseup', this._bottomDockPointerUpHandler, true);
    },

    unbindBottomDockPointerListeners() {
      if (this._bottomDockPointerMoveHandler) {
        try { window.removeEventListener('pointermove', this._bottomDockPointerMoveHandler, true); } catch(_) {}
        try { window.removeEventListener('mousemove', this._bottomDockPointerMoveHandler, true); } catch(_) {}
      }
      if (this._bottomDockPointerUpHandler) {
        try { window.removeEventListener('pointerup', this._bottomDockPointerUpHandler, true); } catch(_) {}
        try { window.removeEventListener('pointercancel', this._bottomDockPointerUpHandler, true); } catch(_) {}
        try { window.removeEventListener('mouseup', this._bottomDockPointerUpHandler, true); } catch(_) {}
      }
      this._bottomDockPointerMoveHandler = null;
      this._bottomDockPointerUpHandler = null;
    },

    startBottomDockPointerDrag(id, ev) {
      if (!ev || Number(ev.button) !== 0) return;
      if (this.bottomDockContainerDragActive || this._bottomDockContainerPointerActive) return;
      var key = String(id || '').trim();
      if (!key) return;
      var hostEl = ev && ev.currentTarget ? ev.currentTarget : null;
      if (hostEl && typeof hostEl.getBoundingClientRect === 'function') {
        try {
          var rect = hostEl.getBoundingClientRect();
          var width = Number(rect.width || 32);
          var height = Number(rect.height || 32);
          var baseWidth = Number(hostEl && hostEl.offsetWidth ? hostEl.offsetWidth : width || 32);
          var baseHeight = Number(hostEl && hostEl.offsetHeight ? hostEl.offsetHeight : height || 32);
          if (!Number.isFinite(width) || width <= 0) width = 32;
          if (!Number.isFinite(height) || height <= 0) height = 32;
          if (!Number.isFinite(baseWidth) || baseWidth <= 0) baseWidth = width;
          if (!Number.isFinite(baseHeight) || baseHeight <= 0) baseHeight = height;
          var expandedScale = this.bottomDockExpandedScale();
          var expandedWidth = baseWidth * expandedScale;
          var expandedHeight = baseHeight * expandedScale;
          this._bottomDockDragGhostWidth = Math.max(20, Math.min(112, Math.max(width, expandedWidth)));
          this._bottomDockDragGhostHeight = Math.max(20, Math.min(112, Math.max(height, expandedHeight)));
          var offsetX = Number(ev.clientX || 0) - Number(rect.left || 0);
          var offsetY = Number(ev.clientY || 0) - Number(rect.top || 0);
          var relX = Number.isFinite(offsetX) && width > 0 ? (offsetX / width) : 0.5;
          var relY = Number.isFinite(offsetY) && height > 0 ? (offsetY / height) : 0.5;
          relX = Math.max(0, Math.min(1, relX));
          relY = Math.max(0, Math.min(1, relY));
          this._bottomDockPointerGrabOffsetX = relX * this._bottomDockDragGhostWidth;
          this._bottomDockPointerGrabOffsetY = relY * this._bottomDockDragGhostHeight;
        } catch(_) {
          this._bottomDockPointerGrabOffsetX = 16;
          this._bottomDockPointerGrabOffsetY = 16;
          this._bottomDockDragGhostWidth = 32;
          this._bottomDockDragGhostHeight = 32;
        }
      } else {
        this._bottomDockPointerGrabOffsetX = 16;
        this._bottomDockPointerGrabOffsetY = 16;
        this._bottomDockDragGhostWidth = 32;
        this._bottomDockDragGhostHeight = 32;
      }
      try {
        if (hostEl && typeof hostEl.setPointerCapture === 'function' && Number.isFinite(ev.pointerId)) {
          hostEl.setPointerCapture(ev.pointerId);
        }
      } catch(_) {}
      this._bottomDockPointerActive = true;
      this._bottomDockPointerMoved = false;
      this._bottomDockPointerCandidateId = key;
      this._bottomDockPointerStartX = Number(ev.clientX || 0);
      this._bottomDockPointerStartY = Number(ev.clientY || 0);
      this._bottomDockPointerLastX = Number(ev.clientX || 0);
      this._bottomDockPointerLastY = Number(ev.clientY || 0);
      this._bottomDockReorderLockUntil = 0;
      this.bindBottomDockPointerListeners();
    },

    activateBottomDockPointerDrag(ev) {
      if (this._bottomDockPointerMoved) return;
      var dragId = String(this._bottomDockPointerCandidateId || '').trim();
      if (!dragId) return;
      this._bottomDockPointerMoved = true;
      this.bottomDockHoverId = '';
      this.bottomDockHoverWeightById = {};
      this.bottomDockPointerX = 0;
      this.bottomDockPointerY = 0;
      this.bottomDockPreviewVisible = false;
      this.bottomDockPreviewText = '';
      this.bottomDockPreviewMorphFromText = '';
      this.bottomDockPreviewLabelMorphing = false;
      this.bottomDockPreviewWidth = 0;
      this.cancelBottomDockPreviewReflow();
      this._bottomDockRevealTargetDuringSettle = false;
      this.bottomDockDragId = dragId;
      this.bottomDockDragCommitted = false;
      this.bottomDockDragStartOrder = this.normalizeBottomDockOrder(this.bottomDockOrder);
      this.cleanupBottomDockDragGhost();
      this.captureBottomDockDragBoundaries(dragId);
      var originNode = document.querySelector('.bottom-dock-btn[data-dock-id="' + dragId + '"]');
      if (!originNode || !document || !document.body) return;
      var dockEl = document.querySelector('.bottom-dock');
      if (dockEl && dockEl.style && typeof dockEl.style.setProperty === 'function') {
        dockEl.style.setProperty('--bottom-dock-drag-scale', String(this.readBottomDockScale(dockEl)));
      }
      var ghost = document.createElement('div');
      ghost.className = 'bottom-dock-drag-ghost bottom-dock-btn dock-tile';
      var tone = '';
      var iconKind = '';
      try {
        tone = String(originNode.getAttribute('data-dock-tone') || '').trim();
        iconKind = String(originNode.getAttribute('data-dock-icon') || '').trim();
      } catch(_) {
        tone = '';
        iconKind = '';
      }
      if (tone) ghost.setAttribute('data-dock-tone', tone);
      if (iconKind) ghost.setAttribute('data-dock-icon', iconKind);
      if (originNode.classList && typeof originNode.classList.contains === 'function') {
        if (originNode.classList.contains('active')) ghost.classList.add('active');
      }
      ghost.setAttribute('aria-hidden', 'true');
      ghost.innerHTML = String(originNode.innerHTML || '');
      ghost.style.position = 'fixed';
      ghost.style.width = Math.round(Number(this._bottomDockDragGhostWidth || 32)) + 'px';
      ghost.style.height = Math.round(Number(this._bottomDockDragGhostHeight || 32)) + 'px';
      ghost.style.borderRadius = Math.round((Number(this._bottomDockDragGhostWidth || 32) / 32) * 11) + 'px';
      ghost.style.setProperty(
        '--dock-ghost-scale',
        String(Math.max(0.8, Math.min(4, Number(this._bottomDockDragGhostWidth || 32) / 32)))
      );
      var ghostUpDeg = Number(this.bottomDockUpDegForSide(this.bottomDockActiveSide()) || 0);
      var ghostTileRotation = Math.round(ghostUpDeg) + 'deg';
      var ghostIconRotation = '0deg';
      ghost.style.setProperty('--bottom-dock-tile-rotation-deg', ghostTileRotation);
      ghost.style.setProperty('--bottom-dock-icon-rotation-deg', ghostIconRotation);
      var ghostX = Number(ev.clientX || 0) - Number(this._bottomDockPointerGrabOffsetX || 16);
      var ghostY = Number(ev.clientY || 0) - Number(this._bottomDockPointerGrabOffsetY || 16);
      this._bottomDockGhostCurrentX = ghostX;
      this._bottomDockGhostCurrentY = ghostY;
      ghost.style.left = Math.round(ghostX) + 'px';
      ghost.style.top = Math.round(ghostY) + 'px';
      ghost.style.margin = '0';
      ghost.style.pointerEvents = 'none';
      ghost.style.opacity = '1';
      document.body.appendChild(ghost);
      this._bottomDockDragGhostEl = ghost;
      this.setBottomDockGhostTarget(ghostX, ghostY);
    },

    handleBottomDockPointerMove(ev) {
      if (!this._bottomDockPointerActive) return;
      this._bottomDockPointerLastX = Number(ev.clientX || 0);
      this._bottomDockPointerLastY = Number(ev.clientY || 0);
      var movedX = Math.abs(Number(ev.clientX || 0) - Number(this._bottomDockPointerStartX || 0));
      var movedY = Math.abs(Number(ev.clientY || 0) - Number(this._bottomDockPointerStartY || 0));
      if (!this._bottomDockPointerMoved) {
        if (movedX < 5 && movedY < 5) return;
        this.activateBottomDockPointerDrag(ev);
      }
      if (!this._bottomDockPointerMoved) return;
      if (ev && typeof ev.preventDefault === 'function' && ev.cancelable) ev.preventDefault();
      var ghost = this._bottomDockDragGhostEl;
      if (ghost) {
        this.setBottomDockGhostTarget(
          Number(ev.clientX || 0) - Number(this._bottomDockPointerGrabOffsetX || 16),
          Number(ev.clientY || 0) - Number(this._bottomDockPointerGrabOffsetY || 16)
        );
      }
      var dragId = String(this.bottomDockDragId || '').trim();
      if (!dragId) return;
      var insertionIndex = this.bottomDockInsertionIndexFromPointer(dragId, ev);
      if (Number.isFinite(insertionIndex)) {
        var normalizedIndex = Math.max(0, Math.round(Number(insertionIndex || 0)));
        var nowMs = Date.now();
        var lockUntil = Number(this._bottomDockReorderLockUntil || 0);
        if (
          normalizedIndex !== Number(this._bottomDockLastInsertionIndex || -1) &&
          (!Number.isFinite(lockUntil) || lockUntil <= nowMs)
        ) {
          var changed = this.applyBottomDockReorderByIndex(dragId, normalizedIndex, true);
          this._bottomDockLastInsertionIndex = normalizedIndex;
          if (changed) {
            var moveDuration = this.bottomDockMoveDurationMs();
            var lockMs = Math.max(220, Math.min(420, Math.round(moveDuration * 0.55)));
            this._bottomDockReorderLockUntil = nowMs + lockMs;
          }
        }
        return;
      }
      var targetId = '';
      var targetEl = null;
      try {
        var pointerEl = typeof document !== 'undefined' && typeof document.elementFromPoint === 'function'
          ? document.elementFromPoint(Number(ev.clientX || 0), Number(ev.clientY || 0))
          : null;
        targetEl = pointerEl && typeof pointerEl.closest === 'function'
          ? pointerEl.closest('.bottom-dock-btn[data-dock-id]')
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

    endBottomDockPointerDrag() {
      if (!this._bottomDockPointerActive) return;
      this._bottomDockPointerActive = false;
      this.unbindBottomDockPointerListeners();
      if (!this._bottomDockPointerMoved) {
        this._bottomDockPointerCandidateId = '';
        return;
      }
      var dragId = String(this.bottomDockDragId || this._bottomDockPointerCandidateId || '').trim();
      if (dragId) {
        var finalPointerEvent = {
          clientX: Number(this._bottomDockPointerLastX || 0),
          clientY: Number(this._bottomDockPointerLastY || 0)
        };
        var finalInsertionIndex = this.bottomDockInsertionIndexFromPointer(dragId, finalPointerEvent);
        if (Number.isFinite(finalInsertionIndex)) {
          this.applyBottomDockReorderByIndex(dragId, finalInsertionIndex, false);
        } else if (this.bottomDockShouldAppendFromPointer(dragId, finalPointerEvent)) {
          var appendTargetId = this.bottomDockAppendTargetId(dragId);
          if (appendTargetId) {
            this.handleBottomDockDragOver(appendTargetId, finalPointerEvent, true);
          }
        }
      }
      var current = this.normalizeBottomDockOrder(this.bottomDockOrder);
      var start = this.normalizeBottomDockOrder(this.bottomDockDragStartOrder);
      if (JSON.stringify(current) !== JSON.stringify(start)) {
        this.bottomDockOrder = current;
        this.persistBottomDockOrder();
        this.bottomDockDragCommitted = true;
      }
      this._bottomDockSuppressClickUntil = Date.now() + 220;
      var self = this;
      var finalizeDrag = function() {
        var dockEl = document.querySelector('.bottom-dock');
        if (dockEl && dockEl.style && typeof dockEl.style.removeProperty === 'function') {
          dockEl.style.removeProperty('--bottom-dock-drag-scale');
        }
        var dropX = Number(self._bottomDockPointerLastX || 0);
        var dropY = Number(self._bottomDockPointerLastY || 0);
        self.bottomDockDragId = '';
        self.bottomDockHoverId = '';
        self.bottomDockDragStartOrder = [];
        self._bottomDockPointerGrabOffsetX = 16;
        self._bottomDockPointerGrabOffsetY = 16;
        self._bottomDockDragGhostWidth = 32;
        self._bottomDockDragGhostHeight = 32;
        self._bottomDockPointerCandidateId = '';
        self._bottomDockPointerMoved = false;
        self._bottomDockDragBoundaries = [];
        self._bottomDockLastInsertionIndex = -1;
        self.reviveBottomDockHoverFromPoint(dropX, dropY);
        self._bottomDockPointerLastX = 0;
        self._bottomDockPointerLastY = 0;
      };
      this.settleBottomDockDragGhost(dragId, finalizeDrag);
    },
