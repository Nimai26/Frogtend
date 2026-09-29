<script lang="ts">
  // Les émulateurs réglés sur CE PC, par système. Retirer un réglage ne désinstalle rien.
  import { onMount } from 'svelte';
  import { api, type Plateforme } from '$lib/api';
  import { choisir, toast } from '$lib/dialogues/fenetres.svelte';
  import { etat, reglerPc } from '$lib/etat.svelte';
  import { reglerEmulateur } from '$lib/ludotheque/jeu.svelte';

  const e = $derived(Object.entries(etat.pc.emulateurs));
  let plateformes = $state<Plateforme[]>([]);
  onMount(async () => {
    plateformes = await api.plateformes(false).catch(() => []);
  });

  async function retirer(systeme: string) {
    const reste = { ...etat.pc.emulateurs };
    delete reste[systeme];
    await reglerPc('emulateurs', reste);
    toast(`Réglage retiré pour ${systeme} (l’émulateur n’est pas désinstallé).`);
  }

  async function ajouter() {
    const s = await choisir(
      '🕹 Pour quel système ?',
      plateformes.map((p) => ({ valeur: p.nom, libelle: p.nom, detail: `${p.jeux} jeu(x)` })),
    );
    if (s) await reglerEmulateur(s);
  }
</script>

<div class="emulateurs">
  <p class="muted">
    L’émulateur de chaque système, sur ce PC. Frogtend le propose tout seul la première fois que tu lances un jeu qui
    en a besoin, d’après les recommandations de Firehouse. Les jeux PC et MS-DOS n’en ont pas besoin.
  </p>
  {#if e.length === 0}
    <p class="muted">Aucun émulateur réglé pour l’instant.</p>
  {:else}
    <dl class="cx-kv">
      {#each e as [systeme, reglage] (systeme)}
        <dt>{systeme}</dt>
        <dd>
          <span class="chemin" title={`${reglage.programme} ${reglage.ligne}`}>{reglage.nom ?? reglage.programme}</span>
          <button class="btn petit" onclick={() => reglerEmulateur(systeme)}>Changer</button>
          <button class="btn petit" onclick={() => retirer(systeme)}>✕</button>
        </dd>
      {/each}
    </dl>
  {/if}
  <button class="btn" onclick={ajouter} disabled={plateformes.length === 0}>🕹 Régler un émulateur…</button>
</div>

<style>
  .emulateurs {
    display: grid;
    gap: calc(10 * var(--u));
  }
  .emulateurs > .btn {
    justify-self: start;
  }
  .muted {
    margin: 0;
  }
  dd {
    display: flex;
    gap: calc(6 * var(--u));
    align-items: center;
    justify-content: flex-end;
  }
  .chemin {
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .petit {
    min-height: calc(30 * var(--u));
    padding: calc(2 * var(--u)) calc(9 * var(--u));
  }
</style>
