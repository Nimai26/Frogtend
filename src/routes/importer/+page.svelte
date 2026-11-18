<script lang="ts">
  // 📥 Importer (Seb, 02/10) : la même liste que le menu « Importer » de LaunchBox. À gauche les sources, à droite la
  // fenêtre d'import de celle choisie. Les boutiques sans API ouverte passent par GOG Galaxy : on l'EXPLIQUE ici.
  import { onMount } from 'svelte';
  import { page } from '$app/state';
  import { goto } from '$app/navigation';
  import { galaxy, importerGalaxy, lireGalaxy } from '$lib/boutiques/galaxy.svelte';
  import { importerSteam, lireSteam, steam } from '$lib/boutiques/steam.svelte';
  import { compterParGalaxy, explicationGalaxy, source, SOURCES } from '$lib/import/sources';
  import ImportLocal from '$lib/import/ImportLocal.svelte';
  import Rapprocher from '$lib/import/Rapprocher.svelte';
  import { boutiquesPc, lireEtatsPc } from '$lib/boutiques/boutiques-pc.svelte';
  import { ligneEtat } from '$lib/boutiques/jeu-boutique';
  import { etat } from '$lib/etat.svelte';

  onMount(() => {
    lireSteam();
    lireGalaxy();
    lireEtatsPc();
  });

  const s = $derived(source(page.url.searchParams.get('source')) ?? SOURCES[0]);
  /** L'état de la boutique choisie sur ce PC (voie « auto »). */
  const etatPc = $derived(boutiquesPc.etats.find((e) => e.boutique === s.boutique));
  const choisir = (id: string) => goto(`/importer?source=${id}`, { replaceState: true, noScroll: true, keepFocus: true });
  const nbGalaxy = $derived(compterParGalaxy(s, galaxy.jeux));
  const nomBoutique = $derived(s.libelle.replace(/^Jeux /, ''));
</script>

<div class="importer">
  <nav class="liste panel" aria-label="Sources d’import">
    <p class="groupe">📥 Importer</p>
    {#each SOURCES as x (x.id)}
      <button class="rubrique" class:actif={s.id === x.id} aria-current={s.id === x.id} onclick={() => choisir(x.id)}>
        <span aria-hidden="true">{x.icone}</span>
        {x.libelle}
        {#if x.voie === 'bientot'}<span class="tag">bientôt</span>{/if}
      </button>
    {/each}
  </nav>

  <div class="contenu">
    <h1>{s.icone} {s.libelle}</h1>
    <p>{s.resume}</p>

    {#snippet parSteam()}
      <section class="cx-block">
        <h2>Par ton compte Steam (conseillé)</h2>
        <dl class="cx-kv">
          <dt>Compte</dt>
          <dd>{steam.etat?.compte || 'pas encore réglé : Frogtend te le demandera au premier import'}</dd>
          <dt>Jeux importés</dt>
          <dd>{steam.jeux.length}</dd>
        </dl>
        <div class="actions">
          <button class="btn primary" onclick={importerSteam} disabled={steam.enCours}>
            {steam.enCours ? 'Import…' : '⬇ Importer mes jeux Steam'}
          </button>
          <a class="btn" href="/reglages?rubrique=steam">⚙ Options ▸ Comptes ▸ Steam</a>
        </div>
      </section>
      <section class="cx-block">
        <h2>Ou par GOG Galaxy</h2>
        <p class="muted">Si ton compte Steam est relié dans GOG Galaxy, ses jeux arrivent aussi par là, sans clé d’API ({nbGalaxy} jeu(x) Steam lus dans Galaxy).</p>
      </section>
    {/snippet}
    {#snippet parGalaxy()}
      <section class="cx-block">
        <h2>Par GOG Galaxy</h2>
        {#each explicationGalaxy(nomBoutique) as p (p)}<p>{p}</p>{/each}
        <dl class="cx-kv">
          <dt>GOG Galaxy sur ce PC</dt>
          <dd>{galaxy.installe ? '✅ trouvé' : '⚠ pas trouvé — installe-le depuis gog.com/galaxy, puis relie tes comptes'}</dd>
          <dt>Jeux {nomBoutique} lus</dt>
          <dd>
            {nbGalaxy}
            {#if galaxy.installe && galaxy.jeux.length && !nbGalaxy}
              <span class="muted">— aucun : ton compte {nomBoutique} n’est sans doute pas encore relié dans Galaxy.</span>
            {/if}
          </dd>
        </dl>
        <div class="actions">
          <button class="btn primary" onclick={importerGalaxy} disabled={galaxy.enCours || !galaxy.installe}>
            {galaxy.enCours ? 'Lecture…' : '⬇ Lire mes jeux par GOG Galaxy'}
          </button>
          <a class="btn" href="/reglages?rubrique=galaxy">⚙ Options ▸ Comptes ▸ GOG Galaxy</a>
          <a class="btn" href="/aide#galaxy">❓ Aide</a>
        </div>
        <p class="muted">La lecture prend TOUTES les boutiques reliées dans Galaxy d’un coup ; dans la ludothèque, le filtre « Boutique » les sépare.</p>
      </section>
    {/snippet}

    {#if s.voie === 'auto'}
      <section class="cx-block">
        <h2>Tes jeux installés entrent seuls</h2>
        <p>À chaque ouverture de ton profil, Frogtend range dans ta ludothèque les jeux que {nomBoutique} a installés sur ce PC. Rien à relier, aucune clé à donner. Jouer et Installer passent par son lanceur.</p>
        <p class="muted">{etatPc ? ligneEtat(etatPc) : 'Lecture de ce PC…'}</p>
        {#if !etat.profil.boutiquesPc.auto}<p class="muted">⚠ L’ajout tout seul est coupé pour ton profil.</p>{/if}
        <div class="actions"><a class="btn" href="/reglages?rubrique=jeux-installes">⚙ Options ▸ Comptes ▸ Mes jeux installés</a></div>
      </section>
      <details class="cx-block">
        <summary>🛟 Secours : voir aussi les jeux que tu possèdes sans les avoir installés</summary>
        {#if s.secours === 'steam'}{@render parSteam()}{:else}{@render parGalaxy()}{/if}
      </details>
    {:else if s.voie === 'steam'}
      {@render parSteam()}
    {:else if s.voie === 'galaxy'}
      {@render parGalaxy()}
    {:else if s.voie === 'rapprocher'}
      <Rapprocher />
    {:else if s.voie === 'local'}
      {#key s.id}<ImportLocal sorte={s.id as 'rom' | 'dos' | 'mame' | 'windows' | 'manuel' | 'installer-dos'} />{/key}
    {:else}
      <section class="cx-block">
        <h2>Pas encore livré</h2>
        <p>Cet import fait partie du prochain lot (les jeux locaux). Il est noté dans le plan, avec la même idée que LaunchBox, et sera annoncé dans les notes de version quand il sera prêt.</p>
      </section>
    {/if}
  </div>
</div>

<style>
  .importer {
    display: grid;
    grid-template-columns: calc(250 * var(--u)) 1fr;
    height: 100%;
    min-height: 0;
  }
  .liste {
    overflow: auto;
    padding: calc(8 * var(--u)) 0;
    border-radius: 0;
  }
  .groupe {
    margin: calc(6 * var(--u)) calc(12 * var(--u));
    font-weight: 700;
    color: var(--dim);
  }
  .rubrique {
    display: flex;
    align-items: center;
    gap: calc(8 * var(--u));
    width: 100%;
    text-align: left;
    font: inherit;
    color: var(--ink);
    background: transparent;
    border: none;
    border-left: 3px solid transparent;
    padding: calc(7 * var(--u)) calc(12 * var(--u));
    cursor: pointer;
  }
  .rubrique:hover {
    background: color-mix(in srgb, var(--ink) 6%, transparent);
  }
  .rubrique.actif {
    border-left-color: var(--accent);
    background: color-mix(in srgb, var(--accent) 12%, transparent);
  }
  .rubrique .tag {
    margin-left: auto;
  }
  .contenu {
    overflow: auto;
    padding: calc(20 * var(--u));
    display: grid;
    gap: calc(12 * var(--u));
    align-content: start;
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
  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: calc(8 * var(--u));
  }
</style>
