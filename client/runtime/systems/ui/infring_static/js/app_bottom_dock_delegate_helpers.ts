// Layer ownership: client/runtime/systems/ui (dashboard static UX surface only; no runtime authority).
function infringBottomDockDelegateMethods() {
  return {
    bottomDockDefaultOrder: function() {
      return infringBottomDockDefaultOrder(this);
    },
    bottomDockTileConfigById: function(id) {
      return infringBottomDockTileConfigById(this, id);
    },
    bottomDockTileData: function(id, field, fallback) {
      return infringBottomDockTileData(this, id, field, fallback);
    },
    bottomDockTileAnimationName: function(id) {
      return infringBottomDockTileAnimationName(this, id);
    },
    bottomDockTileAnimationDurationAttr: function(id) {
      return infringBottomDockTileAnimationDurationAttr(this, id);
    },
    bottomDockSlotStyle: function(id) {
      return infringBottomDockSlotStyle(this, id);
    },
    bottomDockTileStyle: function(id) {
      return infringBottomDockTileStyle(this, id);
    },
    normalizeBottomDockOrder: function(rawOrder) {
      return infringNormalizeBottomDockOrder(this, rawOrder);
    },
    persistBottomDockOrder: function() {
      infringPersistBottomDockOrder(this);
    },
    bottomDockOrderIndex: function(id) {
      return infringBottomDockOrderIndex(this, id);
    },
    bottomDockAxisBasis: function(sideHint) {
      return infringBottomDockAxisBasis(this, sideHint);
    },
    bottomDockProjectPointToAxis: function(x, y, basis) {
      return infringBottomDockProjectPointToAxis(this, x, y, basis);
    },
    bottomDockAxisHalfExtent: function(width, height, basis) {
      return infringBottomDockAxisHalfExtent(this, width, height, basis);
    },
    bottomDockProjectedRectBounds: function(rect, basis) {
      return infringBottomDockProjectedRectBounds(this, rect, basis);
    },
    bottomDockButtonRects: function() {
      return infringBottomDockButtonRects();
    },
    animateBottomDockFromRects: function(beforeRects) {
      infringAnimateBottomDockFromRects(this, beforeRects);
    },
    setBottomDockHover: function(id, ev) {
      infringSetBottomDockHover(this, id, ev);
    },
    clearBottomDockHover: function(id) {
      infringClearBottomDockHover(this, id);
    },
    readBottomDockSlotCenters: function() {
      return infringReadBottomDockSlotCenters();
    },
    bottomDockWeightForDistance: function(distancePx) {
      return infringBottomDockWeightForDistance(distancePx);
    },
    refreshBottomDockHoverWeights: function() {
      infringRefreshBottomDockHoverWeights(this);
    },
    updateBottomDockPointer: function(ev) {
      infringUpdateBottomDockPointer(this, ev);
    },
    reviveBottomDockHoverFromPoint: function(clientX, clientY) {
      infringReviveBottomDockHoverFromPoint(this, clientX, clientY);
    },
    scheduleBottomDockPreviewReflow: function() {
      infringScheduleBottomDockPreviewReflow(this);
    },
    cancelBottomDockPreviewReflow: function() {
      infringCancelBottomDockPreviewReflow(this);
    },
    syncBottomDockPreview: function() {
      infringSyncBottomDockPreview(this);
    },
    bindBottomDockPointerListeners: function() {
      infringBindBottomDockPointerListeners(this);
    },
    unbindBottomDockPointerListeners: function() {
      infringUnbindBottomDockPointerListeners(this);
    },
    startBottomDockPointerDrag: function(id, ev) {
      infringStartBottomDockPointerDrag(this, id, ev);
    },
    activateBottomDockPointerDrag: function(ev) {
      infringActivateBottomDockPointerDrag(this, ev);
    },
    handleBottomDockPointerMove: function(ev) {
      infringHandleBottomDockPointerMove(this, ev);
    },
    endBottomDockPointerDrag: function() {
      infringEndBottomDockPointerDrag(this);
    },
    shouldSuppressBottomDockClick: function() {
      return infringShouldSuppressBottomDockClick(this);
    },
    clearBottomDockClickAnimation: function() {
      infringClearBottomDockClickAnimation(this);
    },
    triggerBottomDockClickAnimation: function(id, durationOverrideMs) {
      infringTriggerBottomDockClickAnimation(this, id, durationOverrideMs);
    },
    bottomDockIsClickAnimating: function(id) {
      return infringBottomDockIsClickAnimating(this, id);
    },
    handleBottomDockTileClick: function(id, targetPage, ev) {
      infringHandleBottomDockTileClick(this, id, targetPage, ev);
    },
    bottomDockIsDraggingVisual: function(id) {
      return infringBottomDockIsDraggingVisual(this, id);
    },
    bottomDockIsNeighbor: function(id) {
      return infringBottomDockIsNeighbor(this, id);
    },
    bottomDockIsSecondNeighbor: function(id) {
      return infringBottomDockIsSecondNeighbor(this, id);
    },
    bottomDockHoverWeight: function(id) {
      return infringBottomDockHoverWeight(this, id);
    },
    startBottomDockDrag: function(id, ev) {
      infringStartBottomDockDrag(this, id, ev);
    },
    bottomDockShouldInsertAfter: function(targetId, ev, targetEl) {
      return infringBottomDockShouldInsertAfter(this, targetId, ev, targetEl);
    },
    captureBottomDockDragBoundaries: function(dragId) {
      return infringCaptureBottomDockDragBoundaries(this, dragId);
    },
    bottomDockAppendTargetId: function(dragId) {
      return infringBottomDockAppendTargetId(this, dragId);
    },
    bottomDockShouldAppendFromPointer: function(dragId, ev) {
      return infringBottomDockShouldAppendFromPointer(this, dragId, ev);
    },
    bottomDockInsertionIndexFromCoords: function(dragId, clientXRaw, clientYRaw) {
      return infringBottomDockInsertionIndexFromCoords(this, dragId, clientXRaw, clientYRaw);
    },
    bottomDockGhostCenterPoint: function() {
      return infringBottomDockGhostCenterPoint(this);
    },
    bottomDockInsertionIndexFromPointer: function(dragId, ev) {
      return infringBottomDockInsertionIndexFromPointer(this, dragId, ev);
    },
    applyBottomDockReorderByIndex: function(dragId, insertionIndex, animate) {
      return infringApplyBottomDockReorderByIndex(this, dragId, insertionIndex, animate);
    },
    persistBottomDockOrderIfChangedFromDragStart: function() {
      infringPersistBottomDockOrderIfChangedFromDragStart(this);
    },
    completeBottomDockDropCleanup: function(ev) {
      infringCompleteBottomDockDropCleanup(this, ev);
    },
    handleBottomDockContainerDragOver: function(ev) {
      infringHandleBottomDockContainerDragOver(this, ev);
    },
    handleBottomDockContainerDrop: function(ev) {
      infringHandleBottomDockContainerDrop(this, ev);
    },
    handleBottomDockDragOver: function(id, ev, preferAfter) {
      infringHandleBottomDockDragOver(this, id, ev, preferAfter);
    },
    handleBottomDockDrop: function(id, ev, preferAfter) {
      infringHandleBottomDockDrop(this, id, ev, preferAfter);
    },
    endBottomDockDrag: function() {
      infringEndBottomDockDrag(this);
    }
  };
}
