const COMPONENT_TAG = 'infring-taskbar-menu-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement="infring-taskbar-menu-shell" />
<script>
  import { onMount, onDestroy, tick } from 'svelte';

  export let shellPrimitive = 'taskbar-dock';
  export let wrapperRole = 'taskbar-menu';
  export let parentOwnedMechanics = true;
  export let anchorid = '';
  export let fallbackside = 'bottom';
  export let layoutkey = '';

  const MANAGED_CLASSES = [
    'taskbar-anchored-dropdown',
    'is-side-top',
    'is-side-bottom',
    'is-side-left',
    'is-side-right',
    'is-inline-away-left',
    'is-inline-away-right',
    'is-block-away-top',
    'is-block-away-bottom'
  ];

  let probe;
  let timer = 0;

  function popupService() {
    const services = typeof window !== 'undefined' ? window.InfringSharedShellServices : null;
    return services && services.popup ? services.popup : null;
  }

  function hostElement() {
    if (!probe) return null;
    const root = typeof probe.getRootNode === 'function' ? probe.getRootNode() : null;
    const shadowHost = root && root.host ? root.host : null;
    const host = shadowHost || probe.parentElement;
    return host && String(host.tagName || '').toLowerCase() === 'infring-taskbar-menu-shell' ? host : null;
  }

  function anchorRect() {
    const id = String(anchorid || '').trim();
    if (!id || typeof document === 'undefined') return null;
    const node = document.getElementById(id);
    if (!node || typeof node.getBoundingClientRect !== 'function') return null;
    return node.getBoundingClientRect();
  }

  function dropdownClass() {
    const service = popupService();
    if (!service || typeof service.dropdownClass !== 'function') {
      return { 'taskbar-anchored-dropdown': true, 'is-side-bottom': true, 'is-inline-away-right': true, 'is-block-away-bottom': true };
    }
    return service.dropdownClass(anchorRect(), fallbackside || 'bottom', layoutkey || '');
  }

  function syncDropdownClass() {
    const host = hostElement();
    if (!host) return;
    if (!String(anchorid || '').trim()) {
      MANAGED_CLASSES.forEach(function(name) { host.classList.remove(name); });
      return;
    }
    const map = dropdownClass();
    MANAGED_CLASSES.forEach(function(name) {
      host.classList.toggle(name, !!(map && map[name]));
    });
  }

  onMount(function() {
    tick().then(syncDropdownClass);
    timer = window.setInterval(syncDropdownClass, 120);
    window.addEventListener('resize', syncDropdownClass, { passive: true });
    window.addEventListener('scroll', syncDropdownClass, true);
  });

  onDestroy(function() {
    if (timer) window.clearInterval(timer);
    window.removeEventListener('resize', syncDropdownClass);
    window.removeEventListener('scroll', syncDropdownClass, true);
  });

  $: if (anchorid || fallbackside || layoutkey) tick().then(syncDropdownClass);
</script>
<span bind:this={probe} class="taskbar-menu-shell-probe" hidden aria-hidden="true"></span>
<slot />
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};
