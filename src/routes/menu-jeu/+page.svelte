<script lang="ts">
  // Le menu universel en jeu (lot 4 ter, lot OSD) : ouvert par-dessus le jeu par la touche du menu (Pause/Attn par
  // défaut) ou la manette. LE MÊME pour tous les émulateurs (Seb, 04/10) ; à la manette (croix, A, B), au clavier
  // (flèches, Entrée, Échap) et à la souris. Pensé pour tous, enfants compris : de gros boutons, peu de mots.
  import { onMount, tick } from 'svelte';
  import { isTauri } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { adresseFondDuJeu, api, type Annexe, type DocumentJeu, type EtatMenuJeu } from '$lib/api';
  import LecteurLivret from '$lib/ludotheque/LecteurLivret.svelte';
  import LecteurMedia from '$lib/ludotheque/LecteurMedia.svelte';
  import { documentsDuMenu, documentsMontres, libelleDocument, ouvrirDocument, texteChargement, type DocumentOuvert } from '$lib/ludotheque/documents';
  import { arreterDocument, suiviDocument } from '$lib/ludotheque/documentsProgres.svelte';
  import { confirmer, toast } from '$lib/dialogues/fenetres.svelte';
  import { motifDuRefus } from '$lib/dialogues/messages';
  import { accordCheatEngine } from '$lib/mods/cheatengine';
  import ListeSucces from '$lib/succes/ListeSucces.svelte';
  import type { ListeSucces as ListeSuccesDuJeu } from '$lib/succes/liste';
  import { appliquer, arreterManette, demarrerManette, fenetreOuverte, selectionner } from '$lib/taodbox/manette.svelte';
  import type { Commande } from '$lib/taodbox/navigation';

  let menu = $state<EtatMenuJeu | null>(null);
  /** Le fond du menu : un visuel du jeu (Seb, 04/10 : « seul le fond peut être différent en fonction des jeux ») ;
   * sans visuel (jeu importé, hors ligne sans copie), celui du skin. */
  let fondJeu = $state<string | null>(null);
  /** Le visuel n'apparaît (en fondu, avec son voile) qu'une fois chargé : jamais celui du jeu précédent, ni un voile
   * noir sur le skin le temps d'apprendre qu'il n'y en a pas. */
  let fondCharge = $state(false);
  let documents = $state<Annexe[] | null>(null);
  // Les succès du jeu (lot C5), lus à la demande.
  let succes = $state<ListeSuccesDuJeu | null>(null);
  let chargementSucces = $state(false);
  // L'écran des options propres à l'émulateur (lot OSD, i : images par seconde, avance rapide…).
  let options = $state(false);
  // L'avance rapide demandée depuis ce menu, pour CE jeu (une bascule : Frogtend ne connaît pas l'état réel de
  // l'émulateur ; il rappelle seulement ce qui a été demandé ici, pour qu'elle ne soit pas oubliée).
  let rapidePour = $state<number | null>(null);
  const rapide = $derived(!!menu && rapidePour === menu.jeu);
  let lecture = $state<{ titre: string; texte: string } | null>(null);
  // Les documents du jeu (contrat 1.7 : livret, musique, vidéo), lus dans les lecteurs maison (pilotables à la manette).
  let docs = $state<DocumentJeu[]>([]);
  let ouvert = $state<DocumentOuvert | null>(null);
  let ouverture = $state<number | null>(null);
  let liste = $state<HTMLElement | null>(null);
  /** Cheat Engine sur ce PC (fourni par Firehouse) : le menu propose de le brancher sur le jeu. */
  let cheatEngine = $state<string | null>(null);

  /** « Le jeu est figé » reçu du cœur pendant ce chargement : appliqué même si l'état lu est arrivé après. */
  let pauseRecue = false;

  // Le menu est-il montré ? (Un document qui arrive après sa fermeture ne s'ouvre pas dans une fenêtre cachée.)
  let visible = true;

  async function charger() {
    visible = true;
    menu = await api.menuJeuEtat().catch(() => null);
    if (menu && pauseRecue) menu.en_pause = true;
    // Le fond du jeu : sa fiche (un import relié montre celle de Firehouse).
    const fiche = menu ? (menu.fiche ?? (menu.jeu > 0 ? menu.jeu : null)) : null;
    const adresse = fiche ? adresseFondDuJeu(fiche) : null;
    if (adresse !== fondJeu) fondCharge = false;
    fondJeu = adresse;
    // À la toute première ouverture, l'événement du cœur peut arriver avant la page : l'état dit aussi le mode.
    if (menu) modeTaodbox(menu.taodbox);
    cheatEngine = (await api.emulateursInstalles().catch(() => [])).find((e) => e.id === 'cheatengine')?.programme ?? null;
    documents = null;
    succes = null;
    lecture = null;
    ouvert = null;
    options = false;
    await tick();
    selectionner(liste?.querySelector<HTMLButtonElement>('button'));
  }

  /** En Taodbox, le menu est à l'échelle de la télé (×2, plein écran). */
  function modeTaodbox(actif: boolean) {
    document.documentElement.toggleAttribute('data-taodbox', actif);
  }

  /** Les flèches du clavier ; la MANETTE vient du cœur (événement `menu-manette`) : la Gamepad API ne répond pas dans
   * une fenêtre qui vient d'apparaître par-dessus un jeu (essai de Seb, 0.45.0). */
  function relancerManette() {
    arreterManette();
    demarrerManette({ manette: false });
  }

  onMount(() => {
    charger();
    demarrerManette({ manette: false });
    if (!isTauri()) return arreterManette;
    const arrets = [
      listen<{ taodbox?: boolean } | null>('menu-jeu', (e) => {
        pauseRecue = false;
        fondJeu = null;
        fondCharge = false;
        modeTaodbox(!!e.payload?.taodbox);
        relancerManette();
        charger();
      }),
      // La manette lue par le cœur pendant la partie (étape 3 de l'OSD) : les mêmes commandes.
      listen<Commande>('menu-manette', (e) => appliquer(e.payload)),
      // Un jeu PC figé par le cœur (option du jeu) : il est vraiment en pause.
      listen<boolean>('menu-jeu-pause', (e) => {
        pauseRecue = e.payload;
        if (menu) menu.en_pause = e.payload;
      }),
      // Le menu est caché par le cœur (le jeu s'est fermé, Alt+F4…) : rien ne doit continuer de jouer derrière.
      listen('menu-jeu-cache', () => {
        visible = false;
        ouvert = null;
        documents = null;
        succes = null;
        lecture = null;
        options = false;
      }),
    ];
    return () => {
      arreterManette();
      arrets.forEach((a) => a.then((f) => f()));
    };
  });

  async function reprendre() {
    await api.menuJeuReprendre().catch((e) => toast(`Impossible de reprendre : ${motifDuRefus(e)}`, 'erreur'));
  }

  async function action(nom: string, question?: string) {
    if (question && !(await confirmer(question, { libelleValider: 'Oui' }))) return;
    try {
      await api.menuJeuAction(nom);
    } catch (e) {
      toast(`Impossible : ${motifDuRefus(e)}`, 'erreur');
    }
  }

  async function brancherCheatEngine() {
    if (!cheatEngine || !menu) return;
    const accepte = await accordCheatEngine(menu.jeu);
    if (accepte === null) return;
    try {
      await api.cheatengineLancer(cheatEngine, undefined, true, accepte);
      toast('🧰 Cheat Engine se branche sur le jeu.');
    } catch (e) {
      toast(`Impossible : ${motifDuRefus(e)}`, 'erreur');
    }
  }

  async function quitter() {
    if (!(await confirmer(`⏹ Quitter « ${menu?.titre} » ?`, { message: 'Pense à sauvegarder ta partie dans le jeu si besoin.', libelleValider: 'Quitter', danger: true })))
      return;
    try {
      const n = await api.menuJeuQuitter();
      if (n === 0) toast('La fenêtre du jeu est introuvable : ferme-le toi-même.', 'alerte');
    } catch (e) {
      toast(`Impossible de quitter : ${motifDuRefus(e)}`, 'erreur');
    }
  }

  /** Les succès du jeu en détail (lot C5) : lus à la demande (rien pendant la partie sans qu'on le demande). */
  async function voirSucces() {
    if (!menu || chargementSucces) return;
    chargementSucces = true;
    try {
      const l = await api.succesJeu(menu.jeu);
      if (!l) {
        toast('Aucun succès connu pour ce jeu sur ce PC.', 'neutre');
        return;
      }
      succes = l;
      await tick();
      selectionner(liste?.querySelector<HTMLButtonElement>('button'));
    } catch (e) {
      toast(`Impossible de lire les succès : ${motifDuRefus(e)}`, 'erreur');
    } finally {
      chargementSucces = false;
    }
  }

  async function voirDocuments() {
    if (!menu) return;
    try {
      const r = await api.fiche(menu.jeu);
      docs = documentsMontres(await api.documents(menu.jeu).catch(() => []));
      documents = r.fiche.annexes ?? [];
      await tick();
      selectionner(liste?.querySelector<HTMLButtonElement>('button'));
    } catch (e) {
      toast(`Impossible de lire les documents : ${motifDuRefus(e)}`, 'erreur');
    }
  }

  /** « Arrêter » cliqué pendant le téléchargement : l'arrêt voulu n'est pas une erreur. */
  let arretDemande = false;
  function arreterDoc(jeu: number, doc: number) {
    arretDemande = true;
    arreterDocument(jeu, doc);
  }

  async function ouvrirDoc(d: DocumentJeu) {
    if (!menu || ouverture !== null) return;
    const jeu = menu.jeu;
    ouverture = d.id;
    arretDemande = false;
    // Le bouton du document est désactivé pendant le téléchargement : la sélection va sur « Arrêter » (manette).
    if (!d.garde) {
      await tick();
      const arret = liste?.querySelector<HTMLButtonElement>('[data-arret]');
      if (arret) selectionner(arret);
    }
    let lecteur = false;
    try {
      const o = await ouvrirDocument(jeu, d);
      // Revenu au menu (ou une autre partie) pendant le téléchargement : on n'ouvre rien par-dessus.
      if (!visible || !documents || menu?.jeu !== jeu) return;
      ouvert = o;
      lecteur = true;
      docs = docs.map((x) => (x.id === d.id ? { ...x, garde: true } : x));
    } catch (e) {
      if (arretDemande) toast('⏹ Téléchargement arrêté : il reprendra où il en était.', 'neutre');
      else toast(`Impossible d’ouvrir ce document : ${motifDuRefus(e)}`, 'erreur');
    } finally {
      ouverture = null;
      // Rien d'ouvert (arrêt, erreur) : la sélection revient sur le bouton du document.
      if (!lecteur && visible && documents) {
        await tick();
        selectionner(liste?.querySelector<HTMLButtonElement>(`[data-doc="${d.id}"]`) ?? liste?.querySelector<HTMLButtonElement>('button'));
      }
    }
  }

  /** Le lecteur se ferme : la sélection revient sur le bouton du document. */
  async function fermerDocument() {
    const doc = ouvert?.doc;
    ouvert = null;
    await tick();
    selectionner(liste?.querySelector<HTMLButtonElement>(`[data-doc="${doc}"]`) ?? liste?.querySelector<HTMLButtonElement>('button'));
  }

  const docsMenu = $derived(documentsDuMenu(docs, !!menu?.en_pause));

  // Le jeu repart (plus en pause) pendant une musique ou une vidéo : elle s'arrête (décision de Seb, 16/10).
  $effect(() => {
    if (ouvert && !ouvert.livret && !menu?.en_pause) ouvert = null;
  });

  async function ouvrir(a: Annexe) {
    if (!menu) return;
    try {
      if (a.texte) {
        const r = await api.annexeTexte(menu.jeu, a.i, a.cle);
        lecture = { titre: r.titre ?? a.titre, texte: r.texte ?? '' };
        await premierBouton();
      } else {
        await api.ouvrirAnnexe(menu.jeu, a.i);
      }
    } catch (e) {
      toast(`Impossible d’ouvrir « ${a.titre} » : ${motifDuRefus(e)}`, 'erreur');
    }
  }

  /** Échap (ou B) : revenir en arrière, ou reprendre le jeu. Les flèches sont gérées par le module de la manette
   * (une seule navigation, la même qu'en Taodbox). */
  function touche(e: KeyboardEvent) {
    if (fenetreOuverte()) return; // une fenêtre de confirmation a la main (Échap/B la ferme, sans reprendre le jeu)
    if (e.key !== 'Escape') return;
    e.preventDefault();
    if (lecture) revenir(() => (lecture = null));
    else if (succes) revenir(() => (succes = null), '[data-ecran="succes"]');
    else if (documents) revenir(() => (documents = null), '[data-ecran="documents"]');
    else if (options) revenir(() => (options = false), '[data-ecran="options"]');
    else reprendre();
  }

  /** Le premier bouton de la liste affichée (ou celui que désigne `cible`) devient la sélection. */
  async function premierBouton(cible?: string) {
    await tick();
    selectionner((cible ? liste?.querySelector<HTMLButtonElement>(cible) : null) ?? liste?.querySelector<HTMLButtonElement>('button'));
  }

  /** Revenir à l'écran précédent, avec une sélection (sinon plus rien n'est choisi et le 1er A ne fait rien) : le
   * bouton d'où l'on venait s'il est donné (sinon « Reprendre », et un A réflexe relancerait le jeu). */
  function revenir(retour: () => void, cible?: string) {
    retour();
    premierBouton(cible);
  }

  /** La souris : un VRAI mouvement déplace la sélection (pas le contenu qui bouge sous un curseur immobile). */
  function survol(e: PointerEvent) {
    if (!e.movementX && !e.movementY) return;
    const b = (e.target as HTMLElement).closest<HTMLElement>('.liste .btn');
    if (b && b !== document.activeElement) selectionner(b);
  }

  const a = (nom: string) => menu?.actions.includes(nom);
</script>

<svelte:window onkeydown={touche} />

{#if fondJeu}
  <!-- Le visuel du jeu derrière le menu, sous le voile de lisibilité de la charte ; absent, l'image s'efface et le
       fond du skin reste. Le menu, lui, garde toujours la même forme. -->
  <img
    class="fond-jeu"
    class:visible={fondCharge}
    src={fondJeu}
    alt=""
    onload={() => (fondCharge = true)}
    onerror={() => ((fondJeu = null), (fondCharge = false))}
  />
  {#if fondCharge}<div class="voile-jeu" aria-hidden="true"></div>{/if}
{/if}
<!-- Le survol ne fait que déplacer la sélection : le clavier et la manette ont déjà leur propre navigation. -->
<!-- svelte-ignore a11y_no_static_element_interactions, a11y_mouse_events_have_key_events -->
<div class="menu cx-card" onpointermove={survol}>
  {#if !menu}
    <p class="muted">Aucune partie en cours.</p>
  {:else if lecture}
    <header>
      <h1>📖 {lecture.titre}</h1>
    </header>
    <div class="lecture">{lecture.texte}</div>
    <div class="liste" bind:this={liste}>
      <button class="btn" onclick={() => revenir(() => (lecture = null))}>← Retour</button>
    </div>
  {:else if options}
    <header>
      <h1>⚙ Options du jeu</h1>
      <p class="muted">Chaque option se fait dans le jeu : le menu se ferme et le jeu reprend.</p>
    </header>
    <div class="liste" bind:this={liste}>
      {#if a('fps')}<button class="btn" onclick={() => action('fps')}>📊 Compteur de fluidité (images par seconde) : afficher ou masquer</button>{/if}
      {#if a('rapide')}
        <button class="btn" onclick={async () => { const jeu = menu?.jeu ?? null; await action('rapide'); rapidePour = rapide ? null : jeu; }}>
          {rapide ? '⏩ Couper l’avance rapide' : '⏩ Avance rapide (le jeu va plus vite)'}
        </button>
      {/if}
      {#if a('capture')}<button class="btn" onclick={() => action('capture')}>📷 Photo de l’écran (rangée dans le dossier de l’émulateur)</button>{/if}
      {#if a('muet')}<button class="btn" onclick={() => action('muet')}>🔇 Son : couper ou remettre</button>{/if}
      <button class="btn" onclick={() => revenir(() => (options = false))}>← Retour</button>
    </div>
  {:else if succes}
    <header>
      <h1>🏆 Succès</h1>
      <p class="muted">{menu.titre}</p>
    </header>
    <div class="liste" bind:this={liste}>
      <button class="btn" onclick={() => revenir(() => (succes = null), '[data-ecran="succes"]')}>← Retour</button>
      <ListeSucces liste={succes} />
      <button class="btn" onclick={() => revenir(() => (succes = null), '[data-ecran="succes"]')}>← Retour</button>
    </div>
  {:else if documents}
    <header>
      <h1>📖 Documents</h1>
      <p class="muted">{menu.titre}</p>
      {#if docsMenu.length < docs.length}
        <p class="muted">🎵 La musique et la vidéo du jeu : seulement quand le jeu est en pause.</p>
      {/if}
    </header>
    <div class="liste" bind:this={liste}>
      {#each docsMenu as d (d.id)}
        {@const l = libelleDocument(d)}
        <button class="btn" data-doc={d.id} disabled={ouverture !== null} onclick={() => ouvrirDoc(d)}>
          {l.titre} <span class="muted">{ouverture === d.id ? texteChargement(suiviDocument.courant) : l.detail}</span>
        </button>
        {#if ouverture === d.id && !d.garde && menu}
          <button class="btn" data-arret onclick={() => menu && arreterDoc(menu.jeu, d.id)}>⏹ Arrêter</button>
        {/if}
      {/each}
      {#each documents as d (d.i)}
        <button class="btn" onclick={() => ouvrir(d)}>{d.titre}</button>
      {:else}
        {#if !docs.length}<p class="muted">Aucun document pour ce jeu.</p>{/if}
      {/each}
      <button class="btn" onclick={() => revenir(() => (documents = null))}>← Retour</button>
    </div>
  {:else}
    <header>
      <h1>{menu.titre}</h1>
      <p class="muted">
        {menu.en_pause ? '⏸ Le jeu est en pause.' : 'Le jeu continue pendant que ce menu est ouvert.'}
        {#if rapide}<br />⏩ L’avance rapide est activée (Options du jeu pour la couper).{/if}
      </p>
    </header>
    <div class="liste" bind:this={liste}>
      <button class="btn" onclick={reprendre}>▶ Reprendre</button>
      {#if a('sauver')}<button class="btn" onclick={() => action('sauver')}>💾 Sauvegarde rapide</button>{/if}
      {#if a('charger')}
        <button class="btn" onclick={() => action('charger', '📂 Revenir à la sauvegarde rapide ? Ce qui s’est passé depuis sera perdu.')}>
          📂 Charger la sauvegarde rapide
        </button>
      {/if}
      {#if a('disque')}<button class="btn" onclick={() => action('disque')}>💿 Disque suivant</button>{/if}
      <button class="btn" data-ecran="documents" onclick={voirDocuments}>📖 Manuel et documents</button>
      <!-- Un jeu Steam (Galaxy en attendant Steam), ou un jeu d'émulateur (RetroAchievements, lot C3). -->
      {#if (menu.jeu < 0 && menu.plateforme === 'Windows') || menu.plateforme !== 'Windows'}<button class="btn" data-ecran="succes" disabled={chargementSucces} onclick={voirSucces}>🏆 Succès{chargementSucces ? ' (chargement…)' : ''}</button>{/if}
      {#if ['fps', 'rapide', 'capture', 'muet'].some((o) => a(o))}
        <button class="btn" data-ecran="options" onclick={() => revenir(() => (options = true))}>⚙ Options du jeu</button>
      {/if}
      {#if cheatEngine}<button class="btn" onclick={brancherCheatEngine}>🧰 Cheat Engine</button>{/if}
      {#if a('reset')}
        <button class="btn" onclick={() => action('reset', '🔄 Recommencer le jeu depuis le début ? La partie non sauvegardée sera perdue.')}>
          🔄 Recommencer
        </button>
      {/if}
      <button class="btn" onclick={quitter}>⏹ Quitter le jeu</button>
    </div>
    <p class="aide muted">
      Choisir : croix ou ↑ ↓ · Valider : A ou Entrée · Reprendre : B ou Échap
    </p>
  {/if}
</div>

{#if ouvert?.livret && menu}
  <LecteurLivret jeu={menu.jeu} doc={ouvert.doc} pages={ouvert.pages} titre={menu.titre} onfermer={fermerDocument} />
{:else if ouvert && !ouvert.livret && menu}
  <LecteurMedia jeu={menu.jeu} doc={ouvert.doc} video={ouvert.video} titre={menu.titre} onfermer={fermerDocument} />
{/if}

<style>
  .fond-jeu {
    position: fixed;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
    background: var(--fond-media);
    /* 1 000 px au plus, étirés sur la télé : un léger flou le fait paraître voulu. */
    filter: blur(calc(4 * var(--u)));
    opacity: 0;
    /* Coupé avec les animations réduites (Réglages ▸ Apparence). */
    transition: opacity calc(var(--duree) * 2);
    pointer-events: none;
  }
  .fond-jeu.visible {
    opacity: 1;
  }
  .voile-jeu {
    position: fixed;
    inset: 0;
    background: var(--voile);
    pointer-events: none;
  }
  /* Le menu est TOUJOURS la même carte du thème, au centre, avec la même marge des quatre côtés (Seb, 04/10 : « le
   * MÊME pour tous ») ; autour, le visuel du jeu ou, à défaut, le fond du skin. */
  .menu {
    --marge: calc(16 * var(--u));
    position: relative;
    box-sizing: border-box;
    width: min(calc(640 * var(--u)), calc(100% - 2 * var(--marge)));
    height: calc(100vh - 2 * var(--marge));
    margin: var(--marge) auto;
    padding: calc(24 * var(--u));
    display: flex;
    flex-direction: column;
    gap: calc(16 * var(--u));
    overflow: hidden;
  }
  :global([data-taodbox]) .menu {
    --marge: max(calc(16 * var(--u)), 5vh);
  }
  header h1 {
    margin: 0;
    font-size: calc(22 * var(--u));
  }
  header p {
    margin: calc(4 * var(--u)) 0 0;
  }
  .liste {
    display: grid;
    gap: calc(8 * var(--u));
    overflow: auto;
  }
  /* Tous les boutons ont le même style au repos ; SEUL le bouton choisi se remplit de la couleur d'accent (Seb, 05/10 :
   * le contour passait derrière les boutons voisins, et « Reprendre », orange par son style, semblait toujours choisi).
   * Le remplissage est à l'intérieur du bouton : la liste qui défile ne peut pas le cacher. */
  .liste .btn:focus,
  .liste .btn:focus-visible {
    outline: none;
  }
  .liste .btn:focus,
  .liste :global(.btn[data-choisi]) {
    background: var(--accent);
    color: var(--on-accent);
    border-color: var(--accent);
    box-shadow: inset 0 0 0 calc(2 * var(--u)) var(--on-accent);
  }
  .liste .btn {
    min-height: calc(48 * var(--u));
    font-size: calc(17 * var(--u));
    justify-content: flex-start;
    text-align: left;
  }
  .lecture {
    flex: 1;
    overflow: auto;
    white-space: pre-wrap;
    line-height: 1.5;
  }
  .aide {
    margin: auto 0 0;
    text-align: center;
  }
</style>
