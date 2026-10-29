<script lang="ts">
  // 🎮 Les boutons d'un jeu (lot 4d) : en haut, le dessin de la manette du joueur (décoratif, caché au lecteur d'écran) ;
  // en dessous, la liste « bouton de ta manette → bouton d'origine », le vrai contenu. La ligne choisie (clavier,
  // manette, souris) allume son bouton sur le dessin. Aucune couleur en dur : les jetons du thème.
  import { NOMS, raccourcis, sansDessin, schema, type IdBouton, type InfosBoutons } from './boutons';

  let { infos }: { infos: InfosBoutons } = $props();
  const touches = $derived(raccourcis(infos));

  const s = $derived(schema(infos));
  const ps = $derived(infos.famille === 'playstation');
  const noms = $derived(NOMS[infos.famille] ?? NOMS.xbox);
  let allume = $state<IdBouton | null>(null);
  const on = (id: IdBouton) => (allume === id ? 'b allume' : 'b');
</script>

<div class="schema">
  {#if s.etat === 'connu'}
    <p class="muted">
      À gauche, le bouton de ta manette ; à droite, ce qu’il remplace {s.console === 'le clavier' ? 'sur le clavier' : `sur la manette d’origine (${s.console === 'l’arcade' ? 'arcade' : s.console})`}.
    </p>
    <div class="corps-schema">
    <div class="dessin">
    <svg viewBox="0 0 400 250" aria-hidden="true" focusable="false">
      <path class="corps" d="M80,62 Q200,44 320,62 Q372,72 386,150 Q396,228 342,234 Q310,238 282,192 L118,192 Q90,238 58,234 Q4,228 14,150 Q28,72 80,62 Z" />
      <g class={on('lt')}><rect x="82" y="12" width="56" height="22" rx="7" /><text x="110" y="27">{noms.lt}</text></g>
      <g class={on('rt')}><rect x="262" y="12" width="56" height="22" rx="7" /><text x="290" y="27">{noms.rt}</text></g>
      <g class={on('lb')}><rect x="70" y="38" width="80" height="17" rx="8" /><text x="110" y="51">{noms.lb}</text></g>
      <g class={on('rb')}><rect x="250" y="38" width="80" height="17" rx="8" /><text x="290" y="51">{noms.rb}</text></g>
      <g class={on('stickg')}><circle cx="118" cy="112" r="24" /></g>
      <g class={on('l3')}><circle cx="118" cy="112" r="11" /></g>
      <g class={on('stickd')}><circle cx="246" cy="164" r="24" /></g>
      <g class={on('r3')}><circle cx="246" cy="164" r="11" /></g>
      <g class={on('croix')}>
        <path d="M144,140 h22 v16 h16 v22 h-16 v16 h-22 v-16 h-16 v-22 h16 Z" />
      </g>
      <g class={on('select')}><circle cx="174" cy="96" r="9" /></g>
      <g class={on('start')}><circle cx="226" cy="96" r="9" /></g>
      {#each [['a', 286, 136], ['b', 312, 110], ['x', 260, 110], ['y', 286, 84]] as [id, x, y] (id)}
        <g class={on(id as IdBouton)}>
          <circle cx={x} cy={y} r="13" />
          {#if !ps}
            <text x={x} y={Number(y) + 5}>{noms[id as IdBouton]}</text>
          {:else if id === 'a'}
            <path class="symbole" d="M{Number(x) - 5},{Number(y) - 5} l10,10 M{Number(x) + 5},{Number(y) - 5} l-10,10" />
          {:else if id === 'b'}
            <circle class="symbole" cx={x} cy={y} r="5.5" />
          {:else if id === 'x'}
            <rect class="symbole" x={Number(x) - 5} y={Number(y) - 5} width="10" height="10" />
          {:else}
            <path class="symbole" d="M{x},{Number(y) - 6} l6,10 h-12 Z" />
          {/if}
        </g>
      {/each}
    </svg>
    </div>
    <div class="lignes">
      {#each s.lignes as l (l.id)}
        <button
          class="btn ligne"
          aria-label="{l.nom} : {l.role}"
          onfocus={() => (allume = l.id)}
          onblur={() => (allume = null)}
          onpointerenter={() => (allume = l.id)}
          onpointerleave={() => (allume = null)}
        >
          <span class="nom">{l.nom}</span>
          <span aria-hidden="true">→</span>
          <span class="role">{l.role}</span>
        </button>
      {/each}
    </div>
    </div>
    {#if s.remarque}<p class="muted">{s.remarque}</p>{/if}
  {:else}
    <p class="muted">{sansDessin(s)}</p>
  {/if}
  <h2>⚡ Les raccourcis de Frogtend pendant une partie</h2>
  <div class="lignes">
    {#each touches as t (t.geste)}
      <button class="btn ligne" aria-label="{t.geste} : {t.effet}">
        <span class="nom">{t.geste}</span>
        <span aria-hidden="true">→</span>
        <span class="role">{t.effet}</span>
      </button>
    {/each}
  </div>
  <p class="muted">Ils se changent dans Frogtend, après la partie : ⚙ Options ▸ Émulateurs.</p>
</div>

<style>
  .schema {
    display: grid;
    gap: calc(10 * var(--u));
  }
  .schema p,
  .schema h2 {
    margin: 0;
  }
  h2 {
    font-size: calc(18 * var(--u));
    margin-top: calc(6 * var(--u));
  }
  /* Le dessin reste visible pendant que la liste défile (sa ligne choisie l'allume) ; en grand (Taodbox, télé) : le
   * dessin à gauche, la liste à droite. */
  .schema {
    container-type: inline-size;
  }
  .dessin {
    position: sticky;
    top: 0;
    z-index: 1;
    background: var(--panel);
  }
  svg {
    display: block;
    width: 100%;
    max-height: 30vh;
  }
  @container (min-width: 720px) {
    .corps-schema {
      display: grid;
      grid-template-columns: 1fr 1fr;
      gap: calc(16 * var(--u));
      align-items: start;
    }
    svg {
      max-height: 60vh;
    }
  }
  .corps {
    fill: var(--bg2);
    stroke: var(--ink);
    stroke-opacity: 0.5;
    stroke-width: 2.5;
    vector-effect: non-scaling-stroke;
  }
  .b :is(rect, circle, path):not(.symbole) {
    fill: var(--bg);
    stroke: var(--ink);
    stroke-opacity: 0.6;
    stroke-width: 2;
    vector-effect: non-scaling-stroke;
  }
  .b text {
    fill: var(--ink);
    font-size: 17px;
    font-weight: 700;
    text-anchor: middle;
  }
  .b .symbole {
    fill: none;
    stroke: var(--ink);
    stroke-width: 2;
    vector-effect: non-scaling-stroke;
  }
  .b.allume :is(rect, circle, path):not(.symbole) {
    fill: var(--accent);
  }
  .b.allume text {
    fill: var(--on-accent);
  }
  .b.allume .symbole {
    stroke: var(--on-accent);
  }
  .lignes {
    display: grid;
    gap: calc(4 * var(--u));
  }
  .ligne {
    display: grid;
    grid-template-columns: minmax(calc(110 * var(--u)), auto) auto 1fr;
    gap: calc(8 * var(--u));
    text-align: left;
    justify-items: start;
  }
  .nom {
    font-weight: 700;
  }
</style>
