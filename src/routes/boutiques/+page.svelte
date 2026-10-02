<script lang="ts">
  // « Mes boutiques » (lot 9) : les jeux possédés sur les comptes de la personne (Steam d'abord). Ils restent dans son
  // Frogtend et ne remontent jamais dans Firehouse. Jouer et installer passent par la boutique elle-même.
  import { onMount } from 'svelte';
  import { duree } from '$lib/api';
  import { importerSteam, lireSteam, ouvrirDansSteam, steam } from '$lib/boutiques/steam.svelte';

  let filtre = $state('');
  let seulementInstalles = $state(false);
  onMount(lireSteam);

  const affiches = $derived(
    steam.jeux.filter((j) => (!seulementInstalles || j.installe) && j.nom.toLowerCase().includes(filtre.trim().toLowerCase())),
  );
</script>

<div class="boutiques">
  <header>
    <h1>🛒 Mes boutiques</h1>
    <p class="muted">Les jeux de tes comptes. Steam pour commencer ; Epic, GOG, Amazon et EA viendront ensuite.</p>
  </header>

  <section class="cx-block">
    <div class="tete">
      <h2>Steam</h2>
      <span class="muted">{steam.etat?.compte ? `compte ${steam.etat.compte}` : 'compte pas encore réglé'}</span>
      <span class="espace"></span>
      <button class="btn primary" onclick={importerSteam} disabled={steam.enCours}>
        {steam.enCours ? 'Import…' : steam.jeux.length ? '🔄 Mettre à jour' : '⬇ Importer mes jeux Steam'}
      </button>
    </div>

    {#if steam.jeux.length}
      <div class="outils">
        <input type="search" bind:value={filtre} placeholder="Chercher un jeu" aria-label="Chercher un jeu Steam" />
        <label><input type="checkbox" bind:checked={seulementInstalles} /> installés sur ce PC</label>
        <span class="muted">{affiches.length} / {steam.jeux.length}</span>
      </div>
      <ul class="liste">
        {#each affiches as j (j.id)}
          <li>
            <span class="nom">
              <strong>{j.nom}</strong>
              <span class="muted">{j.minutes ? duree(j.minutes * 60) : 'jamais joué'}{j.installe ? ' · ✅ installé' : ''}</span>
            </span>
            {#if j.installe}
              <button class="btn petit primary" onclick={() => ouvrirDansSteam(j, 'jouer')}>▶ Jouer</button>
            {:else}
              <button class="btn petit" onclick={() => ouvrirDansSteam(j, 'installer')}>⬇ Installer (Steam)</button>
            {/if}
          </li>
        {/each}
      </ul>
    {:else if !steam.enCours}
      <p class="muted">Aucun jeu Steam importé pour l’instant.</p>
    {/if}
  </section>
</div>

<style>
  .boutiques {
    display: grid;
    gap: calc(14 * var(--u));
    padding: calc(20 * var(--u));
    max-width: calc(900 * var(--u));
    margin: 0 auto;
    width: 100%;
    box-sizing: border-box;
    align-content: start;
    overflow: auto;
    height: 100%;
  }
  h1,
  h2 {
    margin: 0;
  }
  header p {
    margin: calc(4 * var(--u)) 0 0;
  }
  .tete,
  .outils {
    display: flex;
    align-items: center;
    gap: calc(10 * var(--u));
    flex-wrap: wrap;
  }
  .espace {
    flex: 1;
  }
  .outils {
    margin: calc(10 * var(--u)) 0;
  }
  .outils input[type='search'] {
    flex: 1;
    min-width: calc(180 * var(--u));
  }
  .liste {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: calc(6 * var(--u));
  }
  .liste li {
    display: flex;
    align-items: center;
    gap: calc(10 * var(--u));
  }
  .nom {
    flex: 1;
    display: grid;
  }
</style>
