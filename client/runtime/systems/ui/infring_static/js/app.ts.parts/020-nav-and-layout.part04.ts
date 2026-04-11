    bottomDockNearestSnapId(anchorX, anchorY) {
      var defs = this.bottomDockSnapDefinitions();
      if (!defs.length) return 'center';
      var anchor = this.bottomDockClampDragAnchor(anchorX, anchorY);
      var bestId = defs[0].id;
      var bestDist = Number.POSITIVE_INFINITY;
      for (var i = 0; i < defs.length; i += 1) {
        var row = defs[i];
        if (!row) continue;
        var snapAnchor = this.bottomDockAnchorForSnapId(row.id);
        var dx = Number(anchor.x || 0) - Number(snapAnchor.x || 0);
        var dy = Number(anchor.y || 0) - Number(snapAnchor.y || 0);
        var dist = (dx * dx) + (dy * dy);
        if (!Number.isFinite(dist)) continue;
        if (dist >= bestDist) continue;
        bestDist = dist;
        bestId = row.id;
      }
      return String(bestId || 'center');
    },

    persistBottomDockPlacement() {
      var key = String(this.bottomDockPlacementId || '').trim().toLowerCase();
      var snap = this.bottomDockSnapDefinitionById(key);
      this.bottomDockPlacementId = String(snap && snap.id || 'center');
      try {
        localStorage.setItem('infring-bottom-dock-placement', this.bottomDockPlacementId);
      } catch(_) {}
    },

    bottomDockContainerStyle() {
      var activeSnapId = this.bottomDockContainerDragActive
        ? this.bottomDockNearestSnapId(this.bottomDockContainerDragX, this.bottomDockContainerDragY)
        : this.bottomDockPlacementId;
      var side = this.bottomDockSideForSnapId(activeSnapId);
      var anchor = this.bottomDockContainerDragActive
        ? this.bottomDockClampAnchor(this.bottomDockContainerDragX, this.bottomDockContainerDragY, side)
        : this.bottomDockAnchorForSnapId(this.bottomDockPlacementId);
      var rotationDeg = Number(this.bottomDockRotationDeg);
      if (!Number.isFinite(rotationDeg)) {
        rotationDeg = this.bottomDockResolveRotationForSide(side, anchor.x, anchor.y);
        this.bottomDockRotationDeg = rotationDeg;
      }
      var upDeg = Number(this.bottomDockUpDegForSide(side) || 0);
      var tileRotationDeg = upDeg - Number(rotationDeg || 0);
      var iconRotationDeg = 0;
      var durationMs = this.bottomDockContainerDragActive ? 0 : this.bottomDockMoveDurationMs();
      return (
        '--bottom-dock-anchor-x:' + Math.round(Number(anchor.x || 0)) + 'px;' +
        '--bottom-dock-anchor-y:' + Math.round(Number(anchor.y || 0)) + 'px;' +
        '--bottom-dock-position-transition:' + Math.max(0, Math.round(Number(durationMs || 0))) + 'ms;' +
        '--bottom-dock-up-deg:' + Math.round(Number(upDeg || 0)) + 'deg;' +
        '--bottom-dock-rotation-deg:' + Math.round(Number(rotationDeg || 0)) + 'deg;' +
        '--bottom-dock-tile-rotation-deg:' + Math.round(Number(tileRotationDeg || 0)) + 'deg;' +
        '--bottom-dock-icon-rotation-deg:' + Math.round(Number(iconRotationDeg || 0)) + 'deg;'
      );
    },

    bindBottomDockContainerPointerListeners() {
      if (this._bottomDockContainerPointerMoveHandler || this._bottomDockContainerPointerUpHandler) return;
      var self = this;
      this._bottomDockContainerPointerMoveHandler = function(ev) { self.handleBottomDockContainerPointerMove(ev); };
      this._bottomDockContainerPointerUpHandler = function(ev) { self.endBottomDockContainerPointerDrag(ev); };
      window.addEventListener('pointermove', this._bottomDockContainerPointerMoveHandler, true);
      window.addEventListener('pointerup', this._bottomDockContainerPointerUpHandler, true);
      window.addEventListener('pointercancel', this._bottomDockContainerPointerUpHandler, true);
      window.addEventListener('mousemove', this._bottomDockContainerPointerMoveHandler, true);
      window.addEventListener('mouseup', this._bottomDockContainerPointerUpHandler, true);
    },

    unbindBottomDockContainerPointerListeners() {
      if (this._bottomDockContainerPointerMoveHandler) {
        try { window.removeEventListener('pointermove', this._bottomDockContainerPointerMoveHandler, true); } catch(_) {}
        try { window.removeEventListener('mousemove', this._bottomDockContainerPointerMoveHandler, true); } catch(_) {}
      }
      if (this._bottomDockContainerPointerUpHandler) {
        try { window.removeEventListener('pointerup', this._bottomDockContainerPointerUpHandler, true); } catch(_) {}
        try { window.removeEventListener('pointercancel', this._bottomDockContainerPointerUpHandler, true); } catch(_) {}
        try { window.removeEventListener('mouseup', this._bottomDockContainerPointerUpHandler, true); } catch(_) {}
      }
      this._bottomDockContainerPointerMoveHandler = null;
      this._bottomDockContainerPointerUpHandler = null;
    },

    startBottomDockContainerPointerDrag(ev) {
      if (!ev || Number(ev.button) !== 0) return;
      if (String(this.bottomDockDragId || '').trim()) return;
      var target = ev && ev.target ? ev.target : null;
      if (target && typeof target.closest === 'function') {
        var tileNode = target.closest('.bottom-dock-btn[data-dock-id]');
        if (tileNode) return;
      }
      if (this._bottomDockContainerSettleTimer) {
        try { clearTimeout(this._bottomDockContainerSettleTimer); } catch(_) {}
      }
      this._bottomDockContainerSettleTimer = 0;
      this.bottomDockContainerSettling = false;
      var anchor = this.bottomDockAnchorForSnapId(this.bottomDockPlacementId);
      this._bottomDockContainerPointerActive = true;
      this._bottomDockContainerPointerMoved = false;
      this._bottomDockContainerPointerStartX = Number(ev.clientX || 0);
      this._bottomDockContainerPointerStartY = Number(ev.clientY || 0);
      this._bottomDockContainerPointerLastX = Number(ev.clientX || 0);
      this._bottomDockContainerPointerLastY = Number(ev.clientY || 0);
      this._bottomDockContainerOriginX = Number(anchor.x || 0);
      this._bottomDockContainerOriginY = Number(anchor.y || 0);
      this.bottomDockContainerDragX = Number(anchor.x || 0);
      this.bottomDockContainerDragY = Number(anchor.y || 0);
      this.bindBottomDockContainerPointerListeners();
      try {
        if (ev.currentTarget && typeof ev.currentTarget.setPointerCapture === 'function' && Number.isFinite(ev.pointerId)) {
          ev.currentTarget.setPointerCapture(ev.pointerId);
        }
      } catch(_) {}
      if (ev.cancelable && typeof ev.preventDefault === 'function') ev.preventDefault();
    },

    handleBottomDockContainerPointerMove(ev) {
      if (!this._bottomDockContainerPointerActive) return;
      var nextX = Number(ev.clientX || 0);
      var nextY = Number(ev.clientY || 0);
      this._bottomDockContainerPointerLastX = nextX;
      this._bottomDockContainerPointerLastY = nextY;
      var movedX = Math.abs(nextX - Number(this._bottomDockContainerPointerStartX || 0));
      var movedY = Math.abs(nextY - Number(this._bottomDockContainerPointerStartY || 0));
      if (!this._bottomDockContainerPointerMoved) {
        if (movedX < 4 && movedY < 4) return;
        this._bottomDockContainerPointerMoved = true;
        this.bottomDockContainerDragActive = true;
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
      }
      var candidateX = Number(this._bottomDockContainerOriginX || 0) + (nextX - Number(this._bottomDockContainerPointerStartX || 0));
      var candidateY = Number(this._bottomDockContainerOriginY || 0) + (nextY - Number(this._bottomDockContainerPointerStartY || 0));
      var anchor = this.bottomDockClampDragAnchor(candidateX, candidateY);
      this.bottomDockContainerDragX = Number(anchor.x || 0);
      this.bottomDockContainerDragY = Number(anchor.y || 0);
      var nearestId = this.bottomDockNearestSnapId(anchor.x, anchor.y);
      var side = this.bottomDockSideForSnapId(nearestId);
      this.bottomDockRotationDeg = this.bottomDockResolveRotationForSide(side, anchor.x, anchor.y);
      if (ev.cancelable && typeof ev.preventDefault === 'function') ev.preventDefault();
    },

    endBottomDockContainerPointerDrag() {
      if (!this._bottomDockContainerPointerActive) return;
      this._bottomDockContainerPointerActive = false;
      this.unbindBottomDockContainerPointerListeners();
      if (!this._bottomDockContainerPointerMoved) {
        this.bottomDockContainerDragActive = false;
        this._bottomDockContainerPointerMoved = false;
        return;
      }
      var anchor = this.bottomDockClampDragAnchor(this.bottomDockContainerDragX, this.bottomDockContainerDragY);
      var nearestId = this.bottomDockNearestSnapId(anchor.x, anchor.y);
      this.bottomDockPlacementId = nearestId;
      this.bottomDockRotationDeg = this.bottomDockResolveRotationForSide(this.bottomDockSideForSnapId(nearestId), anchor.x, anchor.y);
      this.persistBottomDockPlacement();
      this.bottomDockContainerDragActive = false;
      this.bottomDockContainerSettling = true;
      this._bottomDockContainerPointerMoved = false;
      if (this._bottomDockContainerSettleTimer) {
        try { clearTimeout(this._bottomDockContainerSettleTimer); } catch(_) {}
      }
      var self = this;
      var settleMs = this.bottomDockMoveDurationMs() + 36;
      this._bottomDockContainerSettleTimer = window.setTimeout(function() {
        self._bottomDockContainerSettleTimer = 0;
        self.bottomDockContainerSettling = false;
      }, settleMs);
    },

    settleBottomDockDragGhost(dragId, done) {
      var finish = typeof done === 'function' ? done : function() {};
      var ghost = this._bottomDockDragGhostEl;
      if (!ghost || !document) {
        this.cleanupBottomDockDragGhost();
        finish();
        return;
      }
      var key = String(dragId || '').trim();
      if (!key) {
        this.cleanupBottomDockDragGhost();
        finish();
        return;
      }
      var slot = document.querySelector('.bottom-dock-btn[data-dock-id="' + key + '"]');
      if (!slot || typeof slot.getBoundingClientRect !== 'function') {
        this.cleanupBottomDockDragGhost();
        finish();
        return;
      }
      var rect = slot.getBoundingClientRect();
      var durationMs = this.bottomDockMoveDurationMs();
      var targetWidth = Number(rect && rect.width ? rect.width : 0);
      var targetHeight = Number(rect && rect.height ? rect.height : 0);
      var slotStyle = null;
      if (!Number.isFinite(targetWidth) || targetWidth <= 0) {
        targetWidth = Number(ghost.offsetWidth || 32);
      }
      if (!Number.isFinite(targetHeight) || targetHeight <= 0) {
        targetHeight = Number(ghost.offsetHeight || 32);
      }
      var slotRadiusPx = 0;
      try {
        if (typeof window !== 'undefined' && typeof window.getComputedStyle === 'function') {
          slotStyle = window.getComputedStyle(slot);
        }
        var rawRadius = slotStyle ? String(slotStyle.borderTopLeftRadius || slotStyle.borderRadius || '') : '';
        var rawWidth = slotStyle ? String(slotStyle.width || '') : '';
        var parsedRadius = parseFloat(rawRadius);
        var parsedWidth = parseFloat(rawWidth);
        if (Number.isFinite(parsedRadius) && parsedRadius >= 0) {
          if (Number.isFinite(parsedWidth) && parsedWidth > 0) {
            slotRadiusPx = (parsedRadius / parsedWidth) * targetWidth;
          } else {
            slotRadiusPx = parsedRadius;
          }
        }
      } catch(_) {}
      if (!slotRadiusPx) {
        slotRadiusPx = Math.round((targetWidth / 32) * 11);
      }
      ghost.style.transition =
        'left ' + durationMs + 'ms var(--ease-smooth), ' +
        'top ' + durationMs + 'ms var(--ease-smooth), ' +
        'width ' + durationMs + 'ms var(--ease-smooth), ' +
        'height ' + durationMs + 'ms var(--ease-smooth), ' +
        'border-radius ' + durationMs + 'ms var(--ease-smooth), ' +
        'opacity ' + durationMs + 'ms var(--ease-smooth)';
      var targetX = Number(rect.left || 0) + ((Number(rect.width || 0) - targetWidth) / 2);
      var targetY = Number(rect.top || 0) + ((Number(rect.height || 0) - targetHeight) / 2);
      var self = this;
      var moveGhost = function() {
        if (slotStyle) {
          ghost.style.background = String(slotStyle.background || ghost.style.background || '');
          ghost.style.border = String(slotStyle.border || ghost.style.border || '');
          ghost.style.borderWidth = String(slotStyle.borderTopWidth || ghost.style.borderWidth || '');
          ghost.style.borderStyle = String(slotStyle.borderTopStyle || ghost.style.borderStyle || '');
          ghost.style.borderColor = String(slotStyle.borderColor || ghost.style.borderColor || '');
          ghost.style.boxShadow = String(slotStyle.boxShadow || ghost.style.boxShadow || '');
          ghost.style.color = String(slotStyle.color || ghost.style.color || '');
        }
        ghost.style.left = targetX + 'px';
        ghost.style.top = targetY + 'px';
        ghost.style.width = targetWidth + 'px';
        ghost.style.height = targetHeight + 'px';
        ghost.style.borderRadius = slotRadiusPx + 'px';
        ghost.style.setProperty(
          '--dock-ghost-scale',
          String(Math.max(0.8, Math.min(4, targetWidth / 32)))
        );
        ghost.style.opacity = '1';
      };
      if (typeof requestAnimationFrame === 'function') requestAnimationFrame(moveGhost);
      else moveGhost();
      if (this._bottomDockGhostCleanupTimer) {
        try { clearTimeout(this._bottomDockGhostCleanupTimer); } catch(_) {}
      }
      this._bottomDockGhostCleanupTimer = window.setTimeout(function() {
        self._bottomDockRevealTargetDuringSettle = true;
        var settleHoldMs = 54;
        var completeSettle = function() {
          self._bottomDockGhostCleanupTimer = 0;
          finish();
          if (typeof requestAnimationFrame !== 'function') {
            self.cleanupBottomDockDragGhost();
            return;
          }
          requestAnimationFrame(function() {
            requestAnimationFrame(function() {
              self.cleanupBottomDockDragGhost();
            });
          });
        };
        self._bottomDockGhostCleanupTimer = window.setTimeout(completeSettle, settleHoldMs);
      }, durationMs + 40);
    },
