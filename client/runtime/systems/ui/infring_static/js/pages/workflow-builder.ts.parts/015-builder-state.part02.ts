      for (var i = 0; i < this.nodeTypes.length; i++) {
        if (this.nodeTypes[i].type === type) { def = this.nodeTypes[i]; break; }
      }
      if (!def) return;
      var node = {
        id: 'node-' + this.nextId++,
        type: type,
        label: def.label,
        color: def.color,
        icon: def.icon,
        x: x || 200,
        y: y || 200,
        width: 180,
        height: 70,
        ports: { in: def.ports.in, out: def.ports.out },
        config: {}
      };
      if (type === 'agent') {
        node.config = { agent_name: '', prompt: '{{input}}', model: '' };
      } else if (type === 'condition') {
        node.config = { expression: '', true_label: 'Yes', false_label: 'No' };
      } else if (type === 'loop') {
        node.config = { max_iterations: 5, until: '' };
      } else if (type === 'parallel') {
        node.config = { fan_count: 3 };
      } else if (type === 'collect') {
        node.config = { strategy: 'all' };
      }
      this.nodes.push(node);
      this.scheduleRender();
      return node;
    },

    deleteNode: function(nodeId) {
      this.connections = this.connections.filter(function(c) {
        return c.from !== nodeId && c.to !== nodeId;
      });
      this.nodes = this.nodes.filter(function(n) { return n.id !== nodeId; });
      if (this.selectedNode && this.selectedNode.id === nodeId) {
        this.selectedNode = null;
        this.showNodeEditor = false;
      }
      this.scheduleRender();
    },

    duplicateNode: function(node) {
      var newNode = this.addNode(node.type, node.x + 30, node.y + 30);
      if (newNode) {
        newNode.config = JSON.parse(JSON.stringify(node.config));
        newNode.label = node.label + ' copy';
      }
    },

    getNode: function(id) {
      for (var i = 0; i < this.nodes.length; i++) {
        if (this.nodes[i].id === id) return this.nodes[i];
      }
      return null;
    },

    // ── Port Positions ───────────────────────────────────

    getInputPortPos: function(node, portIndex) {
      var total = node.ports.in;
      var spacing = node.width / (total + 1);
      return { x: node.x + spacing * (portIndex + 1), y: node.y };
    },

    getOutputPortPos: function(node, portIndex) {
      var total = node.ports.out;
      var spacing = node.width / (total + 1);
      return { x: node.x + spacing * (portIndex + 1), y: node.y + node.height };
    },

    // ── Connection Management ────────────────────────────

    startConnect: function(nodeId, portIndex, e) {
      e.stopPropagation();
      this.connecting = { fromId: nodeId, fromPort: portIndex };
      var node = this.getNode(nodeId);
      var pos = this.getOutputPortPos(node, portIndex);
      this.connectPreview = { x: pos.x, y: pos.y };
    },

    endConnect: function(nodeId, portIndex, e) {
      e.stopPropagation();
      if (!this.connecting) return;
      if (this.connecting.fromId === nodeId) {
        this.connecting = null;
        this.connectPreview = null;
        return;
      }
      // Check for duplicate
      var fromId = this.connecting.fromId;
      var fromPort = this.connecting.fromPort;
      var dup = false;
      for (var i = 0; i < this.connections.length; i++) {
        var c = this.connections[i];
        if (c.from === fromId && c.fromPort === fromPort && c.to === nodeId && c.toPort === portIndex) {
          dup = true;
          break;
        }
      }
      if (!dup) {
        this.connections.push({
          id: 'conn-' + this.nextId++,
          from: fromId,
          fromPort: fromPort,
          to: nodeId,
          toPort: portIndex
        });
      }
      this.connecting = null;
      this.connectPreview = null;
      this.scheduleRender();
    },

    deleteConnection: function(connId) {
      this.connections = this.connections.filter(function(c) { return c.id !== connId; });
      this.selectedConnection = null;
      this.scheduleRender();
    },

    // ── Drag Handling ────────────────────────────────────

    onNodeMouseDown: function(node, e) {
      e.stopPropagation();
      // Detect double-click manually — the native dblclick event never fires
      // because scheduleRender() destroys and recreates all SVG elements between
      // the first and second click, so the browser loses the DOM target for dblclick.
      var now = Date.now();
      if (this._lastClickNodeId === node.id && (now - this._lastClickTime) < 350) {
        // Double-click detected — open editor instead of starting drag
        this._lastClickNodeId = null;
        this._lastClickTime = 0;
        this.editNode(node);
        return;
      }
      this._lastClickNodeId = node.id;
      this._lastClickTime = now;

      this.selectedNode = node;
      this.selectedConnection = null;
      this._didDrag = false;
      this.dragging = node.id;
      var rect = this._getCanvasRect();
      this.dragOffset = {
        x: (e.clientX - rect.left) / this.zoom - this.canvasOffset.x - node.x,
        y: (e.clientY - rect.top) / this.zoom - this.canvasOffset.y - node.y
      };
    },

    onCanvasMouseDown: function(e) {
      if (e.target.closest('.wf-node') || e.target.closest('.wf-port')) return;
      this.selectedNode = null;
      this.selectedConnection = null;
      this.showNodeEditor = false;
      // Start canvas pan
      this._didPan = false;
      this.canvasDragging = true;
      this.canvasDragStart = { x: e.clientX - this.canvasOffset.x * this.zoom, y: e.clientY - this.canvasOffset.y * this.zoom };
    },

    onCanvasMouseMove: function(e) {
      var rect = this._getCanvasRect();
      if (this.dragging) {
        this._didDrag = true;
        var node = this.getNode(this.dragging);
        if (node) {
          node.x = Math.max(0, (e.clientX - rect.left) / this.zoom - this.canvasOffset.x - this.dragOffset.x);
          node.y = Math.max(0, (e.clientY - rect.top) / this.zoom - this.canvasOffset.y - this.dragOffset.y);
        }
        this.scheduleRender();
      } else if (this.connecting) {
        this._didConnect = true;
        this.connectPreview = {
          x: (e.clientX - rect.left) / this.zoom - this.canvasOffset.x,
          y: (e.clientY - rect.top) / this.zoom - this.canvasOffset.y
        };
        this.scheduleRender();
      } else if (this.canvasDragging) {
        this._didPan = true;
        this.canvasOffset = {
          x: (e.clientX - this.canvasDragStart.x) / this.zoom,
          y: (e.clientY - this.canvasDragStart.y) / this.zoom
        };

