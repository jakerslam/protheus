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
    // Legacy x-for inside <svg> breaks because document.importNode
    // doesn't handle SVG namespace correctly. We render nodes/connections
    // manually via createElementNS and schedule re-renders reactively.

    ...infringWorkflowBuilderCanvasMethods(),

    // ── Node editor ──────────────────────────────────────

    editNode: function(node) {
      this.selectedNode = node;
      this.showNodeEditor = true;
      this.scheduleRender();
    },

    renderAgentSelectOptions: function(selectEl) {
      if (!selectEl) return;
      var selectedValue = this.selectedNode && this.selectedNode.config
        ? String(this.selectedNode.config.agent_name || '')
        : '';
      while (selectEl.options && selectEl.options.length > 1) {
        selectEl.remove(1);
      }
      var rows = Array.isArray(this.agents) ? this.agents : [];
      for (var i = 0; i < rows.length; i += 1) {
        var agent = rows[i] || {};
        var label = String(agent.name || agent.id || '').trim();
        if (!label) continue;
        var option = document.createElement('option');
        option.value = label;
        option.textContent = label;
        selectEl.appendChild(option);
      }
      selectEl.value = selectedValue;
    },

    // Called from editor panel inputs to reflect changes on the canvas SVG
    applyNodeEdit: function() {
      this.scheduleRender();
    },

    ...infringWorkflowBuilderPersistTraceMethods(),

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
