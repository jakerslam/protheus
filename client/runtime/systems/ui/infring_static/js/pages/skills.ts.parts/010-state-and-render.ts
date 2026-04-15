// Infring Skills Page — Infring/ClawHub ecosystem + local skills + MCP servers
'use strict';

function skillsPage() {
  return {
    tab: 'installed',
    skills: [],
    loading: true,
    loadError: '',

    // ClawHub state
    clawhubSearch: '',
    clawhubResults: [],
    clawhubBrowseResults: [],
    clawhubLoading: false,
    clawhubError: '',
    clawhubSort: 'trending',
    clawhubNextCursor: null,
    installingSlug: null,
    installResult: null,
    _searchTimer: null,
    _browseCache: {},    // { key: { ts, data } } client-side 60s cache
    _searchCache: {},

    // Skill detail modal
    skillDetail: null,
    detailLoading: false,
    showSkillCode: false,
    skillCode: '',
    skillCodeFilename: '',
    skillCodeLoading: false,

    // MCP servers
    mcpServers: [],
    mcpLoading: false,

    // Category definitions from the Infring ecosystem
    categories: [
      { id: 'coding', name: 'Coding & IDEs' },
      { id: 'git', name: 'Git & GitHub' },
      { id: 'web', name: 'Web & Frontend' },
      { id: 'devops', name: 'DevOps & Cloud' },
      { id: 'browser', name: 'Browser & Automation' },
      { id: 'search', name: 'Search & Research' },
      { id: 'ai', name: 'AI & LLMs' },
      { id: 'data', name: 'Data & Analytics' },
      { id: 'productivity', name: 'Productivity' },
      { id: 'communication', name: 'Communication' },
      { id: 'media', name: 'Media & Streaming' },
      { id: 'notes', name: 'Notes & PKM' },
      { id: 'security', name: 'Security' },
      { id: 'cli', name: 'CLI Utilities' },
      { id: 'marketing', name: 'Marketing & Sales' },
      { id: 'finance', name: 'Finance' },
      { id: 'smart-home', name: 'Smart Home & IoT' },
      { id: 'docs', name: 'PDF & Documents' },
    ],

    runtimeBadge: function(rt) {
      var r = (rt || '').toLowerCase();
      if (r === 'python' || r === 'py') return { text: 'PY', cls: 'runtime-badge-py' };
      if (r === 'node' || r === 'nodejs' || r === 'js' || r === 'javascript') return { text: 'JS', cls: 'runtime-badge-js' };
      if (r === 'wasm' || r === 'webassembly') return { text: 'WASM', cls: 'runtime-badge-wasm' };
      if (r === 'prompt_only' || r === 'prompt' || r === 'promptonly') return { text: 'PROMPT', cls: 'runtime-badge-prompt' };
      return { text: r.toUpperCase().substring(0, 4), cls: 'runtime-badge-prompt' };
    },

    sourceBadge: function(source) {
      if (!source) return { text: 'Local', cls: 'badge-dim' };
      switch (source.type) {
        case 'clawhub': return { text: 'ClawHub', cls: 'badge-info' };
        case 'infring': return { text: 'Infring', cls: 'badge-info' };
        case 'bundled': return { text: 'Built-in', cls: 'badge-success' };
        default: return { text: 'Local', cls: 'badge-dim' };
      }
    },

    formatDownloads: function(n) {
      if (!n) return '0';
      if (n >= 1000000) return (n / 1000000).toFixed(1) + 'M';
      if (n >= 1000) return (n / 1000).toFixed(1) + 'K';
      return n.toString();
    },

    async loadSkills() {
      this.loading = true;
      this.loadError = '';
      try {
        var data = await InfringAPI.get('/api/skills');
        this.skills = (data.skills || []).map(function(s) {
          return {
            name: s.name,
            description: s.description || '',
            version: s.version || '',
            author: s.author || '',
            runtime: s.runtime || 'unknown',
            tools_count: s.tools_count || 0,
            tags: s.tags || [],
            enabled: s.enabled !== false,
            source: s.source || { type: 'local' },
            has_prompt_context: !!s.has_prompt_context
          };
        });
      } catch(e) {
        this.skills = [];
        this.loadError = e.message || 'Could not load skills.';
      }
      this.loading = false;
    },

    async loadData() {
      await this.loadSkills();
    },

    // Debounced search — fires 350ms after user stops typing
    onSearchInput() {
      if (this._searchTimer) clearTimeout(this._searchTimer);
      var q = this.clawhubSearch.trim();
      if (!q) {
        this.clawhubResults = [];
        this.clawhubError = '';
        return;
      }
      var self = this;
      this._searchTimer = setTimeout(function() { self.searchClawHub(); }, 350);
    },

    // ClawHub search
    async searchClawHub() {
      if (!this.clawhubSearch.trim()) {
        this.clawhubResults = [];
        return;
      }
      this.clawhubLoading = true;
      this.clawhubError = '';
      try {
        var data = await InfringAPI.get('/api/clawhub/search?q=' + encodeURIComponent(this.clawhubSearch.trim()) + '&limit=20');
        this.clawhubResults = data.items || [];
        if (data.error) this.clawhubError = data.error;
      } catch(e) {
        this.clawhubResults = [];
        this.clawhubError = e.message || 'Search failed';
      }
      this.clawhubLoading = false;
    },

    // Clear search and go back to browse
    clearSearch() {
      this.clawhubSearch = '';
      this.clawhubResults = [];
      this.clawhubError = '';
      if (this._searchTimer) clearTimeout(this._searchTimer);
    },
