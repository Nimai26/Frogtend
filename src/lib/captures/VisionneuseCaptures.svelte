<script lang="ts">
  // La visionneuse des captures d'écran (✅ Seb 06/11) : la même carcasse que celle des images du jeu
  // (VisionneuseImages.svelte : croix = image précédente/suivante, LB/RB = bouton de la barre, B / Échap = revenir),
  // avec les actions de la personne : en faire le fond du jeu (son profil), supprimer (confirmé), ouvrir le dossier
  // (pas en Taodbox), et le diaporama (plein écran, défilement tout seul, B pour l'arrêter).
  import { onDestroy, onMount, tick, untrack } from 'svelte';
  import { api } from '$lib/api';
  import { retenirFocus } from '$lib/dialogues/focus';
  import { confirmer, toast } from '$lib/dialogues/fenetres.svelte';
  import { motifDuRefus } from '$lib/dialogues/messages';
  import { fenetreOuverte, parcourir, prendreLaMain, selectionner, taodbox } from '$lib/taodbox/manette.svelte';
  import { adresseCapture, dateCapture, DIAPORAMA_MS, type Capture } from './captures';

  let {
    captures,
    depart,
    titre,
    sousLaBarre = false,
    onfermer,
    onchange,
  }: {
    captures: Capture[];
    depart: string;
    titre: string;
    sousLaBarre?: boolean;
    onfermer: (id: string | null) => void;
    /** Une capture a été supprimée ou le fond a changé : la liste est à relire. */
    onchange?: () => void;
  } = $props();

  let liste = $state(untrack(() => [...captures]));
  let index = $state(untrack(() => Math.max(0, captures.findIndex((c) => c.id === depart))));
  let charge = $state(false);
  let echec = $state(false);
  let diaporama = $state(false);
  // Les petites images (Game Boy Advance…) ne font pas un fond net : le bouton le dit au lieu d'échouer.
  let tailles = $state<Record<string, [number, number]>>({});
  let precedent = $state<HTMLButtonElement>();
  let suivant = $state<HTMLButtonElement>();
  let fermeture = $state<HTMLButtonElement>();
  let arret = $state<HTMLButtonElement>();
  let barre = $state<HTMLElement>();
  let minuterie: ReturnType<typeof setInterval> | undefined;

  const courant = $derived(liste[index]);
  const adresse = $derived(courant ? adresseCapture(courant.id) : null);
  const estLeFond = $derived(!!courant?.est_le_fond);
  const tropPetite = $derived(!!courant && !!tailles[courant.id] && (tailles[courant.id][0] < 640 || tailles[courant.id][1] < 360));
  // En Taodbox (télé) : pas de dossier à ouvrir, l'aide de la manette. (Le menu en jeu met `taodbox.actif` même au
  // bureau : on lit le vrai mode de l'écran.)
  const surLaTele = () => document.documentElement.hasAttribute('data-taodbox');

  $effect(() => {
    void adresse;
    charge = false;
    echec = false;
  });

  async function aller(n: number) {
    if (!liste.length) return;
    // Le diaporama tourne en boucle ; à la main, on s'arrête aux bouts.
    const i = diaporama ? (n + liste.length) % liste.length : Math.max(0, Math.min(liste.length - 1, n));
    if (i === index) return;
    index = i;
    await tick();
    if ((document.activeElement as HTMLButtonElement | null)?.disabled) selectionner(index === 0 ? suivant : precedent);
  }

  const fermer = () => onfermer(courant?.id ?? null);

  async function demarrerDiaporama() {
    diaporama = true;
    minuterie = setInterval(() => aller(index + 1), DIAPORAMA_MS);
    await tick();
    selectionner(arret);
  }
  async function arreterDiaporama() {
    clearInterval(minuterie);
    minuterie = undefined;
    diaporama = false;
    await tick();
    selectionner(suivant && !suivant.disabled ? suivant : fermeture);
  }

  async function basculerFond() {
    const c = courant;
    if (!c) return;
    const actif = !estLeFond;
    try {
      await api.captureFond(c.id, actif);
      // Une seule capture est le fond de son jeu.
      liste = liste.map((x) => (x.jeu === c.jeu ? { ...x, est_le_fond: actif && x.id === c.id } : x));
      toast(actif ? `🖼 Cette capture est maintenant le fond de « ${c.titre} » (pour toi).` : `🖼 « ${c.titre} » retrouve son fond d’origine.`, 'ok');
      onchange?.();
    } catch (e) {
      toast(`Impossible : ${motifDuRefus(e)}`, 'erreur');
    }
  }

  async function supprimer() {
    const c = courant;
    if (!c) return;
    const oui = await confirmer('🗑 Supprimer cette capture ?', {
      message: `La capture de « ${c.titre} » prise ${dateCapture(c.quand)} sera effacée du disque.${c.est_le_fond ? ' C’était le fond du jeu : il retrouvera son fond d’origine.' : ''}`,
      libelleValider: 'Supprimer',
      danger: true,
    });
    if (!oui) return;
    try {
      await api.captureSupprimer(c.id);
      liste = liste.filter((x) => x.id !== c.id);
      onchange?.();
      if (!liste.length) return onfermer(null);
      index = Math.min(index, liste.length - 1);
      await tick();
      selectionner(suivant && !suivant.disabled ? suivant : precedent && !precedent.disabled ? precedent : fermeture);
    } catch (e) {
      toast(`Impossible de supprimer : ${motifDuRefus(e)}`, 'erreur');
    }
  }

  async function ouvrirDossier() {
    if (!courant) return;
    await api.captureOuvrirDossier(courant.id).catch((e) => toast(motifDuRefus(e), 'erreur'));
  }

  onMount(() => {
    selectionner([suivant, precedent].find((b) => b && !b.disabled) ?? fermeture);
    const rendre = prendreLaMain((c) => {
      if (c === 'gauche' || c === 'haut') aller(index - 1);
      else if (c === 'droite' || c === 'bas') aller(index + 1);
      else if (c === 'valider') return false;
      else if (c === 'precedent' || c === 'suivant') parcourir(barre, c === 'suivant' ? 1 : -1);
      else if (c === 'retour') (diaporama ? arreterDiaporama() : fermer());
      else return false;
      return true;
    });
    const touche = (e: KeyboardEvent) => {
      if (fenetreOuverte()) return;
      const fleches = !taodbox.actif;
      if (e.key === 'Escape') (diaporama ? arreterDiaporama() : fermer());
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
  onDestroy(() => clearInterval(minuterie));
</script>

<!-- Pas de role="dialog" : ce serait une « fenêtre maison » pour la manette, et la visionneuse ne recevrait plus rien. -->
<section class="visionneuse" class:sous-la-barre={sousLaBarre && !diaporama} class:diaporama aria-label={`Captures : ${titre}`} use:retenirFocus>
  {#if !diaporama}
    <header>
      <strong>📷 {titre}</strong>
      {#if courant}
        <span class="legende" aria-live="polite">{courant.titre} · {dateCapture(courant.quand)} · {index + 1}/{liste.length}{estLeFond ? ' · 🖼 fond du jeu' : ''}</span>
      {/if}
    </header>
  {/if}
  <div class="scene">
    {#if echec}
      <p class="note">Cette capture ne s’affiche pas.</p>
    {:else if adresse && courant}
      {#key adresse}
        <img
          src={adresse}
          alt={`Capture de ${courant.titre}`}
          class:charge
          onload={(e) => {
            charge = true;
            const i = e.currentTarget as HTMLImageElement;
            if (courant) tailles = { ...tailles, [courant.id]: [i.naturalWidth, i.naturalHeight] };
          }}
          onerror={() => (echec = true)}
        />
      {/key}
      {#if !charge}<p class="note attente" aria-live="polite">⏳ Chargement…</p>{/if}
    {/if}
  </div>
  <footer bind:this={barre}>
    {#if diaporama}
      <button class="btn" bind:this={arret} onclick={arreterDiaporama}>⏹ Arrêter le diaporama</button>
      <span class="aide">{index + 1}/{liste.length} · {taodbox.actif ? 'B' : 'Échap'} pour arrêter</span>
    {:else}
      <button class="btn" bind:this={precedent} disabled={index === 0} onclick={() => aller(index - 1)}>◀ Précédente</button>
      <button class="btn" bind:this={suivant} disabled={index >= liste.length - 1} onclick={() => aller(index + 1)}>Suivante ▶</button>
      {#if liste.length > 1}<button class="btn" onclick={demarrerDiaporama}>▶ Diaporama</button>{/if}
      {#if courant?.fond_possible}
        <button class="btn" disabled={!estLeFond && tropPetite} title={tropPetite ? 'Trop petite pour un fond net (moins de 640 × 360)' : ''} onclick={basculerFond}>
          {estLeFond ? '🖼 Rendre le fond d’origine' : tropPetite ? '🖼 Trop petite pour un fond' : '🖼 En faire le fond du jeu'}
        </button>
      {/if}
      {#if !surLaTele()}<button class="btn" onclick={ouvrirDossier}>📂 Ouvrir le dossier</button>{/if}
      <button class="btn" onclick={supprimer}>🗑 Supprimer</button>
      <button class="btn" bind:this={fermeture} onclick={fermer}>✕ Fermer</button>
      {#if surLaTele()}<span class="aide">◀ ▶ captures · LB RB choisir un bouton · A appuyer · B revenir</span>
      {:else}<span class="aide">← → captures · Échap fermer</span>{/if}
    {/if}
  </footer>
</section>

<style>
  .visionneuse {
    position: fixed;
    inset: 0;
    z-index: 50;
    display: grid;
    /* La scène prend la place qui reste, jamais plus : une grande capture ne pousse pas les boutons hors de la
       fenêtre (essai de Seb 07/11, menu en jeu). */
    grid-template-rows: auto minmax(0, 1fr) auto;
    background: var(--voile-opaque);
    color: var(--encre-voile);
  }
  .visionneuse.diaporama {
    grid-template-rows: minmax(0, 1fr) auto;
    background: var(--fond-media);
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
    flex-wrap: wrap;
  }
  .diaporama footer {
    background: var(--voile);
  }
  .legende,
  .aide {
    color: var(--encre-voile-dim);
    margin-left: auto;
  }
  footer {
    justify-content: center;
  }
  .scene {
    min-height: 0;
    display: grid;
    place-items: center;
    position: relative;
    padding: calc(8 * var(--u));
  }
  .diaporama .scene {
    padding: 0;
  }
  .diaporama .scene img {
    inset: 0;
    width: 100%;
    height: 100%;
  }
  /* L'image remplit la scène en gardant ses proportions (jamais sa taille d'origine). */
  .scene img {
    position: absolute;
    inset: calc(8 * var(--u));
    width: calc(100% - 16 * var(--u));
    height: calc(100% - 16 * var(--u));
    object-fit: contain;
    opacity: 0;
    transition: opacity 0.3s;
  }
  .scene img.charge {
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
