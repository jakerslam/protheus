// Layer ownership: client/runtime/systems/ui (dashboard static UX surface only; no runtime authority).
function infringNormalizeBottomDockOrder(page, rawOrder) {
  var target = page && typeof page === 'object' ? page : {};
  var service = target.taskbarDockService();
  if (service && typeof service.normalizeOrder === 'function') return service.normalizeOrder(rawOrder, target.bottomDockDefaultOrder());
  var defaults = target.bottomDockDefaultOrder();
  var source = Array.isArray(rawOrder) ? rawOrder : [];
  var seen = {};
  var ordered = [];
  for (var i = 0; i < source.length; i += 1) {
    var id = String(source[i] || '').trim();
    if (!id || seen[id] || defaults.indexOf(id) < 0) continue;
    seen[id] = true;
    ordered.push(id);
  }
  for (var j = 0; j < defaults.length; j += 1) {
    var fallbackId = defaults[j];
    if (seen[fallbackId]) continue;
    seen[fallbackId] = true;
    ordered.push(fallbackId);
  }
  return ordered;
}

function infringPersistBottomDockOrder(page) {
  var target = page && typeof page === 'object' ? page : {};
  target.bottomDockOrder = target.normalizeBottomDockOrder(target.bottomDockOrder);
  try {
    var service = target.taskbarDockService();
    if (service && typeof service.persistDockOrder === 'function') target.bottomDockOrder = service.persistDockOrder(target.bottomDockOrder, target.bottomDockTileConfig);
    else localStorage.setItem('infring-bottom-dock-order', JSON.stringify(target.bottomDockOrder));
  } catch(_) {}
  infringUpdateShellLayoutConfig(function(config) {
    config.dock.order = target.bottomDockOrder.slice();
  });
}

function infringBottomDockOrderIndex(page, id) {
  var target = page && typeof page === 'object' ? page : {};
  var key = String(id || '').trim();
  if (!key) return 999;
  var service = target.taskbarDockService();
  if (service && typeof service.orderIndex === 'function') {
    return service.orderIndex(key, target.bottomDockOrder, target.bottomDockDefaultOrder());
  }
  var order = target.normalizeBottomDockOrder(target.bottomDockOrder);
  var idx = order.indexOf(key);
  if (idx >= 0) return idx;
  var fallback = target.bottomDockDefaultOrder().indexOf(key);
  return fallback >= 0 ? fallback : 999;
}
