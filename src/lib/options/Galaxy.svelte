<script lang="ts">
  // ⚙ Options ▸ Comptes ▸ GOG Galaxy : lier les comptes des boutiques (Epic, Amazon, EA, Ubisoft, Xbox…) passe par
  // Galaxy ; Seb veut que ce soit expliqué ici aussi. Aucun secret : Galaxy est déjà connecté sur ce PC.
  import { onMount } from 'svelte';
  import { galaxy, importerGalaxy, lireGalaxy, nomPlateforme } from '$lib/boutiques/galaxy.svelte';
  import { explicationGalaxy } from '$lib/import/sources';

  onMount(lireGalaxy);
  const parBoutique = $derived(
    Object.entries(
      galaxy.jeux.reduce<Record<string, number>>((n, j) => {
        const b = nomPlateforme(j.plateforme ?? '');
        n[b] = (n[b] ?? 0) + 1;
        return n;
      }, {}),
    ).sort((a, b) => b[1] - a[1]),
  );
</script>

<div class="galaxy">
  {#each explicationGalaxy() as p (p)}<p class="muted">{p}</p>{/each}
  <dl class="cx-kv">
    <dt>GOG Galaxy sur ce PC</dt>
    <dd>{galaxy.installe ? '✅ trouvé' : '⚠ pas trouvé (gratuit : gog.com/galaxy)'}</dd>
    <dt>Jeux lus</dt>
    <dd>{galaxy.jeux.length}</dd>
    {#each parBoutique as [b, n] (b)}
      <dt>· {b}</dt>
      <dd>{n}</dd>
    {/each}
  </dl>
  <div class="actions">
    <button class="btn primary" onclick={importerGalaxy} disabled={galaxy.enCours || !galaxy.installe}>
      {galaxy.enCours ? 'Lecture…' : '🔄 Relire mes jeux GOG Galaxy'}
    </button>
    <a class="btn" href="/importer?source=epic">📥 Importer…</a>
  </div>
</div>

<style>
  .galaxy {
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
