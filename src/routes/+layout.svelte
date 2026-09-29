<script lang="ts">
  import '$lib/styles/base.css';
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { adresseFond } from '$lib/api';
  import Dialogues from '$lib/dialogues/Dialogues.svelte';
  import { choisir } from '$lib/dialogues/fenetres.svelte';
  import { demarrer, etat, sortirDuProfil } from '$lib/etat.svelte';
  import { depuis, ludo, synchroniser, viderLudotheque } from '$lib/ludotheque/ludotheque.svelte';
  import { verifierMiseAJour } from '$lib/mises-a-jour';
  import ChoixProfil from '$lib/profils/ChoixProfil.svelte';

  let { children } = $props();

  // Vidéo de fond du skin (Firehouse) : masquée si elle ne peut pas être lue (hors ligne, mode simulé).
  let videoEnPanne = $state(false);
  $effect(() => {
    void etat.fondVideo;
    videoEnPanne = false;
  });

  onMount(async () => {
    await demarrer();
    // Vérification discrète : on ne propose que s'il y a une nouvelle version, jamais d'installation sans accord.
    verifierMiseAJour(true);
  });

  const liens = [
    { href: '/', libelle: '🎮 Ludothèque' },
    { href: '/reglages', libelle: '⚙ Réglages' },
    { href: '/a-propos', libelle: 'ℹ À propos' },
  ];

  async function menuProfil() {
    const c = await choisir(`👤 ${etat.profilOuvert?.nom}`, [
      { valeur: 'profil', libelle: '👤 Mon profil', detail: 'nom, code PIN, jeton' },
      { valeur: 'changer', libelle: '🔁 Changer de profil', detail: 'revenir à « Qui joue ? »' },
    ]);
    if (c === 'profil') goto('/profil');
    if (c === 'changer') {
      viderLudotheque();
      await sortirDuProfil();
      goto('/');
    }
  }
</script>

{#if etat.fondVideo && !videoEnPanne}
  <video
    class="fond-video"
    src={adresseFond(etat.fondVideo)}
    autoplay
    muted
    loop
    playsinline
    aria-hidden="true"
    onerror={() => (videoEnPanne = true)}
  ></video>
  <div class="voile-video" aria-hidden="true"></div>
{/if}

{#if etat.pret}
  {#if !etat.profilOuvert}
    <ChoixProfil />
  {:else}
    <div class="application">
      <header class="barre">
        <a class="marque" href="/" aria-label="Frogtend — ludothèque"><img src="/grenouille.png" alt="" />Frogtend</a>
        <nav aria-label="Navigation principale">
          {#each liens as l (l.href)}
            <a class="lien" class:actif={page.url.pathname === l.href} href={l.href}>{l.libelle}</a>
          {/each}
          <button
            class="lien"
            onclick={() => synchroniser()}
            disabled={ludo.synchro.enCours}
            title="Dernière synchronisation : {depuis(ludo.synchroniseeLe)}"
          >
            {#if ludo.synchro.enCours}
              ⏳ Synchronisation… {ludo.synchro.jeux ? `${ludo.synchro.jeux.toLocaleString('fr-FR')} jeux` : ''}
            {:else}
              🔄 Synchroniser
            {/if}
          </button>
        </nav>
        <span class="compte muted">
          {#if page.url.pathname === '/' && ludo.totalLudotheque > 0}
            Affichage de {ludo.total.toLocaleString('fr-FR')} jeu(x) sur {ludo.totalLudotheque.toLocaleString('fr-FR')}
          {/if}
        </span>
        {#if etat.pc.firehouse.simule}
          <span class="tag warn" title="Les jeux affichés sont des exemples, pas ceux de Firehouse">Mode simulé</span>
        {/if}
        <button class="lien profil" onclick={menuProfil}>👤 {etat.profilOuvert.nom} ▾</button>
      </header>
      <main>
        {@render children()}
      </main>
    </div>
  {/if}
{/if}

<Dialogues />

<style>
  /* Derrière toute l'interface : la vidéo du skin, puis son voile de lisibilité (charte § 3). */
  .fond-video,
  .voile-video {
    position: fixed;
    inset: 0;
    width: 100%;
    height: 100%;
    z-index: -1;
    pointer-events: none;
  }
  .fond-video {
    object-fit: cover;
  }
  .voile-video {
    background: var(--voile-video);
  }
  .application {
    height: 100vh;
    display: grid;
    grid-template-rows: auto 1fr;
  }
  .barre {
    display: flex;
    align-items: center;
    gap: calc(6 * var(--u));
    padding: calc(4 * var(--u)) calc(12 * var(--u));
    background: var(--panel);
    border-bottom: 1px solid var(--line);
    backdrop-filter: blur(16px);
    min-width: 0;
  }
  .marque {
    display: flex;
    align-items: center;
    gap: calc(8 * var(--u));
    font-weight: 700;
    font-size: calc(16 * var(--u));
    color: var(--ink);
    text-decoration: none;
    margin-right: calc(10 * var(--u));
  }
  .marque img {
    width: calc(28 * var(--u));
    height: calc(28 * var(--u));
  }
  nav {
    display: flex;
    gap: calc(2 * var(--u));
    flex-wrap: wrap;
  }
  .lien {
    font: inherit;
    font-size: calc(12.5 * var(--u));
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.4px;
    color: var(--ink);
    background: transparent;
    border: 1px solid transparent;
    border-radius: calc(8 * var(--u));
    padding: calc(6 * var(--u)) calc(10 * var(--u));
    min-height: calc(36 * var(--u));
    display: inline-flex;
    align-items: center;
    gap: calc(6 * var(--u));
    text-decoration: none;
    cursor: pointer;
    white-space: nowrap;
  }
  .lien:hover:not(:disabled) {
    background: color-mix(in srgb, var(--ink) 8%, transparent);
  }
  .lien.actif {
    border-color: var(--accent);
    background: color-mix(in srgb, var(--accent) 16%, transparent);
  }
  .lien:disabled {
    opacity: 0.7;
    cursor: default;
  }
  .compte {
    flex: 1;
    text-align: center;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .profil {
    text-transform: none;
  }
  main {
    min-height: 0;
    overflow: auto;
  }
</style>
