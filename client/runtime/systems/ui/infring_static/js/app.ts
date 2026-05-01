// Canonical Shell source-of-truth: assembled runtime app surface.
// Decomposition debt lives under ./app.ts.parts/** and must not count as additive production source.
// Shared rendering helpers split out to keep dashboard part files under size caps.


// Infring App — legacy reactive init, hash router, global store
'use strict';



infringRegisterAppStoreOnAlpineInit();

// Main app component
function app() {
  return {
    ...infringAppInitialState(),

    appsIconBottomRowFill(index) {
      return infringAppsIconBottomRowFill(this, index);
    },

    chatSidebarFlipDurationMs() {
      return infringChatSidebarFlipDurationMs(this);
    },

    readChatSidebarSnapshot() {
      return infringReadChatSidebarSnapshot(this);
    },

    animateChatSidebarFromSnapshot(snapshot) {
      infringAnimateChatSidebarFromSnapshot(this, snapshot);
    },

    maybeAnimateChatSidebarRows() {
      infringMaybeAnimateChatSidebarRows(this);
    },

    cleanupBottomDockDragGhost() {
      infringCleanupBottomDockDragGhost(this);
    },

    setBottomDockGhostTarget(x, y) {
      infringSetBottomDockGhostTarget(this, x, y);
    },

    dragbarService() {
      return infringDragbarService();
    },

    taskbarDockService() {
      return infringTaskbarDockSharedService();
    },

    dragSurfaceMoveDurationMs(rawValue, fallbackMs) {
      return infringDragSurfaceMoveDurationMs(this, rawValue, fallbackMs);
    },

    readBottomDockScale(el) {
      return infringReadBottomDockScale(el);
    },

    bootProgressClamped(rawPercent) {
      return infringBootProgressClamped(rawPercent);
    },

    resetBootProgress() {
      infringResetBootProgress(this);
    },

    bootProgressFromBootStage(rawStage) {
      return infringBootProgressFromBootStage(rawStage);
    },

    setBootProgressPercent(rawPercent, opts) {
      infringSetBootProgressPercent(this, rawPercent, opts);
    },

    setBootProgressEvent(eventName, meta) {
      infringSetBootProgressEvent(this, eventName, meta);
    },
    normalizeConnectionIndicatorState(state) {
      return infringNormalizeConnectionIndicatorState(state);
    },

    queueConnectionIndicatorState(state) {
      infringQueueConnectionIndicatorState(this, state);
    },

    _computeScrollHintState(el) {
      return infringComputeScrollHintState(el);
    },

    ...infringBottomDockInitialState(),

    bottomDockMoveDurationMs() {
      return infringBottomDockMoveDurationMs(this);
    },

    bottomDockExpandedScale() {
      return infringBottomDockExpandedScale(this);
    },

    bottomDockReadViewportSize() {
      return infringBottomDockReadViewportSize();
    },

    bottomDockReadBaseSize() {
      return infringBottomDockReadBaseSize();
    },

    bottomDockNormalizeSide(side) {
      return infringBottomDockNormalizeSide(side);
    },

    bottomDockIsVerticalSide(side) {
      return infringBottomDockIsVerticalSide(side);
    },

    bottomDockRotationDegForSide(side) {
      return infringBottomDockRotationDegForSide(side);
    },

    bottomDockIconRotationDegForSide(side) {
      return infringBottomDockIconRotationDegForSide(side);
    },

    bottomDockUpDegForSide(side) {
      return infringBottomDockUpDegForSide(side);
    },

    bottomDockOrientation(sideHint) {
      return infringBottomDockOrientation(this, sideHint);
    },

    bottomDockOppositeSide(sideHint) {
      return infringBottomDockOppositeSide(sideHint);
    },

    bottomDockWallSide() {
      return infringBottomDockWallSide(this);
    },

    bottomDockOpenSide() {
      return infringBottomDockOpenSide(this);
    },

    bottomDockRotationDegResolved(sideHint) {
      return infringBottomDockRotationDegResolved(this, sideHint);
    },

    bottomDockScreenDeltaToLocal(dx, dy, sideHint) {
      return infringBottomDockScreenDeltaToLocal(this, dx, dy, sideHint);
    },

    bottomDockCanonicalRotationCandidatesForSide(side) {
      return infringBottomDockCanonicalRotationCandidatesForSide(side);
    },

    bottomDockNormalizeRotationDeg(value) {
      return infringBottomDockNormalizeRotationDeg(value);
    },

    bottomDockResolveShortestRotationDeg(currentDeg, targetDeg) {
      return infringBottomDockResolveShortestRotationDeg(currentDeg, targetDeg);
    },

    bottomDockPreferredRotationDirectionForAnchor(anchorX, anchorY) {
      return infringBottomDockPreferredRotationDirectionForAnchor(anchorX, anchorY);
    },

    bottomDockResolveDirectionalRotationDeg(currentDeg, targetDeg, direction) {
      return infringBottomDockResolveDirectionalRotationDeg(currentDeg, targetDeg, direction);
    },

    bottomDockResolveRotationForSide(side, anchorX, anchorY) {
      return infringBottomDockResolveRotationForSide(this, side, anchorX, anchorY);
    },

    bottomDockSnapDefinitions() {
      return infringBottomDockSnapDefinitions(this);
    },

    bottomDockSnapDefinitionById(id) {
      return infringBottomDockSnapDefinitionById(this, id);
    },

    bottomDockSideForSnapId(id) {
      return infringBottomDockSideForSnapId(this, id);
    },

    bottomDockActiveSnapId() {
      return infringBottomDockActiveSnapId(this);
    },

    bottomDockActiveSide() {
      return infringBottomDockActiveSide(this);
    },

    bottomDockWallLockNormalized() {
      return infringBottomDockWallLockNormalized(this);
    },

    bottomDockTaskbarContained() {
      return infringBottomDockTaskbarContained(this);
    },

    bottomDockHoverExpansionDisabled() {
      return infringBottomDockHoverExpansionDisabled(this);
    },

    bottomDockTaskbarContainedAnchorX(sideHint) {
      return infringBottomDockTaskbarContainedAnchorX(this, sideHint);
    },

    bottomDockTaskbarContainedMetrics() {
      return infringBottomDockTaskbarContainedMetrics(this);
    },

    bottomDockSetWallLock(wallRaw) {
      return infringBottomDockSetWallLock(this, wallRaw);
    },

    bottomDockBoundsScaleForSide(sideHint) {
      return infringBottomDockBoundsScaleForSide(this, sideHint);
    },

    bottomDockVisualSizeForSide(sideHint) {
      return infringBottomDockVisualSizeForSide(this, sideHint);
    },

    bottomDockHardBoundsForSide(sideHint) {
      return infringBottomDockHardBoundsForSide(this, sideHint);
    },

    bottomDockTopLeftFromAnchor(anchorX, anchorY, sideHint) {
      return infringBottomDockTopLeftFromAnchor(this, anchorX, anchorY, sideHint);
    },

    bottomDockAnchorFromTopLeft(leftRaw, topRaw, sideHint) {
      return infringBottomDockAnchorFromTopLeft(this, leftRaw, topRaw, sideHint);
    },

    bottomDockLocalWallForRotation(wallRaw, rotationDegRaw) {
      return infringBottomDockLocalWallForRotation(this, wallRaw, rotationDegRaw);
    },

    bottomDockLockRadiusCssVars(wallRaw, rotationDegRaw) {
      return infringBottomDockLockRadiusCssVars(this, wallRaw, rotationDegRaw);
    },

    bottomDockClampDragAnchor(anchorX, anchorY) {
      return infringBottomDockClampDragAnchor(anchorX, anchorY);
    },

    bottomDockClampAnchor(anchorX, anchorY, sideOverride) {
      void sideOverride;
      return infringBottomDockClampDragAnchor(anchorX, anchorY);
    },

    bottomDockAnchorForSnapId(id) {
      return infringBottomDockAnchorForSnapId(this, id);
    },

    bottomDockNearestSnapId(anchorX, anchorY) {
      return infringBottomDockNearestSnapId(this, anchorX, anchorY);
    },

    persistBottomDockPlacement() {
      return infringPersistBottomDockPlacement(this);
    },

    syncDragWallCapHostNode(node, wallRaw) {
      infringSyncDragWallCapHostNode(this, node, wallRaw);
    },

    syncDragWallCaps() {
      infringSyncDragWallCaps(this);
    },

    bottomDockContainerStyle() {
      return infringBottomDockContainerStyle(this);
    },

    bindBottomDockContainerPointerListeners() {
      infringBindBottomDockContainerPointerListeners(this);
    },

    unbindBottomDockContainerPointerListeners() {
      infringUnbindBottomDockContainerPointerListeners(this);
    },

    startBottomDockContainerPointerDrag(ev) {
      infringStartBottomDockContainerPointerDrag(this, ev);
    },

    handleBottomDockContainerPointerMove(ev) {
      infringHandleBottomDockContainerPointerMove(this, ev);
    },

    endBottomDockContainerPointerDrag() {
      infringEndBottomDockContainerPointerDrag(this);
    },

    settleBottomDockDragGhost(dragId, done) {
      infringSettleBottomDockDragGhost(this, dragId, done);
    },

    ...infringTaskbarDockDelegateMethods(),

    overlayWallGapPx() {
      return infringOverlayWallGapPx();
    },

    chatOverlayVerticalBounds() {
      return infringChatOverlayVerticalBounds(this);
    },

    dragSurfaceHardBounds(widthRaw, heightRaw, ignoreTaskbarBoundaryRaw) {
      return infringDragSurfaceHardBounds(this, widthRaw, heightRaw, ignoreTaskbarBoundaryRaw);
    },

    dragSurfaceSoftBounds(widthRaw, heightRaw) {
      return infringDragSurfaceSoftBounds(this, widthRaw, heightRaw);
    },

    dragSurfaceClampWithBounds(bounds, leftRaw, topRaw) {
      return infringDragSurfaceClampWithBounds(this, bounds, leftRaw, topRaw);
    },

    dragSurfaceNearestWall(bounds, leftRaw, topRaw) {
      return infringDragSurfaceNearestWall(this, bounds, leftRaw, topRaw);
    },

    dragSurfaceNormalizeWall(wallRaw) {
      return infringDragSurfaceNormalizeWall(this, wallRaw);
    },

    dragSurfaceApplyWallLock(bounds, leftRaw, topRaw, wallRaw) {
      return infringDragSurfaceApplyWallLock(this, bounds, leftRaw, topRaw, wallRaw);
    },

    dragSurfaceDistanceFromWall(bounds, leftRaw, topRaw, wallRaw) {
      return infringDragSurfaceDistanceFromWall(this, bounds, leftRaw, topRaw, wallRaw);
    },

    dragSurfaceWallLockOvershoot(bounds, leftRaw, topRaw, wallRaw) {
      return infringDragSurfaceWallLockOvershoot(this, bounds, leftRaw, topRaw, wallRaw);
    },

    dragSurfaceCenteredPoint(bounds) {
      return infringDragSurfaceCenteredPoint(this, bounds);
    },

    dragSurfaceWallLockContactThreshold() {
      return infringDragSurfaceWallLockContactThreshold(this);
    },
    dragSurfaceWallLockDistanceThreshold() {
      return infringDragSurfaceWallLockDistanceThreshold(this);
    },
    dragSurfaceWallUnlockDistanceThreshold() {
      return infringDragSurfaceWallUnlockDistanceThreshold(this);
    },
    dragSurfaceWallLockOvershootThreshold() {
      return infringDragSurfaceWallLockOvershootThreshold(this);
    },
    dragSurfaceResolveWallLock(bounds, candidateLeft, candidateTop, nearest, motionDxRaw, motionDyRaw) {
      return infringDragSurfaceResolveWallLock(this, bounds, candidateLeft, candidateTop, nearest, motionDxRaw, motionDyRaw);
    },

    dragSurfaceRadiusByWall(wallRaw) {
      return infringDragSurfaceRadiusByWall(this, wallRaw);
    },

    dragSurfaceLockTransformTimeMs(rawValue) {
      return infringDragSurfaceLockTransformTimeMs(this, rawValue);
    },

    dragSurfaceLockBorderFadeDurationMs(transformMsRaw) {
      return infringDragSurfaceLockBorderFadeDurationMs(this, transformMsRaw);
    },

    dragSurfaceVisualStateStore() {
      return infringDragSurfaceVisualStateStore(this);
    },

    dragSurfaceLockVisualCssVars(surfaceKeyRaw, wallRaw, optionsRaw) {
      return infringDragSurfaceLockVisualCssVars(this, surfaceKeyRaw, wallRaw, optionsRaw);
    },

    dragSurfaceLockRadiusCssVars(wallRaw) {
      return infringDragSurfaceLockRadiusCssVars(this, wallRaw);
    },

    ...infringChatMapDelegateMethods(),

    ...infringChatSidebarPlacementDelegateMethods(),

    ...infringPopupWindowDelegateMethods(),

    ...infringBottomDockDelegateMethods(),

    ...infringDashboardPopupDelegateMethods(),

    ...infringChatSidebarDelegateMethods(),

    get agents() {
      return infringReadAppStoreAgents(this);
    },
    get chatSidebarAgents() {
      return infringChatSidebarAgents(this);
    },
    get chatSidebarRows() {
      return infringChatSidebarRows(this);
    },
    get chatSidebarVirtualized() {
      return infringChatSidebarVirtualized(this);
    },
    get chatSidebarVirtualPadTop() {
      return infringChatSidebarVirtualPadTop(this);
    },
    get chatSidebarVirtualPadBottom() {
      return infringChatSidebarVirtualPadBottom(this);
    },
    get chatSidebarVisibleRows() {
      return infringChatSidebarVisibleRows(this);
    },
    ...infringShellDelegateMethods()
  };
}
