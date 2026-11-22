// 📁 Où sont mes parties (lot S1) : les textes de l'écran, d'après la forme RÉELLE de `parties_emplacements`
// (commandes.rs, `PartiesVues` ; emplacement de Cyberpunk 2077 du relevé 1.48 du 22/11).
import { describe, expect, it } from 'vitest';
import type { EmplacementVu, PartiesVues } from '$lib/api';
import { aDesEmplacements, contenu, jeuDeLAdresse, libelleOrigine, montres, ouvrable, texteAutresEndroits, texteEtat, texteFiltre, texteNuage, texteSource, titreOrigine } from './parties';

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
