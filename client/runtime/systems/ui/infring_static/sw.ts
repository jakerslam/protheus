function isSupportedRequest(request) {
  if (!request || request.method !== 'GET') return false;
  try {
    var url = new URL(request.url);
    return url.protocol === 'http:' || url.protocol === 'https:';
  } catch (_) {
    return false;
  }
}

function fallbackForApiRequest(request) {
  try {
    var url = new URL(request.url);
    if (url.origin !== self.location.origin) return null;
    if (String(url.pathname || '').indexOf('/api/') !== 0) return null;
    return new Response(JSON.stringify({
      ok: false,
      error: 'dashboard_network_unavailable'
    }), {
      status: 503,
      headers: { 'content-type': 'application/json' }
    });
  } catch (_) {
    return null;
  }
}

self.addEventListener('fetch', function(event) {
  if (!isSupportedRequest(event.request)) return;
  event.respondWith(
    fetch(event.request).catch(function(error) {
      var fallback = fallbackForApiRequest(event.request);
      if (fallback) return fallback;
      throw error;
    })
  );
});
