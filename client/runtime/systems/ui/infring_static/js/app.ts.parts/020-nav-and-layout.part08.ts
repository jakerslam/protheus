    setBottomDockHover(id) {
      if (String(this.bottomDockDragId || '').trim()) return;
      if (this.bottomDockContainerDragActive || this._bottomDockContainerPointerActive) return;
      var key = String(id || '').trim();
      this.bottomDockHoverId = key;
      if (this._bottomDockPreviewHideTimer) {
        try { clearTimeout(this._bottomDockPreviewHideTimer); } catch(_) {}
        this._bottomDockPreviewHideTimer = 0;
      }
      if (!Number.isFinite(this.bottomDockPointerX) || this.bottomDockPointerX <= 0) {
        try {
          var slot = document.querySelector('.bottom-dock .dock-tile-slot[data-dock-slot-id="' + key + '"]');
          if (slot && typeof slot.getBoundingClientRect === 'function') {
            var slotRect = slot.getBoundingClientRect();
            this.bottomDockPointerX = Number(slotRect.left || 0) + (Number(slotRect.width || 0) / 2);
            this.bottomDockPointerY = Number(slotRect.top || 0) + (Number(slotRect.height || 0) / 2);
          }
        } catch(_) {}
      }
      this.refreshBottomDockHoverWeights();
      this.syncBottomDockPreview();
      this.scheduleBottomDockPreviewReflow();
    },

    clearBottomDockHover(id) {
      if (id) return;
      this.bottomDockHoverId = '';
      if (!this.bottomDockHoverId) {
        this.bottomDockHoverWeightById = {};
        this.bottomDockPointerX = 0;
        this.bottomDockPointerY = 0;
        this.cancelBottomDockPreviewReflow();
        var self = this;
        if (this._bottomDockPreviewHideTimer) {
          try { clearTimeout(this._bottomDockPreviewHideTimer); } catch(_) {}
        }
        this._bottomDockPreviewHideTimer = window.setTimeout(function() {
          self._bottomDockPreviewHideTimer = 0;
          if (!String(self.bottomDockHoverId || '').trim()) {
            self.bottomDockPreviewVisible = false;
            self.bottomDockPreviewText = '';
            self.bottomDockPreviewMorphFromText = '';
            self.bottomDockPreviewLabelMorphing = false;
            self.bottomDockPreviewWidth = 0;
          }
        }, 40);
        return;
      }
      this.syncBottomDockPreview();
    },

    readBottomDockSlotCenters() {
      var out = [];
      if (typeof document === 'undefined') return out;
      var root = document.querySelector('.bottom-dock');
      if (!root || typeof root.querySelectorAll !== 'function') return out;
      var nodes = root.querySelectorAll('.dock-tile-slot[data-dock-slot-id]');
      for (var i = 0; i < nodes.length; i += 1) {
        var node = nodes[i];
        if (!node || typeof node.getAttribute !== 'function' || typeof node.getBoundingClientRect !== 'function') continue;
        var id = String(node.getAttribute('data-dock-slot-id') || '').trim();
        if (!id) continue;
        var rect = node.getBoundingClientRect();
        var centerX = Number(rect.left || 0) + (Number(rect.width || 0) / 2);
        var centerY = Number(rect.top || 0) + (Number(rect.height || 0) / 2);
        if (!Number.isFinite(centerX) || !Number.isFinite(centerY)) continue;
        out.push({ id: id, centerX: centerX, centerY: centerY });
      }
      return out;
    },

    bottomDockWeightForDistance(distancePx) {
      var d = Math.abs(Number(distancePx || 0));
      if (!Number.isFinite(d)) return 0;
      var sigma = 52;
      var exponent = -((d * d) / (2 * sigma * sigma));
      var weight = Math.exp(exponent);
      if (!Number.isFinite(weight) || weight < 0.008) return 0;
      if (weight > 1) return 1;
      return weight;
    },

    refreshBottomDockHoverWeights() {
      var side = this.bottomDockActiveSide();
      var vertical = this.bottomDockIsVerticalSide(side);
      var primaryPointer = vertical
        ? Number(this.bottomDockPointerY || 0)
        : Number(this.bottomDockPointerX || 0);
      if (!Number.isFinite(primaryPointer) || primaryPointer <= 0) {
        this.bottomDockHoverWeightById = {};
        return;
      }
      var centers = this.readBottomDockSlotCenters();
      if (!centers.length) {
        this.bottomDockHoverWeightById = {};
        return;
      }
      var nearestId = '';
      var nearestDistance = Number.POSITIVE_INFINITY;
      var weights = {};
      for (var i = 0; i < centers.length; i += 1) {
        var item = centers[i];
        if (!item || !item.id) continue;
        var anchor = vertical ? Number(item.centerY || 0) : Number(item.centerX || 0);
        var dist = Math.abs(primaryPointer - anchor);
        if (!Number.isFinite(dist)) continue;
        if (dist < nearestDistance) {
          nearestDistance = dist;
          nearestId = item.id;
        }
        weights[item.id] = this.bottomDockWeightForDistance(dist);
      }
      this.bottomDockHoverWeightById = weights;
      if (nearestId) this.bottomDockHoverId = nearestId;
    },

    updateBottomDockPointer(ev) {
      if (!ev) return;
      if (String(this.bottomDockDragId || '').trim()) return;
      if (this.bottomDockContainerDragActive || this._bottomDockContainerPointerActive) return;
      var x = Number(ev.clientX || 0);
      var y = Number(ev.clientY || 0);
      if (!Number.isFinite(x) || x <= 0) return;
      this.bottomDockPointerX = x;
      if (Number.isFinite(y) && y > 0) this.bottomDockPointerY = y;
      this.refreshBottomDockHoverWeights();
      this.syncBottomDockPreview();
    },

    reviveBottomDockHoverFromPoint(clientX, clientY) {
      if (String(this.bottomDockDragId || '').trim()) return;
      if (this.bottomDockContainerDragActive || this._bottomDockContainerPointerActive) return;
      var x = Number(clientX || 0);
      var y = Number(clientY || 0);
      if (!Number.isFinite(x) || !Number.isFinite(y) || x <= 0 || y <= 0) return;
      var root = document.querySelector('.bottom-dock');
      if (!root || typeof root.getBoundingClientRect !== 'function') return;
      var rect = root.getBoundingClientRect();
      var withinX = x >= (Number(rect.left || 0) - 16) && x <= (Number(rect.right || 0) + 16);
      var withinY = y >= (Number(rect.top || 0) - 18) && y <= (Number(rect.bottom || 0) + 18);
      if (!withinX || !withinY) return;
      this.bottomDockPointerX = x;
      this.bottomDockPointerY = y;
      this.refreshBottomDockHoverWeights();
      this.syncBottomDockPreview();
      this.scheduleBottomDockPreviewReflow();
    },

    scheduleBottomDockPreviewReflow() {
      this.cancelBottomDockPreviewReflow();
      var self = this;
      this._bottomDockPreviewReflowFrames = 10;
      var step = function() {
        if (!String(self.bottomDockHoverId || '').trim()) {
          self._bottomDockPreviewReflowRaf = 0;
          self._bottomDockPreviewReflowFrames = 0;
          return;
        }
        self.syncBottomDockPreview();
        self._bottomDockPreviewReflowFrames = Math.max(0, Number(self._bottomDockPreviewReflowFrames || 0) - 1);
        if (self._bottomDockPreviewReflowFrames <= 0) {
          self._bottomDockPreviewReflowRaf = 0;
          return;
        }
        self._bottomDockPreviewReflowRaf = requestAnimationFrame(step);
      };
      this._bottomDockPreviewReflowRaf = requestAnimationFrame(step);
    },

    cancelBottomDockPreviewReflow() {
      if (this._bottomDockPreviewReflowRaf && typeof cancelAnimationFrame === 'function') {
        try { cancelAnimationFrame(this._bottomDockPreviewReflowRaf); } catch(_) {}
      }
      this._bottomDockPreviewReflowRaf = 0;
      this._bottomDockPreviewReflowFrames = 0;
    },

    scheduleBottomDockPreviewWidthSync() {
      if (this._bottomDockPreviewWidthRaf && typeof cancelAnimationFrame === 'function') {
        try { cancelAnimationFrame(this._bottomDockPreviewWidthRaf); } catch(_) {}
      }
      var self = this;
      var syncWidth = function() {
        self._bottomDockPreviewWidthRaf = 0;
        try {
          var bubble = document && typeof document.querySelector === 'function'
            ? document.querySelector('.bottom-dock-preview-bubble')
            : null;
          if (!bubble) return;
          var stack = (typeof bubble.querySelector === 'function')
            ? bubble.querySelector('.bottom-dock-preview-bubble-label-stack')
            : null;
          var contentWidth = Number(stack && (stack.scrollWidth || stack.offsetWidth) || 0);
          var bubbleStyle = (typeof window !== 'undefined' && typeof window.getComputedStyle === 'function')
            ? window.getComputedStyle(bubble)
            : null;
          var paddingLeft = bubbleStyle ? Number(parseFloat(String(bubbleStyle.paddingLeft || '0')) || 0) : 0;
          var paddingRight = bubbleStyle ? Number(parseFloat(String(bubbleStyle.paddingRight || '0')) || 0) : 0;
          var borderLeft = bubbleStyle ? Number(parseFloat(String(bubbleStyle.borderLeftWidth || '0')) || 0) : 0;
          var borderRight = bubbleStyle ? Number(parseFloat(String(bubbleStyle.borderRightWidth || '0')) || 0) : 0;
          var nextWidth = contentWidth + paddingLeft + paddingRight + borderLeft + borderRight;
          if (!Number.isFinite(nextWidth) || nextWidth <= 0) return;
          self.bottomDockPreviewWidth = Math.max(0, Math.ceil(nextWidth));
        } catch(_) {}
      };
      if (typeof requestAnimationFrame === 'function') {
        this._bottomDockPreviewWidthRaf = requestAnimationFrame(syncWidth);
      } else {
        syncWidth();
      }
    },

    retriggerBottomDockPreviewLabelFx(nextLabel) {
      if (this._bottomDockPreviewLabelFxRaf && typeof cancelAnimationFrame === 'function') {
        try { cancelAnimationFrame(this._bottomDockPreviewLabelFxRaf); } catch(_) {}
      }
      if (this._bottomDockPreviewLabelFxTimer) {
        try { clearTimeout(this._bottomDockPreviewLabelFxTimer); } catch(_) {}
      }
      if (this._bottomDockPreviewLabelMorphTimer) {
        try { clearTimeout(this._bottomDockPreviewLabelMorphTimer); } catch(_) {}
      }
      this._bottomDockPreviewLabelFxRaf = 0;
      this._bottomDockPreviewLabelFxTimer = 0;
      this._bottomDockPreviewLabelMorphTimer = 0;
      this.bottomDockPreviewLabelFxReady = false;
      var self = this;
      var nextText = (typeof nextLabel === 'string')
        ? nextLabel
        : String(this.bottomDockPreviewText || '');
      var previousText = String(this.bottomDockPreviewText || '');
      this.bottomDockPreviewMorphFromText = previousText;
      this.bottomDockPreviewLabelMorphing = Boolean(previousText && nextText && previousText !== nextText);
      this.bottomDockPreviewText = nextText;
      this.scheduleBottomDockPreviewWidthSync();
      var commitLabelAndAnimateIn = function() {
        try {
          var node = document && typeof document.querySelector === 'function'
            ? document.querySelector('.bottom-dock-preview-bubble-label-stack')
            : null;
          if (node) void node.offsetWidth;
        } catch(_) {}
        if (typeof requestAnimationFrame === 'function') {
          self._bottomDockPreviewLabelFxRaf = requestAnimationFrame(function() {
            self._bottomDockPreviewLabelFxRaf = 0;
            self._bottomDockPreviewLabelFxTimer = window.setTimeout(function() {
              self._bottomDockPreviewLabelFxTimer = 0;
              self.bottomDockPreviewLabelFxReady = true;
              if (self._bottomDockPreviewLabelMorphTimer) {
                try { clearTimeout(self._bottomDockPreviewLabelMorphTimer); } catch(_) {}
              }
              self._bottomDockPreviewLabelMorphTimer = window.setTimeout(function() {
                self._bottomDockPreviewLabelMorphTimer = 0;
                self.bottomDockPreviewMorphFromText = '';
                self.bottomDockPreviewLabelMorphing = false;
                self.scheduleBottomDockPreviewWidthSync();
              }, 200);
            }, 16);
          });
        } else {
          self.bottomDockPreviewLabelFxReady = true;
        }
      };
      if (typeof requestAnimationFrame === 'function') {
        this._bottomDockPreviewLabelFxRaf = requestAnimationFrame(function() {
          self._bottomDockPreviewLabelFxRaf = 0;
          commitLabelAndAnimateIn();
        });
      } else {
        this.bottomDockPreviewLabelFxReady = true;
        this.bottomDockPreviewMorphFromText = '';
        this.bottomDockPreviewLabelMorphing = false;
      }
    },

    syncBottomDockPreview() {
      var key = String(this.bottomDockHoverId || '').trim();
      if (!key) {
        this.bottomDockPreviewVisible = false;
        this.bottomDockPreviewText = '';
        this.bottomDockPreviewMorphFromText = '';
        this.bottomDockPreviewHoverKey = '';
        this.bottomDockPreviewLabelMorphing = false;
        this.bottomDockPreviewWidth = 0;
        this.bottomDockPreviewLabelFxReady = true;
        return;
      }
      var text = this.bottomDockTileData(key, 'tooltip', '');
      var label = String(text || '').trim();
      if (!label) {
        this.bottomDockPreviewVisible = false;
        this.bottomDockPreviewText = '';
        this.bottomDockPreviewMorphFromText = '';
        this.bottomDockPreviewHoverKey = '';
        this.bottomDockPreviewLabelMorphing = false;
        this.bottomDockPreviewWidth = 0;
        this.bottomDockPreviewLabelFxReady = true;
        return;
      }
      var root = document.querySelector('.bottom-dock');
      var slot = document.querySelector('.bottom-dock .dock-tile-slot[data-dock-slot-id="' + key + '"]');
      if (!root || !slot) {
        this.bottomDockPreviewVisible = false;
        this.bottomDockPreviewText = '';
        this.bottomDockPreviewMorphFromText = '';
        this.bottomDockPreviewHoverKey = '';
        this.bottomDockPreviewLabelMorphing = false;
        this.bottomDockPreviewWidth = 0;
        this.bottomDockPreviewLabelFxReady = true;
        return;
      }
      var wasVisible = Boolean(this.bottomDockPreviewVisible);
      var previousHoverKey = String(this.bottomDockPreviewHoverKey || '');
      var previousLabel = String(this.bottomDockPreviewText || '');
      var centerX = 0;
      var centerY = 0;
      var anchorY = 0;
      var anchorX = 0;
      var wallSide = this.bottomDockWallSide();
      var openSide = this.bottomDockOpenSide();
      var vertical = this.bottomDockIsVerticalSide(wallSide);
      var dockRect = (typeof root.getBoundingClientRect === 'function')
        ? root.getBoundingClientRect()
        : null;
      if (typeof slot.getBoundingClientRect === 'function' && dockRect) {
        var slotRect = slot.getBoundingClientRect();
        centerX = Number(slotRect.left || 0) + (Number(slotRect.width || 0) / 2);
        centerY = Number(slotRect.top || 0) + (Number(slotRect.height || 0) / 2);
        if (openSide === 'top') {
          anchorY = Number(dockRect.top || 0) - 8;
        } else if (openSide === 'bottom') {
          anchorY = Number(dockRect.bottom || 0) + 8;
        } else if (openSide === 'left') {
          anchorX = Number(dockRect.left || 0) - 8;
        } else {
          anchorX = Number(dockRect.right || 0) + 8;
        }
      } else if (slot.offsetParent === root) {
        var rootRect = root.getBoundingClientRect();
        centerX = Number(rootRect.left || 0) + Number(slot.offsetLeft || 0) + (Number(slot.offsetWidth || 0) / 2);
        centerY = Number(rootRect.top || 0) + Number(slot.offsetTop || 0) + (Number(slot.offsetHeight || 0) / 2);
        if (openSide === 'top') {
          anchorY = Number(rootRect.top || 0) - 8;
        } else if (openSide === 'bottom') {
          anchorY = Number(rootRect.bottom || 0) + 8;
        } else if (openSide === 'left') {
          anchorX = Number(rootRect.left || 0) - 8;
        } else {
          anchorX = Number(rootRect.right || 0) + 8;
        }
      }
      var pointerX = Number(this.bottomDockPointerX || 0);
      var pointerY = Number(this.bottomDockPointerY || 0);
      if (!vertical && Number.isFinite(pointerX) && pointerX > 0) {
        if (dockRect) {
          var minX = Number(dockRect.left || 0);
          var maxX = Number(dockRect.right || 0);
          if (Number.isFinite(minX) && Number.isFinite(maxX) && maxX > minX) {
            pointerX = Math.max(minX, Math.min(maxX, pointerX));
          }
        }
        centerX = pointerX;
      }
      if (vertical && Number.isFinite(pointerY) && pointerY > 0) {
        if (dockRect) {
          var minY = Number(dockRect.top || 0);
          var maxY = Number(dockRect.bottom || 0);
          if (Number.isFinite(minY) && Number.isFinite(maxY) && maxY > minY) {
            pointerY = Math.max(minY, Math.min(maxY, pointerY));
          }
        }
        centerY = pointerY;
      }
      if (!Number.isFinite(centerX)) centerX = 0;
      if (!Number.isFinite(centerY)) centerY = 0;
      if (!Number.isFinite(anchorX)) anchorX = 0;
      if (!Number.isFinite(anchorY)) anchorY = 0;
      this.bottomDockPreviewX = vertical ? anchorX : centerX;
      this.bottomDockPreviewY = vertical ? centerY : anchorY;
      this.bottomDockPreviewHoverKey = key;
      this.bottomDockPreviewVisible = true;
      if (wasVisible && (key !== previousHoverKey || label !== previousLabel)) {
        this.retriggerBottomDockPreviewLabelFx(label);
      } else {
        this.bottomDockPreviewText = label;
        this.scheduleBottomDockPreviewWidthSync();
        if (!this.bottomDockPreviewLabelMorphing) {
          this.bottomDockPreviewMorphFromText = '';
        }
        if (!wasVisible && !this.bottomDockPreviewLabelFxReady) {
          this.bottomDockPreviewLabelFxReady = true;
        }
      }
    },
