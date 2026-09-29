<script lang="ts">
  import { etat } from '$lib/etat.svelte';
</script>

<section class="panel">
  <header>🎮 Ludothèque</header>
  <div class="contenu">
    <p>La ludothèque de Firehouse s’affichera ici : c’est le lot 1.</p>
    <p class="muted">
      Firehouse : {etat.pc.firehouse.adresse}
      {etat.pc.firehouse.simule ? '(mode simulé : aucune connexion n’est faite)' : ''}
    </p>
  </div>
</section>

<style>
  .contenu {
    padding: calc(16 * var(--u));
  }
</style>
