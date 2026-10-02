<script lang="ts">
  // 🏆 Les succès d'un jeu, dans le panneau de détails : Steam pour ses jeux, RetroAchievements pour les jeux émulés
  // présents sur le PC (version compatible ou non, et laquelle l'est).
  import { api, type JeuResume } from '$lib/api';
  import { resumeRetro, type SuccesRetro } from './succes.svelte';

  let { jeu, surPc }: { jeu: JeuResume; surPc: boolean } = $props();

  let steam = $state<[number, number] | null>(null);
  let ra = $state<SuccesRetro | null>(null);
  let chargement = $state(false);

  $effect(() => {
    const j = jeu;
    const pc = surPc;
    steam = null;
    ra = null;
    let annule = false;
    chargement = true;
    (async () => {
      try {
        if (j.source === 'steam' && j.cle_boutique) {
          const s = await api.succesSteam(j.cle_boutique).catch(() => null);
          if (!annule) steam = s;
        } else if (pc) {
          const r = await api.succesRetro(j.id).catch(() => null);
          if (!annule) ra = r;
        }
      } finally {
        if (!annule) chargement = false;
      }
    })();
    return () => {
      annule = true;
    };
  });

  const resume = $derived(ra ? resumeRetro(ra) : null);
</script>

{#if steam}
  <div class="succes cx-block">
    <strong>🏆 Succès Steam</strong>
    <span>{steam[0]} / {steam[1]} obtenus</span>
    <div class="bar"><span style="width: {steam[1] ? Math.round((steam[0] / steam[1]) * 100) : 0}%"></span></div>
  </div>
{:else if ra && resume}
  <div class="succes cx-block">
    <strong>🏆 RetroAchievements</strong>
    {#if ra.jeu && ra.jeu.succes}
      <span>{ra.jeu.obtenus} / {ra.jeu.succes} succès · {ra.jeu.points_obtenus} / {ra.jeu.points} points{ra.jeu.obtenus_hardcore ? ` · ${ra.jeu.obtenus_hardcore} en hardcore` : ''}</span>
      <div class="bar"><span style="width: {Math.round((ra.jeu.obtenus / ra.jeu.succes) * 100)}%"></span></div>
    {/if}
    <p class={resume.ton === 'ok' ? 'ok' : resume.ton === 'alerte' ? 'alerte' : 'muted'}>{resume.texte}</p>
  </div>
{:else if chargement && (surPc || jeu.source === 'steam')}
  <p class="muted">🏆 Succès…</p>
{/if}

<style>
  .succes {
    display: grid;
    gap: calc(4 * var(--u));
  }
  p {
    margin: 0;
  }
  .ok {
    color: var(--ok-texte);
  }
  .alerte {
    color: var(--warn-texte);
  }
</style>
