<script lang="ts">
  // 📦 Les contenus additionnels d'un jeu du PC : trouvés sur le disque. PS3 : cochés par la personne, installés
  // seulement après son oui (taille annoncée). Switch, Ryubing et Eden (Seb 16/11 : « pour tout ce qui est update, DLC,
  // on peut passer en automatique ») : utilisés tout seuls à chaque partie — montrés ici, sans rien demander. Wii U :
  // inclus dans le .wua.
  import { api, taille } from '$lib/api';
  import { confirmer, toast } from '$lib/dialogues/fenetres.svelte';
  import { motifDuRefus } from '$lib/dialogues/messages';
  import {
    cochesCoherentes,
    emulateurAutomatique,
    quelquesNoms,
    libelleGenre,
    lignesRemarque,
    nomCourt,
    raisonNonPose,
    resume,
    resumeAutomatique,
    type ContenusDuJeu,
  } from './contenus';

  let { id, titre }: { id: number; titre: string } = $props();

  let donnees = $state<ContenusDuJeu | null>(null);
  let ouvert = $state(false);
  let coches = $state<string[]>([]);
  let enCours = $state(false);

  async function charger(jeu: number) {
    donnees = await api.contenusDuJeu(jeu).catch(() => null);
    coches = [];
  }

  $effect(() => {
    const jeu = id;
    donnees = null;
    ouvert = false;
    charger(jeu);
  });

  const choisis = $derived(donnees?.contenus.filter((c) => coches.includes(c.id)) ?? []);
  const total = $derived(choisis.reduce((n, c) => n + c.taille, 0));
  // Ryubing ou Eden : donnés tout seuls à chaque partie (aucune case, aucun accord).
  const automatique = $derived(emulateurAutomatique(donnees?.systeme));

  async function installer() {
    if (!choisis.length) return;
    const oui = await confirmer(`📦 Installer ${choisis.length} contenu(s) pour « ${titre} » ?`, {
      message: [
        `${taille(total)} à installer par RPCS3 (il travaille sans fenêtre, puis se ferme).`,
        ...choisis.slice(0, 8).map((c) => `• ${nomCourt(c.nom, titre)} (${libelleGenre(c.genre)})${c.licence && c.genre !== 'licence' ? ' — avec sa licence' : ''}`),
        choisis.length > 8 ? `… et ${choisis.length - 8} autre(s).` : '',
        'Les licences vont dans le compte RPCS3 de TON profil : un autre profil devra les installer pour lui.',
        'Ferme RPCS3 avant : Frogtend refuse d’installer pendant qu’il est ouvert.',
      ]
        .filter(Boolean)
        .join('\n'),
      libelleValider: 'Installer',
    });
    if (!oui) return;
    enCours = true;
    try {
      const b = await api.contenusInstaller(id, choisis.map((c) => c.id));
      const deja = b.deja ? ` (${b.deja} mise(s) à jour déjà dépassée(s) par celle installée : rien à refaire)` : '';
      toast(
        b.refuses.length
          ? `📦 ${b.installes} installé(s)${deja}, ${b.refuses.length} refusé(s) : ${b.refuses.map(([n, m]) => `${nomCourt(n, titre)} (${m})`).join(', ')}`
          : `✅ ${b.installes} contenu(s) installé(s) pour « ${titre} »${deja}.`,
        b.refuses.length ? 'alerte' : 'ok',
      );
      await charger(id);
    } catch (e) {
      toast(`Installation impossible : ${motifDuRefus(e)}`, 'erreur');
    } finally {
      enCours = false;
    }
  }
</script>

{#if donnees?.etat === 'emulateur'}
  <p class="muted">📦 Règle {donnees.systeme === 'switch' ? 'Eden ou Ryubing' : 'RPCS3'} (⚙ Options ▸ Émulateurs) pour gérer les mises à jour et DLC de ce jeu.</p>
{:else if donnees?.etat === 'ok' && automatique && (donnees.contenus.length || donnees.ignores?.length || donnees.remarque)}
  <!-- Ryubing et Eden (Seb 16/11) : tout est utilisé seul au lancement — on montre, on ne demande rien. -->
  <div class="contenus cx-block">
    {#if donnees.contenus.length}
      <button class="btn" aria-expanded={ouvert} onclick={() => (ouvert = !ouvert)}>
        📦 Mises à jour et DLC : {resumeAutomatique(donnees.contenus)} {ouvert ? '▴' : '▾'}
      </button>
    {/if}
    {#each lignesRemarque(donnees.remarque) as l (l)}<p class="muted">{l}</p>{/each}
    {#if donnees.ignores?.length}
      <p class="muted">
        ⚠ {donnees.ignores.length} fichier(s) illisible(s), peut-être abîmé(s) au téléchargement : {quelquesNoms(
          donnees.ignores.map((n) => nomCourt(n, titre)),
        )}. Pour les réparer : retire le jeu de ta ludothèque puis remets-le (ses parties sont gardées).
      </p>
    {/if}
    {#if ouvert && donnees.contenus.length}
      <p class="muted">Frogtend les donne à {automatique} tout seul, à chaque partie (rien n’est copié) : la mise à jour la plus récente et tous les DLC de ce jeu, rangés à côté de lui ou dans les emplacements de la Switch (⚙ Options ▸ 📁 Emplacements), reconnus à leur contenu.</p>
      <ul>
        {#each donnees.contenus as c (c.id)}
          <li>
            {#if c.installe}
              <span title={`Utilisé par ${automatique} à chaque partie`}>✅</span>
            {:else}
              <span title={raisonNonPose(c)}>—</span>
            {/if}
            <span class="nom">{nomCourt(c.nom, titre)}</span>
            <span class="muted">{libelleGenre(c.genre)} · {taille(c.taille)}{c.installe ? '' : ` · ${raisonNonPose(c)}`}</span>
          </li>
        {/each}
      </ul>
    {/if}
  </div>
{:else if donnees?.etat === 'ok' && (donnees.contenus.length || donnees.ignores?.length)}
  <div class="contenus cx-block">
    {#if donnees.ignores?.length}
      <p class="muted">
        ⚠ {donnees.ignores.length} fichier(s) de contenus illisible(s), peut-être abîmé(s) au téléchargement : {quelquesNoms(
          donnees.ignores,
        )}. Pour les réparer : retire le jeu de ta ludothèque puis remets-le (ses parties sont gardées).
      </p>
    {/if}
    <button class="btn" onclick={() => (ouvert = !ouvert)}>📦 Contenus additionnels : {resume(donnees.contenus)} {ouvert ? '▴' : '▾'}</button>
    {#if ouvert && donnees.systeme === 'wiiu'}
      <p class="muted">Inclus dans le fichier .wua du jeu : Cemu les charge tout seul, rien à installer.</p>
      <ul>
        {#each donnees.contenus as c (c.id)}<li><span>✅</span><span class="nom">{c.nom}</span></li>{/each}
      </ul>
    {:else if ouvert}
      <ul>
        {#each donnees.contenus as c (c.id)}
          <li>
            {#if c.installe}
              <span title="Déjà installé pour ton profil">✅</span>
              <span class="nom">{nomCourt(c.nom, titre)}</span>
            {:else}
              <!-- La case ET son nom : toute la ligne se clique (cible assez grande, souris comme manette). -->
              <label class="choix">
                <input
                  type="checkbox"
                  bind:group={coches}
                  value={c.id}
                  onchange={() => donnees && (coches = cochesCoherentes(donnees.contenus, coches, c.id))}
                />
                <span class="nom">{nomCourt(c.nom, titre)}</span>
              </label>
            {/if}
            <span class="muted">{libelleGenre(c.genre)} · {taille(c.taille)}{c.licence ? ' · licence' : ''}</span>
          </li>
        {/each}
      </ul>
      <div class="actions">
        <button class="btn petit" onclick={() => (coches = donnees!.contenus.filter((c) => !c.installe).map((c) => c.id))}>Tout cocher</button>
        <button class="btn petit primary" onclick={installer} disabled={enCours || !choisis.length}>
          {enCours ? 'Installation…' : `📦 Installer (${choisis.length}, ${taille(total)})`}
        </button>
      </div>
    {/if}
  </div>
{/if}

<style>
  .contenus {
    display: grid;
    gap: calc(6 * var(--u));
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: calc(4 * var(--u));
    max-height: calc(260 * var(--u));
    overflow: auto;
  }
  li {
    display: flex;
    gap: calc(6 * var(--u));
    align-items: center;
    flex-wrap: wrap;
  }
  .nom {
    flex: 1;
  }
  .choix {
    display: flex;
    flex: 1;
    gap: calc(6 * var(--u));
    align-items: center;
    min-height: var(--cible-min);
    cursor: pointer;
  }
  .actions {
    display: flex;
    gap: calc(6 * var(--u));
    flex-wrap: wrap;
  }
  p {
    margin: 0;
  }
</style>
