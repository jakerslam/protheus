// Chat slash alias normalization, persistence, and formatting helpers.
'use strict';

function infringChatSlashAliasMethods() {
  return {
    defaultSlashAliases: function() {
      return {
        '/status': '/status',
        '/opt': '/continuity',
        '/q': '/queue',
        '/ctx': '/context',
        '/mods': '/model',
        '/mem': '/compact'
      };
    },

    normalizeSlashCommandName: function(value) {
      var name = String(value || '').trim().toLowerCase();
      if (!name) return '';
      return name.startsWith('/') ? name : ('/' + name);
    },

    findSlashCommandDefinition: function(value) {
      var target = this.normalizeSlashCommandName(value);
      if (!target) return null;
      var rows = Array.isArray(this.slashCommands) ? this.slashCommands : [];
      for (var i = 0; i < rows.length; i += 1) {
        var row = rows[i] && typeof rows[i] === 'object' ? rows[i] : null;
        if (!row) continue;
        if (this.normalizeSlashCommandName(row.cmd) === target) return row;
      }
      return null;
    },

    formatSlashCommandUsage: function(value) {
      var target = this.normalizeSlashCommandName(value);
      if (!target) return '';
      var def = this.findSlashCommandDefinition(target);
      var desc = String(def && def.desc ? def.desc : '').trim();
      return desc ? ('`' + target + '` — ' + desc) : ('`' + target + '`');
    },

    loadSlashAliases: function() {
      var defaults = this.defaultSlashAliases();
      var persisted = {};
      try {
        var raw = localStorage.getItem(this.slashAliasStorageKey || '');
        if (raw) {
          var parsed = JSON.parse(raw);
          if (parsed && typeof parsed === 'object') persisted = parsed;
        }
      } catch(_) {}
      var merged = {};
      Object.keys(defaults).forEach(function(key) {
        var target = String(defaults[key] || '').trim().toLowerCase();
        var alias = String(key || '').trim().toLowerCase();
        if (!alias.startsWith('/') || !target.startsWith('/')) return;
        merged[alias] = target;
      });
      Object.keys(persisted).forEach(function(key) {
        var alias = String(key || '').trim().toLowerCase();
        var target = String(persisted[key] || '').trim().toLowerCase();
        if (!alias.startsWith('/') || !target.startsWith('/')) return;
        merged[alias] = target;
      });
      this.slashAliasMap = merged;
      return merged;
    },

    saveSlashAliases: function() {
      try {
        localStorage.setItem(
          this.slashAliasStorageKey || '',
          JSON.stringify(this.slashAliasMap || {})
        );
      } catch(_) {}
    },

    resolveSlashAlias: function(inputCmd, cmdArgs) {
      var cmd = this.normalizeSlashCommandName(inputCmd);
      var args = String(cmdArgs || '').trim();
      var aliases = this.slashAliasMap || {};
      var visited = {};
      var expandedCmd = cmd;
      var expandedArgs = args;
      var rendered = cmd + (args ? (' ' + args) : '');
      for (var depth = 0; depth < 5; depth += 1) {
        var target = String(aliases[expandedCmd] || '').trim();
        if (!target) break;
        if (visited[expandedCmd]) break;
        visited[expandedCmd] = true;
        rendered = target + (expandedArgs ? (' ' + expandedArgs) : '');
        var targetParts = target.split(/\s+/).filter(Boolean);
        if (!targetParts.length) break;
        expandedCmd = this.normalizeSlashCommandName(targetParts[0]);
        var trailing = targetParts.slice(1).join(' ').trim();
        if (trailing) {
          expandedArgs = trailing + (expandedArgs ? (' ' + expandedArgs) : '');
        }
      }
      return { cmd: expandedCmd, args: expandedArgs.trim(), expanded: rendered };
    },

    formatSlashAliasRows: function() {
      var self = this;
      var aliases = this.slashAliasMap || {};
      var rows = Object.keys(aliases)
        .sort()
        .map(function(alias) {
          var target = String(aliases[alias] || '').trim();
          var targetCommand = self.normalizeSlashCommandName(target.split(/\s+/)[0] || '');
          var usage = self.formatSlashCommandUsage(targetCommand);
          return '- `' + alias + '` → `' + target + '`' + (usage ? ('\n  ↳ ' + usage) : '');
        });
      return rows.join('\n');
    },

    executeSlashAliases: function() {
      this.loadSlashAliases();
      console.log('[slash aliases]', this.slashAliasMap || {});
      InfringToast.info('Slash aliases logged to DevTools console.');
    },

    executeSlashAliasCommand: function(cmdArgs) {
      var aliasTokens = String(cmdArgs || '').trim().split(/\s+/).filter(Boolean);
      if (aliasTokens.length < 2) {
        InfringToast.info('Usage: /alias /shortcut /target [extra args]');
        return;
      }
      var aliasKey = String(aliasTokens[0] || '').trim().toLowerCase();
      var aliasTarget = String(aliasTokens.slice(1).join(' ') || '').trim().toLowerCase();
      if (!aliasKey.startsWith('/') || !aliasTarget.startsWith('/')) {
        InfringToast.info('Alias and target must both start with /.');
        return;
      }
      this.loadSlashAliases();
      this.slashAliasMap[aliasKey] = aliasTarget;
      this.saveSlashAliases();
      InfringToast.success('Saved alias ' + aliasKey + ' -> ' + aliasTarget);
    },

    // Backward-compat shim for legacy callers during naming migration.
    runSlashAliases: function() {
      this.executeSlashAliases();
    },

    // Backward-compat shim for legacy callers during naming migration.
    runSlashAliasCommand: function(cmdArgs) {
      this.executeSlashAliasCommand(cmdArgs);
    },
  };
}

function chatFilteredSlashCommands(vm) {
  var base = Array.isArray(vm.slashCommands) ? vm.slashCommands.slice() : [];
  var aliases = vm.slashAliasMap || {};
  Object.keys(aliases).forEach(function(alias) {
    if (!base.some(function(c) { return c && c.cmd === alias; })) {
      base.push({
        cmd: alias,
        desc: 'Alias → ' + String(aliases[alias] || ''),
        source: 'alias'
      });
    }
  });
  if (!vm.slashFilter) return base;
  var f = vm.slashFilter;
  return base.filter(function(c) {
    return c.cmd.toLowerCase().indexOf(f) !== -1 || c.desc.toLowerCase().indexOf(f) !== -1;
  });
}
