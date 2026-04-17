    shouldSuppressBottomDockClick() {
      var until = Number(this._bottomDockSuppressClickUntil || 0);
      return Number.isFinite(until) && until > Date.now();
    },

    clearBottomDockClickAnimation() {
      if (this._bottomDockClickAnimTimer) {
        try { clearTimeout(this._bottomDockClickAnimTimer); } catch(_) {}
      }
      this._bottomDockClickAnimTimer = 0;
      this.bottomDockClickAnimId = '';
    },

    triggerBottomDockClickAnimation(id, durationOverrideMs) {
      var key = String(id || '').trim();
      if (!key || typeof window === 'undefined' || typeof window.setTimeout !== 'function') return;
      this.clearBottomDockClickAnimation();
      this.bottomDockClickAnimId = key;
      var self = this;
      var durationMs = Number(durationOverrideMs);
      if (!Number.isFinite(durationMs) || durationMs < 120) {
        durationMs = Number(self._bottomDockClickAnimDurationMs || 980);
      }
      if (!Number.isFinite(durationMs) || durationMs < 120) durationMs = 980;
      if (typeof document !== 'undefined') {
        try {
          var tileNode = document.querySelector('.bottom-dock-btn[data-dock-id="' + key + '"]');
          if (tileNode && tileNode.style && typeof tileNode.style.setProperty === 'function') {
            tileNode.style.setProperty('--dock-click-duration', Math.round(durationMs) + 'ms');
          }
        } catch(_) {}
      }
      self._bottomDockClickAnimTimer = window.setTimeout(function() {
        if (typeof document !== 'undefined') {
          try {
            var activeNode = document.querySelector('.bottom-dock-btn[data-dock-id="' + key + '"]');
            if (activeNode && activeNode.style && typeof activeNode.style.removeProperty === 'function') {
              activeNode.style.removeProperty('--dock-click-duration');
            }
          } catch(_) {}
        }
        self._bottomDockClickAnimTimer = 0;
        self.bottomDockClickAnimId = '';
      }, durationMs);
    },

    bottomDockIsClickAnimating(id) {
      var key = String(id || '').trim();
      if (!key) return false;
      return String(this.bottomDockClickAnimId || '').trim() === key;
    },

    handleBottomDockTileClick(id, targetPage, ev) {
      if (this.shouldSuppressBottomDockClick()) return;
      var key = String(id || '').trim();
      var pageKey = String(targetPage || '').trim();
      var clickAnimation = '';
      var clickDurationMs = 0;
      try {
        var triggerEl = ev && ev.currentTarget ? ev.currentTarget : null;
        clickAnimation = String(
          triggerEl && typeof triggerEl.getAttribute === 'function'
            ? (triggerEl.getAttribute('data-dock-click-animation') || '')
            : ''
        ).trim();
        clickDurationMs = Number(
          triggerEl && typeof triggerEl.getAttribute === 'function'
            ? (triggerEl.getAttribute('data-dock-click-duration-ms') || '')
            : ''
        );
      } catch(_) {
        clickAnimation = '';
        clickDurationMs = 0;
      }
      if (!Number.isFinite(clickDurationMs) || clickDurationMs < 120) clickDurationMs = 0;
      if (key && clickAnimation && clickAnimation !== 'none') {
        this.triggerBottomDockClickAnimation(key, clickDurationMs);
      }
      if (pageKey) this.navigate(pageKey);
    },

    normalizeSidebarPopupText(rawText) {
      var text = String(rawText || '').trim();
      if (!text) return '';
      if (this.isSidebarPopupPlaceholderText(text)) return '';
      return text;
    },

    isSidebarPopupPlaceholderText(text) {
      var normalized = String(text || '').trim().toLowerCase();
      return normalized === 'no messages yet'
        || normalized === 'system events and terminal output'
        || normalized === 'no matching text'
        || normalized === 'agent';
    },

    sidebarPopupMetaOrigin(preview, fallbackLabel) {
      var role = String(preview && preview.role || '').trim().toLowerCase();
      if (role === 'user') return 'User';
      if (role === 'assistant' || role === 'agent') return 'Agent';
      if (role) return role.charAt(0).toUpperCase() + role.slice(1);
      return String(fallbackLabel || 'Sidebar').trim() || 'Sidebar';
    },

    hideDashboardPopupBySource(source) {
      var expected = String(source || '').trim();
      if (!expected) return;
      var popup = this.dashboardPopup || {};
      var currentSource = String(popup.source || '').trim();
      if (currentSource !== expected) return;
      this.hideDashboardPopup(String(popup.id || '').trim());
    },

    showCollapsedSidebarAgentPopup(agent, ev) {
      if (!this.sidebarCollapsed || !agent) {
        this.hideDashboardPopupBySource('sidebar');
        return;
      }
      var rawId = String(agent.id || '').trim();
      var rawIdLower = rawId.toLowerCase();
      var isSystemThread = (typeof this.isSystemSidebarThread === 'function')
        ? this.isSystemSidebarThread(agent)
        : (agent.is_system_thread === true || rawIdLower === 'system');
      if (isSystemThread || rawIdLower === 'settings') {
        this.hideDashboardPopupBySource('sidebar');
        return;
      }
      var preview = this.chatSidebarPreview(agent) || {};
      var previewText = this.normalizeSidebarPopupText(preview.text || '');
      var title = String(agent.name || rawId).trim();
      if (!rawId || !title || !previewText) {
        this.hideDashboardPopupBySource('sidebar');
        return;
      }
      this.showDashboardPopup('sidebar-agent:' + rawId, title, ev, {
        source: 'sidebar',
        side: 'right',
        body: previewText,
        meta_origin: this.sidebarPopupMetaOrigin(preview, 'Agent'),
        meta_time: typeof this.formatChatSidebarTime === 'function'
          ? String(this.formatChatSidebarTime(preview.ts) || '').trim()
          : '',
        unread: !!preview.unread_response
      });
    },

    showCollapsedSidebarNavPopup(label, ev) {
      if (!this.sidebarCollapsed) {
        this.hideDashboardPopupBySource('sidebar');
        return;
      }
      var navLabel = String(label || '').trim();
      var navLabelLower = navLabel.toLowerCase();
      if (!navLabel || navLabelLower === 'system' || navLabelLower === 'settings') {
        this.hideDashboardPopupBySource('sidebar');
        return;
      }
      this.showDashboardPopup('sidebar-nav:' + navLabelLower.replace(/[^a-z0-9_-]+/g, '-'), navLabel, ev, {
        source: 'sidebar',
        side: 'right',
        meta_origin: 'Sidebar'
      });
    },

    clearDashboardPopupState() {
      this.dashboardPopup = {
        id: '',
        active: false,
        source: '',
        title: '',
        body: '',
        meta_origin: '',
        meta_time: '',
        unread: false,
        left: 0,
        top: 0,
        side: 'bottom',
        compact: false
      };
    },

    normalizeDashboardPopupSide(sideValue, fallbackSide) {
      var fallback = String(fallbackSide || 'bottom').trim().toLowerCase();
      if (fallback !== 'top' && fallback !== 'left' && fallback !== 'right') fallback = 'bottom';
      var side = String(sideValue || fallback).trim().toLowerCase();
      if (side !== 'top' && side !== 'left' && side !== 'right') side = 'bottom';
      return side;
    },

    dashboardOppositeSide(sideValue) {
      var side = this.normalizeDashboardPopupSide(sideValue, 'bottom');
      if (side === 'top') return 'bottom';
      if (side === 'left') return 'right';
      if (side === 'right') return 'left';
      return 'top';
    },

    dashboardPopupWallAffinity(rect) {
      if (!rect || typeof window === 'undefined') return null;
      var viewportWidth = Number(window.innerWidth || 0);
      var viewportHeight = Number(window.innerHeight || 0);
      if (!Number.isFinite(viewportWidth) || viewportWidth <= 0) viewportWidth = 1;
      if (!Number.isFinite(viewportHeight) || viewportHeight <= 0) viewportHeight = 1;
      var left = Number(rect.left || 0);
      var right = Number(rect.right || 0);
      var top = Number(rect.top || 0);
      var bottom = Number(rect.bottom || 0);
      if (!Number.isFinite(left) || !Number.isFinite(right) || !Number.isFinite(top) || !Number.isFinite(bottom)) {
        return null;
      }
      var width = Math.max(1, Math.abs(right - left));
      var height = Math.max(1, Math.abs(bottom - top));
      var distanceToLeft = Math.max(0, left);
      var distanceToRight = Math.max(0, viewportWidth - right);
      var distanceToTop = Math.max(0, top);
      var distanceToBottom = Math.max(0, viewportHeight - bottom);
      var proximityScore = function(distance) {
        var normalized = Number(distance || 0);
        if (!Number.isFinite(normalized) || normalized < 0) normalized = 0;
        return 1 / (1 + normalized);
      };
      return {
        scores: {
          top: width * proximityScore(distanceToTop),
          bottom: width * proximityScore(distanceToBottom),
          left: height * proximityScore(distanceToLeft),
          right: height * proximityScore(distanceToRight)
        },
        distances: {
          top: distanceToTop,
          bottom: distanceToBottom,
          left: distanceToLeft,
          right: distanceToRight
        }
      };
    },

    dashboardPopupWallAnchorNode(node) {
      if (!node || typeof node.closest !== 'function') return null;
      try {
        return node.closest(
          '[data-popup-wall-anchor], .global-taskbar, .sidebar, .bottom-dock, .doc-window, .chat-window'
        );
      } catch(_) {
        return null;
      }
    },

    dashboardPopupWallRectForNode(node) {
      var anchor = this.dashboardPopupWallAnchorNode(node);
      if (!anchor || typeof anchor.getBoundingClientRect !== 'function') return null;
      try {
        return anchor.getBoundingClientRect();
      } catch(_) {
        return null;
      }
    },

    dashboardPopupSideAwayFromNearestWall(rect, fallbackSide) {
      var fallback = this.normalizeDashboardPopupSide('', fallbackSide);
      var affinity = this.dashboardPopupWallAffinity(rect);
      if (!affinity || !affinity.scores || !affinity.distances) return fallback;
      var scores = affinity.scores;
      var distances = affinity.distances;
      var walls = ['top', 'bottom', 'left', 'right'];
      var fallbackWall = this.dashboardOppositeSide(fallback);
      var winner = walls[0];
      var winnerScore = Number(scores[winner] || 0);
      var epsilon = 0.000001;
      var i;
      for (i = 1; i < walls.length; i += 1) {
        var wall = walls[i];
        var score = Number(scores[wall] || 0);
        if (score > winnerScore + epsilon) {
          winner = wall;
          winnerScore = score;
          continue;
        }
        if (Math.abs(score - winnerScore) <= epsilon) {
          if (wall === fallbackWall && winner !== fallbackWall) {
            winner = wall;
            winnerScore = score;
            continue;
          }
          var wallDistance = Number(distances[wall] || 0);
          var winnerDistance = Number(distances[winner] || 0);
          if (wallDistance < winnerDistance) {
            winner = wall;
            winnerScore = score;
          }
        }
      }
      return this.dashboardOppositeSide(winner);
    },

    dashboardPopupHorizontalAwayFromNearestWall(rect, fallbackSide) {
      var fallback = String(fallbackSide || 'right').trim().toLowerCase();
      if (fallback !== 'left') fallback = 'right';
      var affinity = this.dashboardPopupWallAffinity(rect);
      if (!affinity || !affinity.distances) return fallback;
      var distances = affinity.distances;
      var nearest = Number(distances.left || 0) <= Number(distances.right || 0)
        ? 'left'
        : 'right';
      return nearest === 'left' ? 'right' : 'left';
    },

    dashboardPopupVerticalAwayFromNearestWall(rect, fallbackSide) {
      var fallback = String(fallbackSide || 'bottom').trim().toLowerCase();
      if (fallback !== 'top') fallback = 'bottom';
      var affinity = this.dashboardPopupWallAffinity(rect);
      if (!affinity || !affinity.distances) return fallback;
      var distances = affinity.distances;
      var nearest = Number(distances.top || 0) <= Number(distances.bottom || 0)
        ? 'top'
        : 'bottom';
      return nearest === 'top' ? 'bottom' : 'top';
    },

    taskbarAnchoredDropdownClass(anchorNode, fallbackSide) {
      var fallback = this.normalizeDashboardPopupSide('', fallbackSide || 'bottom');
      var side = fallback;
      var inlineAway = 'right';
      var blockAway = 'bottom';
      if (anchorNode && typeof anchorNode.getBoundingClientRect === 'function') {
        var wallRect = this.dashboardPopupWallRectForNode(anchorNode);
        var sideRect = wallRect || anchorNode.getBoundingClientRect();
        side = this.dashboardPopupSideAwayFromNearestWall(sideRect, fallback);
        inlineAway = this.dashboardPopupHorizontalAwayFromNearestWall(sideRect, 'right');
        blockAway = this.dashboardPopupVerticalAwayFromNearestWall(sideRect, 'bottom');
      }
      return {
        'taskbar-anchored-dropdown': true,
        'is-side-top': side === 'top',
        'is-side-bottom': side === 'bottom',
        'is-side-left': side === 'left',
        'is-side-right': side === 'right',
        'is-inline-away-left': inlineAway === 'left',
        'is-inline-away-right': inlineAway === 'right',
        'is-block-away-top': blockAway === 'top',
        'is-block-away-bottom': blockAway === 'bottom'
      };
    },

    dashboardPopupAnchorPoint(ev, sideOverride) {
      var preferredSide = this.normalizeDashboardPopupSide(sideOverride, 'bottom');
      var node = ev && ev.currentTarget ? ev.currentTarget : null;
      if (!node && ev && ev.target && typeof ev.target.closest === 'function') {
        try {
          node = ev.target.closest('button,[role="button"],.taskbar-reorder-item');
        } catch(_) {
          node = null;
        }
      }
      if (!node || typeof node.getBoundingClientRect !== 'function') {
        return { left: 0, top: 0, side: preferredSide };
      }
      var rect = node.getBoundingClientRect();
      var wallRect = this.dashboardPopupWallRectForNode(node);
      var side = this.dashboardPopupSideAwayFromNearestWall(wallRect || rect, preferredSide);
      var width = Number(rect.width || 0);
      var height = Number(rect.height || 0);
      var left = Math.round(Number(rect.left || 0) + (width / 2));
      var top = Math.round(Number(rect.bottom || 0));
      if (side === 'top') {
        top = Math.round(Number(rect.top || 0));
      } else if (side === 'left') {
        left = Math.round(Number(rect.left || 0));
        top = Math.round(Number(rect.top || 0) + (height / 2));
      } else if (side === 'right') {
        left = Math.round(Number(rect.right || 0));
        top = Math.round(Number(rect.top || 0) + (height / 2));
      }
      return {
        left: left,
        top: top,
        side: side
      };
    },

    showDashboardPopup(id, label, ev, overrides) {
      var popupId = String(id || '').trim();
      var title = String(label || '').trim();
      if (!popupId || !title) {
        this.hideDashboardPopup();
        return;
      }
      var eventType = String((ev && ev.type) || '').toLowerCase();
      if (
        eventType === 'mouseleave' ||
        eventType === 'pointerleave' ||
        eventType === 'blur' ||
        eventType === 'focusout'
      ) {
        this.hideDashboardPopup(popupId);
        return;
      }
      if (ev && ev.isTrusted === false) return;
      var config = overrides && typeof overrides === 'object' ? overrides : {};
      var anchor = this.dashboardPopupAnchorPoint(ev, config.side);
      var body = String(config.body || '').trim();
      this.dashboardPopup = {
        id: popupId,
        active: true,
        source: String(config.source || '').trim(),
        title: title,
        body: body,
        meta_origin: String(config.meta_origin || 'Taskbar').trim(),
        meta_time: String(config.meta_time || '').trim(),
        unread: !!config.unread,
        left: anchor.left,
        top: anchor.top,
        side: anchor.side,
        compact: false
      };
    },

    showTaskbarNavPopup(label, ev) {
      var navLabel = String(label || '').trim();
      if (!navLabel) {
        this.hideDashboardPopup();
        return;
      }
      var navKey = navLabel.toLowerCase().replace(/[^a-z0-9_-]+/g, '-');
      var body = navKey === 'back'
        ? (this.canNavigateBack() ? 'Go to the previous page in this session' : 'No earlier page in this session')
        : (this.canNavigateForward() ? 'Go to the next page in this session' : 'No later page in this session');
      this.showDashboardPopup('taskbar-nav:' + navKey, navLabel, ev, {
        source: 'taskbar',
        side: 'bottom',
        compact: false,
        body: body,
        meta_origin: 'Chat nav'
      });
    },

    showTaskbarUtilityPopup(label, body, ev) {
      var utilityLabel = String(label || '').trim();
      if (!utilityLabel) {
        this.hideDashboardPopup();
        return;
      }
      this.showDashboardPopup(
        'taskbar-utility:' + utilityLabel.toLowerCase().replace(/[^a-z0-9_-]+/g, '-'),
        utilityLabel,
        ev,
        {
          source: 'taskbar',
          side: 'bottom',
          compact: false,
          body: String(body || '').trim(),
          meta_origin: 'Taskbar'
        }
      );
    },

    hideDashboardPopup(rawId) {
      var popupId = String(rawId || '').trim();
      var currentId = String(this.dashboardPopup && this.dashboardPopup.id || '').trim();
      if (popupId && currentId && popupId !== currentId) return;
      this.clearDashboardPopupState();
    },

    bottomDockIsDraggingVisual(id) {
      var key = String(id || '').trim();
      if (!key) return false;
      if (this._bottomDockRevealTargetDuringSettle) return false;
      return String(this.bottomDockDragId || '').trim() === key;
    },

    bottomDockIsNeighbor(id) {
      var hoverId = String(this.bottomDockHoverId || '').trim();
      var key = String(id || '').trim();
      if (!hoverId || !key || hoverId === key) return false;
      return Math.abs(this.bottomDockOrderIndex(hoverId) - this.bottomDockOrderIndex(key)) === 1;
    },

    bottomDockIsSecondNeighbor(id) {
      var hoverId = String(this.bottomDockHoverId || '').trim();
      var key = String(id || '').trim();
      if (!hoverId || !key || hoverId === key) return false;
      return Math.abs(this.bottomDockOrderIndex(hoverId) - this.bottomDockOrderIndex(key)) === 2;
    },

    bottomDockHoverWeight(id) {
      var key = String(id || '').trim();
      if (!key) return 0;
      var weights = this.bottomDockHoverWeightById && typeof this.bottomDockHoverWeightById === 'object'
        ? this.bottomDockHoverWeightById
        : null;
      if (weights && Object.prototype.hasOwnProperty.call(weights, key)) {
        var exact = Number(weights[key] || 0);
        if (Number.isFinite(exact)) return Math.max(0, Math.min(1, exact));
      }
      if (key === String(this.bottomDockHoverId || '').trim()) return 1;
      if (this.bottomDockIsNeighbor(key)) return 0.33;
      if (this.bottomDockIsSecondNeighbor(key)) return 0.11;
      return 0;
    },

    startBottomDockDrag(id, ev) {
      var key = String(id || '').trim();
      if (!key) return;
      this.cleanupBottomDockDragGhost();
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
      this.bottomDockDragId = key;
      this.bottomDockDragCommitted = false;
      this.bottomDockDragStartOrder = this.normalizeBottomDockOrder(this.bottomDockOrder);
      this._bottomDockReorderLockUntil = 0;
      this.captureBottomDockDragBoundaries(key);
      if (ev && ev.dataTransfer) {
        try { ev.dataTransfer.effectAllowed = 'move'; } catch(_) {}
        try { ev.dataTransfer.dropEffect = 'move'; } catch(_) {}
        try {
          var dragNode = ev.currentTarget;
          if (dragNode && typeof ev.dataTransfer.setDragImage === 'function') {
            var rect = dragNode.getBoundingClientRect();
            var ghost = dragNode.cloneNode(true);
            if (ghost && document && document.body) {
              ghost.classList.add('bottom-dock-drag-ghost');
              ghost.style.position = 'fixed';
              ghost.style.left = '-9999px';
              ghost.style.top = '-9999px';
              ghost.style.margin = '0';
              ghost.style.transform = 'none';
              ghost.style.pointerEvents = 'none';
              ghost.style.opacity = '1';
              document.body.appendChild(ghost);
              this._bottomDockDragGhostEl = ghost;
              ev.dataTransfer.setDragImage(
                ghost,
                Math.max(0, Math.round(Number(rect.width || 0) / 2)),
                Math.max(0, Math.round(Number(rect.height || 0) / 2))
              );
            } else {
              ev.dataTransfer.setDragImage(
                dragNode,
                Math.max(0, Math.round(Number(rect.width || 0) / 2)),
                Math.max(0, Math.round(Number(rect.height || 0) / 2))
              );
            }
          }
        } catch(_) {}
        try { ev.dataTransfer.setData('application/x-infring-dock', key); } catch(_) {}
        try { ev.dataTransfer.setData('text/plain', key); } catch(_) {}
      }
    },
