// Layer ownership: client/runtime/systems/ui (dashboard static UX surface only; no runtime authority).
function infringPopupWindowStorageKey(kind, axis) {
  var key = String(kind || '').trim().toLowerCase();
  var lane = String(axis || '').trim().toLowerCase() === 'top' ? 'top' : 'left';
  return 'infring-popup-window-' + (key || 'manual') + '-' + lane;
}

function infringPopupWindowWallLockStorageKey(kind) {
  var key = String(kind || '').trim().toLowerCase() || 'manual';
  return 'infring-popup-window-' + key + '-wall-lock';
}

function infringPopupWindowWallLock(kind) {
  void kind;
  return '';
}

function infringPopupWindowSetWallLock(page, kind, wallRaw) {
  var target = page && typeof page === 'object' ? page : {};
  var key = String(kind || '').trim().toLowerCase();
  void wallRaw;
  if (!key) return '';
  if (!target.popupWindowWallLocks || typeof target.popupWindowWallLocks !== 'object') {
    target.popupWindowWallLocks = {};
  }
  target.popupWindowWallLocks[key] = '';
  try {
    localStorage.removeItem(target.popupWindowWallLockStorageKey(key));
    localStorage.removeItem('infring-popup-window-' + key + '-smash-wall');
  } catch(_) {}
  return '';
}

function infringPopupWindowOpenState(page, kind) {
  var target = page && typeof page === 'object' ? page : {};
  var key = String(kind || '').trim().toLowerCase();
  if (key === 'report') return !!target.reportIssueWindowOpen;
  return !!target.helpManualWindowOpen;
}

function infringPopupWindowSetOpenState(page, kind, open) {
  var target = page && typeof page === 'object' ? page : {};
  var key = String(kind || '').trim().toLowerCase();
  var nextOpen = open !== false;
  if (key === 'report') {
    target.reportIssueWindowOpen = nextOpen;
    return;
  }
  target.helpManualWindowOpen = nextOpen;
}

function infringReadPopupWindowElement(kind) {
  if (typeof document === 'undefined' || typeof document.querySelector !== 'function') return null;
  var key = String(kind || '').trim().toLowerCase();
  if (!key) return null;
  try {
    return document.querySelector('.popup-window[data-popup-window-kind="' + key + '"]');
  } catch(_) {}
  return null;
}

function infringPopupWindowDefaultSize(kind) {
  var key = String(kind || '').trim().toLowerCase();
  if (key === 'report') return { width: 540, height: 360 };
  return { width: 760, height: 560 };
}

function infringReadPopupWindowSize(page, kind) {
  var target = page && typeof page === 'object' ? page : {};
  var node = target.readPopupWindowElement(kind);
  var fallback = target.popupWindowDefaultSize(kind);
  var width = Number(node && node.offsetWidth || 0);
  var height = Number(node && node.offsetHeight || 0);
  if (!Number.isFinite(width) || width <= 0) width = Number(fallback.width || 640);
  if (!Number.isFinite(height) || height <= 0) height = Number(fallback.height || 420);
  return {
    width: Math.max(280, Math.round(width)),
    height: Math.max(180, Math.round(height))
  };
}

function infringPopupWindowBounds(page, kind, widthRaw, heightRaw) {
  var target = page && typeof page === 'object' ? page : {};
  void kind;
  var wallGap = target.overlayWallGapPx();
  var width = Number(widthRaw || 0);
  var height = Number(heightRaw || 0);
  if (!Number.isFinite(width) || width <= 0) width = 640;
  if (!Number.isFinite(height) || height <= 0) height = 420;
  var minLeft = wallGap;
  var maxLeft = target.chatOverlayViewportWidth() - wallGap - width;
  if (!Number.isFinite(maxLeft) || maxLeft < minLeft) maxLeft = minLeft;
  var vertical = target.chatOverlayVerticalBounds();
  var minTop = Number(vertical && vertical.minTop || wallGap) + 2;
  var maxTop = Number(vertical && vertical.maxBottom || target.taskbarReadViewportHeight()) - wallGap - height;
  if (!Number.isFinite(maxTop) || maxTop < minTop) maxTop = minTop;
  return {
    minLeft: minLeft,
    maxLeft: maxLeft,
    minTop: minTop,
    maxTop: maxTop
  };
}

function infringPopupWindowClampPlacement(page, kind, leftRaw, topRaw) {
  var target = page && typeof page === 'object' ? page : {};
  var size = target.readPopupWindowSize(kind);
  var bounds = target.popupWindowBounds(kind, size.width, size.height);
  var left = Number(leftRaw);
  var top = Number(topRaw);
  if (!Number.isFinite(left)) left = bounds.minLeft + ((bounds.maxLeft - bounds.minLeft) * 0.5);
  if (!Number.isFinite(top)) top = bounds.minTop + ((bounds.maxTop - bounds.minTop) * 0.48);
  return {
    left: Math.max(bounds.minLeft, Math.min(bounds.maxLeft, left)),
    top: Math.max(bounds.minTop, Math.min(bounds.maxTop, top))
  };
}

function infringPopupWindowHardBounds(page, kind) {
  var target = page && typeof page === 'object' ? page : {};
  var size = target.readPopupWindowSize(kind);
  return target.dragSurfaceHardBounds(size.width, size.height);
}

function infringPopupWindowEnsurePlacement(page, kind, forceCenter) {
  var target = page && typeof page === 'object' ? page : {};
  var key = String(kind || '').trim().toLowerCase();
  if (!key) return { left: 0, top: 0 };
  if (forceCenter) {
    var centerSize = target.readPopupWindowSize(key);
    var centerBounds = target.popupWindowBounds(key, centerSize.width, centerSize.height);
    var centerPoint = target.dragSurfaceCenteredPoint(centerBounds);
    var centered = target.popupWindowClampPlacement(key, centerPoint.left, centerPoint.top);
    if (!target.popupWindowPlacements || typeof target.popupWindowPlacements !== 'object') {
      target.popupWindowPlacements = {};
    }
    target.popupWindowPlacements[key] = { left: centered.left, top: centered.top };
    return centered;
  }
  var map = (target.popupWindowPlacements && typeof target.popupWindowPlacements === 'object')
    ? target.popupWindowPlacements
    : {};
  var row = map[key] && typeof map[key] === 'object' ? map[key] : { left: null, top: null };
  var left = Number(row.left);
  var top = Number(row.top);
  var hasStored = Number.isFinite(left) && Number.isFinite(top);
  if (!hasStored) {
    try {
      left = Number(localStorage.getItem(target.popupWindowStorageKey(key, 'left')));
      top = Number(localStorage.getItem(target.popupWindowStorageKey(key, 'top')));
    } catch(_) {}
  }
  if (!Number.isFinite(left) || !Number.isFinite(top)) {
    var size = target.readPopupWindowSize(key);
    var bounds = target.popupWindowBounds(key, size.width, size.height);
    left = bounds.minLeft + ((bounds.maxLeft - bounds.minLeft) * 0.5);
    top = bounds.minTop + ((bounds.maxTop - bounds.minTop) * (key === 'report' ? 0.56 : 0.44));
  }
  var clamped = target.popupWindowClampPlacement(key, left, top);
  if (!target.popupWindowPlacements || typeof target.popupWindowPlacements !== 'object') {
    target.popupWindowPlacements = {};
  }
  target.popupWindowPlacements[key] = { left: clamped.left, top: clamped.top };
  return clamped;
}

function infringPopupWindowPersistPlacement(page, kind, leftRaw, topRaw) {
  var target = page && typeof page === 'object' ? page : {};
  var key = String(kind || '').trim().toLowerCase();
  if (!key) return;
  var clamped = target.popupWindowClampPlacement(key, leftRaw, topRaw);
  if (!target.popupWindowPlacements || typeof target.popupWindowPlacements !== 'object') {
    target.popupWindowPlacements = {};
  }
  target.popupWindowPlacements[key] = { left: clamped.left, top: clamped.top };
  try {
    localStorage.setItem(target.popupWindowStorageKey(key, 'left'), String(clamped.left));
    localStorage.setItem(target.popupWindowStorageKey(key, 'top'), String(clamped.top));
  } catch(_) {}
}

function infringPopupWindowResolvedLeft(page, kind) {
  var target = page && typeof page === 'object' ? page : {};
  var key = String(kind || '').trim().toLowerCase();
  if (!key) return target.overlayWallGapPx();
  if (target.popupWindowDragActive && target.popupWindowDragKind === key) {
    return Number(target.popupWindowDragLeft || 0);
  }
  var base = target.popupWindowEnsurePlacement(key);
  return target.popupWindowClampPlacement(key, base.left, base.top).left;
}

function infringPopupWindowResolvedTop(page, kind) {
  var target = page && typeof page === 'object' ? page : {};
  var key = String(kind || '').trim().toLowerCase();
  if (!key) return target.overlayWallGapPx();
  if (target.popupWindowDragActive && target.popupWindowDragKind === key) {
    return Number(target.popupWindowDragTop || 0);
  }
  var base = target.popupWindowEnsurePlacement(key);
  return target.popupWindowClampPlacement(key, base.left, base.top).top;
}

function infringPopupWindowStyle(page, kind) {
  var target = page && typeof page === 'object' ? page : {};
  var key = String(kind || '').trim().toLowerCase();
  if (!key || !target.popupWindowOpenState(key)) return 'display:none;';
  var left = target.popupWindowResolvedLeft(key);
  var top = target.popupWindowResolvedTop(key);
  var durationMs = (target.popupWindowDragActive && target.popupWindowDragKind === key)
    ? 0
    : target.dragSurfaceMoveDurationMs(target._popupWindowMoveDurationMs, 260);
  return (
    'left:' + Math.round(left) + 'px;' +
    'top:' + Math.round(top) + 'px;' +
    'transition:left ' + Math.max(0, Math.round(durationMs)) + 'ms var(--ease-smooth), top ' + Math.max(0, Math.round(durationMs)) + 'ms var(--ease-smooth);'
  );
}

function infringOpenPopupWindow(page, kind) {
  var target = page && typeof page === 'object' ? page : {};
  var key = String(kind || '').trim().toLowerCase();
  if (!key) return;
  target.popupWindowSetOpenState(key, true);
  target.popupWindowSetWallLock(key, '');
  target.popupWindowEnsurePlacement(key, true);
  target.$nextTick(function() {
    target.popupWindowEnsurePlacement(key, true);
  });
}

function infringClosePopupWindow(page, kind) {
  var target = page && typeof page === 'object' ? page : {};
  var key = String(kind || '').trim().toLowerCase();
  if (!key) return;
  if (target._popupWindowPointerActive && target.popupWindowDragKind === key) {
    target.endPopupWindowPointerDrag();
  }
  target.popupWindowSetOpenState(key, false);
}

function infringPopupWindowDelegateMethods() {
  return {
    popupWindowStorageKey: function(kind, axis) {
      return infringPopupWindowStorageKey(kind, axis);
    },
    popupWindowWallLockStorageKey: function(kind) {
      return infringPopupWindowWallLockStorageKey(kind);
    },
    popupWindowWallLock: function(kind) {
      return infringPopupWindowWallLock(kind);
    },
    popupWindowSetWallLock: function(kind, wallRaw) {
      return infringPopupWindowSetWallLock(this, kind, wallRaw);
    },
    popupWindowOpenState: function(kind) {
      return infringPopupWindowOpenState(this, kind);
    },
    popupWindowSetOpenState: function(kind, open) {
      infringPopupWindowSetOpenState(this, kind, open);
    },
    readPopupWindowElement: function(kind) {
      return infringReadPopupWindowElement(kind);
    },
    popupWindowDefaultSize: function(kind) {
      return infringPopupWindowDefaultSize(kind);
    },
    readPopupWindowSize: function(kind) {
      return infringReadPopupWindowSize(this, kind);
    },
    popupWindowBounds: function(kind, widthRaw, heightRaw) {
      return infringPopupWindowBounds(this, kind, widthRaw, heightRaw);
    },
    popupWindowClampPlacement: function(kind, leftRaw, topRaw) {
      return infringPopupWindowClampPlacement(this, kind, leftRaw, topRaw);
    },
    popupWindowHardBounds: function(kind) {
      return infringPopupWindowHardBounds(this, kind);
    },
    popupWindowEnsurePlacement: function(kind, forceCenter) {
      return infringPopupWindowEnsurePlacement(this, kind, forceCenter);
    },
    popupWindowPersistPlacement: function(kind, leftRaw, topRaw) {
      infringPopupWindowPersistPlacement(this, kind, leftRaw, topRaw);
    },
    popupWindowResolvedLeft: function(kind) {
      return infringPopupWindowResolvedLeft(this, kind);
    },
    popupWindowResolvedTop: function(kind) {
      return infringPopupWindowResolvedTop(this, kind);
    },
    popupWindowStyle: function(kind) {
      return infringPopupWindowStyle(this, kind);
    },
    openPopupWindow: function(kind) {
      infringOpenPopupWindow(this, kind);
    },
    closePopupWindow: function(kind) {
      infringClosePopupWindow(this, kind);
    },
    bindPopupWindowPointerListeners: function() {
      infringBindPopupWindowPointerListeners(this);
    },
    unbindPopupWindowPointerListeners: function() {
      infringUnbindPopupWindowPointerListeners(this);
    },
    startPopupWindowPointerDrag: function(kind, ev) {
      infringStartPopupWindowPointerDrag(this, kind, ev);
    },
    handlePopupWindowPointerMove: function(ev) {
      infringHandlePopupWindowPointerMove(this, ev);
    },
    endPopupWindowPointerDrag: function() {
      infringEndPopupWindowPointerDrag(this);
    }
  };
}
