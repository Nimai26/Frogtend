<script lang="ts">
  // La grille des jaquettes. Un clic (ou Entrée) sélectionne un jeu ; les flèches se déplacent dans la grille ;
  // le reste de la liste se charge en arrivant en bas.
  import { goto } from '$app/navigation';
  import { estJeuDeBoutique, ouvrirJeuDeBoutique } from '$lib/boutiques/jeu-boutique';
  import type { JeuResume } from '$lib/api';
  import { etat } from '$lib/etat.svelte';
  import Jaquette from './Jaquette.svelte';
  import { chargerSuite, ludo } from './ludotheque.svelte';
  import { tele } from './telechargements.svelte';

  const pourcent = (recus: number, total: number) => (total > 0 ? Math.floor((recus / total) * 100) : 0);

  /** Ce qu'on écrit sous la jaquette d'un jeu pas encore complet. */
  function libelleEtat(etat: string, p: number): string {
    if (etat === 'en_cours') return '⬇ ' + p + ' %';
    if (etat === 'pause') return '⏸ En pause';
    if (etat === 'attente') return '⏳ En attente';
    return '⛔ Échec';
  }

  let grille = $state<HTMLElement>();
  let sentinelle = $state<HTMLElement>();

  const r = $derived(etat.profil.ludotheque);

  function sousTitre(j: JeuResume): string {
    switch (r.sousTitre) {
      case 'developpeur':
        return j.developpeur ?? '';
      case 'editeur':
        return j.editeur ?? '';
      case 'annee':
        return j.annee ? String(j.annee) : '';
      case 'plateforme':
        return j.plateforme;
      default:
        return '';
    }
  }

  // Chargement du lot suivant quand la fin de la grille devient visible.
  $effect(() => {
    if (!sentinelle) return;
    const obs = new IntersectionObserver((e) => e[0].isIntersecting && chargerSuite(), { rootMargin: '600px' });
    obs.observe(sentinelle);
    return () => obs.disconnect();
  });

  /** Nombre de cartes par ligne, d'après leur position à l'écran. */
  function colonnes(cartes: HTMLElement[]): number {
    if (cartes.length === 0) return 1;
    const haut = cartes[0].offsetTop;
    const n = cartes.findIndex((c) => c.offsetTop !== haut);
    return n === -1 ? cartes.length : n;
  }

  function surTouche(e: KeyboardEvent, index: number) {
    const pas: Record<string, number> = { ArrowRight: 1, ArrowLeft: -1 };
    const cartes = Array.from(grille?.querySelectorAll<HTMLElement>('.carte') ?? []);
    const n = colonnes(cartes);
    const deplacement = pas[e.key] ?? (e.key === 'ArrowDown' ? n : e.key === 'ArrowUp' ? -n : 0);
    if (!deplacement) return;
    e.preventDefault();
    const cible = cartes[Math.min(cartes.length - 1, Math.max(0, index + deplacement))];
    cible?.focus();
    cible?.scrollIntoView({ block: 'nearest' });
  }
</script>

<section class="zone" aria-label="Jeux">
  {#if ludo.jeux.length === 0 && !ludo.chargement}
    <div class="vide">
      {#if ludo.totalLudotheque === 0 && ludo.espace === 'ludotheque'}
        <p class="grand">📭 Ta ludothèque est vide pour l’instant.</p>
        <p class="muted">Ouvre le « 🛒 Catalogue Firehouse », choisis un jeu et « ➕ Mets-le dans ta ludothèque ».</p>
        <a class="btn primary" href="/catalogue">🛒 Ouvrir le catalogue Firehouse</a>
      {:else if ludo.totalLudotheque === 0}
        <p class="grand">📭 Le catalogue n’a pas encore été lu.</p>
        <p class="muted">
          {ludo.synchro.enCours
            ? 'Lecture du catalogue de Firehouse en cours…'
            : 'Clique sur « 🔄 Synchroniser », en haut (il faut une connexion à Internet).'}
        </p>
      {:else}
        <p class="grand">🔍 Aucun jeu ne correspond.</p>
        <p class="muted">Essaie un autre mot, une autre plateforme, ou retire le filtre de genre.</p>
      {/if}
    </div>
  {:else}
    <ul
      class="grille"
      bind:this={grille}
      style="--taille: {r.tailleJaquette}"
      data-densite={etat.profil.apparence.densite}
    >
      {#each ludo.jeux as j, i (j.id)}
        <li>
          <button
            class="carte"
            class:selectionnee={ludo.selection?.id === j.id}
            aria-pressed={ludo.selection?.id === j.id}
            onclick={() => (ludo.selection = j)}
            ondblclick={() => (estJeuDeBoutique(j) ? ouvrirJeuDeBoutique(j) : goto(`/jeu/${j.id}`))}
            onfocus={() => (ludo.selection = j)}
            onkeydown={(e) => surTouche(e, i)}
          >
            <Jaquette
              id={j.id}
              titre={j.titre}
              plateforme={j.plateforme}
              disponible={j.jaquette}
              largeur={r.tailleJaquette}
            />
            {#if tele.jeux[j.id] && tele.jeux[j.id].etat !== 'telecharge'}
              {@const t = tele.jeux[j.id]}
              <span class="progres" title="{pourcent(t.recus, t.total)} %">
                <span class="bar"><span style="width: {pourcent(t.recus, t.total)}%"></span></span>
                <span class="muted">{libelleEtat(t.etat, pourcent(t.recus, t.total))}</span>
              </span>
            {:else if ludo.espace === 'catalogue' && tele.jeux[j.id]}
              <span class="tag ok badge">✅ Dans ta ludothèque</span>
            {/if}
            <span class="titre">{j.titre}</span>
            {#if sousTitre(j)}<span class="sous-titre">{sousTitre(j)}</span>{/if}
          </button>
        </li>
      {/each}
    </ul>
    <div bind:this={sentinelle} class="sentinelle" aria-hidden="true"></div>
    {#if ludo.jeux.length < ludo.total}
      <p class="muted suite">
        {(ludo.total - ludo.jeux.length).toLocaleString('fr-FR')} autre(s) jeu(x) — fais défiler pour les voir.
      </p>
    {/if}
  {/if}
</section>

<style>
  .zone {
    overflow-y: auto;
    min-height: 0;
    padding: calc(6 * var(--u)) calc(10 * var(--u)) calc(20 * var(--u));
  }
  .grille {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(calc(var(--taille) * var(--u)), 1fr));
    gap: calc(26 * var(--u)) calc(22 * var(--u));
  }
  .grille[data-densite='compacte'] {
    gap: calc(12 * var(--u)) calc(10 * var(--u));
  }
  .carte {
    font: inherit;
    color: var(--ink);
    background: transparent;
    border: 1px solid transparent;
    border-radius: calc(12 * var(--u));
    padding: calc(8 * var(--u));
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: calc(4 * var(--u));
    text-align: left;
    cursor: pointer;
    transition:
      transform var(--duree),
      background var(--duree),
      border-color var(--duree);
  }
  .carte:hover {
    background: color-mix(in srgb, var(--ink) 5%, transparent);
    transform: translateY(-3px);
  }
  .carte.selectionnee {
    background: color-mix(in srgb, var(--accent) 20%, transparent);
    border-color: var(--accent);
  }
  .progres {
    display: grid;
    gap: calc(3 * var(--u));
    margin-top: calc(4 * var(--u));
    font-size: calc(11 * var(--u));
  }
  .badge {
    justify-self: start;
    margin-top: calc(4 * var(--u));
  }
  .titre {
    margin-top: calc(6 * var(--u));
    font-size: calc(14 * var(--u));
    line-height: 1.25;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .sous-titre {
    font-size: calc(12 * var(--u));
    color: var(--dim);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .vide {
    height: 100%;
    display: grid;
    place-content: center;
    text-align: center;
    gap: calc(6 * var(--u));
  }
  .grand {
    font-size: calc(18 * var(--u));
    margin: 0;
  }
  .sentinelle {
    height: 1px;
  }
  .suite {
    text-align: center;
  }
</style>
