<script lang="ts">
  import { page } from '$app/stores';
  import {
    dashboardClassicHref,
    dashboardPageHref,
    dashboardPages,
    resolveDashboardPageFromPathname,
  } from '$lib/dashboard';

  $: activePage = resolveDashboardPageFromPathname($page.url.pathname);
  $: classicHref = dashboardClassicHref(activePage.key);
</script>

<div class="shell">
  <aside class="sidebar">
    <div class="brand">
      <span class="mark" aria-hidden="true">&infin;</span>
      <div>
        <div class="eyebrow">Dashboard</div>
        <div class="title">Infring</div>
      </div>
    </div>
    <nav class="nav" aria-label="Dashboard sections">
      {#each dashboardPages as item}
        <a
          class:item-active={item.key === activePage.key}
          class="nav-link"
          href={dashboardPageHref(item.key)}
        >
          <span>{item.title}</span>
          <span class:item-native={item.mode === 'native'} class:item-legacy={item.mode === 'legacy'} class="pill">
            {item.mode === 'native' ? 'Native' : 'Fallback'}
          </span>
        </a>
      {/each}
    </nav>
  </aside>

  <div class="main">
    <header class="topbar">
      <div>
        <p class="eyebrow">SvelteKit primary shell</p>
        <h1>{activePage.title}</h1>
        <p class="summary">{activePage.summary}</p>
      </div>
      <div class="actions">
        <span class:mode-native={activePage.mode === 'native'} class="mode-pill">
          {activePage.mode === 'native' ? 'Native page' : 'Classic fallback page'}
        </span>
        <a class="ghost" href={classicHref}>Open classic dashboard</a>
      </div>
    </header>
    <div class="content">
      <slot />
    </div>
  </div>
</div>

<style>
  :global(body) {
    margin: 0;
    background:
      radial-gradient(circle at top left, rgba(31, 73, 143, 0.28), transparent 30%),
      linear-gradient(180deg, #08111f 0%, #0d1624 100%);
    color: #ebf2ff;
    font-family: "IBM Plex Sans", "Inter", system-ui, sans-serif;
  }

  .shell {
    min-height: 100vh;
    display: grid;
    grid-template-columns: 280px minmax(0, 1fr);
  }

  .sidebar {
    border-right: 1px solid rgba(158, 188, 255, 0.12);
    background: rgba(7, 14, 24, 0.92);
    backdrop-filter: blur(16px);
    padding: 24px 18px;
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 24px;
  }

  .mark {
    width: 34px;
    height: 34px;
    border-radius: 12px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border: 1px solid rgba(122, 168, 255, 0.32);
    background: rgba(40, 79, 138, 0.28);
    font-size: 22px;
  }

  .eyebrow {
    margin: 0;
    color: #8aa4cf;
    text-transform: uppercase;
    letter-spacing: 0.12em;
    font-size: 0.72rem;
  }

  .title,
  h1 {
    margin: 0;
    font-weight: 650;
  }

  .nav {
    display: grid;
    gap: 8px;
  }

  .nav-link {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 12px 14px;
    border-radius: 18px;
    color: inherit;
    text-decoration: none;
    background: rgba(255, 255, 255, 0.02);
    border: 1px solid transparent;
  }

  .nav-link:hover,
  .nav-link.item-active {
    border-color: rgba(158, 188, 255, 0.18);
    background: rgba(64, 102, 166, 0.18);
  }

  .pill,
  .mode-pill {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: 999px;
    padding: 0.2rem 0.6rem;
    font-size: 0.72rem;
    border: 1px solid rgba(158, 188, 255, 0.18);
    background: rgba(255, 255, 255, 0.05);
  }

  .item-native,
  .mode-native {
    background: rgba(55, 126, 86, 0.22);
    border-color: rgba(102, 198, 135, 0.3);
  }

  .item-legacy {
    background: rgba(155, 115, 42, 0.18);
    border-color: rgba(234, 183, 89, 0.24);
  }

  .main {
    min-width: 0;
    padding: 24px;
  }

  .topbar {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
    margin-bottom: 20px;
  }

  .summary {
    margin: 8px 0 0;
    max-width: 60ch;
    color: #bdd0f0;
  }

  .actions {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
  }

  .ghost {
    color: inherit;
    text-decoration: none;
    border-radius: 16px;
    padding: 0.75rem 1rem;
    border: 1px solid rgba(158, 188, 255, 0.18);
    background: rgba(255, 255, 255, 0.04);
  }

  .ghost:hover {
    background: rgba(255, 255, 255, 0.08);
  }

  .content {
    min-width: 0;
  }

  @media (max-width: 980px) {
    .shell {
      grid-template-columns: 1fr;
    }

    .sidebar {
      border-right: 0;
      border-bottom: 1px solid rgba(158, 188, 255, 0.12);
    }

    .topbar {
      flex-direction: column;
    }
  }
</style>
