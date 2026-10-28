<script lang="ts">
  import '$lib/styles/base.css';
  import { onMount, untrack } from 'svelte';
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { api } from '$lib/api';
  import { arreterManette, demarrerManette } from '$lib/taodbox/manette.svelte';
  import { isTauri } from '@tauri-apps/api/core';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { adresseFond } from '$lib/api';
  import BarreTitre from '$lib/BarreTitre.svelte';
  import Dialogues from '$lib/dialogues/Dialogues.svelte';
  import { demarrer, demarrerMenuJeu, etat } from '$lib/etat.svelte';
  import { verifierMiseAJour } from '$lib/mises-a-jour';
  import { arreterSuivi, suivreTelechargements } from '$lib/ludotheque/telechargements.svelte';
  import { attenteLancement, partie, proposerCouperGameBar, suivreParties } from '$lib/ludotheque/jeu.svelte';
  import EcranLancement from '$lib/ludotheque/EcranLancement.svelte';
  import { listen } from '@tauri-apps/api/event';
  import { traiterLiensNexus } from '$lib/mods/nexus.svelte';
  import ChoixProfil from '$lib/profils/ChoixProfil.svelte';
  import { auDemarrage } from '$lib/sauvegarde.svelte';
  import { gratuitsAuDemarrage } from '$lib/boutiques/gratuits.svelte';
  import { ecouterQuestionsRacine } from '$lib/mods/racine';

  let { children } = $props();

  // Vidéo de fond du skin (Firehouse) : masquée si elle ne peut pas être lue (hors ligne, mode simulé).
  let videoEnPanne = $state(false);
  $effect(() => {
    void etat.fondVideo;
    videoEnPanne = false;
  });

  /** La fenêtre du menu en jeu : seulement son contenu (ni barre, ni profils, ni suivis). */
  const estMenuJeu = isTauri() && getCurrentWindow().label === 'menu-jeu';

  onMount(async () => {
    if (estMenuJeu) {
      await demarrerMenuJeu();
      return;
    }
    await demarrer();
    // Lancé en Taodbox (`--taodbox`, par exemple depuis Sunshine) : directement le mode canapé.
    if (isTauri() && (await api.taodboxLance().catch(() => false))) await goto('/taodbox');
    // Vérification discrète : on ne propose que s'il y a une nouvelle version, jamais d'installation sans accord.
    verifierMiseAJour(true);
  });

  // Taodbox : plein écran, échelle ×2, la manette pilote tout (y compris le choix du profil et les fenêtres maison).
  const enTaodbox = $derived(!estMenuJeu && page.url.pathname.startsWith('/taodbox'));
  $effect(() => {
    if (estMenuJeu) return;
    const actif = enTaodbox;
    document.documentElement.toggleAttribute('data-taodbox', actif);
    if (actif) demarrerManette();
    else arreterManette();
    if (isTauri()) getCurrentWindow().setFullscreen(actif).catch(() => {});
  });

  // Les téléchargements et les parties du profil ouvert : suivis tant qu'il est ouvert, oubliés quand il se ferme.
  $effect(() => {
    if (estMenuJeu) return;
    if (etat.profilOuvert) {
      suivreTelechargements();
      suivreParties();
      untrack(() => auDemarrage());
      // Une seule fois par PC, la question du bouton Xbox (Game Bar de Windows) — sans attendre une sauvegarde.
      untrack(() => void proposerCouperGameBar().catch(() => {}));
      // Les jeux Steam installés sur ce PC deviennent des jeux du PC (lancés par Steam, parties suivies).
      untrack(() => void api.steamJeux().then(() => suivreTelechargements()).catch(() => {}));
      untrack(() => gratuitsAuDemarrage());
    } else {
      arreterSuivi();
    }
  });

  // Nexus (lot 0.82.0) : un lien « Mod Manager Download » arrivé est traité quand un profil est ouvert, hors partie et
  // hors Taodbox (sinon il attend : rien ne s'ouvre par-dessus un jeu ni au canapé).
  let courrierNexus = $state(0);
  onMount(() => {
    if (!isTauri() || estMenuJeu) return;
    const arret = listen('nxm-recu', () => courrierNexus++);
    // Un mod rangé dans un seul dossier : la question posée pendant son installation.
    const racine = ecouterQuestionsRacine();
    return () => {
      void arret.then((f) => f());
      void racine.then((f) => f());
    };
  });
  $effect(() => {
    void courrierNexus;
    if (estMenuJeu || !etat.profilOuvert || enTaodbox || partie.enJeu !== null || partie.installation !== null) return;
    untrack(() => traiterLiensNexus());
  });
</script>

{#if estMenuJeu}
  {#if etat.pret}{@render children()}{/if}
{:else}
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

<div class="application" inert={attenteLancement.montre}>
  {#if !enTaodbox}<BarreTitre />{/if}
  <main>
    {#if etat.pret}
      {#if !etat.profilOuvert}
        <ChoixProfil />
      {:else}
        {@render children()}
      {/if}
    {/if}
  </main>
</div>

{#if etat.profilOuvert}<EcranLancement />{/if}
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
  /* La fenêtre elle-même ne défile jamais : seul le contenu (main) le fait. */
  :global(html:has(.application)),
  :global(body:has(.application)) {
    overflow: hidden;
  }
  .application {
    height: 100vh;
    width: 100%;
    min-width: 0;
    display: grid;
    grid-template-rows: auto 1fr;
  }
  main {
    min-height: 0;
    overflow: auto;
  }
</style>
