<script lang="ts">
  import { api } from '$lib/api';
  import Emplacements from '$lib/reglages/Emplacements.svelte';
  import { confirmer, toast } from '$lib/dialogues/fenetres.svelte';
  import { motifDuRefus } from '$lib/dialogues/messages';
  import { etat, reglerPc, reglerProfil, reinitialiserPc, reinitialiserProfil } from '$lib/etat.svelte';
  import { ludo, rechargerPlateformes, synchroniser } from '$lib/ludotheque/ludotheque.svelte';
  import {
    DEFAUTS_PC,
    ECHELLE_MAX,
    ECHELLE_MIN,
    TAILLE_JAQUETTE_MAX,
    TAILLE_JAQUETTE_MIN,
    verifierAdresse,
  } from '$lib/reglages/reglages';
  import { jetonsDuSkin, libelleSkin, nomsDesSkins } from '$lib/skins/skins';

  const a = $derived(etat.profil.apparence);
  const l = $derived(etat.profil.ludotheque);

  $effect(() => {
    if (ludo.plateformes.length === 0) rechargerPlateformes().catch(() => {});
  });

  function basculerPlateforme(nom: string, visible: boolean) {
    const masquees = l.plateformesMasquees.filter((p) => p !== nom);
    reglerProfil('ludotheque.plateformesMasquees', visible ? masquees : [...masquees, nom]);
  }

  /** Après un changement d'adresse ou de mode : reconnexion du profil, puis synchronisation. */
  async function appliquerConnexion() {
    try {
      await api.reconnecter();
      await synchroniser();
    } catch (e) {
      toast(`Impossible de se connecter avec ces réglages : ${motifDuRefus(e)}`, 'erreur');
    }
  }
  const noms = $derived(etat.catalogue ? nomsDesSkins(etat.catalogue) : []);

  let adresse = $state(etat.pc.firehouse.adresse);

  /** Les jetons d'un skin, posés sur sa vignette d'aperçu (les couleurs viennent du skin, pas d'ici). */
  function styleApercu(nom: string) {
    if (!etat.catalogue) return '';
    const j = jetonsDuSkin(etat.catalogue, nom);
    return ['--bg', '--bg2', '--panel', '--line', '--ink', '--accent', '--accent2', '--on-accent']
      .map((k) => `${k}: ${j[k]}`)
      .join('; ');
  }

  async function enregistrerAdresse() {
    const r = verifierAdresse(adresse);
    if ('refus' in r) {
      toast(`Refusé : ${r.refus}`, 'erreur');
      return;
    }
    adresse = r.adresse;
    await reglerPc('firehouse.adresse', r.adresse);
    toast('Enregistré.');
    if (!etat.pc.firehouse.simule) await appliquerConnexion();
  }

  /** Le skin choisi ici devient celui du compte Firehouse (utilisé aussi dans le cockpit). */
  async function enregistrerSkin() {
    const nom = a.skin;
    if (!nom) return;
    try {
      await api.enregistrerSkin(nom);
      etat.skinFirehouse = nom;
      toast(`✅ Skin « ${libelleSkin(nom)} » enregistré dans ton compte Firehouse.`);
    } catch (e) {
      toast(`Refusé : ${motifDuRefus(e)}`, 'erreur');
    }
  }

  async function toutReinitialiser() {
    const oui = await confirmer('↺ Revenir aux réglages d’origine ?', {
      message: 'L’apparence de ton profil revient à celle d’origine. Les réglages de ce PC (Firehouse) ne changent pas.',
      libelleValider: 'Revenir à l’origine',
    });
    if (!oui) return;
    await reinitialiserProfil();
    toast('✅ Apparence remise à l’origine.');
  }
</script>

<div class="page">
  <section class="panel">
    <header>🎨 Apparence <span class="muted">— ton profil</span></header>
    <div class="corps">
      <div class="cx-block">
        <h3>Skin</h3>
        <p class="muted">
          Les skins viennent de Firehouse. Par défaut, Frogtend suit celui que tu as choisi dans Firehouse ; tu peux en
          prendre un autre ici, pour ce PC seulement.
        </p>
        <div class="skins" role="radiogroup" aria-label="Skin">
          <button
            class="btn suivre"
            role="radio"
            aria-checked={a.skin === null}
            class:actif={a.skin === null}
            onclick={() => reglerProfil('apparence.skin', null)}
          >
            ↺ Suivre mon skin Firehouse
          </button>
          {#each noms as nom (nom)}
            <button
              class="skin cx-card"
              role="radio"
              aria-checked={a.skin === nom}
              class:actif={a.skin === nom}
              style={styleApercu(nom)}
              onclick={() => reglerProfil('apparence.skin', nom)}
            >
              <span class="fond"><span class="surface"><span class="accent"></span><span class="accent2"></span></span></span>
              <span class="nom">{libelleSkin(nom)}</span>
            </button>
          {/each}
        </div>
        <div class="actions gauche">
          <button class="btn" onclick={enregistrerSkin} disabled={!a.skin || a.skin === etat.skinFirehouse}>
            💾 Enregistrer ce skin dans mon compte Firehouse
          </button>
          <span class="muted">
            {etat.skinFirehouse ? `Ton skin Firehouse : ${libelleSkin(etat.skinFirehouse)}` : ''}
          </span>
        </div>
      </div>

      <div class="cx-form-section">
        <label class="champ">
          <span>Taille du texte : {Math.round(a.echelle * 100)} %</span>
          <input
            type="range"
            min={ECHELLE_MIN}
            max={ECHELLE_MAX}
            step="0.05"
            value={a.echelle}
            onchange={(e) => reglerProfil('apparence.echelle', Number(e.currentTarget.value))}
          />
        </label>
        <label class="champ">
          <span>Animations</span>
          <select value={a.animations} onchange={(e) => reglerProfil('apparence.animations', e.currentTarget.value)}>
            <option value="normales">Normales</option>
            <option value="reduites">Réduites</option>
          </select>
        </label>
        <label class="champ case">
          <input
            type="checkbox"
            checked={a.fondVideo}
            onchange={(e) => reglerProfil('apparence.fondVideo', e.currentTarget.checked)}
          />
          <span>Fond vidéo (pour les skins qui en ont un, comme Firehouse)</span>
        </label>
      </div>

      <div class="actions">
        <button class="btn" onclick={() => reinitialiserProfil('apparence.echelle')}>↺ Taille d’origine</button>
        <button class="btn" onclick={toutReinitialiser}>↺ Toute l’apparence d’origine</button>
      </div>
    </div>
  </section>

  <section class="panel">
    <header>🎮 Ludothèque <span class="muted">— ton profil</span></header>
    <div class="corps">
      <div class="cx-form-section">
        <label class="champ">
          <span>Taille des jaquettes : {l.tailleJaquette} px</span>
          <input
            type="range"
            min={TAILLE_JAQUETTE_MIN}
            max={TAILLE_JAQUETTE_MAX}
            step="10"
            value={l.tailleJaquette}
            onchange={(e) => reglerProfil('ludotheque.tailleJaquette', Number(e.currentTarget.value))}
          />
        </label>
        <label class="champ">
          <span>Sous le titre d’un jeu, afficher</span>
          <select value={l.sousTitre} onchange={(e) => reglerProfil('ludotheque.sousTitre', e.currentTarget.value)}>
            <option value="developpeur">Le développeur</option>
            <option value="editeur">L’éditeur</option>
            <option value="annee">L’année</option>
            <option value="plateforme">La plateforme</option>
            <option value="rien">Rien</option>
          </select>
        </label>
        <label class="champ">
          <span>Tri par défaut</span>
          <select value={l.tri} onchange={(e) => reglerProfil('ludotheque.tri', e.currentTarget.value)}>
            <option value="titre">Titre</option>
            <option value="annee">Année (ancien d’abord)</option>
            <option value="annee_desc">Année (récent d’abord)</option>
          </select>
        </label>
        <label class="champ">
          <span>Espacement de la grille</span>
          <select value={a.densite} onchange={(e) => reglerProfil('apparence.densite', e.currentTarget.value)}>
            <option value="aeree">Aéré</option>
            <option value="compacte">Serré</option>
          </select>
        </label>
        <label class="champ case">
          <input
            type="checkbox"
            checked={l.panneauPlateformes}
            onchange={(e) => reglerProfil('ludotheque.panneauPlateformes', e.currentTarget.checked)}
          />
          <span>Colonne des plateformes (à gauche)</span>
        </label>
        <label class="champ case">
          <input
            type="checkbox"
            checked={l.panneauDetails}
            onchange={(e) => reglerProfil('ludotheque.panneauDetails', e.currentTarget.checked)}
          />
          <span>Colonne des détails (à droite)</span>
        </label>
      </div>

      <details class="cx-block">
        <summary>Plateformes affichées dans la colonne de gauche</summary>
        {#if ludo.plateformes.length === 0}
          <p class="muted">Aucune plateforme pour l’instant : synchronise la ludothèque d’abord.</p>
        {:else}
          <div class="plateformes">
            {#each ludo.plateformes as p (p.nom)}
              <label class="case">
                <input
                  type="checkbox"
                  checked={!l.plateformesMasquees.includes(p.nom)}
                  onchange={(e) => basculerPlateforme(p.nom, e.currentTarget.checked)}
                />
                <span>{p.nom} <span class="muted">({p.jeux})</span></span>
              </label>
            {/each}
          </div>
        {/if}
      </details>

      <div class="actions">
        <button class="btn" onclick={() => reinitialiserProfil('ludotheque')}>↺ Ludothèque d’origine</button>
      </div>
    </div>
  </section>

  <section class="panel">
    <header>📁 Emplacements des jeux <span class="muted">— ce PC</span></header>
    <div class="corps"><Emplacements /></div>
  </section>

  <section class="panel">
    <header>🔌 Firehouse <span class="muted">— ce PC</span></header>
    <div class="corps">
      <div class="cx-form-section">
        <label class="champ">
          <span>Adresse de Firehouse</span>
          <input type="url" bind:value={adresse} onkeydown={(e) => e.key === 'Enter' && enregistrerAdresse()} />
        </label>
        <label class="champ case">
          <input
            type="checkbox"
            checked={etat.pc.firehouse.simule}
            onchange={async (e) => {
              await reglerPc('firehouse.simule', e.currentTarget.checked);
              await appliquerConnexion();
            }}
          />
          <span>Mode simulé : des exemples, sans connexion à Firehouse (pour essayer ou travailler hors ligne)</span>
        </label>
      </div>
      <div class="actions">
        <button
          class="btn"
          onclick={async () => {
            await reinitialiserPc('firehouse.adresse');
            adresse = DEFAUTS_PC.firehouse.adresse;
            toast('Adresse d’origine rétablie.');
          }}
        >
          ↺ Adresse d’origine
        </button>
        <button class="btn primary" onclick={enregistrerAdresse}>Enregistrer l’adresse</button>
      </div>
    </div>
  </section>
</div>

<style>
  .page {
    display: grid;
    gap: calc(16 * var(--u));
  }
  .corps {
    padding: calc(16 * var(--u));
    display: grid;
    gap: calc(14 * var(--u));
  }
  .skins {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(calc(150 * var(--u)), 1fr));
    gap: calc(12 * var(--u));
    margin: calc(10 * var(--u)) 0;
  }
  .suivre {
    grid-column: 1 / -1;
    justify-self: start;
  }
  .suivre.actif,
  .skin.actif {
    border-color: var(--accent);
    box-shadow: 0 0 0 calc(2 * var(--u)) var(--accent);
  }
  .skin {
    font: inherit;
    padding: 0;
    overflow: hidden;
    cursor: pointer;
    color: var(--ink);
    text-align: left;
  }
  /* L'aperçu est peint avec les jetons DU skin montré (posés en style sur la vignette). */
  .fond {
    display: block;
    height: calc(64 * var(--u));
    background: var(--bg);
    padding: calc(10 * var(--u));
  }
  .surface {
    display: flex;
    gap: calc(6 * var(--u));
    height: 100%;
    background: var(--panel);
    border: 1px solid var(--line);
    border-radius: calc(8 * var(--u));
    padding: calc(8 * var(--u));
    align-items: flex-end;
  }
  .accent,
  .accent2 {
    width: calc(26 * var(--u));
    height: calc(12 * var(--u));
    border-radius: calc(4 * var(--u));
    background: var(--accent);
  }
  .accent2 {
    background: var(--accent2);
  }
  .nom {
    display: block;
    padding: calc(8 * var(--u)) calc(10 * var(--u));
    background: var(--bg2);
    font-size: calc(13 * var(--u));
  }
  .case {
    display: flex;
    align-items: center;
    gap: calc(10 * var(--u));
  }
  .case input {
    width: auto;
    min-height: 0;
    inline-size: calc(20 * var(--u));
    block-size: calc(20 * var(--u));
  }
  .case span {
    margin: 0;
    color: var(--ink);
  }
  .actions.gauche {
    justify-content: flex-start;
    align-items: center;
  }
  .plateformes {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(calc(240 * var(--u)), 1fr));
    gap: calc(6 * var(--u));
    margin-top: calc(10 * var(--u));
  }
  .actions {
    display: flex;
    gap: calc(8 * var(--u));
    justify-content: flex-end;
    flex-wrap: wrap;
  }
</style>
