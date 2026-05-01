// Chat attachment picker and local file attachment helpers.
'use strict';

function infringChatAttachmentMethods() {
  return {
    currentInputToggleMode() {
      if (this.attachPickerSessionActive) return 'attach';
      return this.recording ? 'voice' : 'send';
    },

    beginAttachPickerSession() {
      if (typeof this.isSystemThreadActive === 'function' && this.isSystemThreadActive()) return;
      if (this.terminalMode) this.toggleTerminalMode();
      this.attachPickerRestoreMode = this.recording ? 'voice' : 'send';
      this.attachPickerSessionActive = true;
      this.showAttachMenu = false;
      this.armAttachPickerFocusTracking();
      var self = this;
      this.$nextTick(function() {
        var input = self.$refs && self.$refs.fileInput ? self.$refs.fileInput : null;
        if (!input || typeof input.click !== 'function') {
          self.endAttachPickerSession();
          return;
        }
        try {
          input.click();
        } catch (_) {
          self.endAttachPickerSession();
        }
      });
    },

    armAttachPickerFocusTracking() {
      var self = this;
      if (this._attachPickerFocusTimer) {
        clearTimeout(this._attachPickerFocusTimer);
        this._attachPickerFocusTimer = 0;
      }
      if (this._attachPickerFocusListener) {
        window.removeEventListener('focus', this._attachPickerFocusListener);
        this._attachPickerFocusListener = null;
      }
      this._attachPickerFocusListener = function() {
        if (self._attachPickerFocusTimer) clearTimeout(self._attachPickerFocusTimer);
        self._attachPickerFocusTimer = setTimeout(function() {
          self._attachPickerFocusTimer = 0;
          if (self.attachPickerSessionActive) self.endAttachPickerSession();
        }, 180);
      };
      window.addEventListener('focus', this._attachPickerFocusListener, { once: true });
    },

    endAttachPickerSession() {
      this.attachPickerSessionActive = false;
      this.showAttachMenu = false;
      if (this._attachPickerFocusTimer) {
        clearTimeout(this._attachPickerFocusTimer);
        this._attachPickerFocusTimer = 0;
      }
      if (this._attachPickerFocusListener) {
        window.removeEventListener('focus', this._attachPickerFocusListener);
        this._attachPickerFocusListener = null;
      }
    },

    handleAttachInputChange(event) {
      var input = event && event.target ? event.target : null;
      var files = input && input.files ? input.files : null;
      if (files && files.length) this.addFiles(files);
      if (input) input.value = '';
      this.endAttachPickerSession();
    },

    addFiles(files) {
      var self = this;
      var acceptedMimeTypes = [
        'image/png',
        'image/jpeg',
        'image/gif',
        'image/webp',
        'text/plain',
        'application/pdf',
        'text/markdown',
        'application/json',
        'text/csv'
      ];
      var acceptedExtensions = ['.txt', '.pdf', '.md', '.json', '.csv'];
      var existingKeys = {};
      var rows = Array.isArray(this.attachments) ? this.attachments : [];
      var attachmentKeyFor = function(file) {
        if (!file) return '';
        return [
          String(file.name || '').trim().toLowerCase(),
          Number(file.size || 0),
          Number(file.lastModified || 0)
        ].join('|');
      };
      var isSupportedMimeType = function(mimeType) {
        if (typeof mimeType !== 'string') return false;
        if (mimeType.indexOf('image/') === 0) return true;
        return acceptedMimeTypes.indexOf(mimeType) !== -1;
      };
      var isSupportedFile = function(file) {
        if (!file) return false;
        if (isSupportedMimeType(file.type)) return true;
        var ext = file.name.lastIndexOf('.') !== -1
          ? file.name.substring(file.name.lastIndexOf('.')).toLowerCase()
          : '';
        return acceptedExtensions.indexOf(ext) !== -1;
      };
      for (var existingIdx = 0; existingIdx < rows.length; existingIdx++) {
        var existing = rows[existingIdx];
        if (!existing || !existing.file) continue;
        var existingKey = attachmentKeyFor(existing.file);
        if (existingKey) existingKeys[existingKey] = true;
      }
      for (var i = 0; i < files.length; i++) {
        var file = files[i];
        var dedupeKey = attachmentKeyFor(file);
        if (dedupeKey && existingKeys[dedupeKey]) {
          InfringToast.info('Already attached: ' + file.name);
          continue;
        }
        if (file.size > 10 * 1024 * 1024) {
          InfringToast.warn('File "' + file.name + '" exceeds 10MB limit');
          continue;
        }
        var typeOk = isSupportedFile(file);
        if (!typeOk) {
          InfringToast.warn('File type not supported: ' + file.name);
          continue;
        }
        var preview = null;
        if (isSupportedMimeType(file.type) && file.type.indexOf('image/') === 0) {
          preview = URL.createObjectURL(file);
        }
        self.attachments.push({ file: file, preview: preview, uploading: false });
        if (dedupeKey) existingKeys[dedupeKey] = true;
      }
    },
    removeAttachment(idx) {
      var att = this.attachments[idx];
      if (att && att.preview) URL.revokeObjectURL(att.preview);
      this.attachments.splice(idx, 1);
    },
    handleDrop(e) {
      e.preventDefault();
      if (e.dataTransfer && e.dataTransfer.files && e.dataTransfer.files.length) {
        this.addFiles(e.dataTransfer.files);
      }
    },
  };
}
