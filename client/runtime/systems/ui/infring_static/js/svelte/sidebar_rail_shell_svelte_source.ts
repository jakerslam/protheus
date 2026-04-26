const COMPONENT_TAG = 'infring-sidebar-rail-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement="infring-sidebar-rail-shell" />
<script>
  export let dragbarSurface = 'chat-sidebar';
  export let wall = '';
  export let dragging = false;
  export let parentOwnedMechanics = true;
</script>
<slot />
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};
