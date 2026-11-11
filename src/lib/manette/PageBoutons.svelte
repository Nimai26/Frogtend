<script lang="ts">
  // 🎮 Les boutons d'un jeu du PC (lot 4d) : `?jeu=<id>&titre=…` ; servie par `/boutons` (bureau) et
  // `/taodbox/boutons` (canapé : l'adresse garde le mode Taodbox, sa manette et son plein écran).
  import { page } from '$app/state';
  import { api } from '$lib/api';
  import { tick } from 'svelte';
  import { motifDuRefus } from '$lib/dialogues/messages';
  import { etat } from '$lib/etat.svelte';
  import { nettoyer } from '$lib/ludotheque/clavier';
  import { auCanape } from '$lib/taodbox/canape';
  import { fenetreOuverte, selectionner } from '$lib/taodbox/manette.svelte';
  import SchemaManette from './SchemaManette.svelte';
  import { infosDeLaFiche, type InfosBoutons } from './boutons';

  const jeu = $derived(Number(page.url.searchParams.get('jeu')));
  const titre = $derived(page.url.searchParams.get('titre') ?? '');
  let infos = $state<InfosBoutons | null | undefined>(undefined);
  let erreur = $state<string | null>(null);
  let page_ = $state<HTMLElement | null>(null);

  $effect(() => {
    const id = jeu;
    infos = undefined;
    erreur = null;
    const commande = etat.profil.commandes[String(id)];
    api
      .boutonsDuJeu(id, commande?.mode === 'reference' && commande.genre === 'Wiimote' ? (commande.reference ?? null) : null)
      .then(async (i) => {
        // Le choix de « 🎮 Commandes » pour ce jeu et sa correspondance manette → clavier (jeu PC) : `infosDeLaFiche`.
        infos = !i ? i : infosDeLaFiche(i, commande, nettoyer(etat.profil.clavier[String(id)]) ?? null);
        await tick();
        if (auCanape()) selectionner(page_?.querySelector<HTMLButtonElement>('button'));
      })
      .catch((e) => (erreur = motifDuRefus(e)));
  });

  /** Échap (ou B à la manette) : revenir d'où l'on vient. */
  function touche(e: KeyboardEvent) {
    if (e.key !== 'Escape' || fenetreOuverte()) return;
    e.preventDefault();
    history.back();
  }
</script>

<svelte:window onkeydown={touche} />

<div class="page-boutons" bind:this={page_}>
  <header>
    <button class="btn" onclick={() => history.back()}>← Retour</button>
    <h1>🗺 Les boutons{titre ? ` — ${titre}` : ''}</h1>
  </header>
  {#if erreur}
    <p class="muted">⚠ {erreur}</p>
  {:else if infos === undefined}
    <p class="muted">Chargement…</p>
  {:else if infos === null}
    <p class="muted">Ce jeu n’est pas sur ce PC.</p>
  {:else}
    <SchemaManette {infos} />
  {/if}
</div>

<style>
  .page-boutons {
    display: grid;
    gap: calc(14 * var(--u));
    padding: calc(20 * var(--u));
    max-width: calc(760 * var(--u));
    margin: 0 auto;
    width: 100%;
    box-sizing: border-box;
    align-content: start;
    overflow: auto;
    height: 100%;
  }
  h1 {
    margin: 0;
  }
</style>
