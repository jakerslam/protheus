    },

    isFirstInSourceRun: function(idx, rows) {
      var list = Array.isArray(rows) ? rows : this.messages;
      if (!Array.isArray(list) || idx < 0 || idx >= list.length) return false;
      var curr = list[idx];
      if (!curr || curr.is_notice) return false;
      var currKey = this.messageSourceKey(curr);
      if (!currKey) return false;
      if (idx === 0) return true;
      var prev = list[idx - 1];
      if (!prev || this.isStackBoundaryNoticeMessage(prev)) return true;
      var prevKey = this.messageSourceKey(prev);
      return prevKey !== currKey;
    },

    isLastInSourceRun: function(idx, rows) {
      var list = Array.isArray(rows) ? rows : this.messages;
      if (!Array.isArray(list) || idx < 0 || idx >= list.length) return false;
      var curr = list[idx];
      if (!curr || curr.is_notice) return false;
      var currKey = this.messageSourceKey(curr);
      if (!currKey) return false;
      if (idx >= list.length - 1) return true;
      var next = list[idx + 1];
      if (!next || this.isStackBoundaryNoticeMessage(next)) return true;
      var nextKey = this.messageSourceKey(next);
      return nextKey !== currKey;
    },

    messagePreview: function(msg) {
      if (!msg) return '';
      if (msg.is_notice && msg.notice_label) {
        return String(msg.notice_label);
      }
      var raw = '';
      if (typeof msg.text === 'string' && msg.text.trim()) {
        raw = msg.text;
      } else if (Array.isArray(msg.tools) && msg.tools.length) {
        raw = 'Tool calls: ' + msg.tools.map(function(tool) {
          return tool && tool.name ? tool.name : 'tool';
        }).join(', ');
      } else {
        raw = '[' + (msg.role || 'message') + ']';
      }
      var compact = raw.replace(/\s+/g, ' ').trim();
      if (compact.length > 140) return compact.slice(0, 137) + '...';
      return compact;
    },

    messageMapPreview: function(msg) {
      if (this.messageMapMarkerType(msg) === 'tool') {
        return this.messageToolPreview(msg);
      }
      return this.messagePreview(msg);
    },

    messageToolPreview: function(msg) {
      if (!msg || !Array.isArray(msg.tools) || !msg.tools.length) {
        return this.messagePreview(msg);
      }
      var self = this;
      var compactToolText = function(value, maxLen) {
        if (value == null) return '';
        var raw = '';
        if (typeof value === 'string') {
          raw = value;
        } else {
          try {
            raw = JSON.stringify(value);
          } catch (e) {
            raw = String(value);
          }
        }
        var compact = raw.replace(/\s+/g, ' ').trim();
        if (!compact) return '';
        if (compact.length > maxLen) return compact.slice(0, maxLen - 3) + '...';
        return compact;
      };
