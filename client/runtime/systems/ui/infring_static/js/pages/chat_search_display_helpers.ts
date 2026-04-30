// Chat search toggle and display-window method helpers.
'use strict';

function infringChatSearchDisplayMethods() {
  return {
    // Search: toggle open/close
    toggleSearch: function() {
      this.searchOpen = !this.searchOpen;
      if (this.searchOpen) {
        var self = this;
        this.$nextTick(function() {
          var el = document.getElementById('chat-search-input');
          if (el) el.focus();
        });
      } else {
        this.searchQuery = '';
      }
    },

    messageDisplayScopeKey: function() {
      return chatMessageDisplayScopeKey(this);
    },

    // Backward-compat shim for legacy callers during naming migration.
    _messageDisplayScopeKey: function() {
      return this.messageDisplayScopeKey();
    },

    ensureMessageDisplayWindow: function(totalCount) {
      chatEnsureMessageDisplayWindow(this, totalCount);
    },

    expandDisplayedMessages: function() {
      chatExpandDisplayedMessages(this);
    },
  };
}
