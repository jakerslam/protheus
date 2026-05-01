// Infring Visual Workflow Builder — Drag-and-drop workflow designer
'use strict';

function workflowBuilder() {
  return {
    // -- Canvas state --
    nodes: [],
    connections: [],
    selectedNode: null,
    selectedConnection: null,
    dragging: null,
    dragOffset: { x: 0, y: 0 },
    connecting: null, // { fromId, fromPort }
    connectPreview: null, // { x, y } mouse position during connect drag
    canvasOffset: { x: 0, y: 0 },
    canvasDragging: false,
    canvasDragStart: { x: 0, y: 0 },
    zoom: 1,
    nextId: 1,
    workflowName: '',
    workflowDescription: '',
    showSaveModal: false,
    showNodeEditor: false,
    showTomlPreview: false,
    tomlOutput: '',
    agents: [],
    traceWorkflows: [],
    traceWorkflowId: '',
    traceRuns: [],
    traceLoading: false,
    traceError: '',
    _canvasEl: null,

    // Node types with their configs
    nodeTypes: [
      { type: 'agent', label: 'Agent Step', color: '#6366f1', icon: 'A', ports: { in: 1, out: 1 } },
      { type: 'parallel', label: 'Parallel Fan-out', color: '#f59e0b', icon: 'P', ports: { in: 1, out: 3 } },
      { type: 'condition', label: 'Condition', color: '#10b981', icon: '?', ports: { in: 1, out: 2 } },
      { type: 'loop', label: 'Loop', color: '#ef4444', icon: 'L', ports: { in: 1, out: 1 } },
      { type: 'collect', label: 'Collect', color: '#8b5cf6', icon: 'C', ports: { in: 3, out: 1 } },
      { type: 'start', label: 'Start', color: '#22c55e', icon: 'S', ports: { in: 0, out: 1 } },
      { type: 'end', label: 'End', color: '#ef4444', icon: 'E', ports: { in: 1, out: 0 } }
    ],

    _renderScheduled: false,
    _lastClickNodeId: null,
    _lastClickTime: 0,
    _didDrag: false,
    _didConnect: false,
    _didPan: false,

    async init() {
      var self = this;
      // Load agents for the agent step dropdown
      try {
        var list = await InfringAPI.get('/api/agents');
        self.agents = Array.isArray(list) ? list : [];
      } catch(_) {
        self.agents = [];
      }
      // Add default start node
      self.addNode('start', 60, 200);
      await self.refreshTraceCatalog();
    },

    // ── SVG Manual Rendering ────────────────────────────
    // Alpine.js x-for inside <svg> breaks because document.importNode
    // doesn't handle SVG namespace correctly. We render nodes/connections
    // manually via createElementNS and schedule re-renders reactively.

    scheduleRender: function() {
      if (this._renderScheduled) return;
      this._renderScheduled = true;
      var self = this;
      requestAnimationFrame(function() {
        self._renderScheduled = false;
        self.renderCanvas();
      });
    },

    renderCanvas: function() {
      var container = document.getElementById('wf-render-group');
      if (!container) return;
      var SVG_NS = 'http://www.w3.org/2000/svg';
      var self = this;

      // Clear previous rendered content
      while (container.firstChild) container.removeChild(container.firstChild);

      // ── Connections ──
      for (var ci = 0; ci < this.connections.length; ci++) {
        var conn = this.connections[ci];
        var d = this.getConnectionPath(conn);
        if (!d) continue;
        var path = document.createElementNS(SVG_NS, 'path');
        path.setAttribute('d', d);
        path.setAttribute('fill', 'none');
        path.setAttribute('stroke', (this.selectedConnection && this.selectedConnection.id === conn.id) ? 'var(--accent)' : 'var(--text-dim)');
        path.setAttribute('stroke-width', (this.selectedConnection && this.selectedConnection.id === conn.id) ? '3' : '2');
        path.style.cursor = 'pointer';
        (function(c) {
          path.addEventListener('click', function(e) { e.stopPropagation(); self.selectedConnection = c; self.scheduleRender(); });
        })(conn);
        container.appendChild(path);
      }

      // ── Connection preview ──
      if (this.connecting && this.connectPreview) {
        var pd = this.getPreviewPath();
        if (pd) {
          var preview = document.createElementNS(SVG_NS, 'path');
          preview.setAttribute('d', pd);
          preview.setAttribute('fill', 'none');
          preview.setAttribute('stroke', 'var(--accent)');
          preview.setAttribute('stroke-width', '2');
          preview.setAttribute('stroke-dasharray', '6,3');
          container.appendChild(preview);
        }
      }

      // ── Nodes ──
      for (var ni = 0; ni < this.nodes.length; ni++) {
        var node = this.nodes[ni];
        var g = document.createElementNS(SVG_NS, 'g');
        g.classList.add('wf-node');
        g.setAttribute('transform', 'translate(' + node.x + ',' + node.y + ')');
        (function(n) {
          g.addEventListener('mousedown', function(e) { self.onNodeMouseDown(n, e); });
          g.addEventListener('dblclick', function() { self.editNode(n); });
        })(node);

        // Node body rect
        var rect = document.createElementNS(SVG_NS, 'rect');
        rect.setAttribute('x', '0'); rect.setAttribute('y', '0');
        rect.setAttribute('width', node.width); rect.setAttribute('height', node.height);
        rect.setAttribute('rx', '8'); rect.setAttribute('ry', '8');
        rect.setAttribute('fill', (self.selectedNode && self.selectedNode.id === node.id) ? 'var(--card-bg)' : 'var(--bg-secondary)');
        rect.setAttribute('stroke', (self.selectedNode && self.selectedNode.id === node.id) ? node.color : 'var(--border)');
        rect.setAttribute('stroke-width', '2');
        rect.style.cursor = 'grab';
        g.appendChild(rect);

        // Color accent bar
        var bar = document.createElementNS(SVG_NS, 'rect');
        bar.setAttribute('x', '0'); bar.setAttribute('y', '0');
        bar.setAttribute('width', '6'); bar.setAttribute('height', node.height);
        bar.setAttribute('rx', '3'); bar.setAttribute('ry', '0');
        bar.setAttribute('fill', node.color);
        g.appendChild(bar);

        // Icon circle + text
        var circle = document.createElementNS(SVG_NS, 'circle');
        circle.setAttribute('cx', '28'); circle.setAttribute('cy', node.height / 2);
        circle.setAttribute('r', '14'); circle.setAttribute('fill', node.color);
        circle.setAttribute('opacity', '0.15');
        g.appendChild(circle);

        var iconText = document.createElementNS(SVG_NS, 'text');
        iconText.setAttribute('x', '28'); iconText.setAttribute('y', node.height / 2 + 4);
        iconText.setAttribute('text-anchor', 'middle'); iconText.setAttribute('fill', node.color);
        iconText.setAttribute('style', 'font-size:12px;font-weight:700;pointer-events:none');
        iconText.textContent = node.icon;
        g.appendChild(iconText);

        // Label
        var label = document.createElementNS(SVG_NS, 'text');
        label.setAttribute('x', '50'); label.setAttribute('y', node.height / 2 - 4);
        label.setAttribute('fill', 'var(--text)');
        label.setAttribute('style', 'font-size:12px;font-weight:600;pointer-events:none');
        label.textContent = node.label;
        g.appendChild(label);

        // Sub-label
        var subLabel = document.createElementNS(SVG_NS, 'text');
        subLabel.setAttribute('x', '50'); subLabel.setAttribute('y', node.height / 2 + 12);
        subLabel.setAttribute('fill', 'var(--text-dim)');
        subLabel.setAttribute('style', 'font-size:10px;pointer-events:none');
        if (node.type === 'agent') subLabel.textContent = node.config.agent_name || 'No agent';
        else if (node.type === 'condition') subLabel.textContent = node.config.expression || 'No condition';
        else if (node.type === 'loop') subLabel.textContent = 'max ' + (node.config.max_iterations || 5) + ' iters';
        else if (node.type === 'parallel') subLabel.textContent = (node.config.fan_count || 3) + ' branches';
        else if (node.type === 'collect') subLabel.textContent = node.config.strategy || 'all';
        g.appendChild(subLabel);

        // Input ports
        for (var pi = 0; pi < node.ports.in; pi++) {
          var inp = document.createElementNS(SVG_NS, 'circle');
          inp.classList.add('wf-port', 'wf-port-in');
          inp.setAttribute('cx', node.width / (node.ports.in + 1) * (pi + 1));
          inp.setAttribute('cy', '0'); inp.setAttribute('r', '6');
          inp.setAttribute('fill', 'var(--bg-secondary)');
          inp.setAttribute('stroke', 'var(--text-dim)'); inp.setAttribute('stroke-width', '2');
          (function(nid, idx) {
            inp.addEventListener('mouseup', function(e) { e.stopPropagation(); self.endConnect(nid, idx, e); });
          })(node.id, pi);
          g.appendChild(inp);
        }

        // Output ports
        for (var po = 0; po < node.ports.out; po++) {
          var outp = document.createElementNS(SVG_NS, 'circle');
          outp.classList.add('wf-port', 'wf-port-out');
          outp.setAttribute('cx', node.width / (node.ports.out + 1) * (po + 1));
          outp.setAttribute('cy', node.height); outp.setAttribute('r', '6');
          outp.setAttribute('fill', 'var(--bg-secondary)');
          outp.setAttribute('stroke', node.color); outp.setAttribute('stroke-width', '2');
          (function(nid, idx) {
            outp.addEventListener('mousedown', function(e) { e.stopPropagation(); self.startConnect(nid, idx, e); });
          })(node.id, po);
          g.appendChild(outp);
        }

        container.appendChild(g);
      }
    },

    // ── Node Management ──────────────────────────────────

    addNode: function(type, x, y) {
      var def = null;
