function infringChatTerminalComposeMethods() {
  return {
    toggleTerminalMode() {
      var self = this;
      if (this.isSystemThreadAgent && this.isSystemThreadAgent(this.currentAgent)) {
        this.terminalMode = true;
        if (typeof this.closeComposerMenus === 'function') this.closeComposerMenus();
        else {
          this.showAttachMenu = false;
          this.showModelSwitcher = false;
          if (typeof this.closeGitTreeMenu === 'function') this.closeGitTreeMenu();
          else this.showGitTreeMenu = false;
        }
        this.showSlashMenu = false;
        this.showModelPicker = false;
        this.terminalCursorFocused = false;
        this.$nextTick(function() {
          if (typeof self.closeComposerMenus === 'function') self.closeComposerMenus();
          var input = document.getElementById('msg-input');
          if (input) input.focus();
          self.refreshChatInputOverlayMetrics();
        });
        return;
      }
      if (typeof this.closeComposerMenus === 'function') this.closeComposerMenus();
      else {
        this.showAttachMenu = false;
        this.showModelSwitcher = false;
        if (typeof this.closeGitTreeMenu === 'function') this.closeGitTreeMenu();
        else this.showGitTreeMenu = false;
      }
      this.showSlashMenu = false;
      this.showModelPicker = false;
      this.terminalMode = !this.terminalMode;
      this.resetInputHistoryNavigation('chat');
      this.resetInputHistoryNavigation('terminal');
      this.terminalCursorFocused = false;
      if (!this.terminalMode) this.terminalSelectionStart = 0;
      if (this.terminalMode && !this.terminalCwd) {
        this.terminalCwd = '/workspace';
      }
      if (this.terminalMode && this.currentAgent) {
        this.connectWs(this.currentAgent.id);
      }
      if (this.terminalMode && Array.isArray(this.attachments) && this.attachments.length) {
        for (var i = 0; i < this.attachments.length; i++) {
          if (this.attachments[i] && this.attachments[i].preview) {
            try { URL.revokeObjectURL(this.attachments[i].preview); } catch(_) {}
          }
        }
        this.attachments = [];
      }
      this.$nextTick(function() {
        if (typeof self.closeComposerMenus === 'function') self.closeComposerMenus();
        var input = document.getElementById('msg-input');
        if (input) {
          input.focus();
          if (self.terminalMode) {
            self.setTerminalCursorFocus(true, { target: input });
            self.updateTerminalCursor({ target: input });
          }
        }
        self.scheduleConversationPersist();
        self.refreshChatInputOverlayMetrics();
      });
    },

    setTerminalCursorFocus(active, event) {
      if (!this.terminalMode) {
        this.terminalCursorFocused = false;
        return;
      }
      this.terminalCursorFocused = !!active;
      if (this.terminalCursorFocused) this.updateTerminalCursor(event);
    },

    updateTerminalCursor(event) {
      if (!this.terminalMode) {
        this.terminalSelectionStart = 0;
        return;
      }
      var text = String(this.inputText || '');
      var active = (typeof document !== 'undefined' && document.activeElement && document.activeElement.id === 'msg-input')
        ? document.activeElement
        : null;
      var el = event && event.target ? event.target : (active || document.getElementById('msg-input'));
      var pos = text.length;
      if (el && Number.isFinite(Number(el.selectionStart))) pos = Number(el.selectionStart);
      if (!Number.isFinite(pos) || pos < 0) pos = text.length;
      if (pos > text.length) pos = text.length;
      this.terminalSelectionStart = Math.floor(pos);
    },

    _terminalPromptLine: function(cwd, command) {
      var path = String(cwd || this.terminalPromptPath || '/workspace');
      var cmd = String(command || '').trim();
      if (!cmd) return path + ' %';
      return path + ' % ' + cmd;
    },

    installChatMapWheelLock() {
      var self = this;
      var maps = document.querySelectorAll('.chat-map-scroll');
      for (var i = 0; i < maps.length; i++) {
        var map = maps[i];
        if (!map || map.__ofWheelLock) continue;
        map.__ofWheelLock = true;
        map.addEventListener('wheel', function(ev) {
          var target = ev.currentTarget;
          if (!target) return;
          if (!target.matches(':hover')) return;
          // Keep wheel behavior scoped to chat map so the page does not scroll beneath it.
          var delta = Number(ev.deltaY || 0);
          if (delta !== 0) {
            target.scrollTop += delta;
          }
          ev.preventDefault();
        }, { passive: false });
      }
      var scrollers = document.querySelectorAll('.messages#messages');
      for (var si = 0; si < scrollers.length; si++) {
        var scroller = scrollers[si];
        if (!scroller || scroller.__ofBottomWheelLock) continue;
        scroller.__ofBottomWheelLock = true;
        scroller.addEventListener('wheel', function(ev) {
          self._lastMessagesWheelAt = Date.now();
          if (Number(ev.deltaY || 0) <= 0) return;
          self._stickToBottom = true;
        }, { passive: true });
      }
    },
  };
}

function chatTerminalPromptPath(vm) {
  return vm.terminalCwd || '/workspace';
}

function chatTerminalPromptPrefix(vm) {
  return chatTerminalPromptPath(vm) + ' % ';
}

function chatTerminalPromptChars(vm) {
  var len = chatTerminalPromptPrefix(vm).length;
  if (!Number.isFinite(len)) return 18;
  if (len < 18) return 18;
  return len;
}

function chatTerminalCursorIndex(vm) {
  var text = String(vm.inputText || '');
  var max = text.length;
  var raw = Number(vm.terminalSelectionStart);
  if (!Number.isFinite(raw)) return max;
  if (raw < 0) return 0;
  if (raw > max) return max;
  return Math.floor(raw);
}

function chatTerminalCursorRow(vm) {
  var text = String(vm.inputText || '');
  if (!text) return 0;
  var upto = text.slice(0, chatTerminalCursorIndex(vm));
  var parts = upto.split('\n');
  return Math.max(0, parts.length - 1);
}

function chatTerminalCursorColumn(vm) {
  var text = String(vm.inputText || '');
  if (!text) return 0;
  var upto = text.slice(0, chatTerminalCursorIndex(vm));
  var parts = upto.split('\n');
  return (parts[parts.length - 1] || '').length;
}

function chatTerminalCursorStyle(vm) {
  return '--terminal-cursor-ch:' + chatTerminalCursorColumn(vm) +
    '; --terminal-cursor-row:' + chatTerminalCursorRow(vm) + ';';
}
