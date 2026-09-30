<script lang="ts">
  // Le menu universel en jeu (lot 4 ter) : ouvert par-dessus le jeu par la touche du menu (Pause/Attn par défaut).
  // Pensé pour tous, enfants compris : de gros boutons, peu de mots, les flèches et Entrée, Échap pour reprendre.
  import { onMount, tick } from 'svelte';
  import { isTauri } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { api, type Annexe, type EtatMenuJeu } from '$lib/api';
  import { confirmer, toast } from '$lib/dialogues/fenetres.svelte';
  import { motifDuRefus } from '$lib/dialogues/messages';

  let menu = $state<EtatMenuJeu | null>(null);
  let documents = $state<Annexe[] | null>(null);
  let lecture = $state<{ titre: string; texte: string } | null>(null);
  let liste = $state<HTMLElement | null>(null);

  async function charger() {
    menu = await api.menuJeuEtat().catch(() => null);
    documents = null;
    lecture = null;
    await tick();
    liste?.querySelector<HTMLButtonElement>('button')?.focus();
  }

  onMount(() => {
    charger();
    if (!isTauri()) return;
    const arret = listen('menu-jeu', () => charger());
    return () => {
      arret.then((f) => f());
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
      liste?.querySelector<HTMLButtonElement>('button')?.focus();
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
      } else {
        await api.ouvrirAnnexe(menu.jeu, a.i);
      }
    } catch (e) {
      toast(`Impossible d’ouvrir « ${a.titre} » : ${motifDuRefus(e)}`, 'erreur');
    }
  }

  /** Flèches haut/bas entre les boutons ; Échap : revenir en arrière, ou reprendre le jeu. */
  function touche(e: KeyboardEvent) {
    if (document.querySelector('[role="dialog"]')) return; // une fenêtre de confirmation a la main
    if (e.key === 'Escape') {
      e.preventDefault();
      if (lecture) lecture = null;
      else if (documents) documents = null;
      else reprendre();
      return;
    }
    if (e.key !== 'ArrowDown' && e.key !== 'ArrowUp') return;
    const boutons = [...(liste?.querySelectorAll<HTMLButtonElement>('button:not([disabled])') ?? [])];
    if (!boutons.length) return;
    e.preventDefault();
    const i = boutons.indexOf(document.activeElement as HTMLButtonElement);
    const n = e.key === 'ArrowDown' ? (i + 1) % boutons.length : (i - 1 + boutons.length) % boutons.length;
    boutons[n].focus();
  }

  const a = (nom: string) => menu?.actions.includes(nom);
</script>

<svelte:window onkeydown={touche} />

<div class="menu cx-card">
  {#if !menu}
    <p class="muted">Aucune partie en cours.</p>
  {:else if lecture}
    <header>
      <h1>📖 {lecture.titre}</h1>
    </header>
    <div class="lecture">{lecture.texte}</div>
    <div class="liste" bind:this={liste}>
      <button class="btn" onclick={() => (lecture = null)}>← Retour</button>
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
      <button class="btn" onclick={() => (documents = null)}>← Retour</button>
    </div>
  {:else}
    <header>
      <h1>{menu.titre}</h1>
      <p class="muted">
        {menu.en_pause ? '⏸ Le jeu est en pause.' : 'Le jeu continue pendant que ce menu est ouvert.'}
      </p>
    </header>
    <div class="liste" bind:this={liste}>
      <button class="btn primary" onclick={reprendre}>▶ Reprendre</button>
      {#if a('sauver')}<button class="btn" onclick={() => action('sauver')}>💾 Sauvegarde rapide</button>{/if}
      {#if a('charger')}
        <button class="btn" onclick={() => action('charger', '📂 Revenir à la sauvegarde rapide ? Ce qui s’est passé depuis sera perdu.')}>
          📂 Charger la sauvegarde rapide
        </button>
      {/if}
      {#if a('disque')}<button class="btn" onclick={() => action('disque')}>💿 Disque suivant</button>{/if}
      <button class="btn" onclick={voirDocuments}>📖 Manuel et documents</button>
      {#if a('reset')}
        <button class="btn" onclick={() => action('reset', '🔄 Recommencer le jeu depuis le début ? La partie non sauvegardée sera perdue.')}>
          🔄 Recommencer
        </button>
      {/if}
      <button class="btn danger" onclick={quitter}>⏹ Quitter le jeu</button>
    </div>
    <p class="aide muted">↑ ↓ pour choisir · Entrée pour valider · Échap pour reprendre</p>
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
