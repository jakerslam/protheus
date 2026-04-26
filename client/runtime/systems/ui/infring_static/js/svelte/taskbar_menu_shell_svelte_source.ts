const COMPONENT_TAG = 'infring-taskbar-menu-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement="infring-taskbar-menu-shell" />
<script>
  export let shellPrimitive = 'taskbar-dock';
  export let wrapperRole = 'taskbar-menu';
  export let parentOwnedMechanics = true;
</script>
<slot />
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};
