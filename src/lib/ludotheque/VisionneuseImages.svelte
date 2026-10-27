<script lang="ts">
  // La visionneuse de la galerie d'un jeu (lot Médias M2) : une image à la fois, en grand, pilotable entièrement au
  // clavier et à la manette (←/→ : image précédente/suivante ; B / Échap : revenir). Même carcasse que le lecteur du
  // livret (sans `role="dialog"`, sinon la manette ne lui enverrait plus rien), sans le toucher.
  import { onMount, tick, untrack } from 'svelte';
  import { adresseMedia } from '$lib/api';
  import { retenirFocus } from '$lib/dialogues/focus';
  import { fenetreOuverte, prendreLaMain, selectionner, taodbox } from '$lib/taodbox/manette.svelte';
  import { grosPixels, largeurUtile, creditMedia, libelleMedia, type MediaJeu } from './galerie';

  let {
    jeu,
    medias,
    depart,
    titre,
    sousLaBarre = false,
    onfermer,
  }: { jeu: number; medias: MediaJeu[]; depart: number; titre: string; sousLaBarre?: boolean; onfermer: (id: number) => void } = $props();

  // L'image de départ dès le premier rendu : les boutons ◀ ▶ sont justes avant que la sélection ne s'y pose.
  let index = $state(untrack(() => Math.max(0, medias.findIndex((m) => m.id === depart))));
  // L'image regardée, par son id : si la liste change (une vignette ratée retirée), on reste sur la même image.
  let regardee = untrack(() => depart);
  let charge = $state(false);
  let echec = $state(false);
  let precedent = $state<HTMLButtonElement>();
  let suivant = $state<HTMLButtonElement>();
  let fermeture = $state<HTMLButtonElement>();

  const courant = $derived(medias[index]);
  $effect(() => {
    const i = medias.findIndex((m) => m.id === regardee);
    if (i >= 0 && i !== untrack(() => index)) index = i;
  });
  $effect(() => {
    if (courant) regardee = courant.id;
  });
  // 1 000 px au plus (jamais l'original), jamais plus grand que l'original.
  const adresse = $derived(courant ? adresseMedia(jeu, courant.id, largeurUtile(courant, 1000)) : null);

  $effect(() => {
    void adresse;
    charge = false;
    echec = false;
  });

  async function aller(n: number) {
    const i = Math.max(0, Math.min(medias.length - 1, n));
    if (i === index) return;
    index = i;
    await tick();
    if ((document.activeElement as HTMLButtonElement | null)?.disabled) selectionner(index === 0 ? suivant : precedent);
  }

  const fermer = () => onfermer(courant?.id ?? depart);

  onMount(() => {
    selectionner([suivant, precedent].find((b) => b && !b.disabled) ?? fermeture);
    const rendre = prendreLaMain((c) => {
      if (c === 'gauche' || c === 'haut') aller(index - 1);
      else if (c === 'droite' || c === 'bas') aller(index + 1);
      else if (c === 'valider') return false;
      else fermer();
      return true;
    });
    const touche = (e: KeyboardEvent) => {
      if (fenetreOuverte()) return;
      const fleches = !taodbox.actif;
      if (e.key === 'Escape') fermer();
      else if (fleches && e.key === 'ArrowLeft') aller(index - 1);
      else if (fleches && e.key === 'ArrowRight') aller(index + 1);
      else return;
      e.preventDefault();
      e.stopPropagation();
    };
    window.addEventListener('keydown', touche, { capture: true });
    return () => {
      rendre();
      window.removeEventListener('keydown', touche, { capture: true });
    };
  });
</script>

<!-- Pas de role="dialog" : ce serait une « fenêtre maison » pour la manette, et la visionneuse ne recevrait plus rien. -->
<section class="visionneuse" class:sous-la-barre={sousLaBarre} aria-label={`Images : ${titre}`} use:retenirFocus>
  <header>
    <strong>🖼 {titre}</strong>
    {#if courant}
      {@const credit = creditMedia(courant)}
      <span class="legende" aria-live="polite">{libelleMedia(courant)} · {index + 1}/{medias.length}</span>
      <!-- Le crédit (licence de l'image) : à part, jamais relu à chaque image par le lecteur d'écran. -->
      {#if credit}<span class="credit">{credit}{#if courant.url_source} — {courant.url_source}{/if}</span>{/if}
    {/if}
  </header>
  <div class="scene">
    {#if echec}
      <p class="note">Cette image ne s’affiche pas.</p>
    {:else if adresse && courant}
      {#key adresse}
        <img
          src={adresse}
          alt={libelleMedia(courant)}
          class:charge
          class:pixels={grosPixels(courant, 1000)}
          onload={() => (charge = true)}
          onerror={() => (echec = true)}
        />
      {/key}
      {#if !charge}<p class="note attente" aria-live="polite">⏳ Chargement…</p>{/if}
    {/if}
  </div>
  <footer>
    <button class="btn" bind:this={precedent} disabled={index === 0} onclick={() => aller(index - 1)}>◀ Précédente</button>
    <button class="btn" bind:this={suivant} disabled={index >= medias.length - 1} onclick={() => aller(index + 1)}>Suivante ▶</button>
    <button class="btn" bind:this={fermeture} onclick={fermer}>✕ Fermer</button>
    {#if taodbox.actif}<span class="aide">◀ ▶ images · B revenir</span>
    {:else}<span class="aide">← → images · Échap fermer</span>{/if}
  </footer>
</section>

<style>
  .visionneuse {
    position: fixed;
    inset: 0;
    z-index: 50;
    display: grid;
    grid-template-rows: auto 1fr auto;
    background: var(--voile-opaque);
    color: var(--encre-voile);
  }
  /* Sur la fiche : sous la barre de titre (BarreTitre.svelte : 34 u + son filet), pour garder réduire et déplacer. */
  .visionneuse.sous-la-barre {
    top: calc(34 * var(--u) + 1px);
  }
  header,
  footer {
    display: flex;
    align-items: center;
    gap: calc(10 * var(--u));
    padding: calc(10 * var(--u)) calc(16 * var(--u));
  }
  .legende,
  .aide {
    color: var(--encre-voile-dim);
    margin-left: auto;
  }
  header {
    flex-wrap: wrap;
  }
  .credit {
    color: var(--encre-voile-dim);
    font-size: calc(12 * var(--u));
    flex-basis: 100%;
    text-align: right;
    overflow-wrap: anywhere;
  }
  footer {
    justify-content: center;
    flex-wrap: wrap;
  }
  .scene {
    min-height: 0;
    display: grid;
    place-items: center;
    position: relative;
    padding: calc(8 * var(--u));
  }
  .scene img {
    max-width: 100%;
    max-height: 100%;
    object-fit: contain;
    /* Un logo transparent se lit sur le damier de la charte. */
    background: repeating-conic-gradient(var(--damier-clair) 0% 25%, var(--damier-fonce) 0% 50%) 50% / 16px 16px;
    opacity: 0;
    transition: opacity 0.15s;
  }
  .scene img.charge {
    opacity: 1;
  }
  /* Une petite image d'époque : agrandie à la scène, gros pixels nets, sans damier (elle est opaque). */
  .scene img.pixels {
    image-rendering: pixelated;
    width: 100%;
    height: 100%;
    background: none;
  }
  .note {
    color: var(--encre-voile-note);
    text-align: center;
  }
  .attente {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    margin: 0;
  }
</style>
