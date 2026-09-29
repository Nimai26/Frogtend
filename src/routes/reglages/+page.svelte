<script lang="ts">
  import { confirmer, toast } from '$lib/dialogues/fenetres.svelte';
  import { etat, reglerPc, reglerProfil, reinitialiserPc, reinitialiserProfil } from '$lib/etat.svelte';
  import { DEFAUTS_PC, ECHELLE_MAX, ECHELLE_MIN, verifierAdresse } from '$lib/reglages/reglages';
  import { jetonsDuSkin, libelleSkin, nomsDesSkins } from '$lib/skins/skins';

  const a = $derived(etat.profil.apparence);
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
        <p class="muted">
          Enregistrer ce skin dans ton compte Firehouse : bientôt (la route manque encore côté Firehouse).
        </p>
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
        <!-- Densité (grille de la ludothèque) et fond vidéo (servi par la vraie API) : affichés quand ils
             auront un effet, au lot 1. -->
      </div>

      <div class="actions">
        <button class="btn" onclick={() => reinitialiserProfil('apparence.echelle')}>↺ Taille d’origine</button>
        <button class="btn" onclick={toutReinitialiser}>↺ Toute l’apparence d’origine</button>
      </div>
    </div>
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
            onchange={(e) => reglerPc('firehouse.simule', e.currentTarget.checked)}
          />
          <span>Mode simulé (l’API de Firehouse pour Frogtend est en construction)</span>
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
  .actions {
    display: flex;
    gap: calc(8 * var(--u));
    justify-content: flex-end;
    flex-wrap: wrap;
  }
</style>
