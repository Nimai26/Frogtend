// 📁 Où sont mes parties (lot S1) : les textes de l'écran, d'après la forme RÉELLE de `parties_emplacements`
// (commandes.rs, `PartiesVues` ; emplacement de Cyberpunk 2077 du relevé 1.48 du 22/11).
import { describe, expect, it } from 'vitest';
import type { EmplacementVu, PartiesVues } from '$lib/api';
import {
  aDesEmplacements,
  abritable,
  contenu,
  jeuDeLAdresse,
  libelleOrigine,
  messageAnnonce,
  messageChoix,
  messageDeja,
  messageRien,
  montres,
  noteAbri,
  optionsAbri,
  ouvrable,
  texteAutresEndroits,
  texteContenuAbri,
  texteDate,
  texteEtat,
  texteFait,
  texteFiltre,
  texteNuage,
  texteSorteAbri,
  texteSource,
  titreOrigine,
} from './parties';

const cyberpunk: EmplacementVu = {
  n: 0,
  dossier: 'C:\\Users\\Léa\\Saved Games\\CD Projekt Red\\Cyberpunk 2077',
  filtre: null,
  nature: 'dossier',
  origine: 'firehouse',
  os: 'Windows',
  source: 'ludusavi',
  nuage_boutique: false,
  lisible: true,
  existe: true,
  fichiers: 12,
  en_ligne: 0,
  octets: 3 * 1024 * 1024,
  trop: false,
  illisible: false,
};
const illisible: EmplacementVu = { ...cyberpunk, n: 1, dossier: null, lisible: false, existe: false, fichiers: 0, octets: 0 };
const vues = (p: Partial<PartiesVues>): PartiesVues => ({
  firehouse: 'lu', lu_le: '1763800000', hors_ligne: false, motif: null, jeton_refuse: false, moteur: null, source: 'ludusavi', reconnu_par: 'gog_id', autres_systemes: [], emplacements: [cyberpunk], ...p,
});

describe('📁 Où sont mes parties', () => {
  it('seulement pour un jeu PC', () => {
    expect(aDesEmplacements('Windows')).toBe(true);
    expect(aDesEmplacements('MS-DOS')).toBe(false);
    expect(aDesEmplacements('Sony Playstation 3')).toBe(false);
    expect(aDesEmplacements(undefined)).toBe(false);
  });

  it('`?jeu=` vide ou illisible : aucun jeu, donc aucune demande', () => {
    expect(jeuDeLAdresse('-7')).toBe(-7);
    expect(jeuDeLAdresse('900')).toBe(900);
    for (const v of [null, undefined, '', '  ', '0', 'abc', '1.5']) expect(jeuDeLAdresse(v)).toBeNull();
  });

  it('dit « d’après Firehouse », la source précise seulement au survol, jamais « moteur »', () => {
    expect(libelleOrigine('firehouse')).toBe('d’après Firehouse');
    expect(titreOrigine('firehouse', 'ludusavi')).toBe('Source : Ludusavi');
    expect(titreOrigine('firehouse', 'pcgamingwiki')).toBe('Source : PCGamingWiki');
    expect(titreOrigine('personne', null)).toBeUndefined();
    expect(libelleOrigine('personne')).toBe('ajouté par toi');
    for (const o of ['firehouse', 'moteur', 'manifeste', 'personne', '?']) expect(libelleOrigine(o)).not.toMatch(/moteur/i);
  });

  it('dit ce que contient un dossier, ou pourquoi rien', () => {
    expect(contenu(cyberpunk)).toBe('12 fichiers · 3 Mo');
    expect(contenu({ ...cyberpunk, fichiers: 1, octets: 7 })).toBe('1 fichier · 7 o');
    expect(contenu({ ...cyberpunk, fichiers: 0, octets: 0 })).toBe('Le dossier est là, mais vide.');
    expect(contenu({ ...cyberpunk, existe: false, fichiers: 0 })).toBe('Pas encore de parties ici (le dossier n’existe pas).');
    expect(contenu({ ...cyberpunk, trop: true })).toContain('pas comptés');
    expect(contenu({ ...cyberpunk, illisible: true, fichiers: 0 })).toBe('Le dossier est là, mais Frogtend ne peut pas le lire.');
    expect(contenu(illisible)).toBe('Frogtend ne sait pas lire cet emplacement sur ce PC.');
  });

  it('les endroits sans dossier sur ce PC ne sont pas des cartes : une phrase sous la liste', () => {
    const v = vues({ emplacements: [cyberpunk, illisible, { ...illisible, n: 2, lisible: true }] });
    expect(montres(v)).toEqual([cyberpunk]);
    expect(texteAutresEndroits(v)).toBe('2 autres endroits connus ne concernent pas ce PC.');
    expect(texteAutresEndroits(vues({ emplacements: [cyberpunk, illisible] }))).toBe('1 autre endroit connu ne concerne pas ce PC.');
    expect(texteAutresEndroits(vues({}))).toBeNull();
  });

  it('« Ouvrir le dossier » seulement pour un dossier qui existe', () => {
    expect(ouvrable(cyberpunk)).toBe(true);
    expect(ouvrable({ ...cyberpunk, existe: false })).toBe(false);
    expect(ouvrable({ ...cyberpunk, dossier: null })).toBe(false);
  });

  it('dit quand Firehouse ne sait pas, et pour quelle version il sait', () => {
    expect(texteEtat(vues({}))).toBeNull();
    expect(texteEtat(vues({ firehouse: 'inconnu', emplacements: [] }))).toBe('Firehouse ne connaît pas encore l’endroit des parties de ce jeu pour ce PC.');
    expect(texteEtat(vues({ firehouse: 'vide', emplacements: [] }))).toBe('Firehouse ne connaît aucun endroit de parties de ce jeu pour ce PC.');
    // Un endroit illisible seul compte comme rien.
    expect(texteEtat(vues({ firehouse: 'vide', emplacements: [illisible] }))).toContain('aucun endroit');
    // TerraScape (Epic, relevé réel 1.48) : connu pour Steam, Mac ou Linux — dit honnêtement, rien de deviné.
    expect(texteEtat(vues({ firehouse: 'inconnu', emplacements: [], autres_systemes: ['Mac ou Linux', 'Steam'] }))).toBe(
      'Firehouse connaît l’endroit des parties de ce jeu pour la version Mac, Linux ou Steam, pas encore pour la tienne.',
    );
    expect(texteEtat(vues({ firehouse: 'inconnu', emplacements: [], autres_systemes: ['Steam'] }))).toBe(
      'Firehouse connaît l’endroit des parties de ce jeu pour la version Steam, pas encore pour la tienne.',
    );
  });

  it('hors ligne ou Firehouse en panne, sans rien de gardé : « pas encore pu demander », jamais « ne connaît pas »', () => {
    const t = 'Frogtend n’a pas encore pu demander à Firehouse où ce jeu range ses parties. Réessaie une fois connecté.';
    expect(texteEtat(vues({ firehouse: 'inconnu', emplacements: [], hors_ligne: true }))).toBe(t);
    expect(texteEtat(vues({ firehouse: 'inconnu', emplacements: [], motif: 'Firehouse ne répond pas correctement.' }))).toBe(t);
    expect(texteSource(vues({ firehouse: 'inconnu', emplacements: [], hors_ligne: true }))).toBeNull();
  });

  it('jeton refusé : dit clairement', () => {
    expect(texteEtat(vues({ firehouse: 'inconnu', emplacements: [], jeton_refuse: true }))).toContain('refusé le jeton');
    expect(texteSource(vues({ jeton_refuse: true }))).toBe('Firehouse a refusé le jeton de ce profil : voici ce que Frogtend savait déjà.');
  });

  it('quand le cache sert : sans ponctuation doublée', () => {
    expect(texteSource(vues({}))).toBeNull();
    expect(texteSource(vues({ hors_ligne: true }))).toBe('Hors ligne : voici ce que Frogtend savait déjà.');
    expect(texteSource(vues({ motif: 'Firehouse ne répond pas correctement.' }))).toBe('Firehouse n’a pas pu répondre : Firehouse ne répond pas correctement. Voici ce que Frogtend savait déjà.');
    expect(texteSource(vues({ motif: 'erreur 500' }))).toBe('Firehouse n’a pas pu répondre : erreur 500. Voici ce que Frogtend savait déjà.');
  });

  it('dit les fichiers visés et le nuage de la boutique', () => {
    expect(texteFiltre(cyberpunk)).toBeNull();
    expect(texteFiltre({ ...cyberpunk, nature: 'motif', filtre: '*.sav' })).toBe('seulement les fichiers « *.sav »');
    expect(texteFiltre({ ...cyberpunk, nature: 'fichier', filtre: 'save.dat' })).toBe('seulement le fichier « save.dat »');
    expect(texteNuage(cyberpunk)).toBeNull();
    expect(texteNuage({ ...cyberpunk, nuage_boutique: true })).toBe('aussi gardé dans le nuage de la boutique');
  });
});

describe('🛟 Mettre mes parties à l’abri (lot S2) : les textes', () => {
  const apercu = { fichiers: 12, octets: 3 * 1024 * 1024, emplacements: 2, echecs: [], en_ligne_seulement: 0, octets_en_ligne: 0, deja: false, gros: false, gros_avec_nuage: false, profil: 'Léa' };

  it('le bouton : seulement un emplacement lisible ici avec des fichiers comptés', () => {
    expect(abritable(vues({}))).toBe(true);
    expect(abritable(vues({ emplacements: [{ ...cyberpunk, fichiers: 0 }] }))).toBe(false);
    expect(abritable(vues({ emplacements: [{ ...cyberpunk, trop: true }] }))).toBe(false);
    expect(abritable(vues({ emplacements: [{ ...cyberpunk, illisible: true }] }))).toBe(false);
    expect(abritable(vues({ emplacements: [{ ...cyberpunk, existe: false }] }))).toBe(false);
    expect(abritable(vues({ emplacements: [{ ...cyberpunk, lisible: false }] }))).toBe(false);
  });

  it('l’annonce dit combien, quelle taille, et le profil propriétaire', () => {
    const m = messageAnnonce(apercu);
    expect(m).toContain('12 fichiers · 3 Mo (2 endroits)');
    expect(m).toContain('Ces parties seront rangées dans ton profil « Léa ».');
    expect(m).not.toContain('2 Go');
    const tout = messageAnnonce({ ...apercu, fichiers: 1, emplacements: 1, gros: true, en_ligne_seulement: 3, echecs: [{ dossier: 'C:\\X', raison: 'un fichier ne se lit pas' }] });
    expect(tout).toContain('1 fichier · ');
    expect(tout).not.toContain('endroits');
    expect(tout).toContain('plus de 2 Go');
    expect(tout).toContain('3 fichiers sont seulement dans ton nuage (OneDrive…)');
    expect(tout).toContain('⚠ C:\\X : un fichier ne se lit pas.');
    expect(messageRien({ ...apercu, fichiers: 0, echecs: [{ dossier: 'D', raison: 'r' }] })).toContain('⚠ D : r.');
  });

  it('le compte rendu, après', () => {
    expect(texteFait({ date: null, fichiers: 0, octets: 0, emplacements: 0, echecs: [], en_ligne_seulement: 0, telecharges: 0, deja: true })).toBe('✅ Déjà à l’abri : rien n’a changé depuis ton dernier abri.');
    expect(texteFait({ date: '1', fichiers: 2, octets: 7, emplacements: 1, echecs: [], en_ligne_seulement: 1, telecharges: 0, deja: false })).toBe(
      '✅ 2 fichiers de parties (7 o) mis à l’abri dans ton profil. 1 fichier est seulement dans ton nuage (OneDrive…), pas sur ce PC : pas copié.',
    );
    // Un endroit en échec : « ⚠ » en tête, et jamais un chemin complet dans un toast qui s'efface seul.
    const echec = texteFait({ date: '1', fichiers: 2, octets: 7, emplacements: 1, echecs: [{ dossier: 'C:\\Users\\Léa\\Jeu', raison: 'r' }], en_ligne_seulement: 0, telecharges: 0, deja: false });
    expect(echec.startsWith('⚠ 2 fichiers de parties')).toBe(true);
    expect(echec).toContain('1 endroit n’a pas pu être copié');
    expect(echec).not.toContain('Léa');
    expect(messageDeja({ ...apercu, deja: true })).toBeNull();
    expect(messageDeja({ ...apercu, deja: true, en_ligne_seulement: 1 })).toContain('1 fichier est seulement dans ton nuage');
  });

  it('un abri de la liste', () => {
    const a = { date: '1763800000', fichiers: 2, octets: 7, sorte: 'emplacements', a_moi: true, echecs: 0, en_ligne_seulement: 0, sans_origine: 0 };
    expect(texteDate('1763800000')).toContain('2025');
    expect(texteDate('abc')).toBe('date inconnue');
    expect(texteContenuAbri(a)).toBe('2 fichiers · 7 o');
    expect(texteSorteAbri(a)).toBe('mis à l’abri à la main · dans ton profil');
    expect(texteSorteAbri({ ...a, sorte: 'retrait', a_moi: false })).toBe('mis à l’abri par Frogtend (avant un mod, une réinstallation ou un retrait) · ancien abri, sans propriétaire connu');
    expect(texteSorteAbri({ ...a, sorte: 'jeu' })).toBe('mis à l’abri par Frogtend (avant un mod, une réinstallation ou un retrait) · dans ton profil');
    expect(noteAbri(a)).toBeNull();
    expect(noteAbri({ ...a, sans_origine: 1 })).toBe('Frogtend ne sait plus d’où venait 1 de ces fichiers : il est gardé, mais il faudra le remettre à la main.');
    expect(noteAbri({ ...a, sans_origine: 4 })).toBe('Frogtend ne sait plus d’où venaient 4 de ces fichiers : ils sont gardés, mais il faudra les remettre à la main.');
    expect(noteAbri({ ...a, echecs: 2 })).toBe('2 endroits n’avaient pas pu être copiés.');
  });
});

describe('☁ Les parties seulement dans OneDrive (Seb, 22/11)', () => {
  const apercu = { fichiers: 0, octets: 0, emplacements: 1, echecs: [], en_ligne_seulement: 9, octets_en_ligne: 188_950_000, deja: false, gros: false, gros_avec_nuage: false, profil: 'Seb' };

  it('la fiche dit tout de suite ce qui n’est pas sur ce PC', () => {
    expect(contenu({ ...cyberpunk, fichiers: 9, octets: 188_950_000, en_ligne: 9 })).toBe('9 fichiers · 180,2 Mo — dans ton nuage OneDrive, pas sur ce PC');
    expect(contenu({ ...cyberpunk, fichiers: 9, octets: 188_950_000, en_ligne: 4 })).toBe('9 fichiers · 180,2 Mo — dont 4 dans ton nuage OneDrive');
    expect(contenu({ ...cyberpunk, en_ligne: 0 })).not.toContain('OneDrive');
  });

  it('le choix : télécharger (nombre et taille), « sans eux » seulement s’il y a aussi des fichiers sur ce PC', () => {
    const seul = optionsAbri(apercu);
    expect(seul.map((o) => o.valeur)).toEqual(['telecharger']);
    expect(seul[0].libelle).toBe('☁ Télécharger depuis OneDrive puis mettre à l’abri');
    expect(seul[0].detail).toBe('9 fichiers · 180,2 Mo à télécharger');
    const deux = optionsAbri({ ...apercu, fichiers: 2, octets: 7, gros_avec_nuage: true });
    expect(deux.map((o) => o.valeur)).toEqual(['telecharger', 'local']);
    expect(deux[0].detail).toContain('plus de 2 Go');
    expect(deux[1].libelle).toBe('🛟 Mettre à l’abri sans eux');
    const m = messageChoix(apercu);
    expect(m).toContain('9 fichiers (180,2 Mo) sont seulement dans ton nuage OneDrive, pas sur ce PC.');
    expect(m).toContain('ils resteront ensuite sur ce PC');
    expect(m).toContain('ton profil « Seb »');
  });

  it('le compte rendu dit combien ont été téléchargés', () => {
    expect(texteFait({ date: '1', fichiers: 9, octets: 7, emplacements: 1, echecs: [], en_ligne_seulement: 0, telecharges: 9, deja: false })).toBe('✅ 9 fichiers de parties (7 o) mis à l’abri dans ton profil (dont 9 téléchargés depuis OneDrive).');
  });
});
