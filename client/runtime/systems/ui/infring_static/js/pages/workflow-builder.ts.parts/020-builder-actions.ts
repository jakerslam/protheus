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

