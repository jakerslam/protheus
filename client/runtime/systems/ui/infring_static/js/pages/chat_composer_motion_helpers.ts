// Chat composer reduced-motion and send-morph animation helpers.
'use strict';

function infringChatComposerMotionMethods() {
  return {
    prefersReducedMotion: function() {
      if (typeof window === 'undefined' || typeof window.matchMedia !== 'function') return false;
      try {
        return !!window.matchMedia('(prefers-reduced-motion: reduce)').matches;
      } catch (_) {
        return false;
      }
    },

    captureComposerSendMorph: function(textInput) {
      if (this.prefersReducedMotion() || this.terminalMode || this.showFreshArchetypeTiles) return null;
      if (typeof document === 'undefined') return null;
      var shell = document.querySelector('.input-row .composer-shell');
      var input = document.getElementById('msg-input');
      if (!shell || !input) return null;
      var text = String(textInput == null ? '' : textInput).trim();
      if (!text) return null;
      var rect = input.getBoundingClientRect();
      if (!(rect.width > 80 && rect.height > 24)) return null;
      var ghost = document.createElement('div');
      ghost.className = 'composer-send-morph-ghost';
      ghost.textContent = text.length > 260 ? (text.slice(0, 257) + '...') : text;
      ghost.style.left = rect.left + 'px';
      ghost.style.top = rect.top + 'px';
      ghost.style.width = rect.width + 'px';
      ghost.style.minHeight = rect.height + 'px';
      document.body.appendChild(ghost);
      shell.classList.add('composer-shell-send-morph');
      return { shell: shell, ghost: ghost };
    },

    clearComposerSendMorph: function(snapshot) {
      if (!snapshot || typeof snapshot !== 'object') return;
      if (snapshot.shell && snapshot.shell.classList) snapshot.shell.classList.remove('composer-shell-send-morph');
      if (snapshot.ghost && snapshot.ghost.parentNode) snapshot.ghost.parentNode.removeChild(snapshot.ghost);
    },

    playComposerSendMorphToMessage: function(snapshot, messageId) {
      if (!snapshot || !snapshot.ghost) return;
      if (this.prefersReducedMotion()) {
        snapshot.ghost.style.opacity = '0.56';
        setTimeout(this.clearComposerSendMorph.bind(this, snapshot), 240);
        return;
      }
      var row = document.getElementById('chat-msg-' + String(messageId || '').trim());
      var bubble = row ? row.querySelector('.message-bubble') : null;
      if (!bubble) {
        this.clearComposerSendMorph(snapshot);
        return;
      }
      var rect = bubble.getBoundingClientRect();
      if (!(rect.width > 24 && rect.height > 20)) {
        this.clearComposerSendMorph(snapshot);
        return;
      }
      var ghost = snapshot.ghost;
      var self = this;
      ghost.classList.add('in-flight');
      var finish = function() { self.clearComposerSendMorph(snapshot); };
      ghost.addEventListener('transitionend', finish, { once: true });
      requestAnimationFrame(function() {
        ghost.style.left = rect.left + 'px';
        ghost.style.top = rect.top + 'px';
        ghost.style.width = rect.width + 'px';
        ghost.style.minHeight = rect.height + 'px';
        ghost.style.opacity = '0.2';
      });
      setTimeout(finish, 760);
    },
  };
}
