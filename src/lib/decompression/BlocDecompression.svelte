<script lang="ts">
  // 🗜 « Décompresser pour jouer » : proposé seulement quand le jeu est une archive qui contient une image disque.
  // Rien ne se fait sans le oui de la personne (taille, dossier, durée annoncés) ; l'archive est gardée.
  import { isTauri } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { api, taille } from '$lib/api';
  import { confirmer, toast } from '$lib/dialogues/fenetres.svelte';
  import { motifDuRefus } from '$lib/dialogues/messages';
  import { rechargerJeuxDuPc } from '$lib/ludotheque/telechargements.svelte';
  import { dossierDe, messageConfirmation, type ArchiveDeJeu } from './decompression';

  let { id, titre }: { id: number; titre: string } = $props();

  let archive = $state<ArchiveDeJeu | null>(null);
  let progres = $state<{ ecrits: number; total: number } | null>(null);

  $effect(() => {
    const jeu = id;
    archive = null;
    api.archiveDuJeu(jeu).then((a) => (archive = a)).catch(() => (archive = null));
  });

  async function decompresser() {
    const a = archive;
    if (!a) return;
    if (!a.deja) {
      const libre = await api.espaceLibre(dossierDe(a.archive)).catch(() => null);
      const oui = await confirmer(`🗜 Décompresser « ${titre} » pour pouvoir y jouer ?`, {
        message: messageConfirmation(a, libre),
        libelleValider: 'Décompresser',
      });
      if (!oui) return;
    }
    progres = { ecrits: 0, total: a.taille };
    const arreter = isTauri()
      ? await listen<{ jeu: number; ecrits: number; total: number }>('decompression', (e) => {
          if (e.payload.jeu === id) progres = { ecrits: e.payload.ecrits, total: e.payload.total };
        })
      : () => {};
    try {
      await api.decompresserJeu(id);
      await rechargerJeuxDuPc();
      toast(`✅ « ${titre} » est prêt : ▶ Jouer lance maintenant le jeu décompressé. L’archive est toujours là.`);
      archive = null;
    } catch (e) {
      toast(`Décompression impossible : ${motifDuRefus(e)}`, 'erreur');
    } finally {
      arreter();
      progres = null;
    }
  }
</script>

{#if archive}
  <div class="decompression cx-block">
    <p>
      🗜 Ce jeu est rangé dans une archive ({taille(archive.taille)} une fois décompressé) : l’émulateur ne peut pas le
      lire tel quel.
    </p>
    {#if progres}
      <span class="muted">Décompression… {taille(progres.ecrits)} / {taille(progres.total)}</span>
      <div class="bar"><span style="width: {progres.total ? Math.round((progres.ecrits / progres.total) * 100) : 0}%"></span></div>
    {:else}
      <button class="btn primary" onclick={decompresser}>
        {archive.deja ? '🗜 Utiliser le jeu déjà décompressé' : '🗜 Décompresser pour jouer'}
      </button>
    {/if}
  </div>
{/if}

<style>
  .decompression {
    display: grid;
    gap: calc(6 * var(--u));
  }
  p {
    margin: 0;
  }
</style>
