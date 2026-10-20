<script lang="ts">
  // ⚙ Options ▸ Comptes ▸ Nexus Mods (lot 0.82.0) : la clé d'API PERSONNELLE de CE profil (dans le coffre de Windows,
  // jamais réaffichée, envoyée seulement à Nexus).
  import { onMount } from 'svelte';
  import { lireNexus, nexus, oublierNexus, ouvrirPageCleNexus, reglerNexus } from '$lib/mods/nexus.svelte';

  onMount(lireNexus);
  const e = $derived(nexus.etat);
</script>

<div class="nx">
  <p class="muted">
    Nexus Mods est le plus grand site de mods pour les jeux PC. Avec ta clé d’API personnelle (gratuite, dans ton
    compte Nexus), Frogtend installe un mod quand tu cliques « Mod Manager Download » sur sa page, après ton accord.
    Chaque profil a sa clé.
  </p>
  <dl class="cx-kv">
    <dt>Compte</dt>
    <dd>{e?.compte || 'pas encore réglé'}</dd>
    <dt>Clé d’API</dt>
    <dd>{e?.cle_enregistree ? '✅ enregistrée dans le coffre de Windows' : 'aucune'}</dd>
  </dl>
  <div class="actions">
    <button class="btn primary" onclick={reglerNexus}>✏ Régler ma clé Nexus</button>
    <button class="btn" onclick={ouvrirPageCleNexus}>🌐 Trouver ma clé (site de Nexus Mods)</button>
    {#if e?.compte || e?.cle_enregistree}<button class="btn danger" onclick={oublierNexus}>Oublier cette clé</button>{/if}
  </div>
</div>

<style>
  .nx {
    display: grid;
    gap: calc(12 * var(--u));
  }
  .muted {
    margin: 0;
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: calc(8 * var(--u));
  }
</style>
