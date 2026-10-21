<script lang="ts">
  // 🏆 Les succès d'un jeu en détail (lot C) : `?source=steam|galaxy&cle=…&titre=…`, ou `?jeu=<id du PC>&titre=…` (un
  // jeu du PC : son jeu Steam, ou RetroAchievements pour un jeu d'émulateur, lot C3) ; servie par `/succes`
  // (bureau) et `/taodbox/succes` (canapé : l'adresse garde le mode Taodbox, sa manette et son plein écran).
  import { page } from '$app/state';
  import { api } from '$lib/api';
  import { tick } from 'svelte';
  import { motifDuRefus } from '$lib/dialogues/messages';
  import { auCanape } from '$lib/taodbox/canape';
  import { fenetreOuverte, selectionner } from '$lib/taodbox/manette.svelte';
  import ListeSucces from '$lib/succes/ListeSucces.svelte';
  import type { ListeSucces as Liste } from '$lib/succes/liste';

  const source = $derived<'steam' | 'galaxy'>(page.url.searchParams.get('source') === 'steam' ? 'steam' : 'galaxy');
  const cle = $derived(page.url.searchParams.get('cle') ?? '');
  const jeu = $derived.by(() => {
    const v = page.url.searchParams.get('jeu');
    return v ? Number(v) : NaN;
  });
  const titre = $derived(page.url.searchParams.get('titre') ?? '');
  let liste = $state<Liste | null | undefined>(undefined);
  let erreur = $state<string | null>(null);

  let page_ = $state<HTMLElement | null>(null);

  $effect(() => {
    const [s, c, j] = [source, cle, jeu];
    liste = undefined;
    erreur = null;
    (Number.isFinite(j) ? api.succesJeu(j) : api.succesListe(s, c))
      .then(async (l) => {
        liste = l;
        // Au canapé (Taodbox) : la sélection sur le premier succès, pour la manette.
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

<div class="page-succes" bind:this={page_}>
  <header>
    <button class="btn" onclick={() => history.back()}>← Retour</button>
    <h1>🏆 Succès{titre ? ` — ${titre}` : ''}</h1>
  </header>
  {#if erreur}
    <p class="muted">⚠ {erreur}</p>
  {:else if liste === undefined}
    <p class="muted">Chargement…</p>
  {:else if liste === null}
    <p class="muted">Aucun succès connu pour ce jeu sur ce PC (le jeu n’en a pas, sa version n’est pas reconnue, ou le compte n’est pas relié).</p>
  {:else}
    <ListeSucces {liste} />
  {/if}
</div>

<style>
  .page-succes {
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
  h1 {
    margin: 0;
  }
</style>
