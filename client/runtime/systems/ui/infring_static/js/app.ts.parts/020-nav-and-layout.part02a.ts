    bottomDockOrder: (() => {
      var defaults = ['chat', 'overview', 'agents', 'scheduler', 'skills', 'runtime', 'settings'];
      try {
        var raw = localStorage.getItem('infring-bottom-dock-order');
        var parsed = raw ? JSON.parse(raw) : [];
        if (!Array.isArray(parsed)) return defaults.slice();
        var seen = {};
        var ordered = [];
        for (var i = 0; i < parsed.length; i++) {
          var id = String(parsed[i] || '').trim();
          if (!id || seen[id] || defaults.indexOf(id) < 0) continue;
          seen[id] = true;
          ordered.push(id);
        }
        for (var j = 0; j < defaults.length; j++) {
          var fallbackId = defaults[j];
          if (seen[fallbackId]) continue;
          seen[fallbackId] = true;
          ordered.push(fallbackId);
        }
        return ordered;
      } catch(_) {
        return defaults.slice();
      }
    })(),
    bottomDockTileConfig: {
      chat: { icon: 'messages', tone: 'message', tooltip: 'Messages', label: 'Messages' },
      overview: { icon: 'home', tone: 'bright', tooltip: 'Home', label: 'Home' },
      agents: { icon: 'agents', tone: 'bright', tooltip: 'Agents', label: 'Agents' },
      scheduler: { icon: 'automation', tone: 'muted', tooltip: 'Automation', label: 'Automation', animation: ['automation-gears', 1200] },
      skills: { icon: 'apps', tone: 'default', tooltip: 'Apps', label: 'Apps' },
      runtime: { icon: 'system', tone: 'bright', tooltip: 'System', label: 'System', animation: ['system-terminal', 2000] },
      settings: { icon: 'settings', tone: 'muted', tooltip: 'Settings', label: 'Settings', animation: ['spin', 4000] }
    },
    appsIconBottomRowColors: (() => {
      var palette = ['#14b8a6', '#06b6d4', '#38bdf8', '#22c55e', '#f59e0b', '#ef4444', '#a855f7', '#f43f5e', '#64748b'];
      var out = [];
      for (var i = 0; i < 3; i += 1) {
        out.push(palette[Math.floor(Math.random() * palette.length)]);
      }
      return out;
    })(),
    bottomDockDragId: '',
    bottomDockDragStartOrder: [],
    bottomDockDragCommitted: false,
    bottomDockHoverId: '',
    bottomDockHoverWeightById: {},
    bottomDockPointerX: 0,
    bottomDockPointerY: 0,
    bottomDockPreviewText: '',
    bottomDockPreviewMorphFromText: '',
    bottomDockPreviewHoverKey: '',
    bottomDockPreviewX: 0,
    bottomDockPreviewY: 0,
    bottomDockPreviewWidth: 0,
    bottomDockPreviewVisible: false,
    bottomDockPreviewLabelMorphing: false,
    bottomDockPreviewLabelFxReady: true,
    _bottomDockPreviewHideTimer: 0,
    _bottomDockPreviewReflowRaf: 0,
    _bottomDockPreviewReflowFrames: 0,
    _bottomDockPreviewWidthRaf: 0,
    _bottomDockPreviewLabelFxRaf: 0,
    _bottomDockPreviewLabelFxTimer: 0,
    _bottomDockPreviewLabelMorphTimer: 0,
    bottomDockClickAnimId: '',
    _bottomDockDragGhostEl: null,
    _bottomDockClickAnimTimer: 0,
    _bottomDockClickAnimDurationMs: 980,
    _bottomDockSuppressClickUntil: 0,
    _bottomDockPointerActive: false,
    _bottomDockPointerMoved: false,
    _bottomDockPointerCandidateId: '',
    _bottomDockPointerStartX: 0,
    _bottomDockPointerStartY: 0,
    _bottomDockPointerLastX: 0,
    _bottomDockPointerLastY: 0,
    _bottomDockPointerGrabOffsetX: 16,
    _bottomDockPointerGrabOffsetY: 16,
    _bottomDockDragGhostWidth: 32,
    _bottomDockDragGhostHeight: 32,
    _bottomDockPointerMoveHandler: null,
    _bottomDockPointerUpHandler: null,
    _bottomDockGhostTargetX: 0,
    _bottomDockGhostTargetY: 0,
    _bottomDockGhostCurrentX: 0,
    _bottomDockGhostCurrentY: 0,
    _bottomDockGhostRaf: 0,
    _bottomDockGhostCleanupTimer: 0,
    _bottomDockMoveDurationMs: 360,
    _bottomDockExpandedScale: 1.54,
    bottomDockRotationDeg: Number.NaN,
    _bottomDockRevealTargetDuringSettle: false,
    _bottomDockDragBoundaries: [],
    _bottomDockLastInsertionIndex: -1,
    _bottomDockReorderLockUntil: 0,
    bottomDockPlacementId: (() => {
      try {
        var raw = String(localStorage.getItem('infring-bottom-dock-placement') || '').trim().toLowerCase();
        var allowed = {
          left: true,
          center: true,
          right: true,
          'top-left': true,
          'top-center': true,
          'top-right': true,
          'left-top': true,
          'left-bottom': true,
          'right-top': true,
          'right-bottom': true
        };
        if (allowed[raw]) return raw;
        if (raw === 'left-center') return 'left-top';
        if (raw === 'right-center') return 'right-top';
      } catch(_) {}
      return 'center';
    })(),
    bottomDockSnapPoints: [
      { id: 'left', x: 0.16, y: 0.995, side: 'bottom' },
      { id: 'center', x: 0.50, y: 0.995, side: 'bottom' },
      { id: 'right', x: 0.84, y: 0.995, side: 'bottom' },
      { id: 'top-left', x: 0.16, y: 0.005, side: 'top' },
      { id: 'top-center', x: 0.50, y: 0.005, side: 'top' },
      { id: 'top-right', x: 0.84, y: 0.005, side: 'top' },
      { id: 'left-top', x: 0.005, y: (1 / 3), side: 'left' },
      { id: 'left-bottom', x: 0.005, y: (2 / 3), side: 'left' },
      { id: 'right-top', x: 0.995, y: (1 / 3), side: 'right' },
      { id: 'right-bottom', x: 0.995, y: (2 / 3), side: 'right' }
    ],
    bottomDockContainerDragActive: false,
    bottomDockContainerSettling: false,
    bottomDockContainerDragX: 0,
    bottomDockContainerDragY: 0,
    _bottomDockContainerPointerActive: false,
    _bottomDockContainerPointerMoved: false,
    _bottomDockContainerPointerStartX: 0,
    _bottomDockContainerPointerStartY: 0,
    _bottomDockContainerPointerLastX: 0,
    _bottomDockContainerPointerLastY: 0,
    _bottomDockContainerOriginX: 0,
    _bottomDockContainerOriginY: 0,
    _bottomDockContainerPointerMoveHandler: null,
    _bottomDockContainerPointerUpHandler: null,
    _bottomDockContainerSettleTimer: 0,
