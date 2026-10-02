<script lang="ts">
  // ⚙ Options ▸ Comptes ▸ RetroAchievements : le compte et la clé d'API Web de CE profil (la clé va dans le coffre de
  // Windows et ne s'affiche plus jamais).
  import { onMount } from 'svelte';
  import { connecterEmulateurs, lireRetro, oublierRetro, ouvrirPageCleRetro, reglerRetro, retro } from '$lib/succes/succes.svelte';

  onMount(lireRetro);
  const e = $derived(retro.etat);
</script>

<div class="ra">
  <p class="muted">
    RetroAchievements ajoute des succès aux jeux rétro émulés. Frogtend vérifie si ta version d’un jeu est compatible
    (sinon, il te dit laquelle l’est) et montre tes succès obtenus. Il faut ton nom d’utilisateur et ta clé d’API Web
    (sur le site, section « Keys »). Chaque profil a son compte.
  </p>
  <dl class="cx-kv">
    <dt>Compte</dt>
    <dd>{e?.compte || 'pas encore réglé'}</dd>
    <dt>Clé d’API Web</dt>
    <dd>{e?.cle_enregistree ? '✅ enregistrée dans le coffre de Windows' : 'aucune'}</dd>
    <dt>Émulateurs</dt>
    <dd>{e?.emulateurs_connectes ? '✅ RetroArch et PCSX2 jouent avec ton compte (DuckStation : connexion faite dans DuckStation)' : 'pas encore connectés : tu ne gagnes pas de succès en jouant'}</dd>
  </dl>
  <p class="muted">Chaque profil joue avec SON compte ; un profil sans compte a les succès coupés.</p>
  <div class="actions">
    <button class="btn primary" onclick={reglerRetro}>✏ Régler mon compte RetroAchievements</button>
    <button class="btn" onclick={connecterEmulateurs} disabled={!e?.compte}>🎮 Connecter mes émulateurs</button>
    <button class="btn" onclick={ouvrirPageCleRetro}>🌐 Trouver ma clé (site de RetroAchievements)</button>
    {#if e?.compte || e?.cle_enregistree}<button class="btn danger" onclick={oublierRetro}>Oublier ce compte</button>{/if}
  </div>
</div>

<style>
  .ra {
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
