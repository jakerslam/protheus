import App from './App.svelte';

const target = document.getElementById('app');

if (!target) {
  throw new Error('svelte_dashboard_mount_target_missing');
}

new App({
  target,
});
