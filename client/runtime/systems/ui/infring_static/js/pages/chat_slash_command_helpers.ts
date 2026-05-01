function infringChatSlashCommandMethods() {
  return {
    // Fetch dynamic slash commands from server.
    fetchCommands: function() {
      var self = this;
      InfringAPI.get('/api/commands').then(function(data) {
        if (data.commands && data.commands.length) {
          var existing = {};
          self.slashCommands.forEach(function(c) { existing[c.cmd] = true; });
          data.commands.forEach(function(c) {
            if (!existing[c.cmd]) {
              self.slashCommands.push({ cmd: c.cmd, desc: c.desc || '', source: c.source || 'server' });
              existing[c.cmd] = true;
            }
          });
        }
      }).catch(function() { /* silent - use bundled list */ });
    },

    async executeSlashCommand(cmd, cmdArgs) {
      this.showSlashMenu = false;

// Layer ownership: client/runtime/systems/ui (dashboard static UX surface only; no runtime authority).
      this.inputText = '';
      var self = this;
      cmdArgs = cmdArgs || '';
      switch (cmd) {
        case '/help':
          self.inputText = '/';
          self.showSlashMenu = true;
          InfringToast.info('Slash commands are available in the command palette.');
          break;
        case '/agents':
          location.hash = 'agents';
          break;
        case '/new':
          if (self.currentAgent) {
            self.inputText = 'Use the session_control route to reset this agent session and return a structured receipt.';
            await self.sendMessage();
          }
          break;
        case '/compact':
          if (self.currentAgent) {
            self.inputText = 'Use the session_compaction route to compact this agent session and return a structured receipt.';
            await self.sendMessage();
          }
          break;
        case '/stop':
          self.stopAgent();
          break;
        case '/usage':
          if (self.currentAgent) {
            self.inputText = 'Use the runtime_usage tool route to report this session usage with a structured receipt.';
            await self.sendMessage();
          }
          break;
        case '/think':
          if (cmdArgs === 'on') {
            self.thinkingMode = 'on';
          } else if (cmdArgs === 'off') {
            self.thinkingMode = 'off';
          } else if (cmdArgs === 'stream') {
            self.thinkingMode = 'stream';
          } else {
            // Cycle: off -> on -> stream -> off
            if (self.thinkingMode === 'off') self.thinkingMode = 'on';
            else if (self.thinkingMode === 'on') self.thinkingMode = 'stream';
            else self.thinkingMode = 'off';
          }
          var modeLabel = self.thinkingMode === 'stream' ? 'enabled (streaming reasoning)' : (self.thinkingMode === 'on' ? 'enabled' : 'disabled');
          InfringToast.info('Extended thinking ' + modeLabel + '.');
          break;

        case '/context':
          // Visual-only update for context ring; no chat message noise.
          self.recomputeContextEstimate();
          self.setContextWindowFromCurrentAgent();
          break;
        case '/verbose':
          self.inputText = 'Use the runtime_control tool route to request verbosity settings' + (cmdArgs ? ': ' + cmdArgs : '.') ;
          await self.sendMessage();
          break;
        case '/queue':
          self.inputText = 'Use the queue_status tool route to summarize current queued work with backend queue receipts.';
          await self.sendMessage();
          break;
        case '/status':
          self.inputText = 'Use the runtime_status tool route to report current system status with a structured receipt.';
          await self.sendMessage();
          break;
        case '/alerts':
          await self.runSlashAlerts();
          break;
        case '/next':
          await self.runSlashNextActions();
          break;
        case '/memory':
          await self.runSlashMemoryHygiene();
          break;
        case '/continuity':
          await self.runSlashContinuity();
          break;
        case '/aliases':
          self.executeSlashAliases();
          break;
        case '/alias':
          self.executeSlashAliasCommand(cmdArgs);
          break;
        case '/opt':
          await self.runSlashOptimizeWorkers();
          break;
        case '/model':
          if (self.currentAgent) {
            if (cmdArgs) {
              self.inputText = 'Use the model_provider_coordination route to switch this agent model to exactly: ' + String(cmdArgs || '').trim();
              await self.sendMessage();
            } else {
              self.inputText = 'Use the model_provider_coordination route to report this agent current selected and runtime model with a structured receipt.';
              await self.sendMessage();
            }
          } else {
            InfringToast.info('Select an agent before requesting model coordination.');
          }
          break;
        case '/apikey':
          await self.runSlashApiKeyDiscovery(cmdArgs);
          break;
        case '/file':
          if (!self.currentAgent) {
            InfringToast.info('Select an agent before requesting a workspace file.');
            break;
          }
          var fileTargetPath = String(cmdArgs || '').trim();
          if (!fileTargetPath) {
            InfringToast.info('Usage: /file <path>');
            break;
          }
          self.inputText = 'Use the workspace_read tool to read this workspace file path exactly: ' + fileTargetPath;
          await self.sendMessage();
          break;
        case '/folder':
          if (!self.currentAgent) {
            InfringToast.info('Select an agent before requesting a workspace folder.');
            break;
          }
          var folderTargetPath = String(cmdArgs || '').trim();
          if (!folderTargetPath) {
            InfringToast.info('Usage: /folder <path>');
            break;
          }
          self.inputText = 'Use the workspace_export tool to export this workspace folder path exactly: ' + folderTargetPath;
          await self.sendMessage();
          break;
        case '/clear':
          self.messages = [];
          break;
        case '/exit':
          InfringAPI.wsDisconnect();
          self._wsAgent = null;
          self.currentAgent = null;
          self.setStoreActiveAgentId(null);
          self.messages = [];
          window.dispatchEvent(new Event('close-chat'));
          break;
        case '/budget':
          self.inputText = 'Use the runtime_budget tool route to report current budget status with a structured receipt.';
          await self.sendMessage();
          break;
        case '/peers':
          self.inputText = 'Use the network_status tool route to report current peer/network status with a structured receipt.';
          await self.sendMessage();
          break;
        case '/a2a':
          self.inputText = 'Use the a2a_discovery tool route to report discovered A2A agents with a structured receipt.';
          await self.sendMessage();
          break;
        case '/memprobe':
          // Heap diagnostic: snapshots the chat page's memory footprint and
          // emits a structured report to chat + console. Run twice with an
          // idle gap (e.g., /memprobe, wait 30s, /memprobe again) to compute
          // a leak rate.
          self.runSlashMemprobe(cmdArgs);
          break;
      }
      this.scheduleConversationPersist();
    },

    isShellOwnedSlashCommand: function(cmd) {
      var normalized = String(cmd || '').trim().toLowerCase();
      if (!normalized) return false;
      switch (normalized) {
        case '/help':
        case '/agents':
        case '/new':
        case '/compact':
        case '/model':
        case '/apikey':
        case '/stop':
        case '/usage':
        case '/think':
        case '/context':
        case '/verbose':
        case '/queue':
        case '/status':
        case '/alerts':
        case '/next':
        case '/continuity':
        case '/aliases':
        case '/alias':
        case '/opt':
        case '/clear':
        case '/exit':
        case '/budget':
        case '/peers':
        case '/a2a':
          return true;
        default:
          return false;
      }
    },

  };
}
