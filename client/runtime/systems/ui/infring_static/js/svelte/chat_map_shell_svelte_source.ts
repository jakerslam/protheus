const COMPONENT_TAG = 'infring-chat-map-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement="infring-chat-map-shell" />
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
