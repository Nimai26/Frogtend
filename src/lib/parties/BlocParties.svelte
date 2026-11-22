<script lang="ts">
  // 📁 Où sont mes parties (lot S1, 22/11) : les emplacements de parties d'un jeu PC sur CE PC, d'après Firehouse,
  // rangés pour CE profil. Lecture seule : rien n'est copié. Le chemin résolu n'apparaît qu'ici, à l'écran, pour la
  // personne du profil (jamais au journal).
  // `canape` (Taodbox) : lisible à la manette, sans action (pas de bouton « Ouvrir le dossier » : l'Explorateur se
  // pilote à la souris) ; chaque ligne se sélectionne, pour que la croix fasse défiler la liste.
  import { api, type PartiesVues } from '$lib/api';
  import { toast } from '$lib/dialogues/fenetres.svelte';
  import { motifDuRefus } from '$lib/dialogues/messages';
  import { contenu, libelleOrigine, montres, ouvrable, texteAutresEndroits, texteEtat, texteFiltre, texteNuage, texteSource, titreOrigine } from './parties';

  let { id, canape = false }: { id: number; canape?: boolean } = $props();
  let vues = $state<PartiesVues | undefined>(undefined);
  let erreur = $state<string | null>(null);
  /** Le dossier en cours d'ouverture (gardien : un seul à la fois). */
  let ouverture = $state<string | null>(null);

  $effect(() => {
    const n = id;
    vues = undefined;
    erreur = null;
    api
      .partiesEmplacements(n)
      .then((v) => {
        if (n === id) vues = v;
      })
      .catch((e) => {
        if (n === id) erreur = motifDuRefus(e);
      });
  });

  async function ouvrir(dossier: string) {
    if (ouverture !== null) return;
    ouverture = dossier;
    try {
      await api.partiesOuvrirDossier(id, dossier);
    } catch (e) {
      toast(`Impossible d’ouvrir le dossier : ${motifDuRefus(e)}`, 'erreur');
    } finally {
      ouverture = null;
    }
  }
</script>

<div class="parties" class:canape>
  <!-- Seule la ligne d'état est annoncée par un lecteur d'écran (jamais toute la liste à chaque changement). -->
  <div class="etat" aria-live="polite">
    {#if erreur}
      <p class="muted">⚠ {erreur}</p>
    {:else if vues === undefined}
      <p class="muted">⏳ Frogtend cherche où sont tes parties…</p>
    {:else}
      {#if texteSource(vues)}<p class="tag warn">{texteSource(vues)}</p>{/if}
      {#if texteEtat(vues)}<p class="muted">{texteEtat(vues)}</p>{/if}
    {/if}
  </div>
  {#if vues && montres(vues).length}
    <ul class="liste">
      {#each montres(vues) as e, i (i)}
        {@const details = [libelleOrigine(e.origine), texteFiltre(e), texteNuage(e)].filter(Boolean).join(' · ')}
        {#if canape}
          <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
          <li class="emplacement" tabindex="0">
            <span class="dossier">{e.dossier}</span>
            <span class="contenu" class:ok={ouvrable(e) && e.fichiers > 0}>{contenu(e)}</span>
            <span class="muted detail" title={titreOrigine(e.origine, e.source)}>{details}</span>
          </li>
        {:else}
          <li class="emplacement">
            <span class="dossier">{e.dossier}</span>
            <span class="contenu" class:ok={ouvrable(e) && e.fichiers > 0}>{contenu(e)}</span>
            <span class="muted detail" title={titreOrigine(e.origine, e.source)}>{details}</span>
            {#if e.dossier && ouvrable(e)}
              {@const d = e.dossier}
              <button class="btn" aria-disabled={ouverture !== null} onclick={() => ouvrir(d)}>
                {ouverture === d ? '⏳ Ouverture…' : '📂 Ouvrir le dossier'}
              </button>
            {/if}
          </li>
        {/if}
      {/each}
    </ul>
  {/if}
  {#if vues && texteAutresEndroits(vues)}<p class="muted">{texteAutresEndroits(vues)}</p>{/if}
</div>

<style>
  .parties {
    display: grid;
    gap: calc(8 * var(--u));
  }
  .parties p {
    margin: 0;
  }
  .etat {
    display: grid;
    gap: calc(6 * var(--u));
  }
  .liste {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: calc(8 * var(--u));
  }
  .emplacement {
    display: grid;
    gap: calc(4 * var(--u));
    justify-items: start;
    padding: calc(10 * var(--u)) calc(12 * var(--u));
    border: 1px solid var(--line);
    border-radius: calc(10 * var(--u));
    background: var(--bg2);
  }
  .emplacement:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }
  .dossier {
    font-family: var(--mono);
    font-size: calc(12.5 * var(--u));
    overflow-wrap: anywhere;
  }
  .contenu {
    font-weight: 600;
  }
  .contenu.ok {
    color: var(--ok-texte);
  }
  .detail {
    overflow-wrap: anywhere;
  }
  /* À 3 m de la télé : un peu plus grand. */
  .canape .dossier,
  .canape .contenu,
  .canape .detail {
    font-size: calc(15 * var(--u));
  }
  /* Un bouton « aria-disabled » reste focalisable (il dit qu'il attend) mais se montre éteint. */
  button[aria-disabled='true'] {
    opacity: 0.6;
    cursor: progress;
  }
</style>
