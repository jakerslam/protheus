const COMPONENT_TAG = 'infring-taskbar-hero-menu-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement="infring-taskbar-hero-menu-shell" />
<script>
  export let shellPrimitive = 'taskbar-dock';
  export let wrapperRole = 'taskbar-hero';
  export let parentOwnedMechanics = true;
</script>
<slot />
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};
