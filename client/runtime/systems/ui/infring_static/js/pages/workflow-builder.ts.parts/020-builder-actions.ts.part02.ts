    async saveWorkflow() {
      var steps = [];
      var stepNodes = this.nodes.filter(function(n) {
        return n.type !== 'start' && n.type !== 'end';
      });
      var nodeById = {};
      var stepNameByNodeId = {};
      for (var m = 0; m < stepNodes.length; m++) {
        var sn = stepNodes[m];
        nodeById[sn.id] = sn;
        stepNameByNodeId[sn.id] = sn.label || ('step-' + (m + 1));
      }
      var outgoingByNodeId = {};
      for (var c = 0; c < this.connections.length; c++) {
        var conn = this.connections[c];
        if (!nodeById[conn.from] || !nodeById[conn.to]) continue;
        if (!outgoingByNodeId[conn.from]) outgoingByNodeId[conn.from] = [];
        outgoingByNodeId[conn.from].push(conn);
      }
      for (var i = 0; i < stepNodes.length; i++) {
        var node = stepNodes[i];
        var step = {
          id: node.id || ('step-' + (i + 1)),
          name: node.label || 'step-' + (i + 1),
          mode: node.type === 'parallel' ? 'fan_out' : node.type === 'loop' ? 'loop' : 'sequential'
        };
        if (node.type === 'agent') {
          step.agent_name = node.config.agent_name || '';
          step.prompt = node.config.prompt || '{{input}}';
        }
        if (node.type === 'condition') {
          step.mode = 'conditional';
        }
        var outgoing = (outgoingByNodeId[node.id] || []).slice().sort(function(a, b) {
          return (Number(a.fromPort) || 0) - (Number(b.fromPort) || 0);
        });
        var targets = [];
        for (var t = 0; t < outgoing.length; t++) {
          var targetName = stepNameByNodeId[outgoing[t].to];
          if (!targetName) continue;
          if (targets.indexOf(targetName) >= 0) continue;
          targets.push(targetName);
        }
        if (node.type === 'parallel') {
          step.fan_targets = targets;
          if (!step.fan_targets.length) step.next = '';
        } else if (node.type === 'condition') {
          step.next_true = targets[0] || '';
          step.next_false = targets[1] || '';
        } else {
          step.next = targets[0] || '';
        }
        steps.push(step);
      }
      try {
        var response = await InfringAPI.post('/api/workflows', {
          name: this.workflowName || 'untitled',
          description: this.workflowDescription || '',
          steps: steps,
          graph: {
            nodes: this.nodes.map(function(node) {
              return {
                id: String(node.id || ''),
                type: String(node.type || ''),
                label: String(node.label || ''),
                x: Number(node.x) || 0,
                y: Number(node.y) || 0
              };
            }),
            connections: this.connections.map(function(conn) {
              return {
                from: String(conn.from || ''),
                from_port: Number(conn.fromPort) || 0,
                to: String(conn.to || ''),
                to_port: Number(conn.toPort) || 0
              };
            })
          }
        });
        InfringToast.success('Workflow saved!');
        this.showSaveModal = false;
        await this.refreshTraceCatalog(true);
        var savedId = String(
          (response && response.workflow && response.workflow.id) ||
          (response && response.id) ||
          ''
        ).trim();
        if (savedId) {
          await this.selectTraceWorkflow(savedId);
        }
      } catch(e) {
        InfringToast.error('Failed to save: ' + e.message);
      }
    },

    refreshTraceCatalog: async function(preserveSelection) {
      var keepSelection = preserveSelection !== false;
      var previous = String(this.traceWorkflowId || '').trim();
      this.traceError = '';
      try {
        var rows = await InfringAPI.get('/api/workflows');
        this.traceWorkflows = Array.isArray(rows) ? rows : [];
      } catch(e) {
        this.traceWorkflows = [];
        this.traceError = e && e.message ? String(e.message) : 'Failed to load workflows';
      }
      if (!this.traceWorkflows.length) {
        this.traceWorkflowId = '';
        this.traceRuns = [];
        return;
      }
      var selected = '';
      if (keepSelection && previous) {
        var keep = this.traceWorkflows.find(function(row) {
          return String((row && row.id) || '') === previous;
        });
        if (keep) selected = String(keep.id || '');
      }
      if (!selected) selected = String((this.traceWorkflows[0] && this.traceWorkflows[0].id) || '');
      await this.selectTraceWorkflow(selected);
    },

    selectTraceWorkflow: async function(workflowId) {
      var id = String(workflowId || '').trim();
      this.traceWorkflowId = id;
      this.traceRuns = [];
      this.traceError = '';
      if (!id) return;
      this.traceLoading = true;
      try {
        var payload = await InfringAPI.get('/api/workflows/' + encodeURIComponent(id) + '/runs');
        var rows = [];
        if (payload && Array.isArray(payload.runs)) rows = payload.runs;
        else if (Array.isArray(payload)) rows = payload;
        this.traceRuns = rows.slice().sort(function(a, b) {
          var left = Number(new Date((a && (a.finished_at || a.started_at)) || 0).getTime() || 0);
          var right = Number(new Date((b && (b.finished_at || b.started_at)) || 0).getTime() || 0);
          return right - left;
        });
      } catch(e) {
        this.traceRuns = [];
        this.traceError = e && e.message ? String(e.message) : 'Failed to load workflow traces';
      } finally {
        this.traceLoading = false;
      }
    },

    traceCards: function() {
      return Array.isArray(this.traceRuns) ? this.traceRuns.slice(0, 6) : [];
    },

    traceStatusLabel: function(run) {
      var status = String((run && run.status) || 'completed').trim().toLowerCase();
      if (!status) status = 'completed';
      return status.replace(/[_-]+/g, ' ');
    },

