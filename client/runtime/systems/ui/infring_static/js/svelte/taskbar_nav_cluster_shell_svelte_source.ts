const COMPONENT_TAG = 'infring-taskbar-nav-cluster-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement="infring-taskbar-nav-cluster-shell" />
<script>
  export let shellPrimitive = 'taskbar-dock';
  export let wrapperRole = 'taskbar-nav';
  export let parentOwnedMechanics = true;
</script>
<slot />
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};
