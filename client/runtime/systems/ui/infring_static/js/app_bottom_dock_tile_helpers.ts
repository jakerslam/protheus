// Layer ownership: client/runtime/systems/ui (dashboard static UX surface only; no runtime authority).
function infringBottomDockDefaultOrder(page) {
  var target = page && typeof page === 'object' ? page : {};
  var service = target.taskbarDockService();
  if (service && typeof service.dockDefaultOrder === 'function') return service.dockDefaultOrder(target.bottomDockTileConfig);
  var registry = (target.bottomDockTileConfig && typeof target.bottomDockTileConfig === 'object')
    ? target.bottomDockTileConfig
    : null;
  if (registry) {
    var ids = Object.keys(registry);
    if (ids.length) return ids;
  }
  return ['chat', 'overview', 'agents', 'scheduler', 'skills', 'runtime', 'settings'];
}

function infringBottomDockTileConfigById(page, id) {
  var target = page && typeof page === 'object' ? page : {};
  var key = String(id || '').trim();
  if (!key) return null;
  var registry = (target.bottomDockTileConfig && typeof target.bottomDockTileConfig === 'object')
    ? target.bottomDockTileConfig
    : null;
  var tile = registry && Object.prototype.hasOwnProperty.call(registry, key) ? registry[key] : null;
  return tile && typeof tile === 'object' ? tile : null;
}

function infringBottomDockTileData(page, id, field, fallback) {
  var target = page && typeof page === 'object' ? page : {};
  var key = String(field || '').trim();
  var tile = target.bottomDockTileConfigById(id);
  var value = (key && tile && Object.prototype.hasOwnProperty.call(tile, key)) ? tile[key] : fallback;
  return (value === undefined || value === null) ? String(fallback || '') : String(value);
}

function infringBottomDockTileAnimationName(page, id) {
  var target = page && typeof page === 'object' ? page : {};
  var tile = target.bottomDockTileConfigById(id);
  var animation = tile && Array.isArray(tile.animation) ? tile.animation : null;
  var name = animation ? String(animation[0] || '').trim() : '';
  return name || 'none';
}

function infringBottomDockTileAnimationDurationAttr(page, id) {
  var target = page && typeof page === 'object' ? page : {};
  var tile = target.bottomDockTileConfigById(id);
  var animation = tile && Array.isArray(tile.animation) ? tile.animation : null;
  if (!animation) return null;
  var durationMs = Number(animation[1]);
  if (!Number.isFinite(durationMs) || durationMs < 120) return null;
  return String(Math.round(durationMs));
}

function infringBottomDockSlotStyle(page, id) {
  var target = page && typeof page === 'object' ? page : {};
  var key = String(id || '').trim();
  var weight = target.bottomDockHoverWeight(key);
  var service = target.taskbarDockService();
  if (service && typeof service.dockSlotStyle === 'function') {
    return service.dockSlotStyle(key, target.bottomDockOrder, weight, target.bottomDockTileConfig);
  }
  var order = key ? target.bottomDockOrderIndex(key) : 999;
  if (!Number.isFinite(weight) || weight < 0) weight = 0;
  if (weight > 1) weight = 1;
  return 'order:' + order + ';--bottom-dock-hover-weight:' + weight.toFixed(4);
}

function infringBottomDockTileStyle(page, id) {
  var target = page && typeof page === 'object' ? page : {};
  var key = String(id || '').trim();
  var tile = target.bottomDockTileConfigById(key);
  var style = tile && typeof tile.style === 'string' ? String(tile.style || '').trim() : '';
  return style || '';
}
