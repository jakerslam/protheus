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
