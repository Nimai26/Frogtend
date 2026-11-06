<script lang="ts">
  // Options ▸ 📷 Captures d'écran (✅ Seb 06/11) : toutes les captures du profil ouvert, par console puis par jeu,
  // les plus récentes d'abord. Un jeu se déplie à la fois (des centaines d'images se chargeraient sinon).
  import { onMount, tick } from 'svelte';
  import { api } from '$lib/api';
  import { etat } from '$lib/etat.svelte';
  import { nomAffiche } from '$lib/ludotheque/ludotheque.svelte';
  import { selectionner } from '$lib/taodbox/manette.svelte';
  import GrilleCaptures from '$lib/captures/GrilleCaptures.svelte';
  import VisionneuseCaptures from '$lib/captures/VisionneuseCaptures.svelte';
  import { parConsole, type Capture } from '$lib/captures/captures';

  let toutes = $state<Capture[] | null>(null);
  let deplie = $state<number | null>(null);
  let ouverte = $state<{ captures: Capture[]; depart: string; titre: string } | null>(null);
  const groupes = $derived(parConsole(toutes ?? []));

  async function relire() {
    toutes = await api.capturesListe().catch(() => []);
  }
  onMount(relire);

  async function fermer(id: string | null) {
    ouverte = null;
    await relire();
    await tick();
    selectionner(document.querySelector<HTMLElement>(`[data-capture="${id}"]`) ?? document.querySelector<HTMLElement>('.jeu-captures'));
  }
</script>

<div class="captures">
  {#if !etat.pc.dossierCaptures}
    <p>Choisis d’abord où ranger tes captures : ⚙ Options ▸ 📁 Emplacements ▸ « 📷 Captures d’écran ». Ensuite, chaque photo prise pendant une partie apparaît ici.</p>
  {:else if toutes === null}
    <p class="muted">⏳ Lecture de tes captures…</p>
  {:else if !toutes.length}
    <p>Aucune capture pour l’instant. Pendant une partie : menu de Frogtend ▸ Options du jeu ▸ 📷 Photo de l’écran, ou la touche de capture de l’émulateur.</p>
    <p class="muted">Pour l’instant, Frogtend range les captures de RetroArch, de la PS3 (RPCS3) et de la Game Boy Advance (mGBA).</p>
  {:else}
    <p class="muted">{toutes.length} capture{toutes.length > 1 ? 's' : ''}, seulement les tiennes. Dossier : {etat.pc.dossierCaptures}</p>
    {#each groupes as g (g.plateforme)}
      <section class="cx-block">
        <h3>{nomAffiche(g.plateforme)}</h3>
        {#each g.jeux as j (j.jeu)}
          <button class="btn jeu-captures" aria-expanded={deplie === j.jeu} onclick={() => (deplie = deplie === j.jeu ? null : j.jeu)}>
            {deplie === j.jeu ? '▾' : '▸'} {j.titre} <span class="muted">({j.captures.length})</span>
          </button>
          {#if deplie === j.jeu}
            <GrilleCaptures captures={j.captures} onouvrir={(id) => (ouverte = { captures: j.captures, depart: id, titre: j.titre })} />
          {/if}
        {/each}
      </section>
    {/each}
  {/if}
</div>

{#if ouverte}
  <VisionneuseCaptures captures={ouverte.captures} depart={ouverte.depart} titre={ouverte.titre} sousLaBarre onfermer={fermer} />
{/if}

<style>
  .captures {
    display: grid;
    gap: calc(12 * var(--u));
  }
  .cx-block {
    display: grid;
    gap: calc(8 * var(--u));
  }
  .jeu-captures {
    justify-self: start;
  }
  p {
    margin: 0;
  }
</style>
