<script lang="ts">
  // L'assistant jeux de Firehouse (lot 7). Une conversation par jeu (?jeu=<id>), ou générale ; gardée par Frogtend
  // tant qu'il est ouvert (Firehouse ne la mémorise pas). L'assistant PROPOSE des actions : Frogtend les montre,
  // demande un oui, refuse ce qu'il ne sait pas faire sans risque, et note chaque décision au journal.
  import { page } from '$app/state';
  import { goto } from '$app/navigation';
  import { tick } from 'svelte';
  import { isTauri } from '@tauri-apps/api/core';
  import { api } from '$lib/api';
  import { adresse, historiquePourEnvoi, juger, resumeDetails, type ActionProposee, type ContexteJeu } from '$lib/assistant/actions';
  import { conversations, type Echange } from '$lib/assistant/conversations.svelte';
  import { confirmer, toast } from '$lib/dialogues/fenetres.svelte';
  import { motifDuRefus } from '$lib/dialogues/messages';
  import { markdownSimple } from '$lib/markdown';
  import { installer, jouer } from '$lib/ludotheque/jeu.svelte';
  import { tele } from '$lib/ludotheque/telechargements.svelte';

  const id = $derived(page.url.searchParams.get('jeu') ? Number(page.url.searchParams.get('jeu')) : null);
  const cle = $derived(id === null ? 'general' : String(id));
  const echanges = $derived(conversations[cle] ?? []);
  let titre = $state<string | null>(null);
  let question = $state('');
  let enCours = $state(false);
  let fil = $state<HTMLElement | null>(null);

  $effect(() => {
    const n = id;
    titre = null;
    if (n !== null) api.fiche(n).then((r) => (titre = r.fiche.titre)).catch(() => {});
  });

  const contexte = $derived<ContexteJeu>({
    id,
    surPc: id !== null && tele.jeux[id]?.etat === 'telecharge',
    installe: id !== null && !!tele.jeux[id]?.installation,
  });

  async function defiler() {
    await tick();
    fil?.scrollTo({ top: fil.scrollHeight, behavior: 'smooth' });
  }

  async function poser(e?: SubmitEvent) {
    e?.preventDefault();
    const q = question.trim();
    if (!q || enCours) return;
    const avant = conversations[cle] ?? [];
    const historique = historiquePourEnvoi(avant.flatMap((x) => [{ role: 'user' as const, content: x.question }, { role: 'assistant' as const, content: x.texte }]));
    conversations[cle] = [...avant, { question: q, texte: '', actions: [], attente: true }];
    question = '';
    enCours = true;
    await defiler();
    try {
      const r = await api.assistant(q, id, historique);
      remplacerDernier({ question: q, texte: r.texte ?? '', actions: r.actions_proposees ?? [], attente: false });
    } catch (err) {
      remplacerDernier({ question: q, texte: '', actions: [], attente: false, erreur: motifDuRefus(err) });
    } finally {
      enCours = false;
      await defiler();
    }
  }

  function remplacerDernier(e: Echange) {
    const l = [...(conversations[cle] ?? [])];
    l[l.length - 1] = e;
    conversations[cle] = l;
  }

  async function journal(a: ActionProposee, decision: string) {
    if (isTauri()) await api.assistantJournal(`${a.type} — ${a.titre}`, decision).catch(() => {});
  }

  async function agir(a: ActionProposee) {
    const v = juger(a, contexte);
    if (!v.faisable) return;
    const details = resumeDetails(a);
    const oui = await confirmer(`🤖 ${a.titre}`, {
      message: [details && `Détails : ${details}`, `Risque annoncé : ${a.risque ?? 'non dit'}.`, a.type === 'ouvrir_url' ? `Adresse : ${adresse(a)}` : '']
        .filter(Boolean)
        .join('\n'),
      libelleValider: v.bouton,
      danger: a.risque === 'modifie le jeu' || a.risque === 'modifie la configuration',
    });
    await journal(a, oui ? 'acceptée' : 'refusée');
    if (!oui) return;
    try {
      if (a.type === 'lancer' && id !== null) {
        if (!contexte.installe && !(await installer(id))) return;
        await jouer(id);
      } else if (a.type === 'installer' && id !== null) await installer(id);
      else if (a.type === 'telecharger' && id !== null) await goto(`/jeu/${id}`);
      else if (a.type === 'demander_jeu') await goto('/demander');
      else if (a.type === 'ouvrir_url') {
        const u = adresse(a);
        if (u) {
          const { openUrl } = await import('@tauri-apps/plugin-opener');
          await openUrl(u);
        }
      }
    } catch (e) {
      toast(`Impossible : ${motifDuRefus(e)}`, 'erreur');
    }
  }

  function oublier() {
    conversations[cle] = [];
  }
</script>

<div class="assistant">
  <header>
    <h1>💬 Assistant jeux{titre ? ` — ${titre}` : ''}</h1>
    <p class="muted">
      Pose une question{titre ? ' sur ce jeu' : ''} : lancement, réglages, astuces, solution… L’assistant de Firehouse
      répond en 10 à 40 secondes, sources à l’appui. S’il propose une action, Frogtend te la montre avant de la faire.
    </p>
  </header>

  <div class="fil" bind:this={fil}>
    {#each echanges as e, i (i)}
      <div class="bulle moi">{e.question}</div>
      {#if e.attente}
        <div class="bulle lui muted">⏳ L’assistant réfléchit (10 à 40 secondes)…</div>
      {:else if e.erreur}
        <div class="bulle lui erreur">⚠ {e.erreur}</div>
      {:else}
        <div class="bulle lui">
          <div class="texte">{@html markdownSimple(e.texte)}</div>
          {#if e.actions.length}
            <ul class="actions">
              {#each e.actions as a, k (k)}
                {@const v = juger(a, contexte)}
                <li>
                  {#if v.faisable}
                    <button class="btn petit" onclick={() => agir(a)}>{v.bouton} — {a.titre}</button>
                  {:else}
                    <span class="muted">🚫 {a.titre} : {v.raison}</span>
                  {/if}
                </li>
              {/each}
            </ul>
          {/if}
        </div>
      {/if}
    {:else}
      <p class="muted">Par exemple : « Comment je lance ce jeu ? », « Où trouver la solution ? », « Quelle manette ? »</p>
    {/each}
  </div>

  <form class="question" onsubmit={poser}>
    <input type="text" bind:value={question} placeholder="Ta question…" aria-label="Ta question" disabled={enCours} />
    <button class="btn primary" type="submit" disabled={enCours || !question.trim()}>Envoyer</button>
    {#if echanges.length}<button class="btn" type="button" onclick={oublier} title="Effacer cette conversation">🗑</button>{/if}
  </form>
</div>

<style>
  .assistant {
    display: flex;
    flex-direction: column;
    gap: calc(12 * var(--u));
    padding: calc(20 * var(--u));
    max-width: calc(900 * var(--u));
    margin: 0 auto;
    width: 100%;
    height: 100%;
    box-sizing: border-box;
  }
  h1 {
    margin: 0;
  }
  header p {
    margin: calc(4 * var(--u)) 0 0;
  }
  .fil {
    flex: 1;
    overflow: auto;
    display: flex;
    flex-direction: column;
    gap: calc(8 * var(--u));
  }
  .bulle {
    max-width: 85%;
    padding: calc(10 * var(--u)) calc(14 * var(--u));
    border-radius: calc(10 * var(--u));
    white-space: pre-wrap;
    line-height: 1.5;
  }
  .moi {
    align-self: flex-end;
    background: color-mix(in srgb, var(--accent) 22%, transparent);
  }
  .lui {
    align-self: flex-start;
    background: color-mix(in srgb, var(--ink) 7%, transparent);
  }
  .erreur {
    box-shadow: inset calc(3 * var(--u)) 0 0 var(--warn);
  }
  .lui .texte {
    white-space: normal;
  }
  .lui .texte :global(h2),
  .lui .texte :global(h3),
  .lui .texte :global(h4) {
    margin: calc(6 * var(--u)) 0;
    font-size: 1.05em;
  }
  .lui .texte :global(p) {
    margin: calc(4 * var(--u)) 0;
  }
  .actions {
    list-style: none;
    margin: calc(8 * var(--u)) 0 0;
    padding: 0;
    display: grid;
    gap: calc(6 * var(--u));
  }
  .question {
    display: flex;
    gap: calc(8 * var(--u));
  }
  .question input {
    flex: 1;
  }
</style>
