<script lang="ts">
  // « Mes boutiques » (lot 9) : les jeux possédés sur les comptes de la personne (Steam d'abord). Ils restent dans son
  // Frogtend et ne remontent jamais dans Firehouse. Jouer et installer passent par la boutique elle-même.
  import { onMount } from 'svelte';
  import { adresseImageBoutique, adresseImageParUrl, duree, type JeuBoutique } from '$lib/api';
  import { galaxy, importerGalaxy, lireGalaxy, nomPlateforme, ouvrirDansGalaxy } from '$lib/boutiques/galaxy.svelte';
  import { importerSteam, lireSteam, ouvrirDansSteam, steam } from '$lib/boutiques/steam.svelte';

  let filtre = $state('');
  let seulementInstalles = $state(false);
  onMount(() => {
    lireSteam();
    lireGalaxy();
  });

  // GOG Galaxy : filtre par boutique d'origine. Les jeux Steam y sont masqués par défaut si Steam est importé à part.
  let boutiqueGalaxy = $state<string>('tout');
  const plateformesGalaxy = $derived([...new Set(galaxy.jeux.map((j) => j.plateforme ?? ''))].sort());
  const affichesGalaxy = $derived(
    galaxy.jeux.filter(
      (j: JeuBoutique) =>
        (boutiqueGalaxy === 'tout' ? !(steam.jeux.length && j.plateforme === 'steam') : j.plateforme === boutiqueGalaxy) &&
        (!seulementInstalles || j.installe) &&
        j.nom.toLowerCase().includes(filtre.trim().toLowerCase()),
    ),
  );

  const affiches = $derived(
    steam.jeux.filter((j) => (!seulementInstalles || j.installe) && j.nom.toLowerCase().includes(filtre.trim().toLowerCase())),
  );
</script>

<div class="boutiques">
  <header>
    <h1>🛒 Mes boutiques</h1>
    <p class="muted">Les jeux de tes comptes : Steam (par son API), et GOG Galaxy qui regroupe GOG, Epic, Xbox, Ubisoft, EA… Ils restent dans ton Frogtend.</p>
  </header>

  <section class="cx-block">
    <div class="tete">
      <h2>Steam</h2>
      <span class="muted">{steam.etat?.compte ? `compte ${steam.etat.compte}` : 'compte pas encore réglé'}</span>
      <span class="espace"></span>
      <button class="btn primary" onclick={importerSteam} disabled={steam.enCours}>
        {steam.enCours ? 'Import…' : steam.jeux.length ? '🔄 Mettre à jour' : '⬇ Importer mes jeux Steam'}
      </button>
    </div>

    {#if steam.jeux.length}
      <div class="outils">
        <input type="search" bind:value={filtre} placeholder="Chercher un jeu" aria-label="Chercher un jeu Steam" />
        <label><input type="checkbox" bind:checked={seulementInstalles} /> installés sur ce PC</label>
        <span class="muted">{affiches.length} / {steam.jeux.length}</span>
      </div>
      <div class="grille">
        {#each affiches as j (j.id)}
          <button
            class="carte"
            title={j.installe ? `▶ Jouer à « ${j.nom} » (Steam)` : `⬇ Installer « ${j.nom} » (Steam)`}
            onclick={() => ouvrirDansSteam(j, j.installe ? 'jouer' : 'installer')}
          >
            <span class="image">
              <span class="remplacement">{j.nom}</span>
              <img src={adresseImageBoutique('steam', j.id)} alt="" loading="lazy" onerror={(e) => ((e.currentTarget as HTMLElement).hidden = true)} />
            </span>
            <span class="nom">{j.nom}</span>
            <span class="muted">{j.installe ? '✅ installé · ▶ Jouer' : '⬇ Installer'}{j.minutes ? ` · ${duree(j.minutes * 60)}` : ''}</span>
          </button>
        {/each}
      </div>
    {:else if !steam.enCours}
      <p class="muted">Aucun jeu Steam importé pour l’instant.</p>
    {/if}
  </section>

  <section class="cx-block">
    <div class="tete">
      <h2>GOG Galaxy</h2>
      <span class="muted">{galaxy.installe ? 'GOG et les boutiques que tu y as reliées' : '⚠ GOG Galaxy n’est pas trouvé sur ce PC'}</span>
      <span class="espace"></span>
      <button class="btn primary" onclick={importerGalaxy} disabled={galaxy.enCours || !galaxy.installe}>
        {galaxy.enCours ? 'Lecture…' : galaxy.jeux.length ? '🔄 Mettre à jour' : '⬇ Lire mes jeux GOG Galaxy'}
      </button>
    </div>
    {#if galaxy.jeux.length}
      <div class="outils">
        <input type="search" bind:value={filtre} placeholder="Chercher un jeu" aria-label="Chercher un jeu" />
        <select bind:value={boutiqueGalaxy} aria-label="Boutique">
          <option value="tout">Toutes les boutiques</option>
          {#each plateformesGalaxy as p (p)}<option value={p}>{nomPlateforme(p)}</option>{/each}
        </select>
        <label><input type="checkbox" bind:checked={seulementInstalles} /> installés sur ce PC</label>
        <span class="muted">{affichesGalaxy.length} / {galaxy.jeux.length}</span>
      </div>
      <div class="grille">
        {#each affichesGalaxy as j (j.id)}
          <button class="carte" title={`Ouvrir « ${j.nom} » dans GOG Galaxy`} onclick={() => ouvrirDansGalaxy(j)}>
            <span class="image">
              <span class="remplacement">{j.nom}</span>
              {#if j.image}<img src={adresseImageParUrl(j.image)} alt="" loading="lazy" onerror={(e) => ((e.currentTarget as HTMLElement).hidden = true)} />{/if}
            </span>
            <span class="nom">{j.nom}</span>
            <span class="muted">{nomPlateforme(j.plateforme ?? '')}{j.installe ? ' · ✅ installé' : ''}{j.minutes ? ` · ${duree(j.minutes * 60)}` : ''}</span>
          </button>
        {/each}
      </div>
    {/if}
  </section>
</div>

<style>
  .boutiques {
    display: grid;
    gap: calc(14 * var(--u));
    padding: calc(20 * var(--u));
    max-width: calc(900 * var(--u));
    margin: 0 auto;
    width: 100%;
    box-sizing: border-box;
    align-content: start;
    overflow: auto;
    height: 100%;
  }
  h1,
  h2 {
    margin: 0;
  }
  header p {
    margin: calc(4 * var(--u)) 0 0;
  }
  .tete,
  .outils {
    display: flex;
    align-items: center;
    gap: calc(10 * var(--u));
    flex-wrap: wrap;
  }
  .espace {
    flex: 1;
  }
  .outils {
    margin: calc(10 * var(--u)) 0;
  }
  .outils input[type='search'] {
    flex: 1;
    min-width: calc(180 * var(--u));
  }
  .boutiques {
    max-width: none;
  }
  .grille {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(calc(150 * var(--u)), 1fr));
    gap: calc(14 * var(--u));
  }
  .carte {
    display: grid;
    gap: calc(4 * var(--u));
    padding: calc(4 * var(--u));
    background: none;
    border: calc(2 * var(--u)) solid transparent;
    border-radius: calc(8 * var(--u));
    color: var(--ink);
    text-align: left;
    cursor: pointer;
  }
  .carte:hover,
  .carte:focus-visible {
    border-color: var(--accent);
  }
  /* Le titre est DERRIÈRE l'image : visible tant qu'elle n'est pas là (ou si Steam n'en a pas). */
  .image {
    position: relative;
    aspect-ratio: 2 / 3;
    border-radius: calc(6 * var(--u));
    overflow: hidden;
    background: color-mix(in srgb, var(--ink) 8%, transparent);
  }
  .image img {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .remplacement {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    padding: calc(8 * var(--u));
    text-align: center;
    color: var(--dim);
  }
  .nom {
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
