      }
    },

    onCanvasMouseUp: function() {
      // Only re-render if something actually moved. Rendering on every mouseup
      // destroys SVG elements between clicks, which prevents dblclick detection.
      var needsRender = this._didDrag || this._didConnect || this._didPan;
      this.dragging = null;
      this.connecting = null;
      this.connectPreview = null;
      this.canvasDragging = false;
      this._didDrag = false;
      this._didConnect = false;
      this._didPan = false;
      if (needsRender) {
        this.scheduleRender();
      }
    },

    onCanvasWheel: function(e) {
      e.preventDefault();
      var delta = e.deltaY > 0 ? -0.05 : 0.05;
      this.zoom = Math.max(0.3, Math.min(2, this.zoom + delta));
    },

    _getCanvasRect: function() {
      if (!this._canvasEl) {
        this._canvasEl = document.getElementById('wf-canvas');
      }
      return this._canvasEl ? this._canvasEl.getBoundingClientRect() : { left: 0, top: 0 };
    },

    // ── Connection Path ──────────────────────────────────

    getConnectionPath: function(conn) {
      var fromNode = this.getNode(conn.from);
      var toNode = this.getNode(conn.to);
      if (!fromNode || !toNode) return '';
      var from = this.getOutputPortPos(fromNode, conn.fromPort);
      var to = this.getInputPortPos(toNode, conn.toPort);
      var dy = Math.abs(to.y - from.y);
      var cp = Math.max(40, dy * 0.5);
      return 'M ' + from.x + ' ' + from.y + ' C ' + from.x + ' ' + (from.y + cp) + ' ' + to.x + ' ' + (to.y - cp) + ' ' + to.x + ' ' + to.y;
    },

    getPreviewPath: function() {
      if (!this.connecting || !this.connectPreview) return '';
      var fromNode = this.getNode(this.connecting.fromId);
      if (!fromNode) return '';
      var from = this.getOutputPortPos(fromNode, this.connecting.fromPort);
      var to = this.connectPreview;
      var dy = Math.abs(to.y - from.y);
      var cp = Math.max(40, dy * 0.5);
      return 'M ' + from.x + ' ' + from.y + ' C ' + from.x + ' ' + (from.y + cp) + ' ' + to.x + ' ' + (to.y - cp) + ' ' + to.x + ' ' + to.y;
    },

    // ── Node editor ──────────────────────────────────────

    editNode: function(node) {
      this.selectedNode = node;
      this.showNodeEditor = true;
      this.scheduleRender();
    },

    // Called from editor panel inputs to reflect changes on the canvas SVG
    applyNodeEdit: function() {
      this.scheduleRender();
    },

    // ── TOML Generation ──────────────────────────────────

    generateToml: function() {
      var self = this;
      var lines = [];
      lines.push('[workflow]');
      lines.push('name = "' + (this.workflowName || 'untitled') + '"');
      lines.push('description = "' + (this.workflowDescription || '') + '"');
      lines.push('');

      // Topological sort the nodes (skip start/end for step generation)
      var stepNodes = this.nodes.filter(function(n) {
        return n.type !== 'start' && n.type !== 'end';
      });

      for (var i = 0; i < stepNodes.length; i++) {
        var node = stepNodes[i];
        lines.push('[[workflow.steps]]');
        lines.push('name = "' + (node.label || 'step-' + (i + 1)) + '"');

        if (node.type === 'agent') {
          lines.push('type = "agent"');
          if (node.config.agent_name) lines.push('agent_name = "' + node.config.agent_name + '"');
          lines.push('prompt = "' + (node.config.prompt || '{{input}}') + '"');
          if (node.config.model) lines.push('model = "' + node.config.model + '"');
        } else if (node.type === 'parallel') {
          lines.push('type = "fan_out"');
          lines.push('fan_count = ' + (node.config.fan_count || 3));
        } else if (node.type === 'condition') {
          lines.push('type = "conditional"');
          lines.push('expression = "' + (node.config.expression || '') + '"');
        } else if (node.type === 'loop') {
          lines.push('type = "loop"');
          lines.push('max_iterations = ' + (node.config.max_iterations || 5));
          if (node.config.until) lines.push('until = "' + node.config.until + '"');
        } else if (node.type === 'collect') {
          lines.push('type = "collect"');
          lines.push('strategy = "' + (node.config.strategy || 'all') + '"');
        }

        // Find what this node connects to
        var outConns = self.connections.filter(function(c) { return c.from === node.id; });
        if (outConns.length === 1) {
          var target = self.getNode(outConns[0].to);
          if (target && target.type !== 'end') {
            lines.push('next = "' + target.label + '"');
          }
        } else if (outConns.length > 1 && node.type === 'condition') {
          for (var j = 0; j < outConns.length; j++) {
            var t2 = self.getNode(outConns[j].to);
            if (t2 && t2.type !== 'end') {
              var branchLabel = j === 0 ? 'true' : 'false';
              lines.push('next_' + branchLabel + ' = "' + t2.label + '"');
            }
          }
        } else if (outConns.length > 1 && node.type === 'parallel') {
          var targets = [];
          for (var k = 0; k < outConns.length; k++) {
            var t3 = self.getNode(outConns[k].to);
            if (t3 && t3.type !== 'end') targets.push('"' + t3.label + '"');
          }
          if (targets.length) lines.push('fan_targets = [' + targets.join(', ') + ']');
        }

        lines.push('');
      }

      this.tomlOutput = lines.join('\n');
      this.showTomlPreview = true;
    },

    // ── Save Workflow ────────────────────────────────────

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

    traceDurationLabel: function(run) {
      var ms = Number(run && run.duration_ms);
      if (!Number.isFinite(ms) || ms <= 0) return '';
      if (ms < 1000) return ms + 'ms';
      if (ms < 60000) return (ms / 1000).toFixed(1).replace(/\.0$/, '') + 's';
      return (ms / 60000).toFixed(1).replace(/\.0$/, '') + 'm';
    },

    traceStepPreview: function(step) {
      var text = String((step && step.output) || '').replace(/\s+/g, ' ').trim();
      if (!text) return '(no output)';
      return text.length > 140 ? text.substring(0, 137) + '...' : text;
    },

    // ── Palette drop ─────────────────────────────────────

    onPaletteDragStart: function(type, e) {
      e.dataTransfer.setData('text/plain', type);
      e.dataTransfer.effectAllowed = 'copy';
    },

    onCanvasDrop: function(e) {
      e.preventDefault();
      var type = e.dataTransfer.getData('text/plain');
      if (!type) return;
      var rect = this._getCanvasRect();
      var x = (e.clientX - rect.left) / this.zoom - this.canvasOffset.x;
      var y = (e.clientY - rect.top) / this.zoom - this.canvasOffset.y;
      this.addNode(type, x - 90, y - 35); // addNode already calls scheduleRender
    },

    onCanvasDragOver: function(e) {
      e.preventDefault();
      e.dataTransfer.dropEffect = 'copy';
    },

    // ── Auto Layout ──────────────────────────────────────

    autoLayout: function() {
      // Simple top-to-bottom layout
      var y = 40;
      var x = 200;
      for (var i = 0; i < this.nodes.length; i++) {
        this.nodes[i].x = x;
        this.nodes[i].y = y;
        y += 120;
      }
      this.scheduleRender();
    },

    // ── Clear ────────────────────────────────────────────

    clearCanvas: function() {
      this.nodes = [];
      this.connections = [];
      this.selectedNode = null;
      this.nextId = 1;
      this.addNode('start', 60, 200); // addNode already calls scheduleRender
    },

    // ── Zoom controls ────────────────────────────────────

    zoomIn: function() {
      this.zoom = Math.min(2, this.zoom + 0.1);
    },

    zoomOut: function() {
      this.zoom = Math.max(0.3, this.zoom - 0.1);
    },

    zoomReset: function() {
      this.zoom = 1;
      this.canvasOffset = { x: 0, y: 0 };
    }
  };
}
