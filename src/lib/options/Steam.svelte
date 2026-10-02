<script lang="ts">
  // ⚙ Options ▸ Comptes ▸ Steam (lot 9, inspiré de LaunchBox) : le compte et la clé d'API de CE profil.
  import { onMount } from 'svelte';
  import { depuis } from '$lib/ludotheque/ludotheque.svelte';
  import { importerSteam, lireSteam, oublierSteam, ouvrirPageCle, reglerSteam, steam } from '$lib/boutiques/steam.svelte';

  onMount(lireSteam);
  const e = $derived(steam.etat);
</script>

<div class="steam">
  <p class="muted">
    Frogtend lit la liste des jeux que tu possèdes sur Steam (API officielle de Steam), pour les retrouver dans « Mes
    boutiques ». Il faut l’URL de ton profil et ta propre clé d’API Steam (gratuite). La clé va dans le coffre de
    Windows et ne s’affiche plus jamais. Chaque profil a son compte.
  </p>
  <dl class="cx-kv">
    <dt>Compte</dt>
    <dd>{e?.compte || 'pas encore réglé'}</dd>
    <dt>Clé d’API</dt>
    <dd>{e?.cle_enregistree ? '✅ enregistrée dans le coffre de Windows' : 'aucune'}</dd>
    <dt>Jeux importés</dt>
    <dd>{e?.nb_jeux ?? 0}{e?.maj_le ? ` (${depuis(Number(e.maj_le))})` : ''}</dd>
    <dt>Steam sur ce PC</dt>
    <dd>{e?.steam_installe ? '✅ installé' : '⚠ pas trouvé (il faut Steam pour jouer et installer)'}</dd>
  </dl>
  <div class="actions">
    <button class="btn primary" onclick={reglerSteam}>✏ Régler mon compte Steam</button>
    <button class="btn" onclick={ouvrirPageCle}>🌐 Créer ma clé d’API (site de Steam)</button>
    <button class="btn" onclick={importerSteam} disabled={steam.enCours}>{steam.enCours ? 'Import…' : '🔄 Importer mes jeux'}</button>
    {#if e?.compte || e?.cle_enregistree}<button class="btn danger" onclick={oublierSteam}>Oublier ce compte</button>{/if}
  </div>
</div>

<style>
  .steam {
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
