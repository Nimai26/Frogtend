<script lang="ts">
  // ❓ Aide : les réponses aux questions qu'on se pose en s'en servant. Commence par les boutiques et GOG Galaxy
  // (demande de Seb, 02/10).
  import { explicationGalaxy, SOURCES } from '$lib/import/sources';
</script>

<div class="aide">
  <h1>❓ Aide</h1>

  <section class="cx-block" id="importer">
    <h2>📥 Importer des jeux</h2>
    <p>Le menu « 📥 Importer » reprend la liste de LaunchBox. Chaque boutique a sa voie :</p>
    <ul>
      {#each SOURCES as s (s.id)}
        <li><a href="/importer?source={s.id}">{s.icone} {s.libelle}</a> — {s.resume}{s.voie === 'local' ? ' (prochain lot)' : ''}</li>
      {/each}
    </ul>
    <p>Les jeux importés des boutiques rejoignent ta ludothèque, rubrique Windows ; les filtres « Boutique » et « Installés » les retrouvent. Ils restent dans ton Frogtend et ne remontent jamais dans Firehouse.</p>
  </section>

  <section class="cx-block" id="galaxy">
    <h2>🟣 GOG Galaxy : toutes tes boutiques d’un coup</h2>
    {#each explicationGalaxy() as p (p)}<p>{p}</p>{/each}
    <p>
      Pourquoi passer par Galaxy ? Epic, Amazon, EA, Ubisoft et Xbox n’offrent pas d’API ouverte pour lire ta liste de
      jeux. Galaxy a des accords (« intégrations ») avec elles : c’est la voie la plus sûre, et ton mot de passe ne passe
      jamais par Frogtend. Steam, lui, a une API : tu peux importer par ton compte Steam OU par Galaxy.
    </p>
    <p>Réglages : <a href="/reglages?rubrique=galaxy">⚙ Options ▸ Comptes ▸ GOG Galaxy</a>.</p>
  </section>
</div>

<style>
  .aide {
    display: grid;
    gap: calc(14 * var(--u));
    padding: calc(20 * var(--u));
    max-width: calc(900 * var(--u));
  }
  h1,
  h2,
  p {
    margin: 0;
  }
  .cx-block {
    display: grid;
    gap: calc(10 * var(--u));
  }
  ul {
    margin: 0;
    display: grid;
    gap: calc(4 * var(--u));
  }
</style>
