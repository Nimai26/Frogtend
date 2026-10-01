<script lang="ts">
  // La fiche d'un jeu, en deux niveaux (décision de Seb) :
  //  - le JEU : résumé, genres, informations, annexes — vrai quelle que soit la version ;
  //  - les VERSIONS : chacune avec SES fichiers et SES notes, qui ne valent pas pour une autre.
  import { page } from '$app/state';
  import { api, estErreurCoeur, taille, type Annexe, type Fiche } from '$lib/api';
  import { informer, toast } from '$lib/dialogues/fenetres.svelte';
  import { motifDuRefus } from '$lib/dialogues/messages';
  import Jaquette from '$lib/ludotheque/Jaquette.svelte';
  import { mettreDansLaLudotheque, tele } from '$lib/ludotheque/telechargements.svelte';
  import { emulateursDuJeu, gererJeu, installer, jouer, jouerAvec, partie } from '$lib/ludotheque/jeu.svelte';

  let fiche = $state<Fiche | null>(null);
  let horsLigne = $state(false);
  let locale = $state(false);
  let erreur = $state('');
  let annexe = $state<{ titre: string; texte: string } | null>(null);

  const id = $derived(Number(page.params.id));

  async function charger(n: number) {
    fiche = null;
    erreur = '';
    annexe = null;
    try {
      const r = await api.fiche(n);
      fiche = r.fiche;
      horsLigne = r.hors_ligne;
      locale = r.locale;
    } catch (e) {
      erreur = estErreurCoeur(e) ? e.motif : motifDuRefus(e);
    }
  }
  $effect(() => {
    charger(id);
  });

  /** Libellé lisible de la qualité (la nature d'une version, qui décide de l'installation). */
  function nature(q: string): { libelle: string; ton: string } {
    const t = q.toLowerCase();
    if (t.includes('prêt à jouer')) return { libelle: '✅ Prêt à jouer', ton: 'ok' };
    if (t.includes('rom')) return { libelle: '💾 ROM', ton: '' };
    if (t.includes('image disque')) return { libelle: '💿 Image disque', ton: '' };
    if (t.includes('repack')) return { libelle: '📦 Repack (installeur)', ton: 'warn' };
    if (t.includes('installeur')) return { libelle: '🧰 Installeur d’origine', ton: 'warn' };
    return { libelle: `❔ ${q || 'Nature non dite'}`, ton: '' };
  }

  async function ouvrirAnnexe(a: Annexe) {
    if (!fiche) return;
    if (!a.texte) {
      if (!locale) {
        await informer(
          `📎 ${a.titre}`,
          `Ce document (${taille(a.taille)}) arrive sur le PC quand tu mets le jeu dans ta ludothèque.`,
        );
        return;
      }
      try {
        await api.ouvrirAnnexe(fiche.id, a.i);
      } catch (e) {
        toast(`Impossible d’ouvrir « ${a.titre} » : ${motifDuRefus(e)}`, 'erreur');
      }
      return;
    }
    try {
      const r = await api.annexeTexte(fiche.id, a.i, a.cle);
      annexe = { titre: r.titre ?? a.titre, texte: r.texte ?? '' };
    } catch (e) {
      if (estErreurCoeur(e) && e.sorte === 'conflit') {
        toast('⚠ La fiche a changé entre-temps : je la relis.', 'alerte');
        await charger(fiche.id);
      } else {
        toast(`Impossible d’ouvrir « ${a.titre} » : ${motifDuRefus(e)}`, 'erreur');
      }
    }
  }

  const ICONES_ANNEXES: Record<string, string> = {
    manuel: '📘',
    solution: '🧭',
    astuces: '💡',
    lancement: '▶',
    presse: '📰',
  };

  /** Markdown simple → paragraphes et gras, sans HTML venu du serveur (tout est échappé). */
  function paragraphes(texte: string): { gras: boolean; t: string }[][] {
    return texte
      .split(/\n{2,}/)
      .map((p) => p.split(/(\*\*[^*]+\*\*)/).filter(Boolean).map((m) =>
        m.startsWith('**') ? { gras: true, t: m.slice(2, -2) } : { gras: false, t: m.replace(/`/g, '') },
      ));
  }
</script>

<div class="page">
  <button class="btn retour" onclick={() => history.back()}>← Revenir</button>

  {#if erreur}
    <div class="panel message">
      <p>⛔ {erreur}</p>
    </div>
  {:else if !fiche}
    <p class="muted">Chargement de la fiche…</p>
  {:else}
    {#if horsLigne}
      <p class="tag warn">Hors ligne : fiche de la dernière consultation</p>
    {:else if locale}
      <p class="tag ok">✅ Dans ta ludothèque : fiche et documents gardés sur ce PC</p>
    {/if}
    {#if tele.jeux[fiche.id]?.etat === 'telecharge'}
      {@const surPc = tele.jeux[fiche.id]}
      <div class="actions-jeu">
        {#if !surPc.installation}
          <button class="btn primary" onclick={() => installer(surPc.id)} disabled={partie.installation === surPc.id}>📦 Installer</button>
        {:else}
          <button class="btn primary" onclick={() => jouer(surPc.id)} disabled={partie.enJeu !== null}>▶ Jouer</button>
          {#if emulateursDuJeu(surPc.plateforme).liste.length > 1}
            <button class="btn" onclick={() => jouerAvec(surPc.id)} disabled={partie.enJeu !== null}>▶ Jouer avec…</button>
          {/if}
        {/if}
        <button class="btn" onclick={() => gererJeu(surPc.id)} disabled={partie.enJeu === surPc.id}>⚙ Gérer le jeu</button>
        <a class="btn" href={`/assistant?jeu=${surPc.id}`}>💬 Demander à l’assistant</a>
        <a class="btn" href={`/triches?jeu=${surPc.id}`}>🎯 Triches et mods</a>
      </div>
    {/if}

    <section class="jeu">
      <div class="jaquette"><Jaquette id={fiche.id} titre={fiche.titre} plateforme={fiche.plateforme} largeur={260} /></div>
      <div class="infos">
        <h1>{fiche.titre}</h1>
        {#if fiche.titre_en && fiche.titre_en !== fiche.titre}<p class="muted">{fiche.titre_en}</p>{/if}
        <p class="sous">
          {fiche.plateforme}{fiche.annee ? ` · ${fiche.annee}` : ''}
        </p>
        {#if fiche.genres?.length}
          <div class="genres">{#each fiche.genres as g (g)}<span class="tag">{g}</span>{/each}</div>
        {/if}
        {#if fiche.resume}<p class="resume">{fiche.resume}</p>{/if}
        <dl class="cx-kv">
          {#if fiche.developpeur}<dt>Développeur</dt><dd>{fiche.developpeur}</dd>{/if}
          {#if fiche.editeur}<dt>Éditeur</dt><dd>{fiche.editeur}</dd>{/if}
          {#if fiche.joueurs}<dt>Joueurs</dt><dd>{fiche.joueurs}{fiche.cooperatif ? ' (coopératif)' : ''}</dd>{/if}
        </dl>
      </div>
    </section>

    {#if fiche.annexes?.length}
      <section class="cx-block">
        <h3>📚 Documents du jeu</h3>
        <div class="annexes">
          {#each fiche.annexes as a (a.i)}
            <button class="btn" onclick={() => ouvrirAnnexe(a)}>
              {ICONES_ANNEXES[a.type] ?? '📎'} {a.titre}
              <span class="muted">{a.texte ? 'à lire' : locale ? 'ouvrir' : taille(a.taille)}</span>
            </button>
          {/each}
        </div>
        {#if annexe}
          <article class="lecture panel">
            <header>{annexe.titre}</header>
            <div class="texte">
              {#each paragraphes(annexe.texte) as p, i (i)}
                <p>{#each p as m, k (k)}{#if m.gras}<strong>{m.t}</strong>{:else}{m.t}{/if}{/each}</p>
              {/each}
            </div>
            <button class="btn" onclick={() => (annexe = null)}>Fermer</button>
          </article>
        {/if}
      </section>
    {/if}

    <section class="cx-block">
      <h3>📦 Versions disponibles</h3>
      {#if !fiche.versions?.length}
        <p class="muted">Aucune version n’est encore rangée dans Firehouse pour ce jeu.</p>
      {:else}
        <div class="versions">
          {#each fiche.versions as v (v.telechargement_id)}
            {@const n = nature(v.qualite)}
            <article class="version cx-card">
              <header>
                <span class="tag {n.ton}">{n.libelle}</span>
                <h4>{v.nom}</h4>
              </header>
              <ul class="fichiers">
                {#each v.fichiers as f (f.nom)}
                  <li><span>{f.nom}</span><span class="muted">{taille(f.taille)}</span></li>
                {/each}
              </ul>
              <p class="muted">
                Total : {taille(v.fichiers.reduce((s, f) => s + f.taille, 0))}
                {#if v.range_le}· rangée le {new Date(v.range_le).toLocaleDateString('fr-FR')}{/if}
              </p>
              {#if v.notes}
                <details>
                  <summary>Notes de cette version</summary>
                  <p class="notes">{v.notes}</p>
                  {#if v.notes_a_traduire}<p class="muted">Ces notes ne sont pas encore traduites.</p>{/if}
                </details>
              {/if}
              {#if !locale}
                <button class="btn" onclick={() => fiche && mettreDansLaLudotheque(fiche.id)}>➕ Mettre dans ma ludothèque</button>
              {/if}
            </article>
          {/each}
        </div>
      {/if}
    </section>
  {/if}
</div>

<style>
  .page {
    padding: calc(16 * var(--u)) calc(24 * var(--u)) calc(32 * var(--u));
    display: grid;
    gap: calc(16 * var(--u));
    max-width: calc(1200 * var(--u));
    margin: 0 auto;
  }
  .actions-jeu {
    display: flex;
    gap: calc(8 * var(--u));
    flex-wrap: wrap;
  }
  .retour {
    justify-self: start;
    text-decoration: none;
  }
  .message {
    padding: calc(16 * var(--u));
  }
  .jeu {
    display: grid;
    grid-template-columns: calc(260 * var(--u)) 1fr;
    gap: calc(24 * var(--u));
  }
  h1 {
    margin: 0;
    font-size: calc(30 * var(--u));
  }
  .sous {
    margin: calc(4 * var(--u)) 0 calc(10 * var(--u));
    color: var(--dim);
  }
  .genres {
    display: flex;
    flex-wrap: wrap;
    gap: calc(6 * var(--u));
  }
  .resume {
    white-space: pre-line;
    line-height: 1.6;
  }
  .infos .cx-kv {
    max-width: calc(520 * var(--u));
  }
  .annexes {
    display: flex;
    flex-wrap: wrap;
    gap: calc(8 * var(--u));
  }
  .lecture {
    margin-top: calc(12 * var(--u));
    display: grid;
    gap: calc(10 * var(--u));
    padding-bottom: calc(12 * var(--u));
  }
  .lecture .texte {
    padding: 0 calc(16 * var(--u));
    line-height: 1.6;
  }
  .lecture > .btn {
    justify-self: end;
    margin-right: calc(12 * var(--u));
  }
  .versions {
    display: grid;
    gap: calc(12 * var(--u));
  }
  .version {
    padding: calc(14 * var(--u));
    display: grid;
    gap: calc(8 * var(--u));
    background: var(--panel);
  }
  .version:hover {
    transform: none;
  }
  .version header {
    display: grid;
    gap: calc(6 * var(--u));
    justify-items: start;
  }
  h4 {
    margin: 0;
    font-size: calc(15 * var(--u));
  }
  .fichiers {
    list-style: none;
    margin: 0;
    padding: 0;
    font-family: var(--mono);
    font-size: calc(12.5 * var(--u));
  }
  .fichiers li {
    display: flex;
    justify-content: space-between;
    gap: calc(12 * var(--u));
    padding: calc(4 * var(--u)) 0;
    border-bottom: 1px solid var(--line);
  }
  .notes {
    white-space: pre-line;
  }
  .version > .btn {
    justify-self: start;
  }
  @media (max-width: 720px) {
    .jeu {
      grid-template-columns: 1fr;
    }
    .jaquette {
      max-width: calc(220 * var(--u));
    }
  }
</style>
