function infringReadUiBackgroundTemplateDefault() {
  var service = infringTaskbarDockService();
  if (service && typeof service.readDisplayBackground === 'function') return service.readDisplayBackground();
  var mode = 'light-wood';
  try {
    var rawDisplaySettings = localStorage.getItem('infring-display-settings') || '';
    var displaySettings = rawDisplaySettings ? JSON.parse(rawDisplaySettings) : {};
    mode = String(displaySettings && displaySettings.background ? displaySettings.background : mode);
    if (mode === 'sand') {
      mode = 'light-wood';
      displaySettings = displaySettings && typeof displaySettings === 'object' ? displaySettings : {};
      displaySettings.background = mode;
      localStorage.setItem('infring-display-settings', JSON.stringify(displaySettings));
    }
    if (!rawDisplaySettings || !displaySettings.background) {
      displaySettings = displaySettings && typeof displaySettings === 'object' ? displaySettings : {};
      displaySettings.background = mode;
      localStorage.setItem('infring-display-settings', JSON.stringify(displaySettings));
    }
  } catch (_) {}
  if (mode === 'unsplash-paper') mode = 'light-wood';
  if (mode !== 'default-grid' && mode !== 'light-wood' && mode !== 'sand') mode = 'light-wood';
  try {
    document.documentElement.setAttribute('data-ui-background-template', mode);
  } catch (_) {}
  return mode;
}

function infringReadTaskbarDockEdgeDefault() {
  var service = infringTaskbarDockService();
  if (service && typeof service.readLayoutConfig === 'function') return service.readLayoutConfig().taskbar.edge;
  try {
    var raw = String(localStorage.getItem('infring-taskbar-dock-edge') || '').trim().toLowerCase();
    if (raw === 'bottom') return 'bottom';
  } catch(_) {}
  return 'top';
}

function infringReadChatSidebarSortModeDefault() {
  try {
    var saved = String(localStorage.getItem('infring-chat-sidebar-sort-mode') || '').trim().toLowerCase();
    return saved === 'topology' ? 'topology' : 'age';
  } catch(_) {
    return 'age';
  }
}

function infringReadChatSidebarTopologyOrderDefault() {
  try {
    var raw = localStorage.getItem('infring-chat-sidebar-topology-order');
    var parsed = raw ? JSON.parse(raw) : [];
    if (!Array.isArray(parsed)) return [];
    return parsed.map(function(id) { return String(id || '').trim(); }).filter(Boolean);
  } catch(_) {
    return [];
  }
}

function infringReadStoredOrderedIds(storageKey, defaults) {
  var fallback = Array.isArray(defaults) ? defaults.slice() : [];
  try {
    var raw = localStorage.getItem(storageKey);
    var parsed = raw ? JSON.parse(raw) : [];
    if (!Array.isArray(parsed)) return fallback;
    var seen = {};
    var ordered = [];
    for (var i = 0; i < parsed.length; i += 1) {
      var id = String(parsed[i] || '').trim();
      if (!id || seen[id] || fallback.indexOf(id) < 0) continue;
      seen[id] = true;
      ordered.push(id);
    }
    for (var j = 0; j < fallback.length; j += 1) {
      var fallbackId = fallback[j];
      if (seen[fallbackId]) continue;
      seen[fallbackId] = true;
      ordered.push(fallbackId);
    }
    return ordered;
  } catch(_) {
    return fallback;
  }
}

function infringReadTaskbarReorderDefault(side) {
  var service = infringTaskbarDockService();
  if (service && typeof service.readTaskbarOrder === 'function') return service.readTaskbarOrder(side);
  if (side === 'left') return infringReadStoredOrderedIds('infring-taskbar-order-left', ['nav_cluster']);
  return infringReadStoredOrderedIds('infring-taskbar-order-right', ['connectivity', 'theme', 'notifications', 'search', 'auth']);
}

function infringReadBottomDockOrderDefault() {
  var service = infringTaskbarDockService();
  if (service && typeof service.readDockOrder === 'function') return service.readDockOrder();
  return infringReadStoredOrderedIds('infring-bottom-dock-order', ['chat', 'overview', 'agents', 'scheduler', 'skills', 'runtime', 'settings']);
}

function infringReadBottomDockTileConfigDefault() {
  var service = infringTaskbarDockService();
  if (service && typeof service.dockTileConfig === 'function') return service.dockTileConfig();
  return {
    chat: { icon: 'messages', tone: 'message', tooltip: 'Messages', label: 'Messages' },
    overview: { icon: 'home', tone: 'bright', tooltip: 'Home', label: 'Home' },
    agents: { icon: 'agents', tone: 'bright', tooltip: 'Agents', label: 'Agents' },
    scheduler: { icon: 'automation', tone: 'muted', tooltip: 'Automation', label: 'Automation', animation: ['automation-gears', 1200] },
    skills: { icon: 'apps', tone: 'default', tooltip: 'Apps', label: 'Apps' },
    runtime: { icon: 'system', tone: 'bright', tooltip: 'System', label: 'System', animation: ['system-terminal', 2000] },
    settings: { icon: 'settings', tone: 'muted', tooltip: 'Settings', label: 'Settings', animation: ['spin', 4000] }
  };
}

function infringReadAppsIconBottomRowColorsDefault() {
  var palette = ['#14b8a6', '#06b6d4', '#38bdf8', '#22c55e', '#f59e0b', '#ef4444', '#a855f7', '#f43f5e', '#64748b'];
  var out = [];
  for (var i = 0; i < 3; i += 1) {
    out.push(palette[Math.floor(Math.random() * palette.length)]);
  }
  return out;
}

function infringReadBottomDockPlacementDefault() {
  var service = infringTaskbarDockService();
  if (service && typeof service.readLayoutConfig === 'function') return service.readLayoutConfig().dock.placement;
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
}

function infringReadBottomDockContainerWallLockDefault() {
  var service = infringTaskbarDockService();
  if (service && typeof service.readLayoutConfig === 'function') return service.readLayoutConfig().dock.wallLock;
  return infringReadStoredWallLock('infring-bottom-dock-wall-lock', 'infring-bottom-dock-smash-wall');
}

function infringBottomDockSnapPointsDefault() {
  return [
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
  ];
}

function infringReadStoredClampedRatio(storageKey, fallback) {
  try {
    var raw = Number(localStorage.getItem(storageKey));
    if (Number.isFinite(raw)) return Math.max(0, Math.min(1, raw));
  } catch(_) {}
  return fallback;
}

function infringReadStoredNumberOrNaN(storageKey) {
  try {
    var raw = Number(localStorage.getItem(storageKey));
    if (Number.isFinite(raw)) return raw;
  } catch(_) {}
  return Number.NaN;
}

function infringReadStoredWallLock(primaryKey, legacyKey) {
  try {
    var raw = String(
      localStorage.getItem(primaryKey)
      || (legacyKey ? localStorage.getItem(legacyKey) : '')
      || ''
    ).trim().toLowerCase();
    if (raw === 'left' || raw === 'right' || raw === 'top' || raw === 'bottom') return raw;
  } catch(_) {}
  return '';
}

function infringAppInitialState() {
  return {
    page: 'agents',
    themeMode: localStorage.getItem('infring-theme-mode') || 'system',
    overlayGlassTemplate: 'simple-glass',
    uiBackgroundTemplate: infringReadUiBackgroundTemplateDefault(),
    theme: (() => {
      var mode = localStorage.getItem('infring-theme-mode') || 'system';
      if (mode === 'system') return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
      return mode;
    })(),
    sidebarCollapsed: localStorage.getItem('infring-sidebar') === 'collapsed',
    mobileMenuOpen: false,
    chatSidebarMode: 'default',
    chatSidebarQuery: '',
    chatSidebarSearchResults: [],
    chatSidebarSearchLoading: false,
    chatSidebarSearchError: '',
    chatSidebarSearchSeq: 0,
    _chatSidebarSearchTimer: 0,
    agentChatsSectionCollapsed: false,
    chatSidebarSortMode: infringReadChatSidebarSortModeDefault(),
    chatSidebarTopologyOrder: infringReadChatSidebarTopologyOrderDefault(),
    chatSidebarDragAgentId: '',

    chatSidebarDropTargetId: '',
    chatSidebarDropAfter: false,
    chatSidebarVisibleBase: 7,
    chatSidebarVisibleStep: 5,
    chatSidebarVisibleCount: 7,
    dashboardPopup: {
      id: '',
      active: false,
      source: '',
      title: '',
      body: '',
      meta_origin: '',
      meta_time: '',
      unread: false,
      left: 0,
      top: 0,
      side: 'bottom',
      inline_away: 'right',
      block_away: 'bottom',
      compact: false
    },
    confirmArchiveAgentId: '',
    sidebarSpawningAgent: false,
    connected: false,
    wsConnected: false,
    connectionState: 'connecting',
    connectionIndicatorState: 'connecting',
    healthSummary: null,
    healthSummaryError: '',
    version: (window.__INFRING_APP_VERSION || '0.0.0'),
    agentCount: 0,
    bootSelectionApplied: false,
    clockTick: Date.now(),
    _dashboardClockTimer: 0,
    _dashboardStatusTimer: 0,
    _dashboardVisibilityHandler: null,
    _themeSwitchReset: 0,
    _lastConnectionIndicatorAt: 0,
    _connectionIndicatorTimer: null,
    _pendingConnectionIndicatorState: '',
    _healthSummaryLoadedAt: 0,
    _healthSummaryLoading: null,
    _healthSummaryLoadSeq: 0,
    _pollStatusInFlight: null,
    _pollStatusQueued: false,
    sidebarHasOverflowAbove: false,
    sidebarHasOverflowBelow: false,
    chatSidebarHasOverflowAbove: false,
    chatSidebarHasOverflowBelow: false,
    _sidebarScrollIndicatorRaf: 0,
    _chatSidebarFlipDurationMs: 240,
    _chatSidebarFlipRaf: 0,
    _chatSidebarLastSnapshot: null,
    _dragSurfaceLockTransformMs: 500,
    _dragSurfaceVisualStates: {},
    chatSidebarDragActive: false,
    chatSidebarDragLeft: 0,
    chatSidebarDragTop: 0,
    _chatSidebarDragRowsCache: null,
    _chatSidebarDragRenderMaxRows: 10,
    _chatSidebarDragRenderRowHeight: 56,
    chatSidebarPlacementX: infringReadStoredClampedRatio('infring-chat-sidebar-placement-x', 0),
    chatSidebarPlacementY: infringReadStoredClampedRatio('infring-chat-sidebar-placement-y', 0.5),
    chatSidebarPlacementTopPx: infringReadStoredNumberOrNaN('infring-chat-sidebar-placement-top-px'),
    chatSidebarWallLock: infringReadStoredWallLock('infring-chat-sidebar-wall-lock', 'infring-chat-sidebar-smash-wall'),
    _chatSidebarMoveDurationMs: 280,
    _chatSidebarPointerActive: false,
    _chatSidebarPointerMoved: false,
    _chatSidebarPointerStartX: 0,
    _chatSidebarPointerStartY: 0,
    _chatSidebarPointerOriginLeft: 0,
    _chatSidebarPointerOriginTop: 0,
    _chatSidebarPointerLastX: 0,
    _chatSidebarPointerLastY: 0,
    _chatSidebarPointerLastAt: 0,
    _chatSidebarPointerVelocity: 0,
    _chatSidebarPointerMoveHandler: null,
    _chatSidebarPointerUpHandler: null,
    _sidebarToggleSuppressUntil: 0,
    chatMapDragActive: false,
    chatMapDragLeft: 0,
    chatMapDragTop: 0,
    chatMapPlacementX: infringReadStoredClampedRatio('infring-chat-map-placement-x', 1),
    chatMapPlacementY: infringReadStoredClampedRatio('infring-chat-map-placement-y', 0.38),
    chatMapWallLock: infringReadStoredWallLock('infring-chat-map-wall-lock', 'infring-chat-map-smash-wall'),
    _chatMapMoveDurationMs: 280,
    _chatMapPointerActive: false,
    _chatMapPointerMoved: false,
    _chatMapPointerStartX: 0,
    _chatMapPointerStartY: 0,
    _chatMapPointerOriginLeft: 0,
    _chatMapPointerOriginTop: 0,
    _chatMapPointerLastX: 0,
    _chatMapPointerLastY: 0,
    _chatMapPointerLastAt: 0,
    _chatMapPointerVelocity: 0,
    _chatMapPointerMoveHandler: null,
    _chatMapPointerUpHandler: null,
    bootSplashVisible: true,
    _bootSplashStartedAt: Date.now(),
    _bootSplashMinMs: 850,
    _bootSplashMaxMs: 5000,
    _bootSplashHideTimer: 0,
    _bootSplashMaxTimer: 0,
    bootProgressPercent: 6,
    bootProgressEvent: 'splash_visible',
    _bootProgressUpdatedAt: Date.now(),
    _taskbarRefreshOverlayTimer: 0,
    _taskbarRefreshReloadTimer: 0,
    taskbarHeroMenuOpen: false,
    taskbarTextMenuOpen: '',
    helpManualWindowOpen: false,
    reportIssueWindowOpen: false,
    reportIssueDraft: '',
    popupWindowPlacements: {
      manual: { left: null, top: null },
      report: { left: null, top: null }
    },
    popupWindowWallLocks: {
      manual: infringReadStoredWallLock('infring-popup-window-manual-wall-lock', 'infring-popup-window-manual-smash-wall'),
      report: infringReadStoredWallLock('infring-popup-window-report-wall-lock', 'infring-popup-window-report-smash-wall')
    },
    popupWindowDragActive: false,
    popupWindowDragKind: '',
    popupWindowDragLeft: 0,
    popupWindowDragTop: 0,
    popupWindowDragWallLock: '',
    _popupWindowMoveDurationMs: 260,
    _popupWindowPointerActive: false,
    _popupWindowPointerMoved: false,
    _popupWindowPointerStartX: 0,
    _popupWindowPointerStartY: 0,
    _popupWindowPointerOriginLeft: 0,
    _popupWindowPointerOriginTop: 0,
    _popupWindowPointerLastX: 0,
    _popupWindowPointerLastY: 0,
    _popupWindowPointerLastAt: 0,
    _popupWindowPointerVelocity: 0,
    _popupWindowPointerMoveHandler: null,
    _popupWindowPointerUpHandler: null,
    taskbarHeroActionPending: '',
    taskbarDockEdge: infringReadTaskbarDockEdgeDefault(),
    taskbarDockDragActive: false,
    taskbarDockDragY: 0,
    _taskbarDockPointerActive: false,
    _taskbarDockPointerMoved: false,
    _taskbarDockPointerStartX: 0,
    _taskbarDockPointerStartY: 0,
    _taskbarDockOriginY: 0,
    _taskbarDockPointerMoveHandler: null,
    _taskbarDockPointerUpHandler: null,
    taskbarReorderLeft: infringReadTaskbarReorderDefault('left'),
    taskbarReorderRight: infringReadTaskbarReorderDefault('right'),
    taskbarDragGroup: '',
    taskbarDragItem: '',
    taskbarDragStartOrder: [],
    _taskbarDragHoldTimer: 0,
    _taskbarDragHoldGroup: '',
    _taskbarDragHoldItem: '',
    _taskbarDragArmedGroup: '',
    _taskbarDragArmedItem: '',
    navBackStack: [],
    navForwardStack: [],
    _navCurrentPage: '',
    _navHistoryAction: '',
    _navHistoryCap: 48
  };
}

function infringBottomDockInitialState() {
  return {
    bottomDockOrder: infringReadBottomDockOrderDefault(),
    bottomDockTileConfig: infringReadBottomDockTileConfigDefault(),
    appsIconBottomRowColors: infringReadAppsIconBottomRowColorsDefault(),
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
    bottomDockPlacementId: infringReadBottomDockPlacementDefault(),
    bottomDockSnapPoints: infringBottomDockSnapPointsDefault(),
    bottomDockContainerDragActive: false,
    bottomDockContainerSettling: false,
    bottomDockContainerDragX: 0,
    bottomDockContainerDragY: 0,
    bottomDockContainerWallLock: infringReadBottomDockContainerWallLockDefault(),
    _bottomDockContainerDragWallLock: '',
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
    _bottomDockContainerSettleTimer: 0
  };
}
