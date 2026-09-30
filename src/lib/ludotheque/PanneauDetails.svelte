<script lang="ts">
  // La colonne de droite : le jeu sélectionné (jaquette, informations, actions) ; sans sélection, la plateforme
  // choisie et un « Jeu au hasard ».
  // Dans le catalogue : « Mettre dans ma ludothèque ». Dans la ludothèque : l'état du jeu sur le PC.
  import { goto } from '$app/navigation';
  import { duree, taille } from '$lib/api';
  import { categorieDe, ICONES_CATEGORIES } from './categories';
  import Jaquette from './Jaquette.svelte';
  import { jeuAuHasard, ludo } from './ludotheque.svelte';
  import { annuler, mettreDansLaLudotheque, mettreEnPause, reprendre, tele } from './telechargements.svelte';
  import { emulateursDuJeu, gererJeu, installer, jouer, jouerAvec, partie } from './jeu.svelte';

  const j = $derived(ludo.selection);
  const surPc = $derived(j ? tele.jeux[j.id] : undefined);
  const plateforme = $derived(ludo.plateformes.find((p) => p.nom === ludo.plateforme));
  const catalogue = $derived(ludo.espace === 'catalogue');
  let ajout = $state(false);

  async function ajouter() {
    if (!j) return;
    ajout = true;
    try {
      await mettreDansLaLudotheque(j.id);
    } finally {
      ajout = false;
    }
  }

  const pourcent = (recus: number, total: number) => (total > 0 ? Math.floor((recus / total) * 100) : 0);
</script>

<aside class="colonne" aria-label="Détails">
  {#if j}
    <div class="jaquette">
      <Jaquette id={j.id} titre={j.titre} plateforme={j.plateforme} disponible={j.jaquette} largeur={260} />
    </div>
    <h2>{j.titre}</h2>

    {#if surPc && surPc.etat !== 'telecharge'}
      <div class="progression cx-block">
        <p class="etat">
          {#if surPc.etat === 'en_cours'}⬇ Téléchargement : {pourcent(surPc.recus, surPc.total)} %
          {:else if surPc.etat === 'attente'}⏳ En attente de téléchargement
          {:else if surPc.etat === 'pause'}⏸ En pause ({pourcent(surPc.recus, surPc.total)} %)
          {:else}⛔ Échec du téléchargement{/if}
        </p>
        <div class="bar"><span style="width: {pourcent(surPc.recus, surPc.total)}%"></span></div>
        <p class="muted">
          {taille(surPc.recus)} sur {taille(surPc.total)}
          {#if surPc.etat === 'en_cours' && surPc.debit}
            · {taille(surPc.debit)}/s · encore {duree((surPc.total - surPc.recus) / surPc.debit)}
          {/if}
        </p>
        {#if surPc.message}<p class="muted">{surPc.message}</p>{/if}
        <div class="boutons">
          {#if surPc.etat === 'en_cours' || surPc.etat === 'attente'}
            <button class="btn" onclick={() => mettreEnPause(j.id)}>⏸ Pause</button>
          {:else}
            <button class="btn primary" onclick={() => reprendre(j.id)}>▶ Reprendre</button>
          {/if}
          <button class="btn danger" onclick={() => annuler(j.id)}>Annuler</button>
        </div>
      </div>
    {/if}

    <div class="actions">
      {#if catalogue && !surPc}
        <button class="btn primary grand" onclick={ajouter} disabled={ajout}>
          {ajout ? 'Préparation…' : '➕ Mettre dans ma ludothèque'}
        </button>
      {:else if catalogue && surPc}
        <p class="tag ok sur-pc">✅ Dans ta ludothèque</p>
      {/if}
      {#if !catalogue && surPc?.etat === 'telecharge'}
        {#if partie.enJeu === j.id}
          <p class="tag ok sur-pc">🎮 Partie en cours…</p>
        {:else if partie.installation === j.id}
          <button class="btn primary grand" disabled>📦 Installation…</button>
        {:else if !surPc.installation}
          <button class="btn primary grand" onclick={() => installer(j.id)}>📦 Installer</button>
        {:else}
          <button class="btn primary grand" onclick={() => jouer(j.id)} disabled={partie.enJeu !== null}>▶ Jouer</button>
          {#if emulateursDuJeu(j.plateforme).liste.length > 1}
            <button class="btn grand" onclick={() => jouerAvec(j.id)} disabled={partie.enJeu !== null}>▶ Jouer avec…</button>
          {/if}
        {/if}
      {/if}
      {#if !catalogue && surPc?.etat === 'telecharge'}
        <button class="btn grand" onclick={() => gererJeu(j.id)} disabled={partie.enJeu === j.id}>
          ⚙ Gérer le jeu <span class="muted">(installer, désinstaller, parties…)</span>
        </button>
      {/if}
      <button class="btn grand" onclick={() => goto(`/jeu/${j.id}`)}>📄 Voir la fiche</button>
    </div>

    <dl class="cx-kv">
      <dt>Plateforme</dt>
      <dd>{j.plateforme}</dd>
      {#if j.annee}<dt>Année</dt><dd>{j.annee}</dd>{/if}
      {#if j.developpeur}<dt>Développeur</dt><dd>{j.developpeur}</dd>{/if}
      {#if j.editeur}<dt>Éditeur</dt><dd>{j.editeur}</dd>{/if}
      {#if catalogue && j.versions != null}<dt>Versions</dt><dd>{j.versions || 'aucune pour l’instant'}</dd>{/if}
      {#if surPc}<dt>Sur ce PC</dt><dd title={surPc.dossier}>{surPc.dossier}</dd>{/if}
      {#if surPc?.temps_jeu}<dt>Temps de jeu</dt><dd>{duree(surPc.temps_jeu)}</dd>{/if}
    </dl>
    {#if j.genres.length}
      <div class="genres">
        {#each j.genres as g (g)}<span class="tag">{g}</span>{/each}
      </div>
    {/if}
  {:else}
    <div class="entete">
      <span class="icone" aria-hidden="true">{ludo.plateforme ? ICONES_CATEGORIES[categorieDe(ludo.plateforme)] : catalogue ? '🛒' : '🎮'}</span>
      <h2>{ludo.plateforme ?? (catalogue ? 'Catalogue Firehouse' : 'Ma ludothèque')}</h2>
      <p class="muted">
        {#if ludo.plateforme}
          {categorieDe(ludo.plateforme)} · {(plateforme?.jeux ?? 0).toLocaleString('fr-FR')} jeu(x)
        {:else}
          {ludo.totalLudotheque.toLocaleString('fr-FR')} jeu(x) sur {ludo.plateformes.length} plateforme(s)
        {/if}
      </p>
    </div>
    <button class="btn hasard" onclick={jeuAuHasard} disabled={ludo.totalLudotheque === 0}>
      <span class="de" aria-hidden="true">🎲</span>
      Jeu au hasard
    </button>
    <p class="muted aide">Choisis un jeu dans la grille pour voir ses informations ici.</p>
  {/if}
</aside>

<style>
  .colonne {
    display: flex;
    flex-direction: column;
    gap: calc(12 * var(--u));
    overflow-y: auto;
    min-height: 0;
    padding: calc(4 * var(--u));
  }
  .jaquette {
    width: min(100%, calc(260 * var(--u)));
    align-self: center;
  }
  h2 {
    margin: 0;
    font-size: calc(22 * var(--u));
    line-height: 1.2;
  }
  .actions {
    display: grid;
    gap: calc(8 * var(--u));
  }
  .grand {
    min-height: calc(44 * var(--u));
    font-size: calc(14 * var(--u));
  }
  .sur-pc {
    justify-self: start;
    margin: 0;
    font-size: calc(13 * var(--u));
  }
  .progression {
    display: grid;
    gap: calc(8 * var(--u));
  }
  .progression p {
    margin: 0;
  }
  .etat {
    font-weight: 600;
  }
  .boutons {
    display: flex;
    gap: calc(8 * var(--u));
    flex-wrap: wrap;
  }
  .cx-kv dd {
    white-space: nowrap;
  }
  .genres {
    display: flex;
    flex-wrap: wrap;
    gap: calc(6 * var(--u));
  }
  .entete {
    text-align: center;
    padding: calc(20 * var(--u)) calc(10 * var(--u)) calc(6 * var(--u));
    background: linear-gradient(160deg, color-mix(in srgb, var(--accent2) 25%, transparent), color-mix(in srgb, var(--accent) 18%, transparent));
    border-radius: calc(14 * var(--u));
  }
  .entete .icone {
    font-size: calc(48 * var(--u));
  }
  .entete h2 {
    text-transform: uppercase;
    letter-spacing: 0.5px;
    margin-top: calc(8 * var(--u));
  }
  .hasard {
    flex-direction: column;
    min-height: calc(110 * var(--u));
    font-size: calc(13 * var(--u));
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }
  .de {
    font-size: calc(34 * var(--u));
  }
  .aide {
    text-align: center;
  }
</style>
