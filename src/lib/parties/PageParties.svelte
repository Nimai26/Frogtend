<script lang="ts">
  // 📁 Où sont mes parties d'un jeu PC (lot S1) : `?jeu=<id>&titre=…` ; servie par `/parties` (bureau) et
  // `/taodbox/parties` (canapé : l'adresse garde le mode Taodbox, sa manette et son plein écran). Lot S2 : « 🛟 Mettre
  // mes parties à l'abri » s'y fait aussi, à la manette.
  import { page } from '$app/state';
  import { tick } from 'svelte';
  import { auCanape } from '$lib/taodbox/canape';
  import { fenetreOuverte, selectionner } from '$lib/taodbox/manette.svelte';
  import BlocParties from './BlocParties.svelte';
  import { jeuDeLAdresse } from './parties';

  // `?jeu=` absent ou vide : rien n'est demandé (jamais le jeu n° 0).
  const jeu = $derived(jeuDeLAdresse(page.url.searchParams.get('jeu')) ?? NaN);
  const titre = $derived(page.url.searchParams.get('titre') ?? '');
  const canape = auCanape();
  let retour = $state<HTMLButtonElement | null>(null);

  // Au canapé : la sélection sur « Retour » d’abord (la croix descend vers la liste et le bouton 🛟).
  $effect(() => {
    if (canape && retour) void tick().then(() => selectionner(retour));
  });

  /** Échap (ou B à la manette) : revenir d'où l'on vient. */
  function touche(e: KeyboardEvent) {
    if (e.key !== 'Escape' || fenetreOuverte()) return;
    e.preventDefault();
    history.back();
  }
</script>

<svelte:window onkeydown={touche} />

<div class="page-parties">
  <header>
    <button class="btn" bind:this={retour} onclick={() => history.back()}>← Retour</button>
    <h1>📁 Où sont mes parties{titre ? ` — ${titre}` : ''}</h1>
  </header>
  {#if Number.isFinite(jeu)}
    <BlocParties id={jeu} {canape} />
  {:else}
    <p class="muted">Aucun jeu choisi.</p>
  {/if}
</div>

<style>
  .page-parties {
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
  header {
    display: flex;
    gap: calc(12 * var(--u));
    align-items: center;
    flex-wrap: wrap;
  }
  h1 {
    margin: 0;
  }
</style>
