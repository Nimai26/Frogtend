<script lang="ts">
  // La barre du haut, qui est AUSSI la barre de titre de la fenêtre (Frogtend dessine la sienne, comme LaunchBox) :
  // logo, menus en petites capitales, compte des jeux, profil, options, et les boutons de la fenêtre.
  // On la saisit pour déplacer la fenêtre ; un double-clic l'agrandit.
  import { isTauri } from '@tauri-apps/api/core';
  import { page } from '$app/state';
  import { goto } from '$app/navigation';
  import { choisir } from '$lib/dialogues/fenetres.svelte';
  import { etat, sortirDuProfil } from '$lib/etat.svelte';
  import { depuis, ludo, synchroniser, viderLudotheque } from '$lib/ludotheque/ludotheque.svelte';
  import { nombreEnCours } from '$lib/ludotheque/telechargements.svelte';

  const enCours = $derived(nombreEnCours());
  const profil = $derived(etat.profilOuvert);
  const chemin = $derived(page.url.pathname);

  const liens = [
    { href: '/', libelle: 'Ma ludothèque' },
    { href: '/catalogue', libelle: 'Catalogue Firehouse' },
    { href: '/boutiques', libelle: 'Boutiques' },
    { href: '/demander', libelle: 'Demander un jeu' },
    { href: '/assistant', libelle: 'Assistant' },
    { href: '/taodbox', libelle: '🛋 Taodbox' },
    { href: '/telechargements', libelle: 'Téléchargements' },
  ];

  async function fenetre(action: 'reduire' | 'agrandir' | 'fermer') {
    if (!isTauri()) return;
    const { getCurrentWindow } = await import('@tauri-apps/api/window');
    const w = getCurrentWindow();
    if (action === 'reduire') await w.minimize();
    else if (action === 'agrandir') await w.toggleMaximize();
    else await w.close();
  }

  async function menuProfil() {
    const c = await choisir(`👤 ${profil?.nom}`, [
      { valeur: 'profil', libelle: '👤 Mon profil', detail: 'nom, code PIN, jeton' },
      { valeur: 'changer', libelle: '🔁 Changer de profil', detail: 'revenir à « Qui joue ? »' },
    ]);
    if (c === 'profil') goto('/reglages?rubrique=profil');
    if (c === 'changer') {
      viderLudotheque();
      await sortirDuProfil();
      goto('/');
    }
  }
</script>

<header class="barre" data-tauri-drag-region>
  <a class="marque" href="/" aria-label="Frogtend — ma ludothèque">
    <img src="/grenouille.png" alt="" />Frogtend
  </a>

  {#if profil}
    <nav aria-label="Navigation principale">
      {#each liens as l (l.href)}
        <a class="lien" class:actif={chemin === l.href} href={l.href}>
          {l.libelle}
          {#if l.href === '/telechargements' && enCours > 0}<span class="nombre">{enCours}</span>{/if}
        </a>
      {/each}
      <button
        class="lien"
        onclick={() => synchroniser()}
        disabled={ludo.synchro.enCours}
        title="Relire le catalogue de Firehouse (dernière fois : {depuis(ludo.synchroniseeLe)})"
      >
        {ludo.synchro.enCours ? `Synchronisation… ${ludo.synchro.jeux || ''}` : 'Synchroniser'}
      </button>
    </nav>
  {/if}

  <span class="espace" data-tauri-drag-region>
    {#if profil && (chemin === '/' || chemin === '/catalogue') && ludo.totalLudotheque > 0}
      Affichage de {ludo.total.toLocaleString('fr-FR')} jeu(x) sur {ludo.totalLudotheque.toLocaleString('fr-FR')}
    {/if}
  </span>

  {#if etat.pc.firehouse.simule}
    <span class="tag warn" title="Des exemples, sans connexion à Firehouse">Mode simulé</span>
  {/if}
  {#if profil}
    <a class="lien" class:actif={chemin === '/reglages'} href="/reglages" title="Options">⚙ Options</a>
    <button class="lien profil" onclick={menuProfil}>👤 {profil.nom} ▾</button>
  {/if}

  <div class="fenetre">
    <button class="bouton-fenetre" aria-label="Réduire" title="Réduire" onclick={() => fenetre('reduire')}>
      <svg viewBox="0 0 10 10" aria-hidden="true"><path d="M0 5h10" /></svg>
    </button>
    <button class="bouton-fenetre" aria-label="Agrandir ou restaurer" title="Agrandir" onclick={() => fenetre('agrandir')}>
      <svg viewBox="0 0 10 10" aria-hidden="true"><rect x="0.5" y="0.5" width="9" height="9" /></svg>
    </button>
    <button class="bouton-fenetre fermer" aria-label="Fermer" title="Fermer" onclick={() => fenetre('fermer')}>
      <svg viewBox="0 0 10 10" aria-hidden="true"><path d="M0 0l10 10M10 0L0 10" /></svg>
    </button>
  </div>
</header>

<style>
  .barre {
    display: flex;
    align-items: stretch;
    height: calc(34 * var(--u));
    background: var(--panel);
    border-bottom: 1px solid var(--line);
    user-select: none;
    min-width: 0;
  }
  .marque {
    display: flex;
    align-items: center;
    gap: calc(7 * var(--u));
    padding: 0 calc(12 * var(--u)) 0 calc(10 * var(--u));
    font-weight: 700;
    font-size: calc(13 * var(--u));
    color: var(--ink);
    text-decoration: none;
  }
  .marque img {
    width: calc(20 * var(--u));
    height: calc(20 * var(--u));
  }
  nav {
    display: flex;
    align-items: stretch;
  }
  .lien {
    font: inherit;
    font-size: calc(11.5 * var(--u));
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.4px;
    color: var(--dim);
    background: transparent;
    border: none;
    border-bottom: 2px solid transparent;
    padding: 0 calc(10 * var(--u));
    display: inline-flex;
    align-items: center;
    gap: calc(6 * var(--u));
    text-decoration: none;
    cursor: pointer;
    white-space: nowrap;
  }
  .lien:hover:not(:disabled) {
    color: var(--ink);
    background: color-mix(in srgb, var(--ink) 6%, transparent);
  }
  .lien.actif {
    color: var(--ink);
    border-bottom-color: var(--accent);
  }
  .lien:disabled {
    cursor: default;
  }
  .lien:focus-visible,
  .bouton-fenetre:focus-visible {
    outline-offset: -2px;
  }
  .profil {
    text-transform: none;
    color: var(--ink);
  }
  .nombre {
    min-width: calc(16 * var(--u));
    padding: 0 calc(4 * var(--u));
    border-radius: 999px;
    background: var(--accent);
    color: var(--on-accent);
    font-size: calc(10 * var(--u));
    text-align: center;
  }
  .espace {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: calc(11.5 * var(--u));
    color: var(--dim);
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
    min-width: 0;
  }
  .tag {
    align-self: center;
    margin-right: calc(8 * var(--u));
  }
  .fenetre {
    display: flex;
  }
  .bouton-fenetre {
    width: calc(46 * var(--u));
    background: transparent;
    border: none;
    color: var(--ink);
    display: grid;
    place-items: center;
    cursor: pointer;
  }
  .bouton-fenetre svg {
    width: calc(10 * var(--u));
    height: calc(10 * var(--u));
    fill: none;
    stroke: currentColor;
    stroke-width: 1;
  }
  .bouton-fenetre:hover {
    background: color-mix(in srgb, var(--ink) 10%, transparent);
  }
  .fermer:hover {
    background: var(--err);
    color: var(--on-err);
  }
</style>
