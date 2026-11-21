<script lang="ts">
  // 🏆 Les succès d'un jeu, dans le panneau de détails : Steam pour ses jeux, RetroAchievements pour les jeux émulés
  // présents sur le PC (version compatible ou non, et laquelle l'est).
  import { goto } from '$app/navigation';
  import { api, type JeuResume } from '$lib/api';
  import { resumeRetro, type SuccesRetro } from './succes.svelte';
  import { succesGalaxy } from '$lib/boutiques/jeu-boutique';

  /** `idPc` : le numéro du jeu SUR CE PC (un jeu importé relié montre sa fiche, son fichier est sous son numéro local). */
  let { jeu, surPc, idPc }: { jeu: JeuResume; surPc: boolean; idPc?: number } = $props();

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
          const r = await api.succesRetro(idPc ?? j.id).catch(() => null);
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
  const galaxy = $derived(succesGalaxy(jeu));

  /** La liste en détail (lot C). */
  function voir(source: 'steam' | 'galaxy', cle: string) {
    goto(`/succes?source=${source}&cle=${encodeURIComponent(cle)}&titre=${encodeURIComponent(jeu.titre)}`);
  }
</script>

{#if galaxy}
  <!-- Lot C4a : aussi pour une carte GOG (compte ou PC), qui passe avant Galaxy et reprend ses succès. -->
  <div class="succes cx-block">
    <strong>🏆 Succès <span class="muted">(GOG Galaxy)</span></strong>
    <span>{galaxy.obtenus} / {galaxy.total} obtenus</span>
    <div class="bar"><span style="width: {Math.round((galaxy.obtenus / galaxy.total) * 100)}%"></span></div>
    {#if galaxy.cle}<button class="btn petit" onclick={() => voir('galaxy', galaxy.cle ?? '')}>🏆 Voir les succès</button>{/if}
  </div>
{:else if steam}
  <div class="succes cx-block">
    <strong>🏆 Succès Steam</strong>
    <span>{steam[0]} / {steam[1]} obtenus</span>
    <div class="bar"><span style="width: {steam[1] ? Math.round((steam[0] / steam[1]) * 100) : 0}%"></span></div>
    {#if jeu.cle_boutique}<button class="btn petit" onclick={() => voir('steam', jeu.cle_boutique ?? '')}>🏆 Voir les succès</button>{/if}
  </div>
{:else if ra && resume}
  <div class="succes cx-block">
    <strong>🏆 RetroAchievements</strong>
    {#if ra.jeu && ra.jeu.succes}
      <span>{ra.jeu.obtenus} / {ra.jeu.succes} succès · {ra.jeu.points_obtenus} / {ra.jeu.points} points{ra.jeu.obtenus_hardcore ? ` · ${ra.jeu.obtenus_hardcore} en hardcore` : ''}</span>
      <div class="bar"><span style="width: {Math.round((ra.jeu.obtenus / ra.jeu.succes) * 100)}%"></span></div>
    {/if}
    <p class={resume.ton === 'ok' ? 'ok' : resume.ton === 'alerte' ? 'alerte' : 'muted'}>{resume.texte}</p>
    {#if ra.jeu && ra.jeu.succes}
      <!-- Lot C3 : la liste en détail (le jeu du PC, reconnu comme ci-dessus). -->
      <button class="btn petit" onclick={() => goto(`/succes?jeu=${idPc ?? jeu.id}&titre=${encodeURIComponent(jeu.titre)}`)}>🏆 Voir les succès</button>
    {/if}
  </div>
{:else if chargement && (surPc || jeu.source === 'steam')}
  <p class="muted">🏆 Succès… {surPc && /wii/i.test(jeu.plateforme) ? '(un jeu Wii se vérifie en ~30 s la première fois, ensuite c’est immédiat)' : ''}</p>
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
