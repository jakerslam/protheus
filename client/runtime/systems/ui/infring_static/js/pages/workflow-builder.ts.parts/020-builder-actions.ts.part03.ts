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
