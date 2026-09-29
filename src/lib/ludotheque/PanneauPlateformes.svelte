<script lang="ts">
  // La colonne de gauche : la recherche, les filtres, puis les plateformes groupées par catégorie.
  import { etat } from '$lib/etat.svelte';
  import { grouperParCategorie, ICONES_CATEGORIES } from './categories';
  import { choisirPlateforme, ludo, rechargerListe } from './ludotheque.svelte';

  let filtresOuverts = $state(false);
  let replies = $state<Record<string, boolean>>({});
  let minuterie: ReturnType<typeof setTimeout> | undefined;

  const visibles = $derived(
    ludo.plateformes.filter((p) => !etat.profil.ludotheque.plateformesMasquees.includes(p.nom)),
  );
  const groupes = $derived(grouperParCategorie(visibles));
  const totalVisible = $derived(visibles.reduce((s, p) => s + p.jeux, 0));
  const filtreActif = $derived(!!ludo.genre);

  function rechercher(texte: string) {
    ludo.texte = texte;
    clearTimeout(minuterie);
    minuterie = setTimeout(rechargerListe, 200);
  }
</script>

<aside class="colonne" aria-label="Plateformes">
  <div class="recherche">
    <input
      type="search"
      placeholder="🔍 Rechercher un jeu"
      aria-label="Rechercher un jeu"
      value={ludo.texte}
      oninput={(e) => rechercher(e.currentTarget.value)}
    />
    <button
      class="btn filtre"
      class:actif={filtresOuverts || filtreActif}
      aria-expanded={filtresOuverts}
      title="Filtres"
      onclick={() => (filtresOuverts = !filtresOuverts)}
    >
      ⚙
    </button>
  </div>

  {#if filtresOuverts}
    <div class="filtres">
      <label class="champ">
        <span>Genre</span>
        <select
          value={ludo.genre ?? ''}
          onchange={(e) => {
            ludo.genre = e.currentTarget.value || null;
            rechargerListe();
          }}
        >
          <option value="">Tous les genres</option>
          {#each ludo.genres as g (g)}<option value={g}>{g}</option>{/each}
        </select>
      </label>
      {#if filtreActif}
        <button
          class="btn"
          onclick={() => {
            ludo.genre = null;
            rechargerListe();
          }}>↺ Retirer le filtre</button
        >
      {/if}
    </div>
  {/if}

  <nav class="liste">
    <button class="plateforme" class:actif={ludo.plateforme === null} onclick={() => choisirPlateforme(null)}>
      <span class="icone" aria-hidden="true">🗂</span>
      <span class="nom">Tout</span>
      <span class="nombre">{totalVisible.toLocaleString('fr-FR')}</span>
    </button>

    {#each groupes as g (g.categorie)}
      <button
        class="categorie"
        aria-expanded={!replies[g.categorie]}
        onclick={() => (replies[g.categorie] = !replies[g.categorie])}
      >
        <span class="fleche" aria-hidden="true">{replies[g.categorie] ? '▸' : '▾'}</span>
        <span class="icone" aria-hidden="true">{ICONES_CATEGORIES[g.categorie]}</span>
        <span class="nom">{g.categorie}</span>
      </button>
      {#if !replies[g.categorie]}
        {#each g.plateformes as p (p.nom)}
          <button
            class="plateforme sous"
            class:actif={ludo.plateforme === p.nom}
            onclick={() => choisirPlateforme(p.nom)}
          >
            <span class="nom">{p.nom}</span>
            <span class="nombre">{p.jeux.toLocaleString('fr-FR')}</span>
          </button>
        {/each}
      {/if}
    {:else}
      <p class="muted vide">Aucune plateforme pour l’instant. Lance une synchronisation avec « 🔄 Synchroniser ».</p>
    {/each}
  </nav>
</aside>

<style>
  .colonne {
    display: flex;
    flex-direction: column;
    gap: calc(10 * var(--u));
    min-height: 0;
  }
  .recherche {
    display: flex;
    gap: calc(8 * var(--u));
  }
  .filtre {
    flex: none;
    width: var(--cible-min);
    padding: 0;
  }
  .filtre.actif {
    border-color: var(--accent);
  }
  .filtres {
    display: grid;
    gap: calc(8 * var(--u));
    background: var(--bg2);
    border-radius: calc(10 * var(--u));
    padding: calc(10 * var(--u));
  }
  .liste {
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: calc(2 * var(--u));
    min-height: 0;
    padding-right: calc(4 * var(--u));
  }
  .plateforme,
  .categorie {
    font: inherit;
    color: var(--ink);
    background: transparent;
    border: 1px solid transparent;
    border-radius: calc(8 * var(--u));
    display: flex;
    align-items: center;
    gap: calc(8 * var(--u));
    min-height: calc(36 * var(--u));
    padding: calc(4 * var(--u)) calc(10 * var(--u));
    cursor: pointer;
    text-align: left;
  }
  .plateforme:hover,
  .categorie:hover {
    background: color-mix(in srgb, var(--ink) 6%, transparent);
  }
  .plateforme.actif {
    background: color-mix(in srgb, var(--accent) 20%, transparent);
    border-color: var(--accent);
  }
  .categorie {
    margin-top: calc(6 * var(--u));
    font-weight: 600;
  }
  .sous {
    padding-left: calc(30 * var(--u));
  }
  .nom {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .nombre {
    color: var(--dim);
    font-size: calc(12 * var(--u));
  }
  .fleche {
    color: var(--dim);
    width: calc(10 * var(--u));
  }
  .vide {
    padding: calc(10 * var(--u));
  }
</style>
