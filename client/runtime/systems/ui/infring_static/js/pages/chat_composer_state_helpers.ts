// Chat composer placeholder and fresh-init input lock helpers.
'use strict';

function infringChatComposerStateMethods() {
  return {
    composerPlaceholder: function(includeCommandHint) {
      if (this.terminalMode) return this.terminalPromptPrefix;
      if (this.recording) return 'Recording... release to send';
      if (this.showFreshArchetypeTiles && this.isFreshInitComposerUnlocked()) {
        return this.freshInitOtherInputPlaceholder();
      }
      var base = this.currentAgent
        ? ('Message ' + (this.currentAgent.name || this.currentAgent.id || 'agent'))
        : 'Message agent';
      return includeCommandHint ? (base + '... (/ for commands)') : (base + '...');
    },

    isFreshInitComposerUnlocked: function() {
      return !!(
        this.showFreshArchetypeTiles &&
        !this.freshInitLaunching &&
        this.freshInitAwaitingOtherPrompt
      );
    },

    isFreshInitComposerLocked: function() {
      return !!(
        this.showFreshArchetypeTiles &&
        !this.freshInitLaunching &&
        !this.freshInitAwaitingOtherPrompt
      );
    },
  };
}
