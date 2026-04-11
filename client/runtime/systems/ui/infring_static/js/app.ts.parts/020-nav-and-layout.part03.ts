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
      var view = this.bottomDockReadViewportSize();
      var dock = this.bottomDockReadBaseSize();
      var side = this.bottomDockNormalizeSide(sideOverride);
      var hoverScale = this.bottomDockExpandedScale();
      if (!Number.isFinite(hoverScale) || hoverScale < 1) hoverScale = 1;
      if (side === 'left' || side === 'right') hoverScale = 1;
      var margin = 8;
      var baseWidth = Math.max(20, Number(dock.width || 0) * hoverScale);
      var baseHeight = Math.max(20, Number(dock.height || 0) * hoverScale);
      var visualWidth = this.bottomDockIsVerticalSide(side) ? baseHeight : baseWidth;
      var visualHeight = this.bottomDockIsVerticalSide(side) ? baseWidth : baseHeight;
      var halfWidth = visualWidth / 2;
      var halfHeight = visualHeight / 2;
      var minX = margin;
      var maxX = Number(view.width || 0) - margin;
      var minY = margin;
      var maxY = Number(view.height || 0) - margin;
      if (side === 'bottom') {
        minX = halfWidth + margin;
        maxX = Number(view.width || 0) - halfWidth - margin;
        minY = visualHeight + margin;
        maxY = Number(view.height || 0) - margin;
      } else if (side === 'top') {
        minX = halfWidth + margin;
        maxX = Number(view.width || 0) - halfWidth - margin;
        minY = margin;
        maxY = Number(view.height || 0) - visualHeight - margin;
      } else if (side === 'left') {
        minX = halfWidth + margin;
        maxX = Number(view.width || 0) - halfWidth - margin;
        minY = halfHeight + margin;
        maxY = Number(view.height || 0) - halfHeight - margin;
      } else if (side === 'right') {
        minX = halfWidth + margin;
        maxX = Number(view.width || 0) - halfWidth - margin;
        minY = halfHeight + margin;
        maxY = Number(view.height || 0) - halfHeight - margin;
      }
      if (!Number.isFinite(minX) || !Number.isFinite(maxX) || maxX <= minX) {
        minX = margin;
        maxX = Number(view.width || 0) - margin;
      }
      if (!Number.isFinite(minY) || !Number.isFinite(maxY) || maxY <= minY) {
        minY = margin;
        maxY = Number(view.height || 0) - margin;
      }
      var x = Number(anchorX);
      var y = Number(anchorY);
      if (!Number.isFinite(x)) {
        if (side === 'left') x = minX;
        else if (side === 'right') x = maxX;
        else x = Number(view.width || 0) * 0.5;
      }
      if (!Number.isFinite(y)) {
        if (side === 'top') y = margin;
        else if (side === 'left' || side === 'right') y = Number(view.height || 0) * 0.5;
        else y = Number(view.height || 0) - margin;
      }
      x = Math.max(minX, Math.min(maxX, x));
      y = Math.max(minY, Math.min(maxY, y));
      return { x: x, y: y };
    },

    bottomDockAnchorForSnapId(id) {
      var snap = this.bottomDockSnapDefinitionById(id);
      var view = this.bottomDockReadViewportSize();
      var x = Number(view.width || 0) * Number(snap && snap.x || 0.5);
      var y = Number(view.height || 0) * Number(snap && snap.y || 0.995);
      var side = this.bottomDockNormalizeSide(snap && snap.side || 'bottom');
      return this.bottomDockClampAnchor(x, y, side);
    },
