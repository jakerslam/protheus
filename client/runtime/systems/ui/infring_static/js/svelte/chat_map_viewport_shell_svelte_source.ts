const COMPONENT_TAG = 'infring-chat-map-viewport-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement="infring-chat-map-viewport-shell" />
<script>
  export let dragbarSurface = 'chat-map';
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
