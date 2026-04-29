function infringDashboardPopupOrigin(page, overrides) {
  var service = page.dashboardPopupService();
  if (service && typeof service.origin === 'function') {
    return service.origin(overrides);
  }
  return Object.assign({
    source: '',
    active: false,
    ready: false,
    side: 'top',
    inline_away: 'right',
    block_away: 'bottom',
    left: 0,
    top: 0,
    compact: false,
    title: '',
    body: '',
    meta_origin: '',
    meta_time: '',
    unread: false
  }, overrides || {});
}

function infringBottomDockPopupOrigin(page) {
  var label = String(page.bottomDockPreviewText || '').trim();
  var left = Math.round(Number(page.bottomDockPreviewX || 0));
  var top = Math.round(Number(page.bottomDockPreviewY || 0));
  if (!page.bottomDockPreviewVisible || !label) return page.dashboardPopupOrigin();
  return page.dashboardPopupOrigin({
    source: 'bottom_dock',
    active: true,
    ready: left > 0 && top > 0,
    side: page.bottomDockOpenSide(),
    inline_away: 'center',
    block_away: 'center',
    left: left,
    top: top,
    compact: false,
    title: label
  });
}

function infringDashboardPopupStateOrigin(page) {
  var service = page.dashboardPopupService();
  if (service && typeof service.stateOrigin === 'function') {
    return service.stateOrigin(page.dashboardPopup);
  }
  var popup = page.dashboardPopup || {};
  var title = String(popup.title || '').trim();
  var body = String(popup.body || '').trim();
  var left = Math.round(Number(popup.left || 0));
  var top = Math.round(Number(popup.top || 0));
  var side = String(popup.side || 'bottom').trim().toLowerCase();
  var inlineAway = String(popup.inline_away || 'right').trim().toLowerCase();
  var blockAway = String(popup.block_away || 'bottom').trim().toLowerCase();
  if (side !== 'top' && side !== 'left' && side !== 'right') side = 'bottom';
  if (inlineAway !== 'left' && inlineAway !== 'right') inlineAway = 'center';
  if (blockAway !== 'top' && blockAway !== 'bottom') blockAway = 'center';
  if (!popup.active || !title) return page.dashboardPopupOrigin();
  return page.dashboardPopupOrigin({
    source: String(popup.source || 'ui').trim(),
    active: true,
    ready: left > 0 && top > 0,
    side: side,
    inline_away: inlineAway,
    block_away: blockAway,
    left: left,
    top: top,
    compact: false,
    title: title,
    body: body,
    meta_origin: String(popup.meta_origin || '').trim(),
    meta_time: String(popup.meta_time || '').trim(),
    unread: !!popup.unread
  });
}

function infringActiveDashboardPopupOrigin(page) {
  var sharedPopup = page.dashboardPopupStateOrigin();
  if (sharedPopup.active && sharedPopup.ready) return sharedPopup;
  var dockPopup = page.bottomDockPopupOrigin();
  if (dockPopup.active && dockPopup.ready) return dockPopup;
  return page.dashboardPopupOrigin();
}

function infringIsDashboardPopupVisible(page) {
  var popup = page.activeDashboardPopupOrigin();
  return !!(popup.active && popup.ready && popup.title);
}

function infringDashboardPopupOverlayClass(page) {
  var popup = page.activeDashboardPopupOrigin();
  var service = page.dashboardPopupService();
  if (service && typeof service.overlayClass === 'function') {
    return service.overlayClass(popup, 'fogged-glass');
  }
  return {
    'is-visible': !!(popup.active && popup.ready && popup.title),
    'is-side-top': popup.side === 'top',
    'is-side-bottom': popup.side === 'bottom',
    'is-side-left': popup.side === 'left',
    'is-side-right': popup.side === 'right',
    'is-inline-away-left': popup.inline_away === 'left',
    'is-inline-away-right': popup.inline_away === 'right',
    'is-inline-away-center': popup.inline_away !== 'left' && popup.inline_away !== 'right',
    'is-block-away-top': popup.block_away === 'top',
    'is-block-away-bottom': popup.block_away === 'bottom',
    'is-block-away-center': popup.block_away !== 'top' && popup.block_away !== 'bottom',
    'is-unread': !!popup.unread
  };
}

function infringDashboardPopupOverlayStyle(page) {
  var popup = page.activeDashboardPopupOrigin();
  var service = page.dashboardPopupService();
  if (service && typeof service.overlayStyle === 'function') {
    return service.overlayStyle(popup);
  }
  if (!popup.active || !popup.ready) return 'left:-9999px;top:-9999px;';
  return 'left:' + Math.round(Number(popup.left || 0)) + 'px;top:' + Math.round(Number(popup.top || 0)) + 'px;';
}
