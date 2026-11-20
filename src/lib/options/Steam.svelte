<script lang="ts">
  // ⚙ Options ▸ Comptes ▸ Steam (lot 9, inspiré de LaunchBox) : le compte et la clé d'API de CE profil.
  import { onMount } from 'svelte';
  import { depuis } from '$lib/ludotheque/ludotheque.svelte';
  import { importerSteam, importSteamEnCours, lireSteam, oublierSteam, ouvrirPageCle, reglerSteam, steam } from '$lib/boutiques/steam.svelte';

  onMount(lireSteam);
  const e = $derived(steam.etat);
</script>

<div class="steam">
  <p class="muted">
    Le plus simple : dans 🛒 Boutiques, « Se connecter à Steam » sur sa page officielle. Les jeux que tu possèdes
    entrent alors dans ta ludothèque, sans clé, et sont relus à chaque ouverture de Frogtend. Chaque profil a son
    compte.
  </p>
  <p class="muted">
    En secours seulement (si la connexion ne marche pas) : l’URL de ton profil et ta propre clé d’API Steam (gratuite).
    La clé va dans le coffre de Windows et ne s’affiche plus jamais.
  </p>
  <dl class="cx-kv">
    <dt>Compte</dt>
    <dd>{e?.connecte ? '✅ connecté à Steam par sa page' : e?.compte || 'pas encore réglé'}</dd>
    <dt>Clé d’API</dt>
    <dd>{e?.cle_enregistree ? '✅ enregistrée dans le coffre de Windows' : 'aucune'}</dd>
    <dt>Jeux importés</dt>
    <dd>{e?.nb_jeux ?? 0}{e?.maj_le ? ` (${depuis(Number(e.maj_le))})` : ''}</dd>
    <dt>Steam sur ce PC</dt>
    <dd>{e?.steam_installe ? '✅ installé' : '⚠ pas trouvé (il faut Steam pour jouer et installer)'}</dd>
  </dl>
  <div class="actions">
    <button class="btn" onclick={reglerSteam}>✏ Régler ma clé d’API (secours)</button>
    <!-- Connecté par la page : la clé n'est pas utile, son lien n'est pas proposé à un novice. -->
    {#if !e?.connecte}<button class="btn" onclick={ouvrirPageCle}>🌐 Créer ma clé d’API (site de Steam)</button>{/if}
    <button class="btn" onclick={importerSteam} disabled={importSteamEnCours()}>{importSteamEnCours() ? 'Import…' : '🔄 Importer mes jeux'}</button>
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
