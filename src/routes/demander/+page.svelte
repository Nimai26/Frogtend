<script lang="ts">
  // « Demander un jeu » (lot 6) : chercher dans toute la base LaunchBox de Firehouse un jeu qui n'est pas encore dans
  // le catalogue, et le demander. Firehouse fait valider la demande, cherche les sources et télécharge : Frogtend ne
  // télécharge jamais rien sur Internet de son côté.
  import { api, type ResultatRecherche } from '$lib/api';
  import { confirmer, toast } from '$lib/dialogues/fenetres.svelte';
  import { motifDuRefus } from '$lib/dialogues/messages';
  import { etatDemande } from '$lib/demandes';
  import { etat } from '$lib/etat.svelte';

  let texte = $state('');
  let resultats = $state<ResultatRecherche[] | null>(null);
  let enCours = $state(false);
  /** Les demandes envoyées pendant cette visite (pour ne pas redemander). */
  let demandes = $state<number[]>([]);

  async function chercher(e?: SubmitEvent) {
    e?.preventDefault();
    if (texte.trim().length < 2) return;
    enCours = true;
    try {
      resultats = (await api.rechercherJeu(texte)).resultats ?? [];
    } catch (err) {
      toast(`Recherche impossible : ${motifDuRefus(err)}`, 'erreur');
    } finally {
      enCours = false;
    }
  }

  async function demander(r: ResultatRecherche) {
    const oui = await confirmer(`📨 Demander « ${r.titre} » ?`, {
      message: `${r.plateforme}${r.annee ? `, ${r.annee}` : ''}.\nFirehouse fera valider la demande, cherchera le jeu et le téléchargera. Il apparaîtra ensuite dans le Catalogue Firehouse.`,
      libelleValider: '📨 Demander',
    });
    if (!oui) return;
    try {
      await api.demanderJeu(r.launchbox_id);
      demandes = [...demandes, r.launchbox_id];
      toast(`📨 Demande envoyée pour « ${r.titre} ».`);
    } catch (err) {
      toast(`La demande n’est pas partie : ${motifDuRefus(err)}`, 'erreur');
    }
  }
</script>

<div class="demander">
  <header>
    <h1>🔎 Demander un jeu</h1>
    <p class="muted">
      Un jeu qui n’est pas dans le Catalogue Firehouse ? Cherche-le dans toute la base LaunchBox, puis demande-le.
    </p>
  </header>

  {#if etat.pc.firehouse.simule}
    <p class="muted">Mode simulé : des exemples (essaie « absent »), aucune demande n’est envoyée.</p>
  {/if}

  <form class="recherche" onsubmit={chercher}>
    <input type="search" bind:value={texte} placeholder="Titre du jeu (au moins deux lettres)" aria-label="Titre du jeu" />
    <button class="btn primary" type="submit" disabled={enCours || texte.trim().length < 2}>
      {enCours ? 'Recherche…' : '🔎 Chercher'}
    </button>
  </form>

  {#if resultats}
    {#if resultats.length === 0}
      <p class="muted">Aucun jeu trouvé pour « {texte} ». Essaie le titre anglais, ou moins de mots.</p>
    {:else}
      <ul class="resultats">
        {#each resultats as r (r.launchbox_id)}
          {@const e = etatDemande(r, demandes)}
          <li class="cx-card">
            <div class="infos">
              <strong>{r.titre_fr || r.titre}</strong>
              {#if r.titre_fr && r.titre_fr !== r.titre}<span class="muted">({r.titre})</span>{/if}
              <span class="muted">
                {r.plateforme}{r.annee ? ` · ${r.annee}` : ''}{r.developpeur ? ` · ${r.developpeur}` : ''}{r.genres?.length
                  ? ` · ${r.genres.join(', ')}`
                  : ''}
              </span>
            </div>
            {#if e.demandable}
              <button class="btn" onclick={() => demander(r)}>📨 Demander</button>
            {:else}
              <span class="etat">{e.texte}</span>
            {/if}
          </li>
        {/each}
      </ul>
    {/if}
  {/if}
</div>

<style>
  .demander {
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
  h1 {
    margin: 0;
  }
  header p {
    margin: calc(4 * var(--u)) 0 0;
  }
  .recherche {
    display: flex;
    gap: calc(8 * var(--u));
  }
  .recherche input {
    flex: 1;
  }
  .resultats {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: calc(8 * var(--u));
  }
  .resultats li {
    display: flex;
    align-items: center;
    gap: calc(12 * var(--u));
    padding: calc(10 * var(--u)) calc(14 * var(--u));
  }
  .infos {
    flex: 1;
    display: grid;
    gap: calc(2 * var(--u));
  }
  .etat {
    white-space: nowrap;
  }
</style>
