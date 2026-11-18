// Les mods Switch (contrat 1.44, ✅ Seb 18/11) : ce que la fiche dit de chaque mod, sans jargon.
import { describe, expect, it, vi } from 'vitest';

vi.mock('$lib/api', () => ({
  taille: (o: number) => `${o} o`,
  duree: (s: number) => `${Math.round(s)} s`,
}));

import type { ModSwitchVu, ModsSwitch, SuiviModsSwitch } from '$lib/api';
import {
  aMettreAJour,
  aTelecharger,
  aUnInterrupteur,
  blocModsVisible,
  cibleDuFocus,
  clesVues,
  conflitAvec,
  dateVue,
  descriptionLongue,
  dureeEstimee,
  estInstalle,
  etapeModsAnnoncee,
  ligneAuteur,
  ligneEtatListe,
  ligneProgresMods,
  allumePasPose,
  lignesDernierePose,
  lignesFinMods,
  majuscule,
  mentions,
  messageFinMods,
  messageModsEnRoute,
  messageTelechargement,
  raisonGrise,
  refusModsEnRoute,
  texteSimple,
  pasEncoreEssaye,
  pourcentEnCours,
  resumeMods,
  texteInterrupteurMod,
  titreTelechargement,
  totalOctets,
} from './mods';

/** Un mod de la forme de `mods_switch_lister` (mods_switch.rs, `ModVu`). */
function mod(x: Partial<ModSwitchVu> = {}): ModSwitchVu {
  return {
    cle: 'wollnashorn-0123456789ab',
    nom: '1080p',
    description: 'Rend le jeu en 1080p.',
    auteur: 'Wollnashorn',
    licence: 'MIT',
    source: 'https://example.invalid/commit',
    revision: 1,
    revision_rangee: null,
    etat: 'pas_telecharge',
    compat: 'compatible',
    raison: null,
    allume: false,
    taille: 1000,
    sha256: 'ab'.repeat(32),
    conflits: [],
    reglage_requis: '',
    touche_sauvegardes: false,
    contenu: { exefs: true, romfs: false, cheats: false },
    emulateurs_essayes: ['Ryubing'],
    servi: true,
    ...x,
  };
}

const SUIVI: SuiviModsSwitch = { actif: true, en_cours: '1080p', recus: 450, total: 1000, faits: 0, a_faire: 3, erreurs: [], arret: null };

describe('l’état d’un mod', () => {
  it('un mod rangé a un interrupteur, jamais un mod grisé', () => {
    expect(aUnInterrupteur(mod({ etat: 'telecharge' }))).toBe(true);
    expect(aUnInterrupteur(mod({ etat: 'revision_dispo' }))).toBe(true);
    expect(aUnInterrupteur(mod({ etat: 'incompatible', revision_rangee: 1 }))).toBe(false);
    expect(aUnInterrupteur(mod())).toBe(false);
    expect(estInstalle(mod({ etat: 'incompatible', revision_rangee: 1 }))).toBe(true);
    expect(estInstalle(mod({ etat: 'incompatible' }))).toBe(false);
  });

  it('à télécharger : jamais un mod grisé ni un mod que Firehouse ne sert plus', () => {
    const l = [mod({ cle: 'a-1' }), mod({ cle: 'b-1', etat: 'incompatible' }), mod({ cle: 'c-1', servi: false }), mod({ cle: 'd-1', etat: 'revision_dispo' })];
    expect(aTelecharger(l).map((m) => m.cle)).toEqual(['a-1']);
    expect(aMettreAJour(l).map((m) => m.cle)).toEqual(['d-1']);
  });

  it('l’accord porte ce que la personne a vu (clé, révision, taille, empreinte) et le total', () => {
    const l = [mod({ cle: 'a-1', revision: 2, taille: 10, sha256: 'aa' }), mod({ cle: 'b-1', taille: 5, sha256: 'bb' })];
    expect(clesVues(l)).toEqual([
      { cle: 'a-1', revision: 2, taille: 10, sha256: 'aa' },
      { cle: 'b-1', revision: 1, taille: 5, sha256: 'bb' },
    ]);
    expect(totalOctets(l)).toBe(15);
  });

  it('le résumé dit ce qui est allumé, à télécharger, à mettre à jour, grisé', () => {
    expect(resumeMods([])).toBe('aucun');
    expect(resumeMods([mod(), mod({ cle: 'b-1' })])).toBe('2 à télécharger');
    expect(
      resumeMods([
        mod({ cle: 'a-1', etat: 'telecharge', allume: true }),
        mod({ cle: 'b-1', etat: 'revision_dispo', allume: false }),
        mod({ cle: 'c-1' }),
        mod({ cle: 'd-1', etat: 'incompatible' }),
      ]),
    ).toBe('1 allumé sur 2, 1 à télécharger, 1 mise à jour, 1 ne va pas avec ta version');
    expect(resumeMods([mod({ etat: 'incompatible' }), mod({ cle: 'b-1', etat: 'incompatible' })])).toBe('2 ne vont pas avec ta version');
  });

  it('la ligne de l’auteur dit « licence : aucune » quand il n’y en a pas', () => {
    expect(ligneAuteur(mod())).toBe('par Wollnashorn · licence : MIT');
    expect(ligneAuteur(mod({ auteur: '', licence: '' }))).toBe('licence : aucune');
    expect(ligneAuteur(mod({ licence: 'aucune' }))).toBe('par Wollnashorn · licence : aucune');
  });

  it('les mentions : réglage requis, parties touchées, version pas vérifiée, plus servi — sans jargon', () => {
    expect(mentions(mod())).toEqual([]);
    const l = mentions(mod({ reglage_requis: 'résolution ×2', touche_sauvegardes: true, compat: 'pas_verifie', servi: false }));
    expect(l).toHaveLength(4);
    expect(l[0]).toContain('résolution ×2');
    expect(l[1]).toBe('💾 Il touche à tes parties : elles seront mises à l’abri avant.');
    expect(l[2]).toContain('pas vérifiée');
    expect(l[3]).toContain('ne le propose plus');
    expect(l.join(' ')).not.toMatch(/exefs|romfs|build|clé/i);
  });

  it('« pas encore essayé » seulement si personne ne l’a essayé', () => {
    expect(pasEncoreEssaye(mod())).toBe(false);
    expect(pasEncoreEssaye(mod({ emulateurs_essayes: [] }))).toBe(true);
    expect(pasEncoreEssaye(mod({ emulateurs_essayes: [], servi: false }))).toBe(false);
  });

  it('l’interrupteur dit son état en mots ; un mod allumé pas mis en place à la dernière partie se voit', () => {
    expect(texteInterrupteurMod(mod({ allume: true }))).toBe('✅ Allumé');
    expect(texteInterrupteurMod(mod())).toBe('⬜ Éteint');
    const dp = { quand: '1', emulateur: 'Eden', raisons: ['« 1080p » pas posé : réglage d’émulateur requis : x'] };
    expect(allumePasPose(mod({ allume: true }), dp)).toBe(true);
    expect(texteInterrupteurMod(mod({ allume: true }), dp)).toBe('⚠ Allumé, pas utilisé');
    expect(texteInterrupteurMod(mod({ allume: false }), dp)).toBe('⬜ Éteint');
    expect(allumePasPose(mod({ allume: true, nom: 'Autre' }), dp)).toBe(false);
    expect(allumePasPose(mod({ allume: true }), null)).toBe(false);
  });

  it('les textes de Firehouse : tels quels, sans mot technique, avec leur majuscule', () => {
    expect(texteSimple('fait pour la version 1.5.0')).toBe('Fait pour la version 1.5.0');
    expect(texteSimple('le BuildId du NSO ne correspond pas')).toBe('Le repère de version du programme du jeu ne correspond pas');
    expect(texteSimple('patch exefs / RomFS en IPS ou pchtxt (build id 8E99)')).toBe('Patch programme du jeu / fichiers du jeu en correctif ou correctif (repère de version 8E99)');
    expect(texteSimple('build_id inconnu')).toBe('Repère de version inconnu');
    // Un mot qui CONTIENT un mot technique ne change pas (« tips », « ipso »).
    expect(texteSimple('des tips ipso facto')).toBe('Des tips ipso facto');
    expect(texteSimple(null)).toBe('');
    expect(majuscule('élan')).toBe('Élan');
    expect(raisonGrise(mod({ raison: 'fait pour la version 1.5.0 (build id 8E99)' }))).toBe('Fait pour la version 1.5.0 (repère de version 8E99)');
    expect(raisonGrise(mod({ raison: null }))).toContain('Il ne va pas avec la version du jeu');
    expect(mentions(mod({ reglage_requis: 'activer le romfs.' }))[0]).toContain(': activer le fichiers du jeu. Tant');
  });

  it('une description longue se replie', () => {
    expect(descriptionLongue('court')).toBe(false);
    expect(descriptionLongue('x'.repeat(200))).toBe(true);
    expect(descriptionLongue('ligne 1\nligne 2')).toBe(true);
  });
});

describe('les conflits', () => {
  const a = mod({ cle: 'a-1', nom: 'A', etat: 'telecharge', conflits: ['b-1'] });
  const b = mod({ cle: 'b-1', nom: 'B', etat: 'telecharge', allume: true });
  it('trouve le mod allumé qui ne va pas avec, dans un sens ou dans l’autre', () => {
    expect(conflitAvec([a, b], 'a-1')?.nom).toBe('B');
    expect(conflitAvec([{ ...a, conflits: [] }, { ...b, conflits: ['a-1'] }], 'a-1')?.nom).toBe('B');
  });
  it('ignore un mod éteint, pas rangé, ou une clé inconnue', () => {
    expect(conflitAvec([a, { ...b, allume: false }], 'a-1')).toBeNull();
    expect(conflitAvec([a, { ...b, etat: 'pas_telecharge' }], 'a-1')).toBeNull();
    expect(conflitAvec([a, b], 'z-1')).toBeNull();
  });
});

describe('l’accord chiffré', () => {
  it('la durée seulement si le débit est connu', () => {
    expect(dureeEstimee(1000, null)).toBeNull();
    expect(dureeEstimee(1000, 0)).toBeNull();
    expect(dureeEstimee(1000, 100)).toBe('environ 10 s');
  });
  it('le titre dit combien, combien de Mo, combien de temps', () => {
    expect(titreTelechargement([mod()], null)).toBe('🧩 Télécharger « 1080p » (1000 o) ?');
    expect(titreTelechargement([mod(), mod({ cle: 'b-1' })], 100)).toBe('🧩 Télécharger 2 mods (2000 o, environ 20 s) ?');
    expect(titreTelechargement([mod()], null, true)).toBe('🧩 Mettre à jour « 1080p » (1000 o) ?');
  });
  it('le message dit « toute la maison » et l’abri des parties si le mod y touche', () => {
    const un = messageTelechargement([mod()]);
    expect(un).toContain('toute la maison');
    expect(un).not.toContain('abri');
    expect(messageTelechargement([mod({ touche_sauvegardes: true })])).toContain('à l’abri');
    const deux = messageTelechargement([mod({ nom: 'A' }), mod({ cle: 'b-1', nom: 'B', touche_sauvegardes: true })]);
    expect(deux).toContain('• A (1000 o)');
    expect(deux).toContain('« B » touche à tes parties');
    expect(messageTelechargement([mod()], true)).toContain('L’ancienne version reste en place');
  });
});

describe('le suivi', () => {
  it('la ligne visible porte le pourcentage et le rang', () => {
    expect(ligneProgresMods(SUIVI)).toBe('⬇ « 1080p » : 45 % (1 sur 3)');
    expect(ligneProgresMods({ ...SUIVI, a_faire: 1 })).toBe('⬇ « 1080p » : 45 %');
    expect(ligneProgresMods({ ...SUIVI, en_cours: null })).toBe('⬇ Téléchargement des mods…');
    expect(ligneProgresMods({ ...SUIVI, actif: false })).toBeNull();
    expect(ligneProgresMods(null)).toBeNull();
    expect(pourcentEnCours(SUIVI)).toBe(45);
    expect(pourcentEnCours({ ...SUIVI, total: 0 })).toBe(0);
  });
  it('le lecteur d’écran n’entend que l’étape, jamais le pourcentage', () => {
    const e = etapeModsAnnoncee(SUIVI);
    expect(e).toBe('Téléchargement du mod « 1080p », 1 sur 3.');
    expect(etapeModsAnnoncee({ ...SUIVI, recus: 900 })).toBe(e);
    expect(etapeModsAnnoncee({ ...SUIVI, actif: false })).toBe('');
  });
  it('la fin dit les échecs et l’arrêt', () => {
    expect(lignesFinMods(SUIVI)).toEqual([]);
    expect(lignesFinMods({ ...SUIVI, actif: false, erreurs: ['1080p : coupure'], arret: 'une partie a commencé' })).toEqual([
      '⚠ Pas téléchargé : 1080p : coupure',
      '⏸ Arrêté : une partie a commencé.',
    ]);
  });
  it('la fin : combien sont prêts', () => {
    expect(messageFinMods({ ...SUIVI, actif: false, faits: 2 })).toBe('🧩 2 mods prêts : ils s’allument d’un simple bouton.');
    expect(messageFinMods({ ...SUIVI, actif: false, faits: 1 })).toBe('🧩 1 mod prêt : il s’allume d’un simple bouton.');
    expect(messageFinMods({ ...SUIVI, actif: false, faits: 0 })).toBeNull();
    expect(messageFinMods(SUIVI)).toBeNull();
  });
  it('une partie refusée pendant que des mods arrivent : reconnue, et dite au canapé avec où la suivre', () => {
    const motif = 'Des mods de ce jeu sont en train d\'arriver : la partie pourra se lancer dès qu\'ils seront installés.';
    expect(refusModsEnRoute({ sorte: 'refus', motif })).toBe(true);
    expect(refusModsEnRoute({ sorte: 'refus', motif: 'Autre chose' })).toBe(false);
    expect(refusModsEnRoute({ sorte: 'reglage', motif })).toBe(false);
    expect(refusModsEnRoute(null)).toBe(false);
    expect(messageModsEnRoute(motif, false)).toBe(`⏳ ${motif}`);
    expect(messageModsEnRoute(motif, true)).toContain('Y, puis « 🧩 Mises à jour, DLC et mods »');
  });
});

describe('la liste', () => {
  const liste = (x: Partial<ModsSwitch>): ModsSwitch => ({
    etat: 'ok',
    vue_le: null,
    tid_base: '01007EF00011E000',
    version_servie: null,
    version_connue: true,
    mods: [mod()],
    remarque: null,
    derniere_pose: null,
    ...x,
  });
  it('hors ligne : la date de la dernière liste vue', () => {
    const s = new Date(2026, 10, 18, 14, 5).getTime() / 1000;
    expect(dateVue(String(s))).toBe('le 18/11 à 14:05');
    expect(dateVue(null)).toBeNull();
    expect(dateVue('hier')).toBe('hier');
    expect(ligneEtatListe(liste({ etat: 'hors_ligne', vue_le: String(s) }))).toContain('vue le 18/11 à 14:05');
    expect(ligneEtatListe(liste({ etat: 'injoignable' }))).toContain('n’a pas pu être lue');
    expect(ligneEtatListe(liste({}))).toBeNull();
  });
  it('le bloc se montre s’il y a des mods ou quelque chose à dire', () => {
    expect(blocModsVisible(liste({}))).toBe(true);
    expect(blocModsVisible(liste({ mods: [] }))).toBe(false);
    expect(blocModsVisible(liste({ mods: [], etat: 'injoignable' }))).toBe(true);
    expect(blocModsVisible(liste({ mods: [], remarque: 'Ryubing n’est pas portable.' }))).toBe(true);
    expect(blocModsVisible(liste({ mods: [], derniere_pose: { quand: '1', emulateur: 'Eden', raisons: ['Eden est ouvert'] } }))).toBe(true);
    expect(blocModsVisible(liste({ mods: [], derniere_pose: { quand: '1', emulateur: 'Eden', raisons: [] } }))).toBe(false);
    expect(blocModsVisible(liste({ etat: 'rien' }))).toBe(false);
    expect(blocModsVisible(null)).toBe(false);
  });
  it('la dernière pose : l’émulateur, la date, une ligne par raison (sans jargon)', () => {
    const s = new Date(2026, 10, 18, 14, 5).getTime() / 1000;
    expect(lignesDernierePose({ quand: String(s), emulateur: 'Eden', raisons: ['« 1080p » pas posé : build id inconnu', 'Eden est ouvert'] })).toEqual([
      '⚠ À la dernière partie (Eden, le 18/11 à 14:05) :',
      '• « 1080p » pas posé : repère de version inconnu',
      '• Eden est ouvert',
    ]);
    expect(lignesDernierePose({ quand: String(s), emulateur: 'Eden', raisons: [] })).toEqual([]);
    expect(lignesDernierePose(null)).toEqual([]);
  });
});

describe('relecture finale (18/11)', () => {
  it('« cheats » et « zip » se disent simplement, en mots entiers', () => {
    expect(texteSimple('cheats dans un zip')).toBe('Triches dans un archive');
    expect(texteSimple('zipper les cheatsheets')).toBe('Zipper les cheatsheets');
  });

  it('la fin d’un téléchargement est dite sans jargon', () => {
    const su = { actif: false, en_cours: null, recus: 0, total: 0, faits: 0, a_faire: 1, erreurs: ['1080p : archive zip sans exefs'], arret: 'cheats pas lus' };
    expect(lignesFinMods(su)).toEqual(['⚠ Pas téléchargé : 1080p : archive archive sans programme du jeu', '⏸ Arrêté : triches pas lus.']);
  });

  it('le focus va à une commande active de la ligne, sinon d’une suivante, sinon à l’en-tête', () => {
    document.body.innerHTML = `
      <button id="entete">Mods</button>
      <ul>
        <li data-cle="a" tabindex="-1"><button disabled>⏳</button></li>
        <li data-cle="b" tabindex="-1"><span>texte</span></li>
        <li data-cle="c" tabindex="-1"><button role="switch">C</button><button>Retirer</button></li>
        <li data-cle="z" tabindex="-1"><button>Z</button></li>
      </ul>`;
    const lignes = [...document.querySelectorAll<HTMLElement>('li[data-cle]')];
    const entete = document.getElementById('entete');
    const ordre = ['z', 'a', 'b', 'c'];
    // La ligne de « a » n'a rien d'actif, « b » non plus : la suivante (« c »), son interrupteur.
    expect(cibleDuFocus(lignes, 'a', ordre, entete)?.textContent).toBe('C');
    // « z » était AVANT dans l'ordre : jamais visé pour « c » sans commande ; « c » a la sienne.
    expect(cibleDuFocus(lignes, 'c', ordre, entete)?.textContent).toBe('C');
    // Une ligne retirée, rien d'actif après elle : l'en-tête (jamais une ligne `tabindex="-1"`).
    expect(cibleDuFocus(lignes.slice(0, 2), 'x', ['x', 'a', 'b'], entete)).toBe(entete);
  });
});
