<script lang="ts">
  // 🧩 La page des mods d'un jeu Switch (✅ Seb 26/11 : « une fenêtre dédiée pour les mods d'un jeu, avec une barre de
  // recherche et un classement par type ») : `?jeu=<id>&titre=…`, servie par `/mods` (bureau) et `/taodbox/mods`
  // (canapé : l'adresse garde le mode Taodbox, sa manette et son plein écran). Modèle : PageSucces.
  // Onglets par sorte (LB / RB au canapé), filtres d'état, recherche (bureau seulement), paquets de 50 (bureau) ou 25
  // (canapé), une commande par ligne ; le détail en colonne au bureau, la ligne dépliée au canapé (A déplie, B replie).
  // La logique (accords, musiques, conflits, téléchargements, mises à jour, retraits) est celle de `ModsSwitchJeu`.
  import { page } from '$app/state';
  import { taille, type ModSwitchVu } from '$lib/api';
  import { etat } from '$lib/etat.svelte';
  import { attenteEmulateurSeul } from '$lib/ludotheque/jeu.svelte';
  import { compteRecherche } from '$lib/dialogues/filtre';
  import { fenetreOuverte, prendreLaMain, selectionner } from '$lib/taodbox/manette.svelte';
  import { tick, untrack } from 'svelte';
  import { lignesRemarque, motifInterrupteursBloques } from './contenus';
  import { ModsSwitchJeu } from './modsSwitch.svelte';
  import {
    FILTRES,
    ONGLETS,
    PAQUET_BUREAU,
    PAQUET_CANAPE,
    aTelecharger,
    chercherMods,
    dansOnglet,
    estMusique,
    etapeModsAnnoncee,
    indexRecherche,
    libelleFiltre,
    libelleOnglet,
    libellePreparer,
    libelleToutTelecharger,
    libelleVoirPlus,
    ligneAuteur,
    ligneConflitAvant,
    ligneEtatListe,
    ligneMusiques,
    ligneProgresMods,
    ligneSorte,
    lignesDernierePose,
    lignesFinMods,
    mentions,
    nombresParFiltre,
    nombresParOnglet,
    pasEncoreEssaye,
    pourcentEnCours,
    pourquoiVide,
    preparerVisible,
    raisonGrise,
    repondAuFiltre,
    resumeMods,
    tableConflits,
    texteInterrupteurMod,
    type FiltreEtat,
    type Onglet,
  } from './mods';

  const jeu = $derived(Number(page.url.searchParams.get('jeu')));
  const titre = $derived(page.url.searchParams.get('titre') ?? '');
  // Le canapé se lit dans l'adresse (comme le mode Taodbox lui-même, posé par la mise en page).
  const canape = $derived(page.url.pathname.startsWith('/taodbox'));
  const paquet = $derived(canape ? PAQUET_CANAPE : PAQUET_BUREAU);

  const ms = new ModsSwitchJeu();

  let onglet = $state<Onglet>('tout');
  let filtre = $state<FiltreEtat | null>(null);
  let recherche = $state('');
  let limite = $state(PAQUET_BUREAU);
  /** Les mods sur lesquels la personne vient d'agir : ils restent affichés, avec leur nouvel état, jusqu'au prochain
   * changement d'onglet, de filtre ou de recherche (jamais une ligne qui disparaît sous le doigt). */
  let gardes = $state<string[]>([]);
  /** Au bureau : le mod dont la fiche est dans la colonne de droite. */
  let choisi = $state<string | null>(null);
  /** Au canapé : les lignes dépliées (A sur le nom). */
  let deplies = $state<string[]>([]);
  /** Au bureau, le focus « en roulement » : la seule commande de la liste qui reçoit Tab. */
  let arret = $state<{ cle: string; sorte: 'nom' | 'commande' } | null>(null);
  let premierFocus = $state(false);

  let page_ = $state<HTMLElement | null>(null);
  let liste_ = $state<HTMLElement | null>(null);
  let retourBtn = $state<HTMLButtonElement | null>(null);
  let champ = $state<HTMLInputElement | null>(null);
  let boutonPlus = $state<HTMLButtonElement | null>(null);

  // Calculés une fois par lecture de la liste (363 mods pour Mario Kart 8 Deluxe).
  const mods = $derived(ms.mods);
  const index = $derived(indexRecherche(mods));
  const conflits = $derived(tableConflits(mods));
  const nombres = $derived(nombresParOnglet(mods));

  const dansLOnglet = $derived(mods.filter((x) => dansOnglet(x, onglet)));
  const nombresFiltres = $derived(nombresParFiltre(dansLOnglet));
  const filtres = $derived(filtre ? dansLOnglet.filter((x) => repondAuFiltre(x, filtre!)) : dansLOnglet);
  const trouves = $derived(!canape && recherche.trim() ? chercherMods(index, recherche) : null);
  const repondent = $derived(trouves ? filtres.filter((x) => trouves.has(x.cle)) : filtres);
  const visibles = $derived.by(() => {
    if (!gardes.length) return repondent;
    const ok = new Set(repondent.map((x) => x.cle));
    return mods.filter((x) => ok.has(x.cle) || gardes.includes(x.cle));
  });
  const montres = $derived(visibles.slice(0, limite));
  const aTelechargerIci = $derived(aTelecharger(repondent));
  const partiel = $derived(onglet !== 'tout' || !!filtre || !!trouves);
  const ongletsMontres = $derived(ONGLETS.filter((o) => o.cle === 'tout' || o.cle === onglet || nombres[o.cle] > 0));
  const filtresMontres = $derived(FILTRES.filter((f) => f.cle === filtre || nombresFiltres[f.cle] > 0));
  // Le mod choisi vient de la liste AFFICHÉE : sa ligne existe toujours (la fiche y renvoie par aria-describedby).
  const detailMod = $derived(canape ? null : (montres.find((x) => x.cle === choisi) ?? montres[0] ?? null));

  // La classe rend le focus dans la liste AFFICHÉE (filtrée), sinon sur « ← Retour » (après « Préparer » aussi).
  ms.ordreVisible = () => visibles.map((x) => x.cle);
  ms.lieuDuFocus = () => ({ conteneur: liste_, secours: retourBtn });

  // Un autre jeu ou un autre profil : tout est remis à zéro (jamais les mods d'un autre profil).
  $effect(() => {
    const [j, p] = [jeu, etat.profilOuvert?.id ?? null];
    untrack(() => {
      ms.ouvrir(j, titre, p);
      onglet = 'tout';
      filtre = null;
      recherche = '';
      choisi = null;
      deplies = [];
      arret = null;
      premierFocus = false;
      remettre();
    });
  });

  // Pendant un téléchargement : relu toutes les 2 s ; `actif` ne change qu'au début et à la fin (une seule minuterie).
  $effect(() => {
    if (!ms.actif) return;
    const g = untrack(() => ms.debutSuivi());
    const minuterie = setInterval(() => ms.releverSuivi(g), 2000);
    return () => clearInterval(minuterie);
  });

  // Au canapé : la sélection sur le premier mod une fois la liste lue (sinon « ← Retour »), pour la manette.
  $effect(() => {
    if (!canape || !ms.charge || premierFocus) return;
    premierFocus = true;
    tick().then(() => selectionner(page_?.querySelector<HTMLElement>('li[data-cle] [data-sorte="nom"]') ?? retourBtn));
  });

  // Au canapé : LB / RB changent d'onglet (en boucle) ; le reste va à la navigation habituelle.
  $effect(() => {
    if (!canape) return;
    return prendreLaMain((c) => {
      if (c !== 'precedent' && c !== 'suivant') return false;
      const l = ongletsMontres;
      const i = l.findIndex((o) => o.cle === onglet);
      const o = l[(i + (c === 'suivant' ? 1 : -1) + l.length) % l.length];
      changerOnglet(o.cle);
      tick().then(() => selectionner(page_?.querySelector<HTMLElement>(`[role="tab"][data-onglet="${o.cle}"]`)));
      return true;
    });
  });

  /** Un autre onglet, filtre ou texte : la liste repart du premier paquet, les mods gardés s'en vont. */
  function remettre() {
    limite = paquet;
    gardes = [];
  }
  function changerOnglet(o: Onglet) {
    onglet = o;
    remettre();
  }
  function changerFiltre(f: FiltreEtat) {
    filtre = filtre === f ? null : f;
    remettre();
  }

  function garder(cles: string[]) {
    const n = cles.filter((c) => !gardes.includes(c));
    if (n.length) gardes = [...gardes, ...n];
  }
  function basculer(x: ModSwitchVu) {
    garder([x.cle]);
    return ms.basculer(x);
  }
  function proposer(x: ModSwitchVu) {
    garder([x.cle]);
    return ms.proposer(x);
  }
  function proposerMiseAJour(x: ModSwitchVu) {
    garder([x.cle]);
    return ms.proposerMiseAJour(x);
  }
  function retirer(x: ModSwitchVu) {
    garder([x.cle]);
    return ms.retirer(x);
  }
  /** « Tout télécharger » : seulement l'onglet ou le filtre affiché ; les ⭐ recommandés cochés d'office. */
  function toutTelecharger() {
    const vus = aTelechargerIci;
    garder(vus.map((x) => x.cle));
    return ms.proposerTout(vus);
  }

  /** « Voir 50 de plus » : le focus va au premier mod ajouté (le clavier et la manette continuent de là). */
  async function voirPlus() {
    const premier = limite;
    limite += paquet;
    await tick();
    const cle = visibles[premier]?.cle;
    const e = cle ? liste_?.querySelector<HTMLElement>(`li[data-cle="${cle}"] [data-sorte="nom"]`) : null;
    if (e) selectionner(e);
  }

  type Commande = 'interrupteur' | 'telecharger' | 'maj' | 'grise' | 'encours' | 'attente' | null;
  /** UNE commande par ligne : l'interrupteur, « Télécharger », « Mettre à jour », ou ⛔ (et, pendant un
   * téléchargement, où il en est). */
  function commandeDe(x: ModSwitchVu): Commande {
    if (x.etat === 'incompatible') return 'grise';
    if (x.etat === 'telecharge') return 'interrupteur';
    if (ms.enCours(x)) return 'encours';
    if (ms.dansLaFile(x)) return 'attente';
    if (x.etat === 'revision_dispo') return x.servi ? 'maj' : 'interrupteur';
    return x.servi ? 'telecharger' : null;
  }
  /** La commande se prend-elle au clavier maintenant ? (Sinon Tab va au nom de la ligne.) */
  function commandeActive(x: ModSwitchVu, c: Commande): boolean {
    if (c === 'grise') return true;
    if (c === 'interrupteur') return !ms.bloque && (ms.basculement === null || ms.basculement === x.cle);
    if (c === 'telecharger' || c === 'maj') return !(ms.occupe || ms.bloque || ms.horsLigne);
    return false;
  }

  const arretEffectif = $derived.by(() => {
    const a = arret && montres.some((x) => x.cle === arret!.cle) ? arret : montres[0] ? { cle: montres[0].cle, sorte: 'nom' as const } : null;
    if (!a || a.sorte === 'nom') return a;
    const x = montres.find((y) => y.cle === a.cle)!;
    return commandeActive(x, commandeDe(x)) ? a : { cle: a.cle, sorte: 'nom' as const };
  });
  /** Au bureau, une seule commande de la liste reçoit Tab ; au canapé, la croix suit la géométrie. */
  function tab(x: ModSwitchVu, sorte: 'nom' | 'commande'): number | undefined {
    if (canape) return undefined;
    return arretEffectif?.cle === x.cle && arretEffectif.sorte === sorte ? 0 : -1;
  }

  function surFocus(x: ModSwitchVu, sorte: 'nom' | 'commande') {
    arret = { cle: x.cle, sorte };
    if (!canape) choisi = x.cle;
  }

  const elementDe = (cle: string, sorte: 'nom' | 'commande') =>
    liste_?.querySelector<HTMLElement>(`li[data-cle="${cle}"] [data-sorte="${sorte}"]:not([disabled])`) ?? null;

  /** Au bureau : haut / bas d'un mod à l'autre (la même colonne ; sous le dernier, « Voir N de plus »), Début / Fin au
   * premier et au dernier affichés, gauche / droite entre le nom et la commande. */
  function surTouche(e: KeyboardEvent, x: ModSwitchVu, sorte: 'nom' | 'commande') {
    if (canape) return;
    let cible: HTMLElement | null = null;
    const viser = (y: ModSwitchVu | undefined) => (y ? (elementDe(y.cle, sorte) ?? elementDe(y.cle, 'nom')) : null);
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      const i = montres.findIndex((y) => y.cle === x.cle);
      const suivant = montres[i + (e.key === 'ArrowDown' ? 1 : -1)];
      cible = suivant ? viser(suivant) : e.key === 'ArrowDown' ? boutonPlus : null;
    } else if (e.key === 'Home') cible = viser(montres[0]);
    else if (e.key === 'End') cible = viser(montres[montres.length - 1]);
    else if (e.key === 'ArrowRight' && sorte === 'nom') cible = elementDe(x.cle, 'commande');
    else if (e.key === 'ArrowLeft' && sorte === 'commande') cible = elementDe(x.cle, 'nom');
    else return;
    e.preventDefault();
    if (cible) {
      cible.focus();
      cible.scrollIntoView?.({ block: 'nearest' });
    }
  }

  /** Au bureau, la barre d'onglets suit le modèle ARIA : gauche / droite (en boucle), Début / Fin ; l'onglet atteint est
   * choisi tout de suite, et seul l'onglet choisi reçoit Tab. */
  async function surToucheOnglet(e: KeyboardEvent) {
    if (canape) return;
    const l = ongletsMontres;
    const i = l.findIndex((o) => o.cle === onglet);
    const j =
      e.key === 'ArrowRight' ? (i + 1) % l.length : e.key === 'ArrowLeft' ? (i - 1 + l.length) % l.length : e.key === 'Home' ? 0 : e.key === 'End' ? l.length - 1 : -1;
    if (j < 0) return;
    e.preventDefault();
    const o = l[j];
    changerOnglet(o.cle);
    await tick();
    page_?.querySelector<HTMLElement>(`[role="tab"][data-onglet="${o.cle}"]`)?.focus();
  }

  function surNom(x: ModSwitchVu) {
    if (!canape) {
      choisi = x.cle;
      return;
    }
    deplies = deplies.includes(x.cle) ? deplies.filter((c) => c !== x.cle) : [...deplies, x.cle];
  }

  async function replier(cle: string) {
    deplies = deplies.filter((c) => c !== cle);
    await tick();
    selectionner(elementDe(cle, 'nom'));
  }

  /** Échap (ou B à la manette) : replier la ligne dépliée (canapé), vider la recherche (bureau), sinon revenir — jamais
   * par-dessus une fenêtre maison (elle se ferme d'abord). */
  function touche(e: KeyboardEvent) {
    if (e.key !== 'Escape' || fenetreOuverte()) return;
    e.preventDefault();
    if (!canape && e.target === champ && recherche) {
      recherche = '';
      remettre();
      return;
    }
    if (canape) {
      const cle = (document.activeElement as HTMLElement | null)?.closest<HTMLElement>('li[data-cle]')?.dataset.cle;
      if (cle && deplies.includes(cle)) {
        replier(cle);
        return;
      }
    }
    history.back();
  }

  // Les identifiants HTML d'un mod (sa clé est « source-12 hex » : déjà sûre).
  const ident = (quoi: string, x: ModSwitchVu) => `mod-${quoi}-${x.cle}`;
</script>

<svelte:window onkeydown={touche} />

{#snippet interrupteur(x: ModSwitchVu, ou: 'ligne' | 'detail')}
  <!-- Un interrupteur : son texte dit l'état (jamais la couleur seule). Celui qui s'enregistre garde le focus
       (aria-disabled) ; les autres sont désactivés pendant ce temps. Le conflit est dit d'avance (aria-describedby). -->
  <button
    class="btn petit"
    role="switch"
    aria-checked={x.allume}
    aria-label={x.nom}
    aria-describedby={[ident(`etat-${ou}`, x), ligneConflitAvant(x, conflits.get(x.cle)) ? ident('conflit', x) : ''].filter(Boolean).join(' ')}
    aria-disabled={ms.bloque || ms.basculement !== null}
    disabled={ms.bloque || (ms.basculement !== null && ms.basculement !== x.cle)}
    data-commande={ou === 'ligne' ? '' : undefined}
    data-sorte={ou === 'ligne' ? 'commande' : undefined}
    tabindex={ou === 'ligne' ? tab(x, 'commande') : undefined}
    onclick={() => basculer(x)}
    onfocus={ou === 'ligne' ? () => surFocus(x, 'commande') : undefined}
    onkeydown={ou === 'ligne' ? (e) => surTouche(e, x, 'commande') : undefined}
  >
    <span id={ident(`etat-${ou}`, x)}>{texteInterrupteurMod(x, ms.pose)}</span>
  </button>
{/snippet}

{#snippet fiche(x: ModSwitchVu, ou: 'ligne' | 'detail')}
  <!-- Le détail d'un mod : jamais d'empreinte, de révision, de clé, de contenu technique ni d'amorces. -->
  {#if ou === 'detail'}
    <h2>{x.nom}{estMusique(x) ? ' 🎵' : ''}</h2>
    {#if ligneSorte(x)}<p class="sorte">{ligneSorte(x)}</p>{/if}
  {/if}
  {#if estMusique(x)}<p class="muted">🎵 Il change des musiques du jeu.</p>{/if}
  {#if x.description.trim()}<p class="description">{x.description}</p>{/if}
  <p class="muted">{ligneAuteur(x)}</p>
  {#each mentions(x) as l (l)}<p class="muted">{l}</p>{/each}
  {#if pasEncoreEssaye(x)}<p class="muted discret">pas encore essayé dans un émulateur</p>{/if}
  {#if ou === 'detail' && ligneConflitAvant(x, conflits.get(x.cle))}<p>{ligneConflitAvant(x, conflits.get(x.cle))}</p>{/if}
  {#if ou === 'detail' && x.etat === 'incompatible'}<p>{raisonGrise(x)}</p>{/if}
  {#if x.etat === 'revision_dispo' && x.servi}
    <!-- La ligne propose la mise à jour ; l'interrupteur de ce mod est ici. -->
    <div class="actions">{@render interrupteur(x, 'detail')}</div>
  {/if}
  {#if !x.servi}
    <div class="actions">
      <button class="btn petit danger" onclick={() => retirer(x)} disabled={ms.bloque || ms.occupe || ms.retrait !== null}>🗑 Retirer</button>
    </div>
  {/if}
{/snippet}

{#snippet ligne(x: ModSwitchVu)}
  {@const c = commandeDe(x)}
  {@const conflit = ligneConflitAvant(x, conflits.get(x.cle))}
  {@const ouverte = canape && deplies.includes(x.cle)}
  <!-- `tabindex="-1"` : le focus y revient après une action qui fait disparaître le bouton choisi. -->
  <li class:grise={x.etat === 'incompatible'} class:choisi={!canape && detailMod?.cle === x.cle} data-cle={x.cle} tabindex="-1">
    <div class="tete">
      <button
        class="nom"
        data-sorte="nom"
        tabindex={tab(x, 'nom')}
        aria-expanded={canape ? ouverte : undefined}
        aria-controls={canape && ouverte ? ident('fiche', x) : undefined}
        aria-current={!canape && detailMod?.cle === x.cle ? 'true' : undefined}
        onclick={() => surNom(x)}
        onfocus={() => surFocus(x, 'nom')}
        onkeydown={(e) => surTouche(e, x, 'nom')}
      >
        {x.nom}{#if estMusique(x)}<span aria-hidden="true">{' 🎵'}</span><span class="annonce">{' (mod de musique)'}</span>{/if}
      </button>
      <span class="action">
        {#if c === 'interrupteur'}
          {@render interrupteur(x, 'ligne')}
        {:else if c === 'telecharger'}
          <button
            class="btn petit"
            data-commande
            data-sorte="commande"
            tabindex={tab(x, 'commande')}
            aria-describedby={conflit ? ident('conflit', x) : undefined}
            onclick={() => proposer(x)}
            onfocus={() => surFocus(x, 'commande')}
            onkeydown={(e) => surTouche(e, x, 'commande')}
            disabled={ms.occupe || ms.bloque || ms.horsLigne}>⬇ Télécharger ({taille(x.taille)})</button
          >
        {:else if c === 'maj'}
          <!-- L'état se lit aussi (sans être une commande : l'interrupteur est dans la fiche). -->
          <span class="muted etat-maj">{texteInterrupteurMod(x, ms.pose)}</span>
          <button
            class="btn petit"
            data-commande
            data-sorte="commande"
            tabindex={tab(x, 'commande')}
            onclick={() => proposerMiseAJour(x)}
            onfocus={() => surFocus(x, 'commande')}
            onkeydown={(e) => surTouche(e, x, 'commande')}
            disabled={ms.occupe || ms.bloque || ms.horsLigne}>⬇ Mettre à jour ({taille(x.taille)})</button
          >
        {:else if c === 'grise'}
          <!-- Jamais posé (l'émulateur l'ignorerait en silence) : un bouton focalisable qui dit pourquoi. -->
          <button
            class="btn petit"
            data-commande
            data-sorte="commande"
            tabindex={tab(x, 'commande')}
            aria-disabled="true"
            aria-describedby={ident('raison', x)}
            onfocus={() => surFocus(x, 'commande')}
            onkeydown={(e) => surTouche(e, x, 'commande')}>⛔ Ne va pas avec ta version</button
          >
        {:else if c === 'encours'}
          <span>⬇ {pourcentEnCours(ms.suivi)} %</span>
        {:else if c === 'attente'}
          <span class="muted">⏳ En attente</span>
        {/if}
      </span>
    </div>
    {#if ligneSorte(x)}<p class="sorte">{ligneSorte(x)}</p>{/if}
    {#if conflit}<p id={ident('conflit', x)}>{conflit}</p>{/if}
    {#if x.etat === 'incompatible'}<p id={ident('raison', x)}>{raisonGrise(x)}</p>{/if}
    {#if ouverte}
      <div class="fiche" id={ident('fiche', x)}>{@render fiche(x, 'ligne')}</div>
    {:else if x.description.trim()}
      <p class="description replie">{x.description}</p>
    {/if}
  </li>
{/snippet}

<div class="page-mods" class:canape bind:this={page_}>
  <header>
    <button class="btn" bind:this={retourBtn} onclick={() => history.back()}>← Retour</button>
    <h1>🧩 Mods{titre ? ` — ${titre}` : ''}</h1>
  </header>
  {#if !ms.charge}
    <p class="muted">Chargement…</p>
  {:else if !ms.liste}
    <p class="muted">📴 La liste des mods n’a pas pu être lue.</p>
  {:else if ms.liste.etat === 'rien'}
    <p class="muted">Ce jeu n’a pas de mods sur ce PC.</p>
  {:else}
    <section class="etat" aria-label="Où en sont les mods">
      <p class="resume">🧩 <span aria-live="polite">{resumeMods(mods)}</span></p>
      {#if ligneEtatListe(ms.liste)}<p class="muted">{ligneEtatListe(ms.liste)}</p>{/if}
      {#each lignesRemarque(ms.liste.remarque) as l (l)}<p class="muted">{l}</p>{/each}
      {#each lignesDernierePose(ms.pose) as l, i (i)}<p class={i ? 'muted' : ''}>{l}</p>{/each}
      <p class="muted">Ces mods viennent de Firehouse. Chacun vaut pour ce jeu, avec Eden comme avec Ryubing, pour toute la maison.</p>
      {#if ms.bloque}<p class="muted">{motifInterrupteursBloques(attenteEmulateurSeul())}</p>{/if}
      {#if ligneMusiques(ms.liste)}<p>{ligneMusiques(ms.liste)}</p>{/if}
      <p class="muted" aria-live="polite">{ms.regardeTaille ? '⏳ Je regarde la taille des musiques…' : ''}</p>
      {#if preparerVisible(ms.liste)}
        <div class="actions">
          <button class="btn petit" onclick={() => ms.preparer()} disabled={ms.bloque || ms.preparation || ms.occupe || ms.regardeTaille}>
            {libellePreparer(ms.liste.musiques?.taille_connue)}
          </button>
        </div>
      {/if}
      <!-- Le lecteur d'écran n'annonce que l'ÉTAPE (le mod en cours) puis la fin, jamais le pourcentage qui bouge. -->
      <span class="annonce" aria-live="polite">{etapeModsAnnoncee(ms.suivi) || ms.annonceFin}</span>
      {#if ligneProgresMods(ms.suivi)}<p>{ligneProgresMods(ms.suivi)}</p>{/if}
      {#each lignesFinMods(ms.suivi) as l, i (i)}<p class="muted">{l}</p>{/each}
    </section>

    {#if mods.length}
      {#if !canape}
        <div class="recherche">
          <input
            class="filtre"
            type="search"
            bind:this={champ}
            placeholder="🔎 Chercher un mod (nom, description, auteur)…"
            aria-label="Chercher un mod"
            bind:value={recherche}
            oninput={remettre}
          />
          <!-- Sans résultat, un seul message : celui de la liste vide (qui dit pourquoi). -->
          <p class="muted" aria-live="polite">{trouves && repondent.length ? compteRecherche(repondent.length, filtres.length) : ''}</p>
        </div>
      {/if}
      <div class="onglets" role="tablist" aria-label="Sortes de mods">
        {#each ongletsMontres as o (o.cle)}
          <button
            class="btn petit"
            role="tab"
            data-onglet={o.cle}
            aria-selected={onglet === o.cle}
            tabindex={canape ? undefined : onglet === o.cle ? 0 : -1}
            onclick={() => changerOnglet(o.cle)}
            onkeydown={surToucheOnglet}
          >
            {libelleOnglet(o.cle, nombres[o.cle])}
          </button>
        {/each}
      </div>
      {#if filtresMontres.length}
        <div class="filtres" role="group" aria-label="Filtrer par état">
          {#each filtresMontres as f (f.cle)}
            <button class="btn petit" aria-pressed={filtre === f.cle} onclick={() => changerFiltre(f.cle)}>{libelleFiltre(f.cle, nombresFiltres[f.cle])}</button>
          {/each}
        </div>
      {/if}
      {#if aTelechargerIci.length >= 2 && !ms.horsLigne}
        <div class="actions">
          <button class="btn petit" onclick={toutTelecharger} disabled={ms.occupe || ms.bloque}>{libelleToutTelecharger(aTelechargerIci, partiel)}</button>
        </div>
      {/if}
    {/if}

    <div class="corps">
      <div class="colonne">
        {#if !visibles.length}
          <p class="muted">{pourquoiVide(onglet, filtre, canape ? '' : recherche, mods.length)}</p>
        {:else}
          <ul bind:this={liste_} aria-label="Mods">
            {#each montres as x (x.cle)}{@render ligne(x)}{/each}
          </ul>
          {#if visibles.length > limite}
            <button class="btn petit" bind:this={boutonPlus} onclick={voirPlus}>{libelleVoirPlus(limite, visibles.length, paquet)}</button>
          {/if}
        {/if}
      </div>
      {#if detailMod}
        <aside class="detail" aria-label="Fiche du mod">{@render fiche(detailMod, 'detail')}</aside>
      {/if}
    </div>
  {/if}
  {#if canape}
    <footer class="aide">🎮 Croix pour choisir · A pour allumer, télécharger ou lire la fiche · LB / RB pour changer d’onglet · B pour revenir</footer>
  {/if}
</div>

<style>
  .page-mods {
    display: grid;
    gap: calc(12 * var(--u));
    padding: calc(20 * var(--u));
    max-width: calc(1200 * var(--u));
    margin: 0 auto;
    width: 100%;
    box-sizing: border-box;
    align-content: start;
    overflow: auto;
    height: 100%;
  }
  /* Au canapé : marge de sécurité de la télé (charte § 7) ; la page défile, la manette suit le focus. */
  .page-mods.canape {
    padding: 5vh 5vw;
    height: 100vh;
    max-width: calc(1000 * var(--u));
  }
  header {
    display: flex;
    align-items: center;
    gap: calc(14 * var(--u));
    flex-wrap: wrap;
  }
  h1 {
    margin: 0;
    font-size: calc(22 * var(--u));
    overflow-wrap: anywhere;
  }
  h2 {
    margin: 0;
    font-size: calc(17 * var(--u));
    overflow-wrap: anywhere;
  }
  .etat {
    display: grid;
    gap: calc(4 * var(--u));
  }
  .resume {
    font-weight: 600;
  }
  .filtre {
    width: 100%;
    box-sizing: border-box;
    min-height: var(--cible-min);
  }
  .recherche {
    display: grid;
    gap: calc(4 * var(--u));
  }
  .onglets,
  .filtres,
  .actions {
    display: flex;
    gap: calc(6 * var(--u));
    flex-wrap: wrap;
  }
  /* L'onglet choisi et le filtre allumé : la sélection de la charte (jamais la couleur seule : aria-selected / aria-pressed
     sont dits, et le contour reste celui du focus). */
  [role='tab'][aria-selected='true'],
  [aria-pressed='true'] {
    background: color-mix(in srgb, var(--accent) 20%, transparent);
    border-color: var(--accent);
    font-weight: 600;
  }
  .corps {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 0.8fr);
    gap: calc(16 * var(--u));
    align-items: start;
  }
  .canape .corps {
    grid-template-columns: minmax(0, 1fr);
  }
  .colonne {
    display: grid;
    gap: calc(10 * var(--u));
    min-width: 0;
  }
  /* La fiche du mod choisi reste en vue pendant que la liste défile. */
  .detail {
    position: sticky;
    top: 0;
    display: grid;
    gap: calc(6 * var(--u));
    padding: calc(12 * var(--u));
    border: 1px solid var(--line);
    border-radius: calc(10 * var(--u));
    background: var(--panel);
    min-width: 0;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: calc(8 * var(--u));
  }
  li {
    display: grid;
    gap: calc(4 * var(--u));
    padding: calc(6 * var(--u)) calc(8 * var(--u)) calc(8 * var(--u));
    border-bottom: 1px solid var(--line);
    border-left: calc(3 * var(--u)) solid transparent;
  }
  li:last-child {
    border-bottom: none;
  }
  li.choisi {
    border-left-color: var(--accent);
    background: color-mix(in srgb, var(--accent) 8%, transparent);
  }
  li:focus {
    outline: calc(2 * var(--u)) solid var(--accent);
    outline-offset: calc(2 * var(--u));
  }
  .tete {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: calc(8 * var(--u));
    align-items: center;
  }
  /* Le nom : un bouton qui se lit comme un titre (la fiche au bureau, la ligne dépliée au canapé). */
  .nom {
    justify-self: start;
    background: none;
    border: none;
    padding: calc(4 * var(--u)) 0;
    min-height: var(--cible-min);
    color: var(--ink);
    font: inherit;
    font-weight: 600;
    text-align: left;
    overflow-wrap: anywhere;
    cursor: pointer;
  }
  .nom:focus-visible {
    outline: calc(2 * var(--u)) solid var(--accent);
    outline-offset: calc(2 * var(--u));
  }
  .action {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    flex-wrap: wrap;
    gap: calc(6 * var(--u));
  }
  .grise .nom {
    color: var(--dim);
  }
  .sorte {
    font-weight: 600;
  }
  .description {
    overflow-wrap: anywhere;
    white-space: pre-line;
  }
  .replie {
    display: -webkit-box;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    overflow: hidden;
    color: var(--dim);
  }
  .fiche {
    display: grid;
    gap: calc(4 * var(--u));
  }
  .discret {
    font-style: italic;
  }
  /* Les petits boutons de la page : la cible garde 40 px au moins (charte § 4). */
  .petit {
    min-height: var(--cible-min);
    padding: calc(5 * var(--u)) calc(11 * var(--u));
    font-size: calc(12 * var(--u));
  }
  /* Un bouton « aria-disabled » reste focalisable (il dit pourquoi) mais se montre éteint. */
  button[aria-disabled='true']:not([role='switch']) {
    opacity: 0.6;
    cursor: default;
  }
  /* Lu par un lecteur d'écran, jamais affiché. */
  .annonce {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }
  .aide {
    color: var(--dim);
    text-align: center;
  }
  p {
    margin: 0;
  }
</style>
