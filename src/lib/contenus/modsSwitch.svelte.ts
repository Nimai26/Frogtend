// 🧩 Les mods Switch d'un jeu (contrat 1.44, ✅ Seb 18/11) : la logique partagée par la bande de droite (BlocMods,
// résumé) et la page des mods (PageMods, bureau et canapé). Servis par Firehouse (validés par un admin), téléchargés
// après l'accord chiffré de la personne, un interrupteur par mod pour toute la maison (Eden comme Ryubing), posés dans
// l'émulateur au lancement seulement ; les musiques de MK8D (contrat 1.55) jamais téléchargées sans un oui chiffré.
// Aucun `$effect` ici : chaque vue crée son instance et garde ses effets (changement de jeu ou de profil → `ouvrir` ;
// suivi toutes les 2 s → `releverSuivi`). Les textes sont dans `mods.ts`.
import { api, estErreurCoeur, taille, type ModSwitchVu, type ModsSwitch, type SuiviModsSwitch } from '$lib/api';
import { choisir, cocher, confirmer, toast } from '$lib/dialogues/fenetres.svelte';
import { motifDuRefus } from '$lib/dialogues/messages';
import { etat, reglerProfil } from '$lib/etat.svelte';
import { lancementImpossible } from '$lib/ludotheque/jeu.svelte';
import { debitConnu } from '$lib/ludotheque/telechargements.svelte';
import { auCanape } from '$lib/taodbox/canape';
import { selectionner } from '$lib/taodbox/manette.svelte';
import { tick } from 'svelte';
import { noterNombreDeMods } from './pageMods';
import {
  aMettreAJour,
  aTelecharger,
  amorcesCommunes,
  cibleDuFocus,
  clesVues,
  conflitAvec,
  dureeEstimee,
  estMusique,
  messageAllumageMusique,
  messageFinMods,
  messagePreparer,
  messageTelechargement,
  messageToutTelecharger,
  ligneTouchentParties,
  musiquesQuiServiront,
  nMods,
  plusMusiques,
  recommande,
  texteSimple,
  titreAllumageMusique,
  titrePreparer,
  titreTelechargement,
  totalOctets,
} from './mods';

/** Les mises à jour automatiques en cours (profil:jeu) : elles ne partent que d'un endroit (`charger`), et jamais deux
 * fois même si la bande et la page sont ouvertes ensemble. */
const autoEnCours = new Set<string>();

export class ModsSwitchJeu {
  id = $state(0);
  titre = $state('');
  /** Le profil pour lequel la liste a été lue (jamais montrée à un autre). */
  profil: string | null = null;
  liste = $state<ModsSwitch | null>(null);
  suivi = $state<SuiviModsSwitch | null>(null);
  /** Les mods demandés à l'instant (en attente dans la file du cœur). */
  enFile = $state<string[]>([]);
  accordEnCours = $state(false);
  basculement = $state<string | null>(null);
  retrait = $state<string | null>(null);
  /** Contrat 1.55 : la taille des musiques est demandée à Firehouse (5 s au plus). */
  regardeTaille = $state(false);
  /** « 🎵 Préparer les musiques » en cours. */
  preparation = $state(false);
  /** Ce que le lecteur d'écran entend à la fin d'un téléchargement (« Mods téléchargés »). */
  annonceFin = $state('');
  charge = $state(false);

  /** Chaque ouverture (autre jeu, autre profil) : ce qui arrive d'une lecture d'avant est jeté. */
  private lecture = 0;
  /** L'ordre des mods AFFICHÉS quand l'action a commencé (pour trouver « la suivante » d'une ligne retirée). */
  private ordre: string[] = [];
  /** La vue dit quelle liste elle affiche (filtrée) : le focus est rendu dans celle-là. */
  ordreVisible: () => string[] = () => this.mods.map((m) => m.cle);
  /** La vue dit où rendre le focus après une action : le conteneur des lignes et l'élément de secours. */
  lieuDuFocus: () => { conteneur: ParentNode | null; secours: HTMLElement | null } = () => ({ conteneur: null, secours: null });

  mods = $derived(this.liste?.mods ?? []);
  nouveaux = $derived(aTelecharger(this.mods));

  // Pendant une partie ou un émulateur ouvert seul : rien ne change (il réécrirait ses réglages par-dessus).
  get bloque() {
    return lancementImpossible();
  }
  get occupe() {
    return !!this.suivi?.actif || this.accordEnCours;
  }
  get horsLigne() {
    return this.liste?.etat !== 'ok';
  }
  get actif() {
    return !!this.suivi?.actif;
  }
  get pose() {
    return this.liste?.derniere_pose ?? null;
  }

  /** Un autre jeu, ou un autre profil : tout est remis à zéro, puis la liste est lue en tâche de fond. */
  ouvrir(id: number, titre: string, profil: string | null) {
    this.lecture++;
    this.id = id;
    this.titre = titre;
    this.profil = profil;
    this.liste = null;
    this.suivi = null;
    this.enFile = [];
    this.annonceFin = '';
    this.charge = false;
    void this.charger(this.lecture);
  }

  private vivante(g: number) {
    return g === this.lecture;
  }

  /** En tâche de fond : la vue est déjà affichée, la liste vient ensuite (ou la dernière vue, hors ligne). */
  async charger(g = this.lecture) {
    const [jeu, profil] = [this.id, this.profil];
    const l = await api.modsSwitchLister(jeu).catch(() => null);
    if (!this.vivante(g)) return;
    this.liste = l;
    this.charge = true;
    if (l && l.etat !== 'injoignable') noterNombreDeMods(profil, jeu, l.etat === 'rien' ? 0 : l.mods.length);
    const su = await api.modsSwitchSuivi(jeu).catch(() => null);
    if (!this.vivante(g)) return;
    this.suivi = su;
    // « À l'avenir, mettre à jour tout seul » (choisi par la personne, désactivable dans les options) : les nouvelles
    // révisions partent seules — jamais pendant un lancement, jamais deux fois.
    const majs = aMettreAJour(l?.mods ?? []);
    const cle = `${profil ?? ''}:${jeu}`;
    if (l?.etat === 'ok' && majs.length && etat.profil.modsSwitch?.auto && !su?.actif && !lancementImpossible() && !this.accordEnCours && !autoEnCours.has(cle)) {
      autoEnCours.add(cle);
      this.accordEnCours = true;
      try {
        await this.telecharger(majs, true);
      } finally {
        this.accordEnCours = false;
        autoEnCours.delete(cle);
      }
    }
  }

  /** Relit la liste (après un choix, un téléchargement) ; en cas d'échec, l'ancienne reste (jamais une vue vide). */
  async relire() {
    const [g, jeu] = [this.lecture, this.id];
    const l = await api.modsSwitchLister(jeu).catch(() => null);
    if (!l || !this.vivante(g)) return;
    this.liste = l;
    if (l.etat !== 'injoignable') noterNombreDeMods(this.profil, jeu, l.etat === 'rien' ? 0 : l.mods.length);
  }

  /** Le début du suivi toutes les 2 s (la vue a vu `actif` passer à vrai) : rend la lecture en cours. */
  debutSuivi(): number {
    this.annonceFin = '';
    return this.lecture;
  }

  /** Une relecture du suivi (rien n'est demandé à Firehouse, seulement au cœur) ; à la fin, la liste est relue. */
  async releverSuivi(g: number) {
    const su = await api.modsSwitchSuivi(this.id).catch(() => null);
    if (!su || !this.vivante(g)) return;
    this.suivi = su;
    // La fin se dit : un toast (combien sont prêts) et, pour le lecteur d'écran, « Mods téléchargés ».
    if (!su.actif) await this.finir(su);
  }

  private noterOrdre() {
    this.ordre = this.ordreVisible();
  }

  /** Après une action (Télécharger, Retirer…), l'élément qui avait le focus a pu disparaître : le focus revient à la
   * ligne du mod (ou à la suivante dans la liste AFFICHÉE, sinon à l'élément de secours), jamais en haut de la page. */
  async rendreFocus(cle: string, conteneur: ParentNode | null, secours: HTMLElement | null) {
    await tick();
    const a = document.activeElement;
    if (a && a !== document.body && a.isConnected) return;
    // La commande de la ligne (ou d'une suivante), que la manette sait viser — jamais la ligne `tabindex="-1"`
    // elle-même : `deplacer` repartirait alors du haut de la page.
    const lignes = [...(conteneur?.querySelectorAll<HTMLElement>('li[data-cle]') ?? [])];
    const cible = cibleDuFocus(lignes, cle, this.ordre, secours);
    if (cible) selectionner(cible);
  }

  private async rendreFocusIci(cle: string) {
    const { conteneur, secours } = this.lieuDuFocus();
    await this.rendreFocus(cle, conteneur, secours);
  }

  /** Met en file la liste que la personne a VUE : si elle a changé entre-temps, le cœur refuse et on la relit.
   * `musiques` (contrat 1.55) : la taille des musiques annoncée dans l'accord — le cœur note alors le oui et les prépare
   * (sans taille annoncée, jamais). */
  async telecharger(l: ModSwitchVu[], auto: boolean, musiques: number | null = null, g = this.lecture) {
    const [jeu, titre] = [this.id, this.titre];
    // Le jeu ou le profil a changé pendant l'accord : rien n'est demandé au cœur pour l'autre.
    if (!this.vivante(g)) return;
    try {
      const n = await (musiques && musiques > 0
        ? api.modsSwitchTelecharger(jeu, clesVues(l), totalOctets(l), auto, musiques)
        : api.modsSwitchTelecharger(jeu, clesVues(l), totalOctets(l), auto));
      if (auto && n) toast(`🧩 ${nMods(n)} de « ${titre} » se ${n > 1 ? 'mettent' : 'met'} à jour tout seul${n > 1 ? 's' : ''} (${taille(totalOctets(l))}), en arrière-plan.`);
      if (!this.vivante(g)) return;
      this.enFile = l.map((m) => m.cle);
      const su = await api.modsSwitchSuivi(jeu).catch(() => null);
      if (!this.vivante(g) || !su) return;
      this.suivi = su;
      // Un mod très léger a pu finir avant cette première lecture : la minuterie ne démarrera jamais, la fin se dit ici.
      if (!su.actif && this.enFile.length) await this.finir(su);
    } catch (e) {
      if (estErreurCoeur(e) && e.sorte === 'conflit') {
        toast(`🧩 ${texteSimple(motifDuRefus(e), false)}`, 'alerte');
        if (this.vivante(g)) await this.relire();
        return;
      }
      // La mise à jour automatique arrive après une autre (l'autre vue, une partie précédente) : rien à dire.
      if (auto && estErreurCoeur(e) && /déjà en train d.arriver/i.test(motifDuRefus(e))) return;
      toast(`Téléchargement impossible : ${texteSimple(motifDuRefus(e), false)}`, 'erreur');
    }
  }

  /** La fin d'un téléchargement : un toast (combien sont prêts), « Mods téléchargés » pour le lecteur d'écran, la
   * liste relue (nouveaux états). */
  async finir(su: SuiviModsSwitch) {
    this.enFile = [];
    const fin = messageFinMods(su);
    if (fin) toast(fin, 'ok');
    this.annonceFin = su.faits ? 'Mods téléchargés.' : 'Téléchargement des mods terminé.';
    await this.relire();
  }

  /** Contrat 1.55 : la taille des musiques à annoncer (« + 41 Mo de musiques ») quand des mods de musique sont parmi
   * `l` — lue chez Firehouse (5 s au plus ; « ⏳ Je regarde la taille des musiques… » pendant ce temps), sinon la
   * dernière connue ; hors ligne, la dernière connue tout de suite. `eteindre` : le mod éteint en même temps. */
  async tailleMusiques(l: ModSwitchVu[], eteindre: string | null = null): Promise<number | null> {
    const cles = l.filter(estMusique).map((m) => m.cle);
    const connue = this.liste?.musiques?.taille_connue ?? null;
    if (this.liste?.etat !== 'ok') return connue;
    this.regardeTaille = true;
    try {
      return (await api.modsSwitchMusiquesTaille(this.id, cles, eteindre).catch(() => null)) ?? connue;
    } finally {
      this.regardeTaille = false;
    }
  }

  /** « ⬇ Télécharger (X Mo) » : l'accord chiffré (taille, durée si connue, toute la maison, abri des parties ; un mod
   * de musique qui servira annonce aussi les musiques). */
  async proposer(m: ModSwitchVu) {
    if (this.occupe || this.bloque) return;
    const g = this.lecture;
    this.accordEnCours = true;
    try {
      this.noterOrdre();
      const annonce = musiquesQuiServiront([m], this.mods).length ? { octets: await this.tailleMusiques([m]) } : null;
      const oui = await confirmer(titreTelechargement([m], debitConnu(), false, annonce), {
        message: messageTelechargement([m], false, annonce),
        libelleValider: '⬇ Télécharger',
      });
      if (oui) await this.telecharger([m], false, annonce?.octets ?? null, g);
    } finally {
      this.accordEnCours = false;
    }
    await this.rendreFocusIci(m.cle);
  }

  /** « Tout télécharger » : seulement les mods `vus` (l'onglet ou le filtre affiché, ✅ Seb 26/11) ; seuls les
   * ⭐ recommandés sont cochés d'office (`coches` pour en décider autrement) ; le total recalculé à chaque case. */
  async proposerTout(vus: ModSwitchVu[], coches: boolean[] = vus.map(recommande)) {
    if (this.occupe || this.bloque || !vus.length) return;
    const g = this.lecture;
    this.accordEnCours = true;
    this.noterOrdre();
    try {
      const debit = debitConnu();
      const servent = musiquesQuiServiront(vus, this.mods);
      const mus = servent.length ? await this.tailleMusiques(servent) : null;
      const annonce = servent.length ? { octets: mus } : null;
      const choisis = await cocher<string>(
        `🧩 Télécharger des mods pour « ${this.titre} » ?`,
        vus.map((m) => ({ valeur: m.cle, libelle: m.nom, detail: `${recommande(m) ? '⭐ ' : ''}${taille(m.taille)}` })),
        {
          coches,
          // Ni la liste (elle est en cases, le résumé dit le total) ni les parties touchées (selon les cases cochées).
          message: messageToutTelecharger(vus, annonce),
          toutCocher: true,
          resume: (v) => {
            const choisis = vus.filter((m) => v.includes(m.cle));
            const musique = choisis.some((m) => servent.includes(m));
            // La taille des musiques vaut pour TOUS les mods de musique : des mods décochés, elle n'est qu'environ.
            const environ = servent.some((m) => !choisis.includes(m));
            const t = totalOctets(choisis) + (musique && mus ? mus : 0);
            const d = t > 0 ? dureeEstimee(t, debit) : null;
            const parties = ligneTouchentParties(choisis);
            return `${nMods(v.length)}, ${taille(totalOctets(choisis))}${musique ? ` ${plusMusiques(mus, environ)}` : ''}${d ? `, ${d}` : ''}${parties ? ` · ${parties}` : ''}`;
          },
          refus: (v) => (v.length ? null : 'Coche au moins un mod.'),
          libelleValider: '⬇ Télécharger',
        },
      );
      if (choisis?.length) {
        const l = vus.filter((m) => choisis.includes(m.cle));
        await this.telecharger(l, false, l.some((m) => servent.includes(m)) ? mus : null, g);
      }
    } finally {
      this.accordEnCours = false;
    }
    await this.rendreFocusIci(vus[0].cle);
  }

  /** Une nouvelle révision : le même accord, avec « à l'avenir, mettre à jour tout seul ». */
  async proposerMiseAJour(m: ModSwitchVu) {
    if (this.occupe || this.bloque) return;
    const g = this.lecture;
    this.accordEnCours = true;
    try {
      const annonce = m.allume && estMusique(m) ? { octets: await this.tailleMusiques([m]) } : null;
      const c = await choisir<'oui' | 'auto' | 'non'>(
        titreTelechargement([m], debitConnu(), true, annonce),
        [
          { valeur: 'oui', libelle: '⬇ Mettre à jour' },
          {
            valeur: 'auto',
            libelle: '⬇ Mettre à jour, et à l’avenir mettre à jour tout seul',
            // Au canapé, l'interrupteur est sur l'écran « Réglages du jeu » (Y) ; au bureau, dans les options.
            detail: auCanape() ? 'Sans redemander (désactivable dans « ⚙ Réglages du jeu » (Y))' : 'Sans redemander (désactivable dans ⚙ Options ▸ 💾 Sauvegarde)',
          },
          { valeur: 'non', libelle: 'Pas maintenant' },
        ],
        messageTelechargement([m], true, annonce),
      );
      if (c !== 'oui' && c !== 'auto') return;
      this.noterOrdre();
      if (!this.vivante(g)) return;
      if (c === 'auto') await reglerProfil('modsSwitch.auto', true);
      await this.telecharger([m], c === 'auto', annonce?.octets ?? null, g);
    } finally {
      this.accordEnCours = false;
    }
    await this.rendreFocusIci(m.cle);
  }

  /** Demande d'éteindre l'autre mod (conflit), puis enregistre ; rend faux si la personne refuse. */
  async accordConflit(m: ModSwitchVu, b: ModSwitchVu): Promise<boolean> {
    return confirmer(`Ne va pas avec « ${b.nom} » : l’allumer éteint l’autre`, {
      message: [
        amorcesCommunes(m, b) ? `« ${m.nom} » et « ${b.nom} » changent la même musique du jeu.` : `« ${m.nom} » et « ${b.nom} » ne peuvent pas marcher ensemble.`,
        `Allumer « ${m.nom} » éteint « ${b.nom} », pour toute la maison.`,
        'Frogtend met tes parties de ce jeu à l’abri avant la prochaine partie.',
      ].join('\n'),
      libelleValider: 'Allumer',
    });
  }

  async basculer(m: ModSwitchVu) {
    // Pris AVANT toute fenêtre : un double appui n'ouvre jamais deux fenêtres ni deux enregistrements.
    if (this.bloque || this.basculement || !this.liste) return;
    const g = this.lecture;
    this.basculement = m.cle;
    const allume = !m.allume;
    try {
      let eteindre: string | null = null;
      let musiques: number | null = null;
      if (allume) {
        // Un conflit déclaré, ou deux mods de musique qui changent la même musique (contrat 1.55) : éteindre l'autre,
        // ou renoncer.
        const b = conflitAvec(this.mods, m.cle);
        if (estMusique(m)) {
          // Un mod de musique : l'accord annonce la taille des musiques ; un conflit de musique se règle dans la MÊME
          // fenêtre.
          const musical = b && estMusique(b) && amorcesCommunes(m, b) ? b : null;
          if (b && !musical) {
            if (!(await this.accordConflit(m, b))) return;
          }
          if (b) eteindre = b.cle;
          musiques = await this.tailleMusiques([m], eteindre);
          const oui = await confirmer(titreAllumageMusique(m, musiques), { message: messageAllumageMusique(m, musiques, musical), libelleValider: 'Allumer' });
          if (!oui) return;
        } else if (b) {
          if (!(await this.accordConflit(m, b))) return;
          eteindre = b.cle;
        }
      } else if (m.touche_sauvegardes) {
        const oui = await confirmer(`Éteindre « ${m.nom} » ?`, {
          message: 'Ce mod touche à tes parties : le jeu peut parfois refuser une partie faite avec lui.\nFrogtend met tes parties de ce jeu à l’abri avant la prochaine partie.\nCe choix vaut pour toute la maison.',
          libelleValider: 'Éteindre',
        });
        if (!oui) return;
      }
      await this.enregistrer(m, allume, eteindre, musiques, g);
      if (this.vivante(g)) await this.relire();
    } finally {
      this.basculement = null;
    }
  }

  /** `g` : la lecture pendant laquelle la personne a choisi ; le jeu ou le profil a changé depuis (une fenêtre restée
   * ouverte) : rien n'est écrit. */
  async enregistrer(m: ModSwitchVu, allume: boolean, eteindre: string | null, musiques: number | null = null, g = this.lecture) {
    if (!this.vivante(g)) return;
    try {
      // La taille des musiques n'est donnée que si elle a été annoncée : le cœur note alors le oui et les prépare.
      const change = await (musiques && musiques > 0
        ? api.modsSwitchChoisir(this.id, m.cle, allume, eteindre, musiques)
        : api.modsSwitchChoisir(this.id, m.cle, allume, eteindre));
      if (change) toast(`✅ « ${m.nom} » ${allume ? 'allumé' : 'éteint'} : tes parties de ce jeu seront mises à l’abri avant la prochaine partie.`);
    } catch (e) {
      // Un conflit que la liste affichée ne montrait pas (elle a changé) : relue, puis la même question.
      if (allume && !eteindre && estErreurCoeur(e) && e.sorte === 'refus') {
        await this.relire();
        const b = this.vivante(g) ? conflitAvec(this.liste?.mods ?? [], m.cle) : null;
        if (b) {
          if (await this.accordConflit(m, b)) await this.enregistrer(m, true, b.cle, musiques, g);
          return;
        }
      }
      toast(`Impossible d’enregistrer : ${texteSimple(motifDuRefus(e), false)}`, 'erreur');
    }
  }

  /** « 🎵 Préparer les musiques (≈ N Mo) » : la taille relue, l'accord, puis le cœur note le oui et les prépare. */
  async preparer() {
    if (this.bloque || this.preparation || !this.liste) return;
    const g = this.lecture;
    this.preparation = true;
    try {
      const octets = await this.tailleMusiques([]);
      if (!octets) {
        toast('🎵 La taille des musiques n’est pas encore connue (Firehouse ne répond pas) : rien n’est téléchargé. Réessaie plus tard.', 'alerte');
        return;
      }
      const oui = await confirmer(titrePreparer(octets), { message: messagePreparer(this.liste), libelleValider: '🎵 Préparer' });
      if (!oui || !this.vivante(g)) return;
      try {
        await api.modsSwitchMusiquesPreparer(this.id, octets);
        toast('🎵 Les musiques des mods se préparent en arrière-plan.', 'ok');
      } catch (e) {
        toast(`Impossible de préparer les musiques : ${texteSimple(motifDuRefus(e), false)}`, 'erreur');
      }
      await this.relire();
    } finally {
      this.preparation = false;
    }
    // Le bouton « Préparer » a disparu : le focus va à l'élément de secours (jamais en haut de la page).
    await tick();
    const a = document.activeElement;
    if (a && a !== document.body && a.isConnected) return;
    const { secours } = this.lieuDuFocus();
    if (secours) selectionner(secours);
  }

  async retirer(m: ModSwitchVu) {
    if (this.bloque || this.retrait || this.occupe) return;
    const g = this.lecture;
    this.retrait = m.cle;
    this.noterOrdre();
    try {
      const oui = await confirmer(`Retirer « ${m.nom} » de ce PC ?`, {
        message: 'Firehouse ne le propose plus. Il quitte les émulateurs à la prochaine partie.\nFrogtend met tes parties de ce jeu à l’abri avant.',
        libelleValider: 'Retirer',
        danger: true,
      });
      if (!oui || !this.vivante(g)) return;
      try {
        await api.modsSwitchRetirer(this.id, m.cle);
        toast(`✅ « ${m.nom} » retiré de ce PC.`);
      } catch (e) {
        toast(`Impossible de retirer : ${texteSimple(motifDuRefus(e), false)}`, 'erreur');
      }
      await this.relire();
    } finally {
      this.retrait = null;
    }
    await this.rendreFocusIci(m.cle);
  }

  /** Le mod est-il dans la file du cœur (en cours ou en attente) ? */
  dansLaFile(m: ModSwitchVu): boolean {
    return !!this.suivi?.actif && (this.enFile.includes(m.cle) || this.suivi.en_cours === m.nom);
  }

  /** Le mod est-il celui qui se télécharge en ce moment ? */
  enCours(m: ModSwitchVu): boolean {
    return !!this.suivi?.actif && this.suivi.en_cours === m.nom;
  }
}
