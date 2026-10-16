<script lang="ts">
  // Le lecteur du livret d'un jeu (contrat 1.7) : chaque page est rendue en image par le cœur (Windows lui-même),
  // et ce lecteur se pilote entièrement au clavier et à la manette — sur la fiche, dans le menu en jeu, en Taodbox.
  // ←/→ : page (agrandie : d'abord défiler sur le côté) ; ↑/↓ : défiler ; A (ou 🔍) : agrandir ou ajuster ;
  // B / Échap : revenir. Les livrets réels sont souvent des doubles pages à l'italienne (Zelda, 16/10).
  import { onMount, tick } from 'svelte';
  import { adressePageLivret } from '$lib/api';
  import { retenirFocus } from '$lib/dialogues/focus';
  import { fenetreOuverte, prendreLaMain, selectionner, taodbox } from '$lib/taodbox/manette.svelte';

  let {
    jeu,
    doc,
    pages,
    titre,
    sousLaBarre = false,
    onfermer,
  }: { jeu: number; doc: number; pages: number; titre: string; sousLaBarre?: boolean; onfermer: () => void } = $props();

  let page = $state(0);
  let agrandi = $state(false);
  let charge = $state(false);
  let echec = $state(false);
  let zone = $state<HTMLDivElement>();
  let precedent = $state<HTMLButtonElement>();
  let suivant = $state<HTMLButtonElement>();
  // Mesurée avant toute demande (sinon une première page serait rendue à une taille supposée, pour rien).
  let largeurZone = $state(0);
  // En revenant en arrière dans une page agrandie : aller à son bord droit, une fois l'image arrivée.
  let auBordDroit = false;

  // La largeur demandée, en pixels de l'écran : celle de la zone (la page entière s'y réduit au besoin), ou le
  // double (agrandie). Pas de préchargement : le cœur ne garde pas les pages rendues (aucun cache entre profils).
  const largeur = $derived(Math.round((agrandi ? 2 : 1) * largeurZone * (window.devicePixelRatio || 1)));
  const adresse = $derived(largeurZone > 0 ? adressePageLivret(jeu, doc, page, largeur) : null);

  async function aller(n: number) {
    const p = Math.max(0, Math.min(pages - 1, n));
    if (p === page) return;
    page = p;
    zone?.scrollTo({ top: 0, left: 0 });
    // Le bouton choisi vient d'être désactivé (première ou dernière page) : la sélection passe à l'autre.
    await tick();
    if ((document.activeElement as HTMLButtonElement | null)?.disabled) selectionner(page === 0 ? suivant : precedent);
  }

  // Une nouvelle image (autre page, autre taille) : « en cours » jusqu'à ce qu'elle arrive.
  $effect(() => {
    void adresse;
    charge = false;
    echec = false;
  });

  function arrivee() {
    charge = true;
    if (auBordDroit && zone) zone.scrollTo({ left: zone.scrollWidth });
    auBordDroit = false;
  }

  function defiler(sens: 1 | -1) {
    zone?.scrollBy({ top: sens * (zone.clientHeight * 0.4), behavior: 'smooth' });
  }

  /** ←/→ : agrandie, d'abord sur le côté ; au bord (ou page entière), la page d'à côté. */
  function cote(sens: 1 | -1) {
    if (agrandi && zone) {
      const reste = sens > 0 ? zone.scrollWidth - zone.clientWidth - zone.scrollLeft : zone.scrollLeft;
      if (reste > 2) {
        zone.scrollBy({ left: sens * (zone.clientWidth * 0.5), behavior: 'smooth' });
        return;
      }
    }
    if (page + sens < 0 || page + sens >= pages) return;
    auBordDroit = agrandi && sens < 0;
    aller(page + sens);
  }

  onMount(() => {
    selectionner(suivant ?? precedent);
    // La manette (et les flèches quand elle pilote la page) passe d'abord par le lecteur.
    const rendre = prendreLaMain((c) => {
      if (c === 'gauche') cote(-1);
      else if (c === 'droite') cote(1);
      else if (c === 'haut') defiler(-1);
      else if (c === 'bas') defiler(1);
      else if (c === 'valider') agrandi = !agrandi;
      else onfermer();
      return true;
    });
    // Le clavier, quand la manette ne pilote pas la page (la fiche, au bureau) ; Échap toujours, avant la page.
    const touche = (e: KeyboardEvent) => {
      if (fenetreOuverte()) return;
      const fleches = !taodbox.actif;
      if (e.key === 'Escape') onfermer();
      else if (fleches && e.key === 'ArrowLeft') cote(-1);
      else if (fleches && e.key === 'ArrowRight') cote(1);
      else if (fleches && e.key === 'ArrowUp') defiler(-1);
      else if (fleches && e.key === 'ArrowDown') defiler(1);
      else if (e.key === 'PageUp') aller(page - 1);
      else if (e.key === 'PageDown') aller(page + 1);
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

<!-- Pas de role="dialog" : ce serait une « fenêtre maison » pour la manette, et le lecteur ne recevrait plus rien. -->
<section class="lecteur" class:sous-la-barre={sousLaBarre} aria-label={`Livret : ${titre}`} use:retenirFocus>
  <header>
    <strong>📖 {titre}</strong>
    <span class="numero" aria-live="polite">Page {page + 1} / {pages}</span>
  </header>
  <div class="zone" class:agrandi bind:this={zone} bind:clientWidth={largeurZone}>
    {#if echec}
      <p class="note">Cette page ne s’affiche pas. Essaie la suivante, ou ferme et rouvre le livret.</p>
    {:else if adresse}
      {#key adresse}
        <img src={adresse} alt={`Page ${page + 1}`} class:charge onload={arrivee} onerror={() => (echec = true)} />
      {/key}
    {/if}
    {#if !charge && !echec}<p class="note attente" aria-live="polite">⏳ Page {page + 1}…</p>{/if}
  </div>
  <footer>
    <button class="btn" bind:this={precedent} disabled={page === 0} onclick={() => aller(page - 1)}>◀ Précédente</button>
    <button class="btn" bind:this={suivant} disabled={page >= pages - 1} onclick={() => aller(page + 1)}>Suivante ▶</button>
    <button class="btn" onclick={() => (agrandi = !agrandi)}>{agrandi ? '🔍 Page entière' : '🔍 Agrandir'}</button>
    <button class="btn" onclick={onfermer}>✕ Fermer</button>
    {#if taodbox.actif}<span class="aide">◀ ▶ pages · ▲ ▼ défiler · A agrandir · B revenir</span>{/if}
  </footer>
</section>

<style>
  .lecteur {
    position: fixed;
    inset: 0;
    z-index: 50;
    display: grid;
    grid-template-rows: auto 1fr auto;
    background: var(--voile-opaque);
    color: var(--encre-voile);
  }
  /* Sur la fiche : sous la barre de titre (BarreTitre.svelte : 34 u + son filet), pour garder réduire et déplacer. */
  .lecteur.sous-la-barre {
    top: calc(34 * var(--u) + 1px);
  }
  header,
  footer {
    display: flex;
    align-items: center;
    gap: calc(10 * var(--u));
    padding: calc(10 * var(--u)) calc(16 * var(--u));
  }
  header .numero,
  .aide {
    color: var(--encre-voile-dim);
    margin-left: auto;
  }
  footer {
    justify-content: center;
    flex-wrap: wrap;
  }
  .zone {
    overflow: auto;
    display: grid;
    justify-items: center;
    align-items: center;
    min-height: 0;
    position: relative;
  }
  .zone img {
    background: var(--fond-page);
    max-height: 100%;
    max-width: 100%;
    opacity: 0;
    transition: opacity 0.15s;
  }
  .zone.agrandi {
    align-items: start;
    justify-items: start;
  }
  .zone.agrandi img {
    max-height: none;
    max-width: none;
    width: 200%;
  }
  .zone img.charge {
    opacity: 1;
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
