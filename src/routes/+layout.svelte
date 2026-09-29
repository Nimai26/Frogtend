<script lang="ts">
  import '$lib/styles/base.css';
  import { onMount } from 'svelte';
  import { page } from '$app/state';
  import Dialogues from '$lib/dialogues/Dialogues.svelte';
  import { demarrer, etat } from '$lib/etat.svelte';
  import { verifierMiseAJour } from '$lib/mises-a-jour';

  let { children } = $props();

  onMount(async () => {
    await demarrer();
    // Vérification discrète : on ne propose que s'il y a une nouvelle version, jamais d'installation sans accord.
    verifierMiseAJour(true);
  });

  const liens = [
    { href: '/', libelle: '🎮 Ludothèque' },
    { href: '/reglages', libelle: '⚙ Réglages' },
    { href: '/a-propos', libelle: 'ℹ À propos' },
  ];
</script>

{#if etat.pret}
  <div class="application">
    <nav class="panel" aria-label="Navigation principale">
      <div class="marque">🐸 Frogtend</div>
      {#each liens as l (l.href)}
        <a class="btn lien" class:actif={page.url.pathname === l.href} href={l.href}>{l.libelle}</a>
      {/each}
      {#if etat.pc.firehouse.simule}
        <span class="tag warn simule" title="Les données affichées sont des exemples, pas celles de Firehouse">
          Mode simulé
        </span>
      {/if}
    </nav>
    <main>
      {@render children()}
    </main>
  </div>
{/if}

<Dialogues />

<style>
  .application {
    display: grid;
    grid-template-columns: calc(220 * var(--u)) 1fr;
    gap: calc(16 * var(--u));
    min-height: 100vh;
    padding: calc(16 * var(--u));
  }
  nav {
    display: flex;
    flex-direction: column;
    gap: calc(8 * var(--u));
    padding: calc(14 * var(--u));
    align-self: start;
    position: sticky;
    top: calc(16 * var(--u));
  }
  .marque {
    font-size: calc(20 * var(--u));
    font-weight: 700;
    margin-bottom: calc(8 * var(--u));
  }
  .lien {
    justify-content: flex-start;
    text-decoration: none;
  }
  .lien.actif {
    border-color: var(--accent);
    background: color-mix(in srgb, var(--accent) 20%, var(--bg2));
  }
  .simule {
    margin-top: calc(8 * var(--u));
    align-self: flex-start;
  }
  main {
    min-width: 0;
  }
  @media (max-width: 720px) {
    .application {
      grid-template-columns: 1fr;
    }
    nav {
      position: static;
    }
  }
</style>
