<script lang="ts">
  // Le rappel sous ▶ Jouer (✅ Seb 11/11) : un jeu Jaguar (BigPEmu) dont une triche est cochée montre au démarrage la
  // fenêtre de triches de BigPEmu, qui attend la personne (A puis B). Dit AVANT le lancement : ensuite, BigPEmu couvre
  // l'écran. Rien s'il n'y a pas de triche cochée (ou pas BigPEmu).
  import { isTauri } from '@tauri-apps/api/core';
  import { api } from '$lib/api';
  import { partie } from './jeu.svelte';

  let { id }: { id: number } = $props();
  let cochees = $state(false);

  $effect(() => {
    const n = id;
    cochees = false;
    // Relu à la fin d'une partie (une triche cochée dans le menu en jeu vaut pour la suivante).
    if (!isTauri() || partie.enJeu !== null) return;
    let vivant = true;
    api
      .bigpemuTrichesCochees(n)
      .then((v) => {
        if (vivant) cochees = v;
      })
      .catch(() => {});
    return () => {
      vivant = false;
    };
  });
</script>

{#if cochees}
  <p class="tag rappel">🎯 Triches cochées : au démarrage, une petite fenêtre en anglais s’affiche. Attends quelques secondes, puis appuie sur <strong>A</strong>, puis sur <strong>B</strong> : le jeu démarre avec tes triches.</p>
{/if}

<style>
  .rappel {
    white-space: normal;
  }
</style>
