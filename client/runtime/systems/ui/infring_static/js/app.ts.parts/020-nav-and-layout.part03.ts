    bottomDockMoveDurationMs() {
      return this.dragSurfaceMoveDurationMs(this._bottomDockMoveDurationMs, 360);
    },

    bottomDockExpandedScale() {
      var raw = Number(this._bottomDockExpandedScale || 1.54);
      if (!Number.isFinite(raw) || raw <= 1) raw = 1.54;
      return raw;
    },

    bottomDockReadViewportSize() {
      var width = 0;
      var height = 0;
      try {
        width = Number(window && window.innerWidth || 0);
        height = Number(window && window.innerHeight || 0);
      } catch(_) {
        width = 0;
        height = 0;
      }
      if (!Number.isFinite(width) || width <= 0) {
        width = Number(document && document.documentElement && document.documentElement.clientWidth || 1440);
      }
      if (!Number.isFinite(height) || height <= 0) {
        height = Number(document && document.documentElement && document.documentElement.clientHeight || 900);
      }
      if (!Number.isFinite(width) || width <= 0) width = 1440;
      if (!Number.isFinite(height) || height <= 0) height = 900;
      return { width: width, height: height };
    },

    bottomDockReadBaseSize() {
      var width = 0;
      var height = 0;
      try {
        var node = document && typeof document.querySelector === 'function'
          ? document.querySelector('.bottom-dock')
          : null;
        if (node) {
          width = Number(node.offsetWidth || 0);
          height = Number(node.offsetHeight || 0);
        }
      } catch(_) {
        width = 0;
        height = 0;
      }
      if (!Number.isFinite(width) || width <= 0) width = 420;
      if (!Number.isFinite(height) || height <= 0) height = 54;
      return { width: width, height: height };
    },

    bottomDockNormalizeSide(side) {
      var key = String(side || '').trim().toLowerCase();
      if (key === 'top' || key === 'left' || key === 'right') return key;
      return 'bottom';
    },

    bottomDockIsVerticalSide(side) {
      var key = this.bottomDockNormalizeSide(side);
      return key === 'left' || key === 'right';
    },

    bottomDockRotationDegForSide(side) {
      var key = this.bottomDockNormalizeSide(side);
      if (key === 'left') return -90;
      if (key === 'right') return 90;
      return 0;
    },

    bottomDockIconRotationDegForSide(side) {
      var key = this.bottomDockNormalizeSide(side);
      if (key === 'left') return 90;
      if (key === 'right') return -90;
      return 0;
    },

    bottomDockUpDegForSide(side) {
      var key = this.bottomDockNormalizeSide(side);
      if (key === 'left' || key === 'right' || key === 'top' || key === 'bottom') return 0;
      return 0;
    },

    bottomDockOrientation(sideHint) {
      var side = this.bottomDockNormalizeSide(sideHint || this.bottomDockActiveSide());
      var horizontal = !this.bottomDockIsVerticalSide(side);
      var axis = horizontal ? 'x' : 'y';
      return {
        side: side,
        horizontal: horizontal,
        axis: axis,
        primarySign: 1,
        upDeg: Number(this.bottomDockUpDegForSide(side) || 0)
      };
    },

    bottomDockOppositeSide(sideHint) {
      var side = this.bottomDockNormalizeSide(sideHint);
      if (side === 'left') return 'right';
      if (side === 'right') return 'left';
      if (side === 'top') return 'bottom';
      return 'top';
    },

    bottomDockWallSide() {
      return this.bottomDockNormalizeSide(this.bottomDockActiveSide());
    },

    bottomDockOpenSide() {
      return this.bottomDockOppositeSide(this.bottomDockWallSide());
    },

    bottomDockRotationDegResolved(sideHint) {
      var side = this.bottomDockNormalizeSide(sideHint || this.bottomDockActiveSide());
      var rotationDeg = Number(this.bottomDockRotationDeg);
      if (!Number.isFinite(rotationDeg)) {
        rotationDeg = Number(this.bottomDockRotationDegForSide(side));
      }
      return Number(this.bottomDockNormalizeRotationDeg(rotationDeg) || 0);
    },

    bottomDockScreenDeltaToLocal(dx, dy, sideHint) {
      var screenDx = Number(dx || 0);
      var screenDy = Number(dy || 0);
      var rotationDeg = this.bottomDockRotationDegResolved(sideHint);
      var theta = (rotationDeg * Math.PI) / 180;
      var cos = Math.cos(theta);
      var sin = Math.sin(theta);
      return {
        x: (screenDx * cos) + (screenDy * sin),
        y: (-screenDx * sin) + (screenDy * cos)
      };
    },

    bottomDockCanonicalRotationCandidatesForSide(side) {
      var key = this.bottomDockNormalizeSide(side);
      if (key === 'left' || key === 'right') return [90, -90];
      return [0];
    },

    bottomDockNormalizeRotationDeg(value) {
      var raw = Number(value);
      var canonical = [-90, 0, 90];
      if (!Number.isFinite(raw)) return 0;
      var best = canonical[0];
      var bestDist = Number.POSITIVE_INFINITY;
      for (var i = 0; i < canonical.length; i += 1) {
        var candidate = canonical[i];
        var dist = Math.abs(raw - candidate);
        if (dist < bestDist) {
          bestDist = dist;
          best = candidate;
        }
      }
      return best;
    },

    bottomDockResolveShortestRotationDeg(currentDeg, targetDeg) {
      var current = Number(currentDeg);
      var target = Number(targetDeg);
      if (!Number.isFinite(target)) target = 0;
      if (!Number.isFinite(current)) return target;
      var best = target;
      var bestDelta = Number.POSITIVE_INFINITY;
      for (var k = -2; k <= 2; k += 1) {
        var candidate = target + (k * 360);
        var delta = Math.abs(candidate - current);
        if (delta < bestDelta) {
          bestDelta = delta;
          best = candidate;
        }
      }
      return best;
    },

    bottomDockPreferredRotationDirectionForAnchor(anchorX, anchorY) {
      var view = this.bottomDockReadViewportSize();
      var x = Number(anchorX);
      var y = Number(anchorY);
      if (!Number.isFinite(x)) x = Number(view.width || 0) * 0.5;
      if (!Number.isFinite(y)) y = Number(view.height || 0) * 0.5;
      var left = x < (Number(view.width || 0) * 0.5);
      var top = y < (Number(view.height || 0) * 0.5);
      // TL + BR => counterclockwise. TR + BL => clockwise.
      return (left === top) ? 'ccw' : 'cw';
    },

    bottomDockResolveDirectionalRotationDeg(currentDeg, targetDeg, direction) {
      var current = Number(currentDeg);
      var target = Number(targetDeg);
      var dir = String(direction || '').trim().toLowerCase();
      if (!Number.isFinite(target)) target = 0;
      if (!Number.isFinite(current)) return target;
      if (dir !== 'cw' && dir !== 'ccw') {
        return this.bottomDockResolveShortestRotationDeg(current, target);
      }
      var best = null;
      var bestAbs = Number.POSITIVE_INFINITY;
      for (var k = -2; k <= 2; k += 1) {
        var candidate = target + (k * 360);
        var delta = candidate - current;
        if (dir === 'cw' && delta < 0) continue;
        if (dir === 'ccw' && delta > 0) continue;
        var absDelta = Math.abs(delta);
        if (absDelta < bestAbs) {
          bestAbs = absDelta;
          best = candidate;
        }
      }
      if (best === null) {
        return this.bottomDockResolveShortestRotationDeg(current, target);
      }
      return best;
    },

    bottomDockResolveRotationForSide(side, anchorX, anchorY) {
      var current = this.bottomDockNormalizeRotationDeg(this.bottomDockRotationDeg);
      var dir = this.bottomDockPreferredRotationDirectionForAnchor(anchorX, anchorY);
      var candidates = this.bottomDockCanonicalRotationCandidatesForSide(side);
      if (!Array.isArray(candidates) || !candidates.length) return current;
      var best = Number(candidates[0] || 0);
      var bestScore = Number.POSITIVE_INFINITY;
      var bestDeltaAbs = Number.POSITIVE_INFINITY;
      for (var i = 0; i < candidates.length; i += 1) {
        var target = Number(candidates[i] || 0);
        var delta = target - current;
        var deltaAbs = Math.abs(delta);
        var directionPenalty = 0;
        if (dir === 'cw' && delta < 0) directionPenalty = 0.35;
        if (dir === 'ccw' && delta > 0) directionPenalty = 0.35;
        var score = deltaAbs + directionPenalty;
        if (score < bestScore || (score === bestScore && deltaAbs < bestDeltaAbs)) {
          best = target;
          bestScore = score;
          bestDeltaAbs = deltaAbs;
        }
      }
      var chosenDelta = best - current;
      if (Math.abs(chosenDelta) > 90) {
        if (dir === 'cw') return current + 90;
        if (dir === 'ccw') return current - 90;
        return current + (chosenDelta > 0 ? 90 : -90);
      }
      return best;
    },

    bottomDockSnapDefinitions() {
      var source = Array.isArray(this.bottomDockSnapPoints) ? this.bottomDockSnapPoints : [];
      var out = [];
      var seen = {};
      for (var i = 0; i < source.length; i += 1) {
        var row = source[i];
        if (!row || typeof row !== 'object') continue;
        var id = String(row.id || '').trim().toLowerCase();
        if (!id || seen[id]) continue;
        var nx = Number(row.x);
        var ny = Number(row.y);
        var side = this.bottomDockNormalizeSide(row.side);
        if (!Number.isFinite(nx)) nx = 0.5;
        if (!Number.isFinite(ny)) ny = 0.995;
        nx = Math.max(0, Math.min(1, nx));
        ny = Math.max(0, Math.min(1, ny));
        seen[id] = true;
        out.push({ id: id, x: nx, y: ny, side: side });
      }
      if (!out.length) {
        out.push({ id: 'center', x: 0.5, y: 0.995, side: 'bottom' });
      }
      return out;
    },

    bottomDockSnapDefinitionById(id) {
      var key = String(id || '').trim().toLowerCase();
      var defs = this.bottomDockSnapDefinitions();
      if (!defs.length) return null;
      for (var i = 0; i < defs.length; i += 1) {
        if (defs[i] && defs[i].id === key) return defs[i];
      }
      for (var j = 0; j < defs.length; j += 1) {
        if (defs[j] && defs[j].id === 'center') return defs[j];
      }
      return defs[0] || null;
    },

    bottomDockSideForSnapId(id) {
      var snap = this.bottomDockSnapDefinitionById(id);
      return this.bottomDockNormalizeSide(snap && snap.side || 'bottom');
    },

    bottomDockActiveSnapId() {
      if (this.bottomDockContainerDragActive) {
        var anchor = this.bottomDockClampDragAnchor(this.bottomDockContainerDragX, this.bottomDockContainerDragY);
        return this.bottomDockNearestSnapId(anchor.x, anchor.y);
      }
      var snap = this.bottomDockSnapDefinitionById(this.bottomDockPlacementId);
      return String(snap && snap.id || 'center');
    },

    bottomDockActiveSide() {
      return this.bottomDockSideForSnapId(this.bottomDockActiveSnapId());
    },

    bottomDockWallLockNormalized() {
      return this.dragSurfaceNormalizeWall(this.bottomDockContainerWallLock);
    },

    bottomDockTaskbarContained() {
      var service = this.taskbarDockService();
      if (service && typeof service.dockTaskbarContained === 'function') {
        return service.dockTaskbarContained(
          this.bottomDockWallLockNormalized(),
          this.taskbarDockEdge,
          this.taskbarDockDragActive,
          this._taskbarDockDraggingContainedBottomDock
        );
      }
      var wall = this.bottomDockWallLockNormalized();
      if (wall !== 'top' && wall !== 'bottom') return false;
      if (this.taskbarDockDragActive && String(this._taskbarDockDraggingContainedBottomDock || '') === wall) return true;
      return wall === this.taskbarDockEdgeNormalized(this.taskbarDockEdge);
    },

    bottomDockHoverExpansionDisabled() {
      return this.bottomDockTaskbarContained();
    },

    bottomDockTaskbarContainedAnchorX(sideHint) {
      var side = this.bottomDockNormalizeSide(sideHint || this.bottomDockActiveSide());
      var view = this.bottomDockReadViewportSize();
      var size = this.bottomDockVisualSizeForSide(side);
      var dockWidth = Math.max(1, Number(size && size.width || 1));
      var left = 16;
      try {
        var textMenu = document.querySelector('.global-taskbar .taskbar-text-menus');
        var rect = textMenu && typeof textMenu.getBoundingClientRect === 'function' ? textMenu.getBoundingClientRect() : null;
        if (rect && Number.isFinite(Number(rect.right))) left = Number(rect.right) + 8;
      } catch(_) {}
      var service = this.taskbarDockService();
      if (service && typeof service.dockTaskbarContainedAnchorX === 'function') {
        return service.dockTaskbarContainedAnchorX({
          side: side,
          viewportWidth: Number(view.width || 0),
          dockWidth: dockWidth,
          leftAnchor: left
        });
      }
      var minX = dockWidth / 2;
      var maxX = Math.max(minX, Number(view.width || 0) - minX - 10);
      return Math.max(minX, Math.min(maxX, left + (dockWidth / 2)));
    },

    bottomDockTaskbarContainedMetrics() {
      var edge = this.taskbarDockEdgeNormalized(this.taskbarDockEdge);
      var height = 32;
      var centerY = edge === 'bottom' ? this.taskbarReadViewportHeight() - 23 : 23;
      try {
        var group = document.querySelector('.global-taskbar .taskbar-visual-group-left');
        var rect = group && typeof group.getBoundingClientRect === 'function' ? group.getBoundingClientRect() : null;
        if (rect && Number.isFinite(Number(rect.height)) && Number(rect.height) > 0) {
          height = Number(rect.height);
          centerY = Number(rect.top || 0) + (height / 2);
        }
      } catch(_) {}
      if (this.taskbarDockDragActive && String(this._taskbarDockDraggingContainedBottomDock || '')) {
        centerY = this.taskbarClampDragY(this.taskbarDockDragY) + (this.taskbarReadHeight() / 2);
      }
      var service = this.taskbarDockService();
      if (service && typeof service.dockTaskbarContainedMetrics === 'function') {
        return service.dockTaskbarContainedMetrics({
          edge: edge,
          viewportHeight: this.taskbarReadViewportHeight(),
          fallbackHeight: 32,
          groupHeight: height,
          groupTop: centerY - (height / 2),
          dragging: this.taskbarDockDragActive && String(this._taskbarDockDraggingContainedBottomDock || ''),
          dragY: this.taskbarClampDragY(this.taskbarDockDragY),
          taskbarHeight: this.taskbarReadHeight()
        });
      }
      return { height: height, centerY: centerY };
    },

    bottomDockSetWallLock(wallRaw) {
      var wall = this.dragSurfaceNormalizeWall(wallRaw);
      this.bottomDockContainerWallLock = wall;
      try {
        if (wall) localStorage.setItem('infring-bottom-dock-wall-lock', wall);
        else localStorage.removeItem('infring-bottom-dock-wall-lock');
        localStorage.removeItem('infring-bottom-dock-smash-wall');
        infringUpdateShellLayoutConfig(function(config) { config.dock.wallLock = wall; });
      } catch(_) {}
      return wall;
    },

    bottomDockBoundsScaleForSide(sideHint) {
      var side = this.bottomDockNormalizeSide(sideHint || this.bottomDockActiveSide());
      if (this.bottomDockTaskbarContained()) return 1;
      if (side === 'left' || side === 'right') return 1;
      var expandedScale = this.bottomDockExpandedScale();
      var baseScale = 0.95;
      var dragging = !!this.bottomDockContainerDragActive
        || !!this.bottomDockContainerSettling
        || !!String(this.bottomDockDragId || '').trim();
      var hovering = !!String(this.bottomDockHoverId || '').trim();
      if (this.bottomDockHoverExpansionDisabled()) hovering = false;
      if (dragging || hovering) baseScale = expandedScale;
      if (!Number.isFinite(baseScale) || baseScale <= 0.01) baseScale = 0.95;
      return baseScale;
    },

    bottomDockVisualSizeForSide(sideHint) {
      var side = this.bottomDockNormalizeSide(sideHint || this.bottomDockActiveSide());
      var dock = this.bottomDockReadBaseSize();
      var scale = this.bottomDockBoundsScaleForSide(side);
      var baseWidth = Math.max(20, Number(dock.width || 0) * scale);
      var baseHeight = Math.max(20, Number(dock.height || 0) * scale);
      var visualWidth = this.bottomDockIsVerticalSide(side) ? baseHeight : baseWidth;
      var visualHeight = this.bottomDockIsVerticalSide(side) ? baseWidth : baseHeight;
      return { side: side, width: visualWidth, height: visualHeight };
    },

    bottomDockHardBoundsForSide(sideHint) {
      var size = this.bottomDockVisualSizeForSide(sideHint);
      var view = this.bottomDockReadViewportSize();
      var width = Number(size && size.width || 0);
      var height = Number(size && size.height || 0);
      if (!Number.isFinite(width) || width < 1) width = 1;
      if (!Number.isFinite(height) || height < 1) height = 1;
      var viewportWidth = Number(view && view.width || 0);
      var viewportHeight = Number(view && view.height || 0);
      if (!Number.isFinite(viewportWidth) || viewportWidth <= 0) viewportWidth = 1440;
      if (!Number.isFinite(viewportHeight) || viewportHeight <= 0) viewportHeight = 900;
      return {
        minLeft: 0,
        maxLeft: Math.max(0, viewportWidth - width),
        minTop: 0,
        maxTop: Math.max(0, viewportHeight - height)
      };
    },

    bottomDockTopLeftFromAnchor(anchorX, anchorY, sideHint) {
      var size = this.bottomDockVisualSizeForSide(sideHint);
      var x = Number(anchorX);
      var y = Number(anchorY);
      if (!Number.isFinite(x)) x = Number(this.bottomDockReadViewportSize().width || 0) * 0.5;
      if (!Number.isFinite(y)) y = Number(this.bottomDockReadViewportSize().height || 0) * 0.5;
      var side = this.bottomDockNormalizeSide(size && size.side);
      var top = y - (Number(size.height || 0) / 2);
      if (side === 'top') top = y;
      else if (side === 'bottom') top = y - Number(size.height || 0);
      return {
        left: x - (Number(size.width || 0) / 2),
        top: top,
        side: side
      };
    },

    bottomDockAnchorFromTopLeft(leftRaw, topRaw, sideHint) {
      var size = this.bottomDockVisualSizeForSide(sideHint);
      var left = Number(leftRaw);
      var top = Number(topRaw);
      if (!Number.isFinite(left)) left = Number(size.width || 0) / -2;
      if (!Number.isFinite(top)) top = Number(size.height || 0) / -2;
      var side = this.bottomDockNormalizeSide(size && size.side);
      var y = top + (Number(size.height || 0) / 2);
      if (side === 'top') y = top;
      else if (side === 'bottom') y = top + Number(size.height || 0);
      return {
        x: left + (Number(size.width || 0) / 2),
        y: y,
        side: side
      };
    },

    bottomDockLocalWallForRotation(wallRaw, rotationDegRaw) {
      var wall = this.dragSurfaceNormalizeWall(wallRaw);
      if (!wall) return '';
      var rotationDeg = Number(rotationDegRaw);
      if (!Number.isFinite(rotationDeg)) rotationDeg = 0;
      var theta = (this.bottomDockNormalizeRotationDeg(rotationDeg) * Math.PI) / 180;
      var vx = 0;
      var vy = 0;
      if (wall === 'left') vx = -1;
      else if (wall === 'right') vx = 1;
      else if (wall === 'top') vy = -1;
      else vy = 1;
      var localX = (vx * Math.cos(theta)) + (vy * Math.sin(theta));
      var localY = (-vx * Math.sin(theta)) + (vy * Math.cos(theta));
      if (Math.abs(localX) >= Math.abs(localY)) {
        return localX >= 0 ? 'right' : 'left';
      }
      return localY >= 0 ? 'bottom' : 'top';
    },

    bottomDockLockRadiusCssVars(wallRaw, rotationDegRaw) {
      var wall = this.dragSurfaceNormalizeWall(wallRaw);
      if (!wall) return '';
      var localWall = this.bottomDockLocalWallForRotation(wall, rotationDegRaw);
      return '--bottom-dock-radius-override:' + this.dragSurfaceRadiusByWall(localWall) + ';';
    },

    bottomDockClampDragAnchor(anchorX, anchorY) {
      var view = this.bottomDockReadViewportSize();
      var margin = 8;
      var minX = margin;
      var maxX = Number(view.width || 0) - margin;
      var minY = margin;
      var maxY = Number(view.height || 0) - margin;
      var x = Number(anchorX);
      var y = Number(anchorY);
      if (!Number.isFinite(x)) x = Number(view.width || 0) * 0.5;
      if (!Number.isFinite(y)) y = Number(view.height || 0) * 0.5;
      x = Math.max(minX, Math.min(maxX, x));
      y = Math.max(minY, Math.min(maxY, y));
      return { x: x, y: y };
    },

    bottomDockClampAnchor(anchorX, anchorY, sideOverride) {
      void sideOverride;
      return this.bottomDockClampDragAnchor(anchorX, anchorY);
    },

    bottomDockAnchorForSnapId(id) {
      var snap = this.bottomDockSnapDefinitionById(id);
      var view = this.bottomDockReadViewportSize();
      var x = Number(view.width || 0) * Number(snap && snap.x || 0.5);
      var y = Number(view.height || 0) * Number(snap && snap.y || 0.995);
      var side = this.bottomDockNormalizeSide(snap && snap.side || 'bottom');
      return this.bottomDockClampAnchor(x, y, side);
    },
