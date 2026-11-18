import { describe, expect, it } from 'vitest';
import type { JeuResume } from '$lib/api';
import { messageLancementBoutique, boutiqueSansInstallation, estBattlenet, MESSAGE_BATTLENET, toujoursLeMeme, BOUTIQUES, estJeuDeBoutique, libelleOuverture, ligneEtat, messageNouveaux, moyenDOuverture, nomBoutique, questionPossedes, messageInstallationCanape, imageCarteAInstaller } from './jeu-boutique';

const jeu = (o: Partial<JeuResume>): JeuResume =>
  ({ id: -5, titre: 'X', annee: null, plateforme: 'Windows', genres: [], developpeur: null, editeur: null, statut: 'boutique', ...o }) as JeuResume;

describe('jeux de boutique', () => {
  it('se reconnaissent à leur id négatif', () => {
    expect(estJeuDeBoutique({ id: -3 })).toBe(true);
    expect(estJeuDeBoutique({ id: 110 })).toBe(false);
  });

  it('s’ouvrent par Steam, par GOG Galaxy, ou par le lanceur de leur boutique', () => {
    expect(moyenDOuverture(jeu({ source: 'steam', cle_boutique: '620' }))).toEqual({ par: 'steam', appid: '620' });
    expect(moyenDOuverture(jeu({ source: 'galaxy', cle_boutique: 'epic_Fortnite' }))).toEqual({ par: 'galaxy', cle: 'epic_Fortnite' });
    expect(moyenDOuverture(jeu({ source: 'galaxy' }))).toBeNull();
    expect(moyenDOuverture(jeu({ source: 'uplay', boutique: 'uplay', cle_boutique: 'uplay_46' }))).toEqual({ par: 'boutique', boutique: 'uplay', cle: 'uplay_46' });
    expect(moyenDOuverture(jeu({ source: 'gog', boutique: 'gog', cle_boutique: 'gog_1' }))).toEqual({ par: 'boutique', boutique: 'gog', cle: 'gog_1' });
  });

  it('nomment le lanceur d’après leur boutique, jamais un nom écrit en dur', () => {
    expect(libelleOuverture(jeu({ source: 'uplay', boutique: 'uplay', cle_boutique: 'uplay_46' }))).toBe('Ubisoft Connect');
    expect(libelleOuverture(jeu({ source: 'amazon', boutique: 'amazon', cle_boutique: 'amazon_x' }))).toBe('Amazon Games');
    expect(libelleOuverture(jeu({ source: 'galaxy', boutique: 'epic', cle_boutique: 'epic_F' }))).toBe('GOG Galaxy');
    expect(libelleOuverture(jeu({ source: 'steam', boutique: 'steam', cle_boutique: '620' }))).toBe('Steam');
    expect(nomBoutique('battlenet')).toBe('Battle.net');
    expect(nomBoutique('inconnue')).toBe('sa boutique');
    expect(BOUTIQUES.map((b) => b.valeur)).toEqual(expect.arrayContaining(['amazon', 'battlenet']));
  });

  it('annoncent en UN toast les jeux nouveaux, et rien s’il n’y en a pas', () => {
    // Lot A : seulement des jeux installés → le texte du lot A.
    expect(messageNouveaux([['epic', 2, 2], ['uplay', 1, 1]])).toBe('🎮 3 nouveaux jeux trouvés sur ce PC (Epic 2, Ubisoft 1)');
    expect(messageNouveaux([['gog', 1, 1]])).toBe('🎮 1 nouveau jeu trouvé sur ce PC (GOG 1)');
    expect(messageNouveaux([['gog', 1]])).toBe('🎮 1 nouveau jeu trouvé sur ce PC (GOG 1)'); // sans le 3e nombre : installés
    // Lot B : des jeux des comptes, des mots pour un novice.
    expect(messageNouveaux([['epic', 497, 1], ['uplay', 1, 0]])).toBe('🎮 498 jeux de tes comptes Epic et Ubisoft ajoutés à ta ludothèque (1 déjà installé). Les autres s’installent avec ⬇ Installer.');
    expect(messageNouveaux([['epic', 500, 3]])).toBe('🎮 500 jeux de tes comptes Epic ajoutés à ta ludothèque (3 déjà installés). Les autres s’installent avec ⬇ Installer.');
    expect(messageNouveaux([['uplay', 4, 0]])).toBe('🎮 4 jeux de tes comptes Ubisoft ajoutés à ta ludothèque. Ils s’installent avec ⬇ Installer.');
    expect(messageNouveaux([['uplay', 1, 0]])).toBe('🎮 1 jeu de ton compte Ubisoft ajouté à ta ludothèque. Il s’installe avec ⬇ Installer.');
    // Une boutique aux seuls jeux installés n'est pas nommée parmi les comptes.
    expect(messageNouveaux([['steam', 2, 2], ['epic', 3, 0], ['uplay', 1, 0]])).toBe('🎮 6 jeux de tes comptes Epic et Ubisoft ajoutés à ta ludothèque (2 déjà installés). Les autres s’installent avec ⬇ Installer.');
    for (const m of [messageNouveaux([['epic', 497, 1]])!]) expect(m).not.toMatch(/possédé|catalogue|cache/);
    expect(messageNouveaux([])).toBeNull();
    expect(messageNouveaux([['steam', 0, 0]])).toBeNull();
  });

  it('montrent au canapé la jaquette RÉDUITE par le cœur, jamais l’image d’origine de la boutique', () => {
    const a = imageCarteAInstaller({ id: -42, image: 'https://cdn1.epicgames.com/item/ns/boite' })!;
    expect(a.startsWith('http://jaquette.localhost/-42')).toBe(true);
    expect(a).toContain('400');
    expect(a).not.toContain('epicgames');
    expect(imageCarteAInstaller({ id: -42, image: null })).toBeNull();
  });

  it('préviennent au canapé que la fenêtre d’installation de la boutique s’ouvre', () => {
    expect(messageInstallationCanape('epic')).toBe('La fenêtre d’installation d’Epic Games s’ouvre : valide-la (elle peut demander la souris ; ⊞/Guide pour revenir à Frogtend).');
    expect(messageInstallationCanape('gog')).toContain('installation de GOG s’ouvre');
  });

  it('demandent UNE question claire avant d’ajouter les jeux des comptes', () => {
    const q = questionPossedes([
      { nom: 'Steam', possedes: null, installes: 3 },
      { nom: 'Epic Games', possedes: 500, installes: 1 },
      { nom: 'Ubisoft Connect', possedes: 4, installes: 0 },
    ])!;
    expect(q.titre).toBe('Ajouter les 500 jeux du compte Epic Games connecté sur ce PC (et 4 d’Ubisoft Connect) à ta ludothèque ?');
    expect(q.message).toContain('Ils viennent du compte de la boutique ouvert sur ce PC.');
    expect(questionPossedes([{ nom: 'Epic Games', possedes: 1, installes: 0 }])!.titre).toBe('Ajouter le jeu du compte Epic Games connecté sur ce PC à ta ludothèque ?');
    expect(questionPossedes([{ nom: 'Epic Games', possedes: 0, installes: 0 }, { nom: 'Steam', installes: 2 }])).toBeNull();
  });

  it('disent l’état de chaque boutique sans mot technique', () => {
    expect(ligneEtat({ nom: 'Epic Games', lanceur: true, lu: true, installes: 1, possedes: 500 })).toBe('Epic Games : 500 jeux dans ton compte, dont 1 installé, mis à jour à chaque ouverture');
    expect(ligneEtat({ nom: 'Ubisoft Connect', lanceur: true, lu: true, installes: 0, possedes: 4 })).toBe('Ubisoft Connect : 4 jeux dans ton compte, dont aucun installé, mis à jour à chaque ouverture');
    expect(ligneEtat({ nom: 'Epic Games', lanceur: true, lu: true, installes: 2, possedes: 500 })).toContain('dont 2 installés');
    expect(ligneEtat({ nom: 'Epic Games', lanceur: true, lu: true, installes: 12, possedes: null })).toBe('Epic Games : 12 jeux sur ce PC, mis à jour à chaque ouverture');
    expect(ligneEtat({ nom: 'Epic Games', lanceur: true, lu: true, installes: 12 })).toBe('Epic Games : 12 jeux sur ce PC, mis à jour à chaque ouverture');
    expect(ligneEtat({ nom: 'Steam', lanceur: false, lu: true, installes: 1 })).toBe('Steam : 1 jeu sur ce PC, mis à jour à chaque ouverture (Steam n’est plus installé sur ce PC : pour jouer, réinstalle-le)');
    for (const e of [{ nom: 'Steam', lanceur: false, lu: true, installes: 1 }, { nom: 'Amazon Games', lanceur: false, lu: false, installes: 0 }]) expect(ligneEtat(e)).not.toMatch(/lanceur|relus|démarrage/);
    expect(ligneEtat({ nom: 'Amazon Games', lanceur: false, lu: false, installes: 0 })).toBe('Amazon Games n’est pas installé sur ce PC');
    expect(ligneEtat({ nom: 'Epic Games', lanceur: true, lu: false, installes: 0 })).toBe('Epic Games : jeux illisibles pour l’instant');
    expect(ligneEtat({ nom: 'Ubisoft Connect', lanceur: true, lu: true, installes: 0 })).toContain('aucun jeu installé');
    for (const l of [ligneEtat({ nom: 'Steam', lanceur: true, lu: true, installes: 1 })]) expect(l).not.toMatch(/manifeste|registre|product\.db/);
  });

  it('ne font rien pour un autre profil ouvert pendant l’attente', () => {
    expect(toujoursLeMeme('seb', 'seb')).toBe(true);
    expect(toujoursLeMeme('seb', 'lou')).toBe(false);
    expect(toujoursLeMeme('seb', undefined)).toBe(false);
    expect(toujoursLeMeme(undefined, undefined)).toBe(false);
  });

  it('Battle.net : jamais un ▶ Jouer qui échoue, lanceur présent ou non', () => {
    expect(estBattlenet({ boutique: 'battlenet', source: 'battlenet' })).toBe(true);
    expect(estBattlenet({ installation: { boutique: { source: 'battlenet' } } })).toBe(true);
    expect(estBattlenet({ boutique: 'battlenet', source: 'galaxy' })).toBe(false);
    expect(estBattlenet({ boutique: 'uplay', source: 'uplay' })).toBe(false);
    expect(MESSAGE_BATTLENET).toContain('ouvre-le depuis Battle.net');
  });

  it('reconnaissent une ligne du PC désinstallée hors de Frogtend (Taodbox ne la montre pas)', () => {
    expect(boutiqueSansInstallation({ id: -9, installation: null, ajoute_par: 'uplay' })).toBe(true);
    expect(boutiqueSansInstallation({ id: -9, installation: { dossier: 'x' }, ajoute_par: 'uplay' })).toBe(false);
    expect(boutiqueSansInstallation({ id: -9, installation: null, ajoute_par: 'local' })).toBe(false);
    expect(boutiqueSansInstallation({ id: 110, installation: null, ajoute_par: 'steam' })).toBe(false);
  });

  it('n’annoncent le lancement qu’en UN toast, avec le retour par ⊞/Guide au canapé', () => {
    expect(messageLancementBoutique('Epic Games', 'TerraScape', false)).toBe('▶ Epic Games lance « TerraScape »… S’il fait une mise à jour, ça peut prendre quelques minutes.');
    expect(messageLancementBoutique('Epic Games', 'TerraScape', true)).toContain('appuie sur ⊞/Guide pour revenir à Frogtend');
  });
});
