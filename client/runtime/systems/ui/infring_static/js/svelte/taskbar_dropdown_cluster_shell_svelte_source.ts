const COMPONENT_TAG = 'infring-taskbar-dropdown-cluster-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement="infring-taskbar-dropdown-cluster-shell" />
<script>
  export let shellPrimitive = 'taskbar-dock';
  export let wrapperRole = 'taskbar-dropdowns';
  export let parentOwnedMechanics = true;
</script>
<slot />
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};
