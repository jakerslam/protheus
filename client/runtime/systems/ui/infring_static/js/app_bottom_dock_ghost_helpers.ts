// Canonical Shell helper source: bottom-dock drag ghost projection.
// Loaded before app.ts by the dashboard asset router.
'use strict';

function infringCleanupBottomDockDragGhost(page) {
  var target = page && typeof page === 'object' ? page : {};
  if (target._bottomDockGhostRaf && typeof cancelAnimationFrame === 'function') {
    try { cancelAnimationFrame(target._bottomDockGhostRaf); } catch(_) {}
  }
  if (target._bottomDockGhostCleanupTimer) {
    try { clearTimeout(target._bottomDockGhostCleanupTimer); } catch(_) {}
  }
  target._bottomDockGhostRaf = 0;
  target._bottomDockGhostCleanupTimer = 0;
  target._bottomDockGhostTargetX = 0;
  target._bottomDockGhostTargetY = 0;
  target._bottomDockGhostCurrentX = 0;
  target._bottomDockGhostCurrentY = 0;
  target._bottomDockDragBoundaries = [];
  target._bottomDockLastInsertionIndex = -1;
  target._bottomDockReorderLockUntil = 0;
  var node = target._bottomDockDragGhostEl;
  if (node && node.parentNode) {
    try { node.parentNode.removeChild(node); } catch(_) {}
  }
  target._bottomDockDragGhostEl = null;
  target._bottomDockRevealTargetDuringSettle = false;
}

function infringSetBottomDockGhostTarget(page, x, y) {
  var target = page && typeof page === 'object' ? page : {};
  var nextX = Number(x || 0);
  var nextY = Number(y || 0);
  var targetX = Number.isFinite(nextX) ? nextX : 0;
  var targetY = Number.isFinite(nextY) ? nextY : 0;
  target._bottomDockGhostTargetX = targetX;
  target._bottomDockGhostTargetY = targetY;
  target._bottomDockGhostCurrentX = targetX;
  target._bottomDockGhostCurrentY = targetY;
  var ghost = target._bottomDockDragGhostEl;
  if (!ghost) return;
  ghost.style.left = Math.round(targetX) + 'px';
  ghost.style.top = Math.round(targetY) + 'px';
}

function infringSettleBottomDockDragGhost(page, dragId, done) {
  var target = page && typeof page === 'object' ? page : {};
  var finish = typeof done === 'function' ? done : function() {};
  var ghost = target._bottomDockDragGhostEl;
  if (!ghost || typeof document === 'undefined') {
    target.cleanupBottomDockDragGhost();
    finish();
    return;
  }
  var key = String(dragId || '').trim();
  if (!key) {
    target.cleanupBottomDockDragGhost();
    finish();
    return;
  }
  var slot = document.querySelector('.bottom-dock-btn[data-dock-id="' + key + '"]');
  if (!slot || typeof slot.getBoundingClientRect !== 'function') {
    target.cleanupBottomDockDragGhost();
    finish();
    return;
  }
  var rect = slot.getBoundingClientRect();
  var durationMs = target.bottomDockMoveDurationMs();
  var targetWidth = Number(rect && rect.width ? rect.width : 0);
  var targetHeight = Number(rect && rect.height ? rect.height : 0);
  var slotStyle = null;
  if (!Number.isFinite(targetWidth) || targetWidth <= 0) {
    targetWidth = Number(ghost.offsetWidth || 32);
  }
  if (!Number.isFinite(targetHeight) || targetHeight <= 0) {
    targetHeight = Number(ghost.offsetHeight || 32);
  }
  var slotRadiusPx = 0;
  try {
    if (typeof window !== 'undefined' && typeof window.getComputedStyle === 'function') {
      slotStyle = window.getComputedStyle(slot);
    }
    var rawRadius = slotStyle ? String(slotStyle.borderTopLeftRadius || slotStyle.borderRadius || '') : '';
    var rawWidth = slotStyle ? String(slotStyle.width || '') : '';
    var parsedRadius = parseFloat(rawRadius);
    var parsedWidth = parseFloat(rawWidth);
    if (Number.isFinite(parsedRadius) && parsedRadius >= 0) {
      if (Number.isFinite(parsedWidth) && parsedWidth > 0) {
        slotRadiusPx = (parsedRadius / parsedWidth) * targetWidth;
      } else {
        slotRadiusPx = parsedRadius;
      }
    }
  } catch(_) {}
  if (!slotRadiusPx) {
    slotRadiusPx = Math.round((targetWidth / 32) * 11);
  }
  ghost.style.transition =
    'left ' + durationMs + 'ms var(--ease-smooth), ' +
    'top ' + durationMs + 'ms var(--ease-smooth), ' +
    'width ' + durationMs + 'ms var(--ease-smooth), ' +
    'height ' + durationMs + 'ms var(--ease-smooth), ' +
    'border-radius ' + durationMs + 'ms var(--ease-smooth), ' +
    'opacity ' + durationMs + 'ms var(--ease-smooth)';
  var targetX = Number(rect.left || 0) + ((Number(rect.width || 0) - targetWidth) / 2);
  var targetY = Number(rect.top || 0) + ((Number(rect.height || 0) - targetHeight) / 2);
  var moveGhost = function() {
    if (slotStyle) {
      ghost.style.background = String(slotStyle.background || ghost.style.background || '');
      ghost.style.border = String(slotStyle.border || ghost.style.border || '');
      ghost.style.borderWidth = String(slotStyle.borderTopWidth || ghost.style.borderWidth || '');
      ghost.style.borderStyle = String(slotStyle.borderTopStyle || ghost.style.borderStyle || '');
      ghost.style.borderColor = String(slotStyle.borderColor || ghost.style.borderColor || '');
      ghost.style.boxShadow = String(slotStyle.boxShadow || ghost.style.boxShadow || '');
      ghost.style.color = String(slotStyle.color || ghost.style.color || '');
    }
    ghost.style.left = targetX + 'px';
    ghost.style.top = targetY + 'px';
    ghost.style.width = targetWidth + 'px';
    ghost.style.height = targetHeight + 'px';
    ghost.style.borderRadius = slotRadiusPx + 'px';
    ghost.style.setProperty('--dock-ghost-scale', String(Math.max(0.8, Math.min(4, targetWidth / 32))));
    ghost.style.opacity = '1';
  };
  if (typeof requestAnimationFrame === 'function') requestAnimationFrame(moveGhost);
  else moveGhost();
  if (target._bottomDockGhostCleanupTimer) {
    try { clearTimeout(target._bottomDockGhostCleanupTimer); } catch(_) {}
  }
  target._bottomDockGhostCleanupTimer = window.setTimeout(function() {
    target._bottomDockRevealTargetDuringSettle = true;
    var settleHoldMs = 54;
    var completeSettle = function() {
      target._bottomDockGhostCleanupTimer = 0;
      finish();
      if (typeof requestAnimationFrame !== 'function') {
        target.cleanupBottomDockDragGhost();
        return;
      }
      requestAnimationFrame(function() {
        requestAnimationFrame(function() {
          target.cleanupBottomDockDragGhost();
        });
      });
    };
    target._bottomDockGhostCleanupTimer = window.setTimeout(completeSettle, settleHoldMs);
  }, durationMs + 40);
}
