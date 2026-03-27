      if (thought) {
        var derived = this.deriveUserFacingFromThought(thought);
        if (derived) return derived;
        return 'I need one more clarification before I can finalize a reliable answer. Tell me the exact expected outcome.';
      }
      return 'I could not produce a final answer this turn. Please retry or clarify what you want next.';
    },

    deriveUserFacingFromThought: function(thoughtText) {
      var thought = String(thoughtText || '').replace(/\s+/g, ' ').trim();
      if (!thought) return '';
      var skip = /^(alright|okay|ok|hmm|let me|i need to|i should|i will|first[, ]|to answer this|it seems|we need to)\b/i;
      var sentences = thought
        .split(/(?<=[.!?])\s+/)
        .map(function(part) { return String(part || '').trim(); })
        .filter(function(part) { return !!part; });
      var keep = [];
      for (var i = 0; i < sentences.length; i++) {
        var sentence = sentences[i];
        var lower = sentence.toLowerCase();
        if (skip.test(sentence) && lower.indexOf('queue depth') < 0 && lower.indexOf('scale') < 0 && lower.indexOf('recommend') < 0 && lower.indexOf('command') < 0) {
          continue;
        }
        if (lower.indexOf('user') >= 0 && lower.indexOf('request') >= 0) continue;
        if (sentence.length < 20) continue;
        keep.push(sentence);
      }
      if (!keep.length) {
        var queueLine = thought.match(/queue depth[^.?!]*[.?!]?/i);
        if (queueLine && queueLine[0]) keep.push(String(queueLine[0]).trim());
        var scaleLine = thought.match(/scale[^.?!]*instances?[^.?!]*[.?!]?/i);
        if (scaleLine && scaleLine[0]) keep.push(String(scaleLine[0]).trim());
      }
      if (!keep.length) return '';
      var message = keep.slice(0, 2).join(' ').replace(/\s+/g, ' ').trim();
      if (!message) return '';
      if (!/[.?!]$/.test(message)) message += '.';
      if (message.length > 300) message = message.slice(0, 297) + '...';
      return message;
    },

    extractArtifactDirectives: function(text) {
      var value = String(text || '');
      if (!value) return [];
      var rx = /\[\[\s*(file|folder)\s*:\s*([^\]]+?)\s*\]\]/gi;
      var out = [];
      var match;
      while ((match = rx.exec(value)) && out.length < 4) {
        var kind = String(match[1] || '').toLowerCase();
        var targetPath = String(match[2] || '').trim();
        if (!targetPath) continue;
        out.push({ kind: kind, path: targetPath });
      }
      return out;
    },

    stripArtifactDirectivesFromText: function(text) {
      var value = String(text || '');
      if (!value) return '';
      return value.replace(/\[\[\s*(file|folder)\s*:\s*[^\]]+?\s*\]\]/gi, '').replace(/\n{3,}/g, '\n\n').trim();
    },

    resolveArtifactDirectives: async function(directives) {
      if (!this.currentAgent || !this.currentAgent.id) return;
      var rows = Array.isArray(directives) ? directives : [];
      if (!rows.length) return;
      for (var i = 0; i < rows.length; i++) {
        var row = rows[i] || {};
        var targetPath = String(row.path || '').trim();
        if (!targetPath) continue;
        try {
          if (row.kind === 'file') {
            var fileRes = await InfringAPI.post('/api/agents/' + this.currentAgent.id + '/file/read', {
              path: targetPath
            });
            var fileMeta = fileRes && fileRes.file ? fileRes.file : null;
            if (fileMeta && fileMeta.ok) {
              this.messages.push({
                id: ++msgId,
                role: 'agent',
                text: '',
                meta: (Number(fileMeta.bytes || 0) > 0 ? (Number(fileMeta.bytes || 0) + ' bytes') : ''),
                tools: [],
                ts: Date.now(),
                file_output: {
                  path: String(fileMeta.path || targetPath),
                  content: String(fileMeta.content || ''),
                  truncated: !!fileMeta.truncated,
                  bytes: Number(fileMeta.bytes || 0)
                }
              });
            }
          } else if (row.kind === 'folder') {
            var folderRes = await InfringAPI.post('/api/agents/' + this.currentAgent.id + '/folder/export', {
              path: targetPath
            });
            var folderMeta = folderRes && folderRes.folder ? folderRes.folder : null;
            var archiveMeta = folderRes && folderRes.archive ? folderRes.archive : null;
            if (folderMeta && folderMeta.ok) {
              this.messages.push({
                id: ++msgId,
                role: 'agent',
                text: '',
                meta: Number(folderMeta.entries || 0) + ' entries',
                tools: [],
                ts: Date.now(),
                folder_output: {
                  path: String(folderMeta.path || targetPath),
                  tree: String(folderMeta.tree || ''),
                  entries: Number(folderMeta.entries || 0),
                  truncated: !!folderMeta.truncated,
                  download_url: archiveMeta && archiveMeta.download_url ? String(archiveMeta.download_url) : '',
                  archive_name: archiveMeta && archiveMeta.file_name ? String(archiveMeta.file_name) : '',
                  archive_bytes: Number(archiveMeta && archiveMeta.bytes ? archiveMeta.bytes : 0)
                }
              });
            }
          }
        } catch (_) {}
      }
      this.scrollToBottom();
      this.scheduleConversationPersist();
    },

    // Remove disclosure/speaker prefixes injected by model/backend responses.
    // Examples:
    //   "[openai/gpt-5] hello" -> "hello"
    //   "Agent: hello" -> "hello"
    //   "**Assistant:** hello" -> "hello"
    stripModelPrefix: function(text) {
      if (!text) return text;
      var out = String(text);
      for (var i = 0; i < 6; i++) {
        var prior = out;
        out = out.replace(/^\s*\[[^\]\n]{2,96}\]\s*/, '');
        // Strip leaked transcript wrappers like "User: ... Agent: <answer>".
        var transcriptLead = out.match(
          /^\s*(?:[-*]\s*)?(?:\*\*)?(?:user|human|you)(?:\*\*)?\s*:\s*[\s\S]{0,1200}?(?:\*\*)?(?:agent|assistant|model|ai|jarvis)(?:\*\*)?\s*:\s*/i
        );
        if (transcriptLead && transcriptLead[0]) {
          out = out.slice(transcriptLead[0].length);
          continue;
        }
        out = out.replace(
          /^\s*(?:[-*]\s*)?(?:\*\*)?(?:agent|assistant|system|model|ai|jarvis|user|human|you)(?:\*\*)?\s*:\s*/i,
          ''
        );
        if (out === prior) break;
      }
      return out;
    },

    formatToolJson: function(text) {
      if (!text) return '';
      try { return JSON.stringify(JSON.parse(text), null, 2); }
      catch(e) { return text; }
    },

    // Voice: start recording
    startRecording: async function() {
      if (this.recording) return;
      try {
        var stream = await navigator.mediaDevices.getUserMedia({ audio: true });
        var mimeType = MediaRecorder.isTypeSupported('audio/webm;codecs=opus') ? 'audio/webm;codecs=opus' :
                       MediaRecorder.isTypeSupported('audio/webm') ? 'audio/webm' : 'audio/ogg';
        this._audioChunks = [];
        this._mediaRecorder = new MediaRecorder(stream, { mimeType: mimeType });
        var self = this;
        this._mediaRecorder.ondataavailable = function(e) {
          if (e.data.size > 0) self._audioChunks.push(e.data);
        };
        this._mediaRecorder.onstop = function() {
          stream.getTracks().forEach(function(t) { t.stop(); });
          self._handleRecordingComplete();
        };
        this._mediaRecorder.start(250);
        this.recording = true;
        this.recordingTime = 0;
        this._recordingTimer = setInterval(function() { self.recordingTime++; }, 1000);
      } catch(e) {
        if (typeof InfringToast !== 'undefined') InfringToast.error('Microphone access denied');
      }
    },

    // Voice: stop recording
    stopRecording: function() {
      if (!this.recording || !this._mediaRecorder) return;
      this._mediaRecorder.stop();
      this.recording = false;
      if (this._recordingTimer) { clearInterval(this._recordingTimer); this._recordingTimer = null; }
    },

    // Voice: handle completed recording — upload and transcribe
    _handleRecordingComplete: async function() {
      var voiceAgent = this.ensureValidCurrentAgent({ clear_when_missing: true });
      if (!this._audioChunks.length || !voiceAgent || !voiceAgent.id) return;
      var blob = new Blob(this._audioChunks, { type: this._audioChunks[0].type || 'audio/webm' });
      this._audioChunks = [];
      if (blob.size < 100) return; // too small

      // Show a temporary "Transcribing..." message
      this.messages.push({ id: ++msgId, role: 'system', text: 'Transcribing audio...', thinking: true, ts: Date.now(), tools: [], system_origin: 'voice:transcribe' });
      this.scrollToBottom();

      try {
        // Upload audio file
        var ext = blob.type.includes('webm') ? 'webm' : blob.type.includes('ogg') ? 'ogg' : 'mp3';
        var file = new File([blob], 'voice_' + Date.now() + '.' + ext, { type: blob.type });
        var upload = await InfringAPI.upload(voiceAgent.id, file);

        // Remove the "Transcribing..." message
        this.messages = this.messages.filter(function(m) { return !m.thinking || m.role !== 'system'; });

        // Use server-side transcription if available, otherwise fall back to placeholder
        var text = (upload.transcription && upload.transcription.trim())
          ? upload.transcription.trim()
          : '[Voice message - audio: ' + upload.filename + ']';
        this._sendPayload(text, [upload], [], { agent_id: voiceAgent.id });
      } catch(e) {
        this.messages = this.messages.filter(function(m) { return !m.thinking || m.role !== 'system'; });
        if (typeof InfringToast !== 'undefined') InfringToast.error('Failed to upload audio: ' + (e.message || 'unknown error'));
      }
    },

    // Voice: format recording time as MM:SS
    formatRecordingTime: function() {
      var m = Math.floor(this.recordingTime / 60);
      var s = this.recordingTime % 60;
      return (m < 10 ? '0' : '') + m + ':' + (s < 10 ? '0' : '') + s;
    },

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

    // Search: filter messages by query
    get filteredMessages() {
      if (!this.searchQuery.trim()) return this.messages;
      var q = this.searchQuery.toLowerCase();
      return this.messages.filter(function(m) {
        return (m.text && m.text.toLowerCase().indexOf(q) !== -1) ||
               (m.tools && m.tools.some(function(t) { return t.name.toLowerCase().indexOf(q) !== -1; }));
      });
    },

    // Search: highlight matched text in a string
    highlightSearch: function(html) {
      if (!this.searchQuery.trim() || !html) return html;
      var q = this.searchQuery.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
      var regex = new RegExp('(' + q + ')', 'gi');
      return html.replace(regex, '<mark style="background:var(--warning);color:var(--bg);border-radius:2px;padding:0 2px">$1</mark>');
    },

    renderMarkdown: renderMarkdown,
    escapeHtml: escapeHtml
  };
}

