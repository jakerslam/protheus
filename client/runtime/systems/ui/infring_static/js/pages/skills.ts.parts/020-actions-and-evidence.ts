    // ClawHub browse by sort (with 60s client-side cache)
    async browseClawHub(sort) {
      this.clawhubSort = sort || 'trending';
      var ckey = 'browse:' + this.clawhubSort;
      var cached = this._browseCache[ckey];
      if (cached && (Date.now() - cached.ts) < 60000) {
        this.clawhubBrowseResults = cached.data.items || [];
        this.clawhubNextCursor = cached.data.next_cursor || null;
        return;
      }
      this.clawhubLoading = true;
      this.clawhubError = '';
      this.clawhubNextCursor = null;
      try {
        var data = await InfringAPI.get('/api/clawhub/browse?sort=' + this.clawhubSort + '&limit=20');
        this.clawhubBrowseResults = data.items || [];
        this.clawhubNextCursor = data.next_cursor || null;
        if (data.error) this.clawhubError = data.error;
        this._browseCache[ckey] = { ts: Date.now(), data: data };
      } catch(e) {
        this.clawhubBrowseResults = [];
        this.clawhubError = e.message || 'Browse failed';
      }
      this.clawhubLoading = false;
    },

    // ClawHub load more results
    async loadMoreClawHub() {
      if (!this.clawhubNextCursor || this.clawhubLoading) return;
      this.clawhubLoading = true;
      try {
        var data = await InfringAPI.get('/api/clawhub/browse?sort=' + this.clawhubSort + '&limit=20&cursor=' + encodeURIComponent(this.clawhubNextCursor));
        this.clawhubBrowseResults = this.clawhubBrowseResults.concat(data.items || []);
        this.clawhubNextCursor = data.next_cursor || null;
      } catch(e) {
        // silently fail on load more
      }
      this.clawhubLoading = false;
    },

    // Show skill detail
    async showSkillDetail(slug) {
      this.detailLoading = true;
      this.skillDetail = null;
      this.installResult = null;
      try {
        var data = await InfringAPI.get('/api/clawhub/skill/' + encodeURIComponent(slug));
        this.skillDetail = data;
      } catch(e) {
        InfringToast.error('Failed to load skill details');
      }
      this.detailLoading = false;
    },

    closeDetail() {
      this.skillDetail = null;
      this.installResult = null;
      this.showSkillCode = false;
      this.skillCode = '';
      this.skillCodeFilename = '';
    },

    async viewSkillCode(slug) {
      if (this.showSkillCode) {
        this.showSkillCode = false;
        return;
      }
      this.skillCodeLoading = true;
      try {
        var data = await InfringAPI.get('/api/clawhub/skill/' + encodeURIComponent(slug) + '/code');
        this.skillCode = data.code || '';
        this.skillCodeFilename = data.filename || 'source';
        this.showSkillCode = true;
      } catch(e) {
        InfringToast.error('Could not load skill source code');
      }
      this.skillCodeLoading = false;
    },

    // Install from ClawHub
    async installFromClawHub(slug) {
      this.installingSlug = slug;
      this.installResult = null;
      try {
        var data = await InfringAPI.post('/api/clawhub/install', { slug: slug });
        this.installResult = data;
        if (data.warnings && data.warnings.length > 0) {
          InfringToast.success('Skill "' + data.name + '" installed with ' + data.warnings.length + ' warning(s)');
        } else {
          InfringToast.success('Skill "' + data.name + '" installed successfully');
        }
        // Update installed state in detail modal if open
        if (this.skillDetail && this.skillDetail.slug === slug) {
          this.skillDetail.installed = true;
        }
        await this.loadSkills();
      } catch(e) {
        var msg = e.message || 'Install failed';
        if (msg.includes('already_installed')) {
          InfringToast.error('Skill is already installed');
        } else if (msg.includes('SecurityBlocked')) {
          InfringToast.error('Skill blocked by security scan');
        } else {
          InfringToast.error('Install failed: ' + msg);
        }
      }
      this.installingSlug = null;
    },

    // Uninstall
    uninstallSkill: function(name) {
      var self = this;
      InfringToast.confirm('Uninstall Skill', 'Uninstall skill "' + name + '"? This cannot be undone.', async function() {
        try {
          await InfringAPI.post('/api/skills/uninstall', { name: name });
          InfringToast.success('Skill "' + name + '" uninstalled');
          await self.loadSkills();
        } catch(e) {
          InfringToast.error('Failed to uninstall skill: ' + e.message);
        }
      });
    },

    // Create prompt-only skill
    async createDemoSkill(skill) {
      try {
        await InfringAPI.post('/api/skills/create', {
          name: skill.name,
          description: skill.description,
          runtime: 'prompt_only',
          prompt_context: skill.prompt_context || skill.description
        });
        InfringToast.success('Skill "' + skill.name + '" created');
        this.tab = 'installed';
        await this.loadSkills();
      } catch(e) {
        InfringToast.error('Failed to create skill: ' + e.message);
      }
    },

    // Load MCP servers
    async loadMcpServers() {
      this.mcpLoading = true;
      try {
        var data = await InfringAPI.get('/api/mcp/servers');
        this.mcpServers = data;
      } catch(e) {
        this.mcpServers = { configured: [], connected: [], total_configured: 0, total_connected: 0 };
      }
      this.mcpLoading = false;
    },

    // Category search on ClawHub
    searchCategory: function(cat) {
      this.clawhubSearch = cat.name;
      this.searchClawHub();
    },

    // Quick start skills (prompt-only, zero deps)
    quickStartSkills: [
      { name: 'code-review-guide', description: 'Adds code review best practices and checklist to agent context.', prompt_context: 'You are an expert code reviewer. When reviewing code:\n1. Check for bugs and logic errors\n2. Evaluate code style and readability\n3. Look for security vulnerabilities\n4. Suggest performance improvements\n5. Verify error handling\n6. Check test coverage' },
      { name: 'writing-style', description: 'Configurable writing style guide for content generation.', prompt_context: 'Follow these writing guidelines:\n- Use clear, concise language\n- Prefer active voice over passive voice\n- Keep paragraphs short (3-4 sentences)\n- Use bullet points for lists\n- Maintain consistent tone throughout' },
      { name: 'api-design', description: 'REST API design patterns and conventions.', prompt_context: 'When designing REST APIs:\n- Use nouns for resources, not verbs\n- Use HTTP methods correctly (GET, POST, PUT, DELETE)\n- Return appropriate status codes\n- Use pagination for list endpoints\n- Version your API\n- Document all endpoints' },
      { name: 'security-checklist', description: 'OWASP-aligned security review checklist.', prompt_context: 'Security review checklist (OWASP aligned):\n- Input validation on all user inputs\n- Output encoding to prevent XSS\n- Parameterized queries to prevent SQL injection\n- Authentication and session management\n- Access control checks\n- CSRF protection\n- Security headers\n- Error handling without information leakage' },
    ],

    // Check if skill is installed by slug
    isSkillInstalled: function(slug) {
      return this.skills.some(function(s) {
        return s.source && s.source.type === 'clawhub' && s.source.slug === slug;
      });
    },

    // Check if skill is installed by name
    isSkillInstalledByName: function(name) {
      return this.skills.some(function(s) { return s.name === name; });
    },
  };
}
