<script lang="ts">
  // Une grille de miniatures de captures (fiche du jeu, menu en jeu, Options) : une cellule fixe par capture, comme
  // la galerie des images du jeu (jeu/[id]/+page.svelte), pour que la croix de la manette s'y déplace bien.
  import { adresseCapture, dateCapture, type Capture } from './captures';

  let { captures, onouvrir, avecTitre = false }: { captures: Capture[]; onouvrir: (id: string) => void; avecTitre?: boolean } = $props();
  let ratees = $state<string[]>([]);
</script>

<div class="galerie">
  {#each captures as c (c.id)}
    {#if !ratees.includes(c.id)}
      <button class="vignette" data-capture={c.id} title={`${c.titre} — ${dateCapture(c.quand)}`} onclick={() => onouvrir(c.id)}>
        <span class="cadre">
          <img src={adresseCapture(c.id)} alt={`Capture de ${c.titre}`} loading="lazy" decoding="async" onerror={() => (ratees = [...ratees, c.id])} />
        </span>
        <span class="muted">{avecTitre ? c.titre : dateCapture(c.quand).replace(/^le /, '')}</span>
      </button>
    {/if}
  {/each}
</div>

<style>
  .galerie {
    display: flex;
    flex-wrap: wrap;
    gap: calc(10 * var(--u));
  }
  .vignette {
    display: grid;
    gap: calc(4 * var(--u));
    width: calc(180 * var(--u));
    padding: calc(4 * var(--u));
    border: 1px solid var(--line);
    border-radius: calc(10 * var(--u));
    background: var(--bg2);
    color: var(--ink);
    cursor: pointer;
    text-align: center;
  }
  .vignette:hover {
    border-color: var(--accent);
  }
  .vignette .cadre {
    height: calc(110 * var(--u));
    overflow: hidden;
    border-radius: calc(6 * var(--u));
    display: grid;
    place-items: center;
    background: var(--fond-media);
  }
  .vignette img {
    max-width: 100%;
    max-height: 100%;
    object-fit: contain;
  }
  .vignette .muted {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: calc(12 * var(--u));
  }
</style>
