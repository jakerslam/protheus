// Layer ownership: client/runtime/systems/ui (dashboard static UX surface only; no runtime authority).
function infringDashboardPopupDelegateMethods() {
  return {
    normalizeSidebarPopupText: function(rawText) {
      return infringNormalizeSidebarPopupText(this, rawText);
    },
    isSidebarPopupPlaceholderText: function(text) {
      return infringIsSidebarPopupPlaceholderText(text);
    },
    sidebarPopupMetaOrigin: function(preview, fallbackLabel) {
      return infringSidebarPopupMetaOrigin(preview, fallbackLabel);
    },
    hideDashboardPopupBySource: function(source) {
      infringHideDashboardPopupBySource(this, source);
    },
    showCollapsedSidebarAgentPopup: function(agent, ev) {
      infringShowCollapsedSidebarAgentPopup(this, agent, ev);
    },
    showCollapsedSidebarNavPopup: function(label, ev) {
      infringShowCollapsedSidebarNavPopup(this, label, ev);
    },
    dashboardPopupService: function() {
      return infringDashboardPopupService();
    },
    clearDashboardPopupState: function() {
      infringClearDashboardPopupState(this);
    },
    normalizeDashboardPopupSide: function(sideValue, fallbackSide) {
      return infringNormalizeDashboardPopupSide(this, sideValue, fallbackSide);
    },
    dashboardOppositeSide: function(sideValue) {
      return infringDashboardOppositeSide(this, sideValue);
    },
    dashboardPopupWallAffinity: function(rect) {
      var service = this.dashboardPopupService();
      if (service && typeof service.wallAffinity === 'function') {
        return service.wallAffinity(rect);
      }
      return infringDashboardPopupWallAffinity(rect);
    },
    dashboardPopupWallAnchorNode: function(node) {
      return infringDashboardPopupWallAnchorNode(node);
    },
    dashboardPopupWallRectForNode: function(node) {
      return infringDashboardPopupWallRectForNode(this, node);
    },
    dashboardPopupUsableAnchorRect: function(node) {
      return infringDashboardPopupUsableAnchorRect(node);
    },
    dashboardPopupSideAwayFromNearestWall: function(rect, fallbackSide) {
      return infringDashboardPopupSideAwayFromNearestWall(this, rect, fallbackSide);
    },
    dashboardPopupHorizontalAwayFromNearestWall: function(rect, fallbackSide) {
      return infringDashboardPopupHorizontalAwayFromNearestWall(this, rect, fallbackSide);
    },
    dashboardPopupVerticalAwayFromNearestWall: function(rect, fallbackSide) {
      return infringDashboardPopupVerticalAwayFromNearestWall(this, rect, fallbackSide);
    },
    dashboardPopupAxisAwareSideAway: function(rect, fallbackSide) {
      return infringDashboardPopupAxisAwareSideAway(this, rect, fallbackSide);
    },
    taskbarAnchoredDropdownClass: function(anchorNode, fallbackSide, layoutKey) {
      return infringTaskbarAnchoredDropdownClass(this, anchorNode, fallbackSide, layoutKey);
    },
    dashboardPopupAnchorPoint: function(ev, sideOverride) {
      return infringDashboardPopupAnchorPoint(this, ev, sideOverride);
    },
    showDashboardPopup: function(id, label, ev, overrides) {
      infringShowDashboardPopup(this, id, label, ev, overrides);
    },
    showTaskbarNavPopup: function(label, ev) {
      infringShowTaskbarNavPopup(this, label, ev);
    },
    showTaskbarUtilityPopup: function(label, body, ev) {
      infringShowTaskbarUtilityPopup(this, label, body, ev);
    },
    hideDashboardPopup: function(rawId) {
      infringHideDashboardPopup(this, rawId);
    },
    dashboardPopupOrigin: function(overrides) {
      return infringDashboardPopupOrigin(this, overrides);
    },
    bottomDockPopupOrigin: function() {
      return infringBottomDockPopupOrigin(this);
    },
    dashboardPopupStateOrigin: function() {
      return infringDashboardPopupStateOrigin(this);
    },
    activeDashboardPopupOrigin: function() {
      return infringActiveDashboardPopupOrigin(this);
    },
    isDashboardPopupVisible: function() {
      return infringIsDashboardPopupVisible(this);
    },
    dashboardPopupOverlayClass: function() {
      return infringDashboardPopupOverlayClass(this);
    },
    dashboardPopupOverlayStyle: function() {
      return infringDashboardPopupOverlayStyle(this);
    }
  };
}
