<script lang="ts">
  // La colonne de droite : le jeu sélectionné (jaquette, informations, actions) ; sans sélection, la plateforme
  // choisie et un « Jeu au hasard ».
  // Dans le catalogue : « Mettre dans ma ludothèque ». Dans la ludothèque : l'état du jeu sur le PC.
  import { goto } from '$app/navigation';
  import { duree, taille } from '$lib/api';
  import { categorieDe, ICONES_CATEGORIES } from './categories';
  import { rangCourt } from './versions';
  import Jaquette from './Jaquette.svelte';
  import { jeuAuHasard, ludo } from './ludotheque.svelte';
  import { annuler, jeuSurPc, mettreDansLaLudotheque, mettreEnPause, reprendre } from './telechargements.svelte';
  import { emulateursDuJeu, gererLaCarte, installer, jouerAvec, jouerLaCarte, partie } from './jeu.svelte';
  import { estJeuDeBoutique, ouvrirJeuDeBoutique } from '$lib/boutiques/jeu-boutique';
  import { nomPlateforme } from '$lib/boutiques/galaxy.svelte';
  import BlocSucces from '$lib/succes/BlocSucces.svelte';
  import BlocContenus from '$lib/contenus/BlocContenus.svelte';
  import BlocDecompression from '$lib/decompression/BlocDecompression.svelte';
  import IconesBoutiques from '$lib/boutiques/IconesBoutiques.svelte';
  import { originesCarte } from '$lib/boutiques/copies';

  const j = $derived(ludo.selection);
  // Un jeu réuni (A6c) : sa copie sur ce PC peut être une autre que la carte (la carte Steam, la copie GOG installée).
  const surPc = $derived(j ? (jeuSurPc(j.id) ?? j.copies?.map((c) => jeuSurPc(c.id)).find((x) => !!x?.installation)) : undefined);
  /** Le numéro du jeu SUR CE PC (un jeu importé relié : son numéro local ; la ligne montre sa fiche). */
  const idPc = $derived(surPc?.id ?? j?.id ?? 0);
  /** Une partie en cours sur CETTE carte, quelle que soit la copie lancée (A6c). */
  const enJeuIci = $derived(partie.enJeu !== null && (partie.enJeu === idPc || !!j?.copies?.some((c) => c.id === partie.enJeu)));
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
            <button class="btn" onclick={() => mettreEnPause(idPc)}>⏸ Pause</button>
          {:else}
            <button class="btn primary" onclick={() => reprendre(idPc)}>▶ Reprendre</button>
          {/if}
          <button class="btn danger" onclick={() => annuler(idPc)}>Annuler</button>
        </div>
      </div>
    {/if}

    <div class="actions">
      {#if estJeuDeBoutique(j) && !surPc}
        <!-- Un jeu de tes boutiques pas encore sur ce PC (ou que Frogtend ne sait pas encore lancer) : Steam ou GOG
             Galaxy s'en chargent. Installé par Steam, c'est un jeu du PC : Jouer passe par Frogtend (partie suivie). -->
        <button class="btn primary grand" onclick={() => j && ouvrirJeuDeBoutique(j)}>
          {j.installe ? '▶ Jouer' : '⬇ Installer'} <span class="muted">({j.source === 'steam' ? 'Steam' : 'GOG Galaxy'})</span>
        </button>
      {:else if catalogue && !surPc}
        <button class="btn primary grand" onclick={ajouter} disabled={ajout}>
          {ajout ? 'Préparation…' : '➕ Mettre dans ma ludothèque'}
        </button>
      {:else if catalogue && surPc}
        <p class="tag ok sur-pc">{surPc.id < 0 ? '✅ Déjà sur ce PC (un fichier importé du disque)' : '✅ Dans ta ludothèque'}</p>
      {/if}
      {#if !catalogue && surPc?.etat === 'telecharge'}
        {#if enJeuIci}
          <p class="tag ok sur-pc">🎮 Partie en cours…</p>
        {:else if partie.installation === idPc}
          <button class="btn primary grand" disabled>📦 Installation…</button>
        {:else if !surPc.installation}
          <button class="btn primary grand" onclick={() => installer(idPc)} disabled={!!surPc.d_un_autre} title={surPc.d_un_autre ? 'Seule la personne qui l’a importé peut l’installer' : undefined}>📦 Installer</button>
        {:else}
          <button class="btn primary grand" onclick={() => jouerLaCarte(idPc)} disabled={partie.enJeu !== null || partie.lancement !== null}>{partie.lancement === idPc ? 'Lancement…' : '▶ Jouer'}</button>
          {#if emulateursDuJeu(j.plateforme).liste.length > 1 || (surPc.installation.versions?.length ?? 0) > 1}
            <button class="btn grand" onclick={() => jouerAvec(idPc)} disabled={partie.enJeu !== null}>▶ Jouer avec…</button>
          {/if}
        {/if}
      {/if}
      {#if !catalogue && surPc?.etat === 'telecharge' && !surPc.d_un_autre}
        <button class="btn grand" onclick={() => gererLaCarte(idPc)} disabled={enJeuIci}>
          ⚙ Gérer le jeu <span class="muted">(installer, désinstaller, parties…)</span>
        </button>
      {/if}
      {#if j.id > 0}<button class="btn grand" onclick={() => goto(`/jeu/${j.id}`)}>📄 Voir la fiche</button>{/if}
    </div>

    <dl class="cx-kv">
      <dt>Plateforme</dt>
      <dd>{j.plateforme}</dd>
      {#if j.annee}<dt>Année</dt><dd>{j.annee}</dd>{/if}
      {#if j.developpeur}<dt>Développeur</dt><dd>{j.developpeur}</dd>{/if}
      {#if j.editeur}<dt>Éditeur</dt><dd>{j.editeur}</dd>{/if}
      {#if catalogue && j.versions != null}<dt>Versions</dt><dd>{j.versions || 'aucune pour l’instant'}</dd>{/if}
      {#if surPc}<dt>Sur ce PC</dt><dd title={surPc.installation?.boutique ? surPc.installation.dossier : surPc.dossier}>{surPc.installation?.boutique ? surPc.installation.dossier : surPc.dossier}</dd>{/if}
      {#if surPc?.temps_jeu}<dt>Temps de jeu</dt><dd>{duree(surPc.temps_jeu)}</dd>{/if}
      {#if (surPc?.installation?.versions?.length ?? 0) > 1}
        <dt>Versions</dt>
        <dd>{surPc?.installation?.versions?.length} ({surPc?.installation?.versions?.map((v) => rangCourt(v)).join(', ')})</dd>
      {/if}
      {#if surPc?.d_un_autre}
        <dt>Origine</dt><dd>importé du disque par une autre personne de ce PC : tu peux y jouer, elle seule peut le gérer</dd>
      {:else if j.source === 'local' || (surPc && surPc.id < 0)}
        <dt>Origine</dt><dd>importé de ton disque (Frogtend n’efface jamais ses fichiers){surPc && surPc.id < 0 && j.id > 0 ? ' ; fiche et images : Firehouse' : ''}</dd>
      {/if}
      {#if estJeuDeBoutique(j)}
        <!-- Un jeu réuni (A6c) : toutes ses boutiques, et le temps de jeu en somme avec le détail (Seb, 21/10). -->
        <dt>{j.copies?.length ? 'Boutiques' : 'Boutique'}</dt><dd><IconesBoutiques origines={originesCarte(j)} /></dd>
        <dt>Sur ce PC</dt><dd>{j.installe ? `✅ installé${j.copies?.length ? ` (${j.copies.filter((c) => c.installe).map((c) => nomPlateforme(c.boutique)).join(', ')})` : ''}` : 'pas installé'}</dd>
        {#if j.minutes}<dt>Temps de jeu</dt><dd>{duree(j.minutes * 60)}{j.copies && j.copies.filter((c) => c.minutes).length > 1 ? ` (${j.copies.filter((c) => c.minutes).map((c) => `${nomPlateforme(c.boutique)} ${duree(c.minutes * 60)}`).join(', ')})` : ''}</dd>{/if}
      {:else if !surPc?.d_un_autre && j.source !== 'local' && !(surPc && surPc.id < 0)}
        <dt>Origine</dt><dd><IconesBoutiques origines={originesCarte(j)} /></dd>
      {/if}
    </dl>
    {#if !catalogue && surPc?.installation}<BlocDecompression id={idPc} titre={j.titre} />{/if}
    {#if !catalogue}<BlocSucces jeu={j} surPc={!!surPc?.installation} idPc={idPc} />{/if}
    {#if !catalogue && surPc?.installation}<BlocContenus id={idPc} titre={j.titre} />{/if}
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
