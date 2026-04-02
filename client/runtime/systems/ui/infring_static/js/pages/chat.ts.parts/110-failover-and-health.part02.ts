      this.inputText = '';
      var self = this;
      cmdArgs = cmdArgs || '';
      switch (cmd) {
        case '/help':
          self.messages.push({ id: ++msgId, role: 'system', text: self.slashCommands.map(function(c) { return '`' + c.cmd + '` — ' + c.desc; }).join('\n'), meta: '', tools: [], system_origin: 'slash:help' });
          self.scrollToBottom();
          break;
        case '/agents':
          location.hash = 'agents';
          break;
        case '/new':
          if (self.currentAgent) {
            InfringAPI.post('/api/agents/' + self.currentAgent.id + '/session/reset', {}).then(function() {
              self.messages = [];
              InfringToast.success('Session reset');
            }).catch(function(e) { InfringToast.error('Reset failed: ' + e.message); });
          }
          break;
        case '/compact':
          if (self.currentAgent) {
            self.messages.push({ id: ++msgId, role: 'system', text: 'Compacting session...', meta: '', tools: [], system_origin: 'slash:compact' });
            InfringAPI.post('/api/agents/' + self.currentAgent.id + '/session/compact', {}).then(function(res) {
              self.messages.push({ id: ++msgId, role: 'system', text: res.message || 'Compaction complete', meta: '', tools: [], system_origin: 'slash:compact' });
              self.scrollToBottom();
            }).catch(function(e) { InfringToast.error('Compaction failed: ' + e.message); });
          }
          break;
        case '/stop':
          self.stopAgent();
          break;
        case '/usage':
          if (self.currentAgent) {
            var approxTokens = self.messages.reduce(function(sum, m) { return sum + Math.round((m.text || '').length / 4); }, 0);
            self.messages.push({ id: ++msgId, role: 'system', text: '**Session Usage**\n- Messages: ' + self.messages.length + '\n- Approx tokens: ~' + approxTokens, meta: '', tools: [], system_origin: 'slash:usage' });
            self.scrollToBottom();
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
          self.messages.push({ id: ++msgId, role: 'system', text: 'Extended thinking **' + modeLabel + '**. ' +
            (self.thinkingMode === 'stream' ? 'Reasoning tokens will appear in a collapsible panel.' :
             self.thinkingMode === 'on' ? 'The agent will show its reasoning when supported by the model.' :
             'Normal response mode.'), meta: '', tools: [], system_origin: 'slash:think' });
          self.scrollToBottom();
          break;
