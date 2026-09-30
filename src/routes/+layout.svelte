<script lang="ts">
  import '$lib/styles/base.css';
  import { onMount } from 'svelte';
  import { adresseFond } from '$lib/api';
  import BarreTitre from '$lib/BarreTitre.svelte';
  import Dialogues from '$lib/dialogues/Dialogues.svelte';
  import { demarrer, etat } from '$lib/etat.svelte';
  import { verifierMiseAJour } from '$lib/mises-a-jour';
  import { arreterSuivi, suivreTelechargements } from '$lib/ludotheque/telechargements.svelte';
  import { suivreParties } from '$lib/ludotheque/jeu.svelte';
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

  // Les téléchargements et les parties du profil ouvert : suivis tant qu'il est ouvert, oubliés quand il se ferme.
  $effect(() => {
    if (etat.profilOuvert) {
      suivreTelechargements();
      suivreParties();
    } else {
      arreterSuivi();
    }
  });
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

<div class="application">
  <BarreTitre />
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
  main {
    min-height: 0;
    overflow: auto;
  }
</style>
