// Chat map stepping, popup, hover, and centering helpers.
'use strict';

function infringChatMapInteractionMethods() {
  return {
    stepMessageMap: function(list, dir) {
      if (!Array.isArray(list) || !list.length) return;
      this.suppressMapPreview = true;
      if (typeof this.hideDashboardPopupBySource === 'function') {
        this.hideDashboardPopupBySource(this.chatMapPopupSource());
      }
      if (this._mapPreviewSuppressTimer) clearTimeout(this._mapPreviewSuppressTimer);
      var visibleIndexes = [];
      var fallbackIndexes = [];
      var searchQuery = String(this.searchQuery || '').trim();
      for (var i = 0; i < list.length; i++) {
        if (this.isMessageDayCollapsed(list[i])) continue;
        fallbackIndexes.push(i);
        if (!searchQuery || !this.messageMatchesSearchQuery || this.messageMatchesSearchQuery(list[i], searchQuery)) visibleIndexes.push(i);

      }
      if (!visibleIndexes.length) visibleIndexes = fallbackIndexes;
      if (!visibleIndexes.length) return;

      var activePos = -1;
      var anchorDomId = String(this.selectedMessageDomId || '');
      if (anchorDomId) {
        for (var p = 0; p < visibleIndexes.length; p++) {
          var vi = visibleIndexes[p];
          if (this.messageDomId(list[vi], vi) === anchorDomId) {
            activePos = p;
            break;
          }
        }
      }
      if (activePos < 0) {
        for (var p2 = 0; p2 < visibleIndexes.length; p2++) {
          if (visibleIndexes[p2] === this.mapStepIndex) {
            activePos = p2;
            break;
          }
        }
      }

      if (activePos < 0) {
        activePos = dir > 0 ? 0 : (visibleIndexes.length - 1);
      } else {
        activePos = activePos + (dir > 0 ? 1 : -1);
        if (activePos < 0) activePos = 0;
        if (activePos > visibleIndexes.length - 1) activePos = visibleIndexes.length - 1;
      }

      var next = visibleIndexes[activePos];
      var msg = list[next];
      if (!msg) return;
      this.setHoveredMessage(msg, next);
      this.jumpToMessage(msg, next);
      this.centerChatMapOnMessage(this.messageDomId(msg, next));
      var self = this;
      this._mapPreviewSuppressTimer = setTimeout(function() {
        self.suppressMapPreview = false;
      }, 220);
    },

    chatMapPopupSource: function() {
      return 'chat-map';
    },

    messageMapPopupTitle: function(msg) {
      if (!msg) return 'Message';
      return this.messageActorLabel(msg);
    },

    messageMapPopupBody: function(msg) {
      if (!msg) return '';
      var preview = typeof this.messageVisiblePreviewText === 'function' ? this.messageVisiblePreviewText(msg) : '';
      if (!preview && typeof this.messageMapPreview === 'function') preview = this.messageMapPreview(msg);
      return String(preview || '').trim();
    },

    showMapItemPopup: function(msg, idx, ev) {
      if (!msg) return;
      var domId = this.messageDomId(msg, idx);
      this.forceMessageRender(msg, idx, 9000);
      this.suppressMapPreview = false;
      this.selectedMessageDomId = domId;
      this.mapStepIndex = idx;
      this.setHoveredMessage(msg, idx);
      if (typeof this.showDashboardPopup !== 'function') return;
      this.showDashboardPopup('chat-map-item:' + domId, this.messageMapPopupTitle(msg), ev, {
        source: this.chatMapPopupSource(),
        side: 'left',
        body: this.messageMapPopupBody(msg),
        meta_origin: 'Chat map',
        meta_time: String(this.messageTimestampLabel(msg) || '').trim()
      });
    },

    hideMapItemPopup: function() {
      if (typeof this.hideDashboardPopupBySource === 'function') {
        this.hideDashboardPopupBySource(this.chatMapPopupSource());
      }
      this.clearHoveredMessage();
    },

    showMapDayPopup: function(msg, ev) {
      if (!msg) return;
      this.suppressMapPreview = false;
      if (typeof this.showDashboardPopup !== 'function') return;
      this.showDashboardPopup('chat-map-day:' + this.messageDayKey(msg), this.messageDayLabel(msg), ev, {
        source: this.chatMapPopupSource(),
        side: 'left',
        body: this.isMessageDayCollapsed(msg)
          ? 'Expand this day in the chat map'
          : 'Collapse this day in the chat map',
        meta_origin: 'Chat map'
      });
    },

    hideMapDayPopup: function() {
      if (typeof this.hideDashboardPopupBySource === 'function') {
        this.hideDashboardPopupBySource(this.chatMapPopupSource());
      }
    },

    setHoveredMessage: function(msg, idx) {
      if (this._hoverClearTimer) {
        clearTimeout(this._hoverClearTimer);
        this._hoverClearTimer = 0;
      }
      if (!msg && msg !== 0) {
        this.hoveredMessageDomId = this.selectedMessageDomId || '';
        this.directHoveredMessageDomId = '';
        return;
      }
      var domId = this.messageDomId(msg, idx);
      this.hoveredMessageDomId = domId;
      this.directHoveredMessageDomId = domId;
    },

    clearHoveredMessage: function() {
      if (this._hoverClearTimer) clearTimeout(this._hoverClearTimer);
      var self = this;
      this._hoverClearTimer = setTimeout(function() {
        self._hoverClearTimer = 0;
        self.hoveredMessageDomId = self.selectedMessageDomId || '';
        self.directHoveredMessageDomId = '';
      }, 42);
    },

    clearHoveredMessageHard: function() {
      if (this._hoverClearTimer) {
        clearTimeout(this._hoverClearTimer);
        this._hoverClearTimer = 0;
      }
      if (typeof this.hideDashboardPopupBySource === 'function') {
        this.hideDashboardPopupBySource(this.chatMapPopupSource());
      }
      this.hoveredMessageDomId = '';
      this.directHoveredMessageDomId = '';
      this.selectedMessageDomId = '';
    },

    isHoveredMessage: function(msg, idx) {
      if (!this.hoveredMessageDomId) return false;
      return this.hoveredMessageDomId === this.messageDomId(msg, idx);
    },

    isDirectHoveredMessage: function(msg, idx) {
      if (!this.directHoveredMessageDomId) return false;
      return this.directHoveredMessageDomId === this.messageDomId(msg, idx);
    },

    centerChatMapOnMessage: function(domId, options) {
      if (!domId) return;
      var immediate = !!(options && options.immediate);
      var map = null;
      var maps = document.querySelectorAll('.chat-map-scroll');
      for (var i = 0; i < maps.length; i++) {
        var candidate = maps[i];
        if (candidate && candidate.offsetParent !== null) {
          map = candidate;
          break;
        }
      }
      if (!map) return;
      var host = map.closest('.chat-map') || map;
      var item = host.querySelector('.chat-map-item[data-msg-dom-id="' + domId + '"]');
      if (!item) return;
      var topGuard = 28;
      var bottomGuard = 28;
      var viewport = Math.max(20, map.clientHeight - topGuard - bottomGuard);
      var desired = item.offsetTop + (item.offsetHeight / 2) - (viewport / 2) - topGuard;
      var max = Math.max(0, map.scrollHeight - map.clientHeight);
      var nextTop = Math.max(0, Math.min(max, desired));
      var diff = Math.abs(map.scrollTop - nextTop);
      if (diff < 3) return;
      map.scrollTo({ top: nextTop, behavior: (immediate || this.suppressMapPreview) ? 'auto' : 'smooth' });
    },
  };
}
