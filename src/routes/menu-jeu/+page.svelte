<script lang="ts">
  // Le menu universel en jeu (lot 4 ter, lot OSD) : ouvert par-dessus le jeu par la touche du menu (Pause/Attn par
  // défaut) ou la manette. LE MÊME pour tous les émulateurs (Seb, 04/10) ; à la manette (croix, A, B), au clavier
  // (flèches, Entrée, Échap) et à la souris. Pensé pour tous, enfants compris : de gros boutons, peu de mots.
  import { onMount, tick } from 'svelte';
  import { isTauri } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { api, type Annexe, type EtatMenuJeu } from '$lib/api';
  import { confirmer, toast } from '$lib/dialogues/fenetres.svelte';
  import { motifDuRefus } from '$lib/dialogues/messages';
  import { appliquer, arreterManette, demarrerManette, fenetreOuverte, selectionner } from '$lib/taodbox/manette.svelte';
  import type { Commande } from '$lib/taodbox/navigation';

  let menu = $state<EtatMenuJeu | null>(null);
  let documents = $state<Annexe[] | null>(null);
  let lecture = $state<{ titre: string; texte: string } | null>(null);
  let liste = $state<HTMLElement | null>(null);
  /** Cheat Engine sur ce PC (fourni par Firehouse) : le menu propose de le brancher sur le jeu. */
  let cheatEngine = $state<string | null>(null);

  /** « Le jeu est figé » reçu du cœur pendant ce chargement : appliqué même si l'état lu est arrivé après. */
  let pauseRecue = false;

  async function charger() {
    menu = await api.menuJeuEtat().catch(() => null);
    if (menu && pauseRecue) menu.en_pause = true;
    // À la toute première ouverture, l'événement du cœur peut arriver avant la page : l'état dit aussi le mode.
    if (menu) modeTaodbox(menu.taodbox);
    cheatEngine = (await api.emulateursInstalles().catch(() => [])).find((e) => e.id === 'cheatengine')?.programme ?? null;
    documents = null;
    lecture = null;
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
    if (!cheatEngine) return;
    try {
      await api.cheatengineLancer(cheatEngine, undefined, true);
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

  async function voirDocuments() {
    if (!menu) return;
    try {
      const r = await api.fiche(menu.jeu);
      documents = r.fiche.annexes ?? [];
      await tick();
      selectionner(liste?.querySelector<HTMLButtonElement>('button'));
    } catch (e) {
      toast(`Impossible de lire les documents : ${motifDuRefus(e)}`, 'erreur');
    }
  }

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
    else if (documents) revenir(() => (documents = null));
    else reprendre();
  }

  /** Le premier bouton de la liste affichée devient la sélection (après un changement d'écran). */
  async function premierBouton() {
    await tick();
    selectionner(liste?.querySelector<HTMLButtonElement>('button'));
  }

  /** Revenir à l'écran précédent, avec une sélection (sinon plus rien n'est choisi et le 1er A ne fait rien). */
  function revenir(retour: () => void) {
    retour();
    premierBouton();
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
  {:else if documents}
    <header>
      <h1>📖 Documents</h1>
      <p class="muted">{menu.titre}</p>
    </header>
    <div class="liste" bind:this={liste}>
      {#each documents as d (d.i)}
        <button class="btn" onclick={() => ouvrir(d)}>{d.titre}</button>
      {:else}
        <p class="muted">Aucun document pour ce jeu.</p>
      {/each}
      <button class="btn" onclick={() => revenir(() => (documents = null))}>← Retour</button>
    </div>
  {:else}
    <header>
      <h1>{menu.titre}</h1>
      <p class="muted">
        {menu.en_pause ? '⏸ Le jeu est en pause.' : 'Le jeu continue pendant que ce menu est ouvert.'}
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
      <button class="btn" onclick={voirDocuments}>📖 Manuel et documents</button>
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

<style>
  .menu {
    box-sizing: border-box;
    height: 100vh;
    margin: 0;
    padding: calc(24 * var(--u));
    display: flex;
    flex-direction: column;
    gap: calc(16 * var(--u));
    overflow: hidden;
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
