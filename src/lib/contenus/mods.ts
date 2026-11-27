// 🧩 Les mods Switch servis par Firehouse (contrat 1.44, ✅ Seb 18/11) : ce que la personne voit sur la fiche d'un jeu
// (bureau) et dans « Réglages du jeu » (Taodbox). Fonctions pures, partagées par les deux écrans. Jamais de jargon :
// ni « exefs », ni « romfs », ni « build id », ni « clé ».

import { environDuree, taille, type ModSwitchVu, type ModsSwitch, type SuiviModsSwitch } from '$lib/api';
import { filtrerOptions, simplifier } from '$lib/dialogues/filtre';

const s = (n: number) => (n > 1 ? 's' : '');

/** Le mod est-il rangé sur ce PC (donc allumable) ? Un mod grisé peut l'être aussi (rangé avant un changement de
 * version) : il garde alors son état, mais jamais d'interrupteur. */
export function estInstalle(m: ModSwitchVu): boolean {
  return m.etat === 'telecharge' || m.etat === 'revision_dispo' || (m.etat === 'incompatible' && m.revision_rangee != null);
}

/** Les mods qui peuvent avoir un interrupteur : rangés et pas grisés. */
export function aUnInterrupteur(m: ModSwitchVu): boolean {
  return m.etat === 'telecharge' || m.etat === 'revision_dispo';
}

/** Les mods à télécharger pour la première fois (jamais un mod grisé : il ne servirait à rien). */
export function aTelecharger(l: ModSwitchVu[]): ModSwitchVu[] {
  return l.filter((m) => m.etat === 'pas_telecharge' && m.servi);
}

/** Les mods déjà rangés dont Firehouse sert une nouvelle révision. */
export function aMettreAJour(l: ModSwitchVu[]): ModSwitchVu[] {
  return l.filter((m) => m.etat === 'revision_dispo' && m.servi);
}

/** La forme que le cœur attend pour l'accord : ce que la personne a VU (clé, révision, taille, empreinte ; le cœur
 * refuse si ça a changé). */
export function clesVues(l: ModSwitchVu[]): { cle: string; revision: number; taille: number; sha256: string }[] {
  return l.map((m) => ({ cle: m.cle, revision: m.revision, taille: m.taille, sha256: m.sha256 }));
}

// Les mots techniques que Firehouse ou les émulateurs peuvent écrire (raison, réglage requis), dits simplement. Les
// plus longs d'abord (« build id » avant « id »…), sans casse, en mots entiers.
const MOTS_TECHNIQUES: [RegExp, string][] = [
  [/\bbuild[\s_-]?ids?\b/gi, 'repère de version'],
  [/\bexefs\b/gi, 'programme du jeu'],
  [/\bromfs\b/gi, 'fichiers du jeu'],
  [/\bnso\b/gi, 'programme du jeu'],
  [/\bpchtxt\b/gi, 'correctif'],
  [/\bips\b/gi, 'correctif'],
  [/\bcheats\b/gi, 'triches'],
  [/\bzip\b/gi, 'archive'],
];

/** Un texte venu de Firehouse (raison, réglage requis), gardé tel quel mais sans jargon, avec sa majuscule. */
export function texteSimple(t: string | null | undefined, avecMajuscule = true): string {
  let r = (t ?? '').trim();
  for (const [motif, simple] of MOTS_TECHNIQUES) r = r.replace(motif, simple);
  return avecMajuscule ? majuscule(r) : r;
}

/** La première lettre en majuscule (« fait pour la version… » → « Fait pour la version… »). */
export function majuscule(t: string): string {
  return t ? t.charAt(0).toLocaleUpperCase('fr-FR') + t.slice(1) : t;
}

export function totalOctets(l: ModSwitchVu[]): number {
  return l.reduce((n, m) => n + m.taille, 0);
}

/** Le résumé du bouton repliable : « 2 allumés sur 3, 1 à télécharger, 1 mise à jour ». */
export function resumeMods(l: ModSwitchVu[]): string {
  if (!l.length) return 'aucun';
  const installes = l.filter(aUnInterrupteur);
  const allumes = installes.filter((m) => m.allume).length;
  const morceaux: string[] = [];
  if (installes.length) morceaux.push(`${allumes} allumé${s(allumes)} sur ${installes.length}`);
  const nouveaux = aTelecharger(l).length;
  if (nouveaux) morceaux.push(`${nouveaux} à télécharger`);
  const majs = aMettreAJour(l).length;
  if (majs) morceaux.push(`${majs} mise${s(majs)} à jour`);
  const grises = l.filter((m) => m.etat === 'incompatible').length;
  if (grises) morceaux.push(`${grises} ${grises > 1 ? 'ne vont' : 'ne va'} pas avec ta version`);
  return morceaux.join(', ') || `${l.length} mod${s(l.length)}`;
}

/** La ligne grise sous le nom : « par Wollnashorn · licence : MIT » (« licence non précisée » si elle n'est pas dite :
 * « aucune » laissait croire à une interdiction). */
export function ligneAuteur(m: ModSwitchVu): string {
  const morceaux: string[] = [];
  if (m.auteur.trim()) morceaux.push(`par ${m.auteur.trim()}`);
  morceaux.push(m.licence.trim() ? `licence : ${m.licence.trim()}` : 'licence non précisée');
  return morceaux.join(' · ');
}

/** Les mentions d'un mod, en mots simples (une par ligne). */
export function mentions(m: ModSwitchVu): string[] {
  const l: string[] = [];
  if (m.reglage_requis.trim())
    l.push(`⚙ Il lui faut un réglage de l’émulateur : ${texteSimple(m.reglage_requis, false).replace(/[.\s]+$/, '')}. Tant que ce réglage n’est pas fait, Frogtend ne le met pas en place.`);
  if (m.touche_sauvegardes) l.push('💾 Il touche à tes parties : elles seront mises à l’abri avant.');
  if (m.compat === 'pas_verifie') l.push('Version du jeu pas vérifiée : il peut ne pas marcher.');
  if (!m.servi) l.push('Firehouse ne le propose plus : il reste sur ce PC tant que tu ne le retires pas.');
  return l;
}

/** La sorte du mod en mots simples (contrat 1.46) ; `null` sans fiche. Une sorte inconnue se dit « autre ». */
export function libelleType(t: string | null | undefined): string | null {
  if (!t) return null;
  const mots: Record<string, string> = {
    technique: 'amélioration technique',
    contenu: 'contenu',
    cosmetique: 'apparence',
    triche: 'triche',
    autre: 'autre',
  };
  return mots[t.toLowerCase()] ?? 'autre';
}

/** « ⭐ Recommandé » : un intérêt de 4 ou 5 sur 5. */
export function recommande(m: Pick<ModSwitchVu, 'interet'>): boolean {
  return (m.interet ?? 0) >= 4;
}

/** La ligne « ⭐ Recommandé · amélioration technique » sous le nom ; `null` s'il n'y a rien à dire. */
export function ligneSorte(m: Pick<ModSwitchVu, 'interet' | 'type'>): string | null {
  const l = [recommande(m) ? '⭐ Recommandé' : null, libelleType(m.type)].filter((x): x is string => !!x);
  return l.length ? l.join(' · ') : null;
}

/** Une description assez longue pour être repliée à deux lignes (avec « Lire plus »). */
export function descriptionLongue(d: string): boolean {
  return d.trim().length > 120 || d.trim().includes('\n');
}

/** La mention discrète : personne ne l'a encore essayé dans un émulateur. */
export function pasEncoreEssaye(m: ModSwitchVu): boolean {
  return m.servi && !m.emulateurs_essayes.length;
}

type DernierePose = ModsSwitch['derniere_pose'];

/** Allumé, mais pas mis en place à la dernière partie (la pose l'a dit : « « nom » pas posé … »). */
export function allumePasPose(m: ModSwitchVu, dp: DernierePose | undefined): boolean {
  return m.allume && !!dp?.raisons.some((r) => r.startsWith(`« ${m.nom} » pas posé`));
}

/** Le texte de l'interrupteur (jamais la couleur seule) ; un mod allumé que la dernière partie n'a pas mis en place
 * se voit. */
export function texteInterrupteurMod(m: ModSwitchVu, dp?: DernierePose): string {
  if (allumePasPose(m, dp)) return '⚠ Allumé, pas utilisé';
  return m.allume ? '✅ Allumé' : '⬜ Éteint';
}

/** « À la dernière partie (Eden, le 18/11 à 14:05) : », puis une ligne par raison ; rien si tout a été posé. */
export function lignesDernierePose(dp: DernierePose | undefined): string[] {
  if (!dp?.raisons.length) return [];
  const quand = dateVue(dp.quand);
  return [
    `⚠ À la dernière partie (${[dp.emulateur, quand].filter(Boolean).join(', ')}) :`,
    ...dp.raisons.slice(0, 8).map((r) => `• ${texteSimple(r)}`),
    ...(dp.raisons.length > 8 ? [`… et ${dp.raisons.length - 8} autre(s).`] : []),
  ];
}

/** La raison d'un mod grisé, dite simplement (jamais vide). */
export function raisonGrise(m: ModSwitchVu): string {
  return texteSimple(m.raison) || 'Il ne va pas avec la version du jeu utilisée sur ce PC.';
}

/** Le mod allumé avec lequel `cle` ne va pas (dans un sens ou dans l'autre), ou `null`. Même règle que le cœur
 * (`mods_switch::conflit` / `en_conflit`) : seuls les mods rangés et allumés comptent ; un conflit déclaré, ou deux
 * mods de musique qui veulent changer la même musique (contrat 1.55 : vu ici, à l'allumage, jamais au lancement). */
export function conflitAvec(l: ModSwitchVu[], cle: string): ModSwitchVu | null {
  const a = l.find((m) => m.cle === cle);
  if (!a) return null;
  return (
    l.find(
      (b) => b.cle !== cle && b.allume && estInstalle(b) && (a.conflits.includes(b.cle) || b.conflits.includes(a.cle) || amorcesCommunes(a, b)),
    ) ?? null
  );
}

// --- Les musiques de Mario Kart 8 Deluxe (contrat 1.55, ✅ Seb 26/11 : les musiques ne se téléchargent jamais sans un
// oui, leur taille annoncée) ---

/** « 1 mod », « 2 mods ». */
export function nMods(n: number): string {
  return `${n} mod${s(n)}`;
}

/** Un mod « de musique » : il remplace des musiques du jeu (ses amorces ne sont pas vides). */
export function estMusique(m: Pick<ModSwitchVu, 'amorces'>): boolean {
  return (m.amorces?.length ?? 0) > 0;
}

/** Deux mods de musique veulent-ils changer la même musique ? */
export function amorcesCommunes(a: Pick<ModSwitchVu, 'amorces'>, b: Pick<ModSwitchVu, 'amorces'>): boolean {
  const lb = b.amorces ?? [];
  return (a.amorces ?? []).some((x) => lb.includes(x));
}

/** Les mods de musique de `l` qui serviront vraiment : pas ceux qui arriveront éteints parce qu'un mod allumé de `tous`
 * ne va pas avec eux (même règle que le cœur, `noter_arrivee`). */
export function musiquesQuiServiront(l: ModSwitchVu[], tous: ModSwitchVu[]): ModSwitchVu[] {
  return l.filter((m) => estMusique(m) && !conflitAvec(tous, m.cle));
}

/** Ce que l'accord annonce des musiques : leur taille (lue chez Firehouse, ou la dernière connue ; `null` : pas
 * connue) ; `environ` : la taille vaut pour un autre choix de mods (des mods décochés). */
export interface AnnonceMusiques {
  octets: number | null;
  environ?: boolean;
}

/** « + 41 Mo de musiques », « + environ 41 Mo de musiques » ; sans taille connue : sans chiffre inventé. */
export function plusMusiques(octets: number | null | undefined, environ = false): string {
  return octets && octets > 0 ? `+ ${environ ? 'environ ' : ''}${taille(octets)} de musiques` : '+ des musiques, taille pas encore connue';
}

/** La ligne de l'accord pour les mods de musique (`null` s'il n'y a rien à annoncer). */
export function ligneAccordMusiques(l: ModSwitchVu[], a: AnnonceMusiques | null | undefined): string | null {
  const musiques = l.filter(estMusique);
  if (!a || !musiques.length) return null;
  const qui = musiques.length > 1 ? 'Ces mods changent' : 'Ce mod change';
  if (!(a.octets && a.octets > 0))
    return `🎵 ${qui} des musiques du jeu. La taille des musiques n’est pas encore connue : rien n’est téléchargé pour elles maintenant ; les musiques seront demandées à la prochaine partie.`;
  return `🎵 ${plusMusiques(a.octets, a.environ)} : ${qui.toLowerCase()} des musiques du jeu. Frogtend télécharge aussi les musiques des mods, préparées par Firehouse, pour toute la maison ; elles ne servent qu’avec la version exacte du jeu pour laquelle elles sont faites.`;
}

/** L'accord à l'allumage d'un mod de musique : le titre. */
export function titreAllumageMusique(m: ModSwitchVu, octets: number | null | undefined): string {
  return `🎵 Allumer « ${m.nom} » (${plusMusiques(octets)}) ?`;
}

/** L'accord à l'allumage d'un mod de musique : le texte (une ligne par paragraphe). `autre` : le mod de musique qui
 * change la même musique, éteint d'un coup (une seule fenêtre). */
export function messageAllumageMusique(m: ModSwitchVu, octets: number | null | undefined, autre?: ModSwitchVu | null): string {
  const l: string[] = [];
  if (autre) l.push(`« ${m.nom} » et « ${autre.nom} » changent la même musique du jeu : allumer « ${m.nom} » éteint « ${autre.nom} », pour toute la maison.`);
  else l.push(`« ${m.nom} » change des musiques du jeu.`);
  if (octets && octets > 0) {
    l.push(`Frogtend télécharge les musiques des mods, préparées par Firehouse (${taille(octets)}), hors partie, pour toute la maison.`);
    l.push('Elles ne servent qu’avec la version exacte du jeu pour laquelle elles sont faites.');
  } else {
    l.push('La taille des musiques n’est pas encore connue : rien n’est téléchargé maintenant ; les musiques seront demandées à la prochaine partie.');
  }
  l.push('Frogtend met tes parties de ce jeu à l’abri avant la prochaine partie.');
  return l.join('\n');
}

/** Le bouton « 🎵 Préparer les musiques (≈ 41 Mo) ». */
export function libellePreparer(octets: number | null | undefined): string {
  return `🎵 Préparer les musiques${octets && octets > 0 ? ` (≈ ${taille(octets)})` : ''}`;
}

/** L'accord du bouton « Préparer » : le titre et le texte. */
export function titrePreparer(octets: number): string {
  return `🎵 Préparer les musiques des mods (${taille(octets)}) ?`;
}
export function messagePreparer(l: ModsSwitch | null | undefined): string {
  const n = l?.musiques?.actives ?? 0;
  return [
    `Les musiques de ${nMods(n)} de musique, préparées par Firehouse, se téléchargent maintenant, hors partie, pour toute la maison.`,
    'Elles ne servent qu’avec la version exacte du jeu pour laquelle elles sont faites.',
    'Frogtend met tes parties de ce jeu à l’abri avant la prochaine partie.',
  ].join('\n');
}

/** Le bouton « Préparer » se montre-t-il ? Des musiques sont à préparer (aucun oui noté pour ce choix et cette
 * version) et Firehouse répond. */
export function preparerVisible(l: ModsSwitch | null | undefined): boolean {
  return !!l?.musiques?.a_preparer && (l.musiques.actives ?? 0) > 0 && !l.musiques.raison && l.etat === 'ok';
}

/** La ligne simple des musiques sur la fiche (bureau et canapé), ou `null` s'il n'y a rien à dire. La dernière partie
 * ne compte que si elle avait EXACTEMENT le même choix de mods de musique (`meme_ensemble`, dit par le cœur). */
export function ligneMusiques(l: ModsSwitch | null | undefined): string | null {
  if (!l) return null;
  const avant = l.musiques;
  const dp = l.derniere_pose?.musiques ?? null;
  const n = avant?.actives ?? 0;
  if (!n) return null;
  // Ce qui se voit déjà avant la partie (la version utilisée ne va pas) prime : rien ne sera posé.
  if (avant?.raison) return `🎵 ${majuscule(texteSimple(avant.raison, false))}.`;
  if (dp && avant?.meme_ensemble) {
    if (dp.en_place) return `🎵 Les musiques de ${nMods(n)} sont en place (préparées par Firehouse).`;
    if (dp.raison && !avant.a_preparer) return `🎵 ${majuscule(texteSimple(dp.raison, false))}.`;
  }
  if (avant?.a_preparer) return `🎵 Musiques de ${nMods(n)} à préparer : elles ne se téléchargent jamais sans ton accord.`;
  return `🎵 Musiques de ${nMods(n)} : elles seront mises en place à la prochaine partie.`;
}

/** « environ 3 min » si le débit de ce PC est connu ; sinon rien (jamais une durée inventée). */
export function dureeEstimee(octets: number, debit: number | null | undefined): string | null {
  return debit && debit > 0 ? environDuree(octets / debit) : null;
}

/** Le titre de l'accord : combien, combien de Mo, combien de temps ; avec un mod de musique qui servira (contrat 1.55),
 * les musiques sont annoncées aussi (`musiques` : « + 41 Mo de musiques » ; absent : rien à annoncer). */
export function titreTelechargement(l: ModSwitchVu[], debit: number | null | undefined, miseAJour = false, musiques?: AnnonceMusiques | null): string {
  const annonce = !!musiques && l.some(estMusique);
  const octets = annonce && musiques?.octets && musiques.octets > 0 ? musiques.octets : 0;
  const d = dureeEstimee(totalOctets(l) + octets, debit);
  const chiffres = `${taille(totalOctets(l))}${annonce ? ` ${plusMusiques(musiques?.octets, musiques?.environ)}` : ''}${d ? `, ${d}` : ''}`;
  if (l.length === 1) return `🧩 ${miseAJour ? 'Mettre à jour' : 'Télécharger'} « ${l[0].nom} » (${chiffres}) ?`;
  return `🧩 ${miseAJour ? 'Mettre à jour' : 'Télécharger'} ${l.length} mods (${chiffres}) ?`;
}

/** Le texte de l'accord (une ligne par paragraphe) ; `musiques` : ce qu'il faut annoncer des musiques (contrat 1.55). */
export function messageTelechargement(l: ModSwitchVu[], miseAJour = false, musiques?: AnnonceMusiques | null): string {
  const lignes: string[] = [];
  if (l.length > 1) {
    lignes.push(...l.slice(0, 8).map((m) => `• ${m.nom} (${taille(m.taille)})`));
    if (l.length > 8) lignes.push(`… et ${l.length - 8} autre(s).`);
  }
  const plus = l.length > 1;
  lignes.push(
    plus
      ? 'Ils viennent de Firehouse, qui les a vérifiés. Ils valent pour toute la maison, avec Eden comme avec Ryubing.'
      : 'Il vient de Firehouse, qui l’a vérifié. Il vaut pour toute la maison, avec Eden comme avec Ryubing.',
  );
  if (miseAJour) lignes.push('L’ancienne version reste en place tant que la nouvelle n’est pas vérifiée.');
  else lignes.push(plus ? 'Une fois téléchargés, chacun s’allume ou s’éteint d’un simple bouton.' : 'Une fois téléchargé, il s’allume ou s’éteint d’un simple bouton.');
  const touchent = l.filter((m) => m.touche_sauvegardes);
  if (touchent.length)
    lignes.push(
      plus
        ? `💾 ${touchent.map((m) => `« ${m.nom} »`).join(', ')} touche${touchent.length > 1 ? 'nt' : ''} à tes parties : Frogtend les met à l’abri avant la prochaine partie.`
        : '💾 Il touche à tes parties : Frogtend les met à l’abri avant la prochaine partie.',
    );
  const m = ligneAccordMusiques(l, musiques);
  if (m) lignes.push(m);
  lignes.push('Le téléchargement se fait en arrière-plan ; il s’arrête si une partie commence.');
  return lignes.join('\n');
}

/** « ⬇ « 1080p » : 45 % (1 sur 3) » — la ligne visible pendant un téléchargement. */
export function ligneProgresMods(su: SuiviModsSwitch | null | undefined): string | null {
  if (!su?.actif || !su.en_cours) return su?.actif ? '⬇ Téléchargement des mods…' : null;
  const pourcent = su.total > 0 ? Math.min(100, Math.floor((su.recus * 100) / su.total)) : 0;
  const rang = su.a_faire > 1 ? ` (${Math.min(su.faits + 1, su.a_faire)} sur ${su.a_faire})` : '';
  return `⬇ « ${su.en_cours} » : ${pourcent} %${rang}`;
}

/** Le pourcentage seul du mod en cours (sur sa ligne). */
export function pourcentEnCours(su: SuiviModsSwitch | null | undefined): number {
  return su?.actif && su.total > 0 ? Math.min(100, Math.floor((su.recus * 100) / su.total)) : 0;
}

/** Ce qu'annonce le lecteur d'écran : l'ÉTAPE (le mod en cours), jamais le pourcentage qui bouge toutes les 2 s. */
export function etapeModsAnnoncee(su: SuiviModsSwitch | null | undefined): string {
  if (!su?.actif) return '';
  if (!su.en_cours) return 'Téléchargement des mods en préparation.';
  return `Téléchargement du mod « ${su.en_cours} »${su.a_faire > 1 ? `, ${Math.min(su.faits + 1, su.a_faire)} sur ${su.a_faire}` : ''}.`;
}

/** Le compte rendu de la fin (erreurs, arrêt) : une ligne par souci ; vide si tout s'est bien passé. */
export function lignesFinMods(su: SuiviModsSwitch | null | undefined): string[] {
  if (!su || su.actif) return [];
  // Les messages du cœur passent par `texteSimple` (jamais « exefs », « zip »… à l'écran).
  const l = su.erreurs.slice(0, 5).map((e) => `⚠ Pas téléchargé : ${texteSimple(e, false)}`);
  if (su.erreurs.length > 5) l.push(`… et ${su.erreurs.length - 5} autre(s) souci(s).`);
  if (su.arret) l.push(`⏸ Arrêté : ${texteSimple(su.arret, false)}.`);
  return l;
}

/** Ce que la manette sait viser dans une ligne de mod : un bouton actif ou un interrupteur (jamais la ligne elle-même,
 * `tabindex="-1"`, que la manette ignore). */
export const COMMANDES_DE_LIGNE = 'button:not([disabled]), [role="switch"]:not([aria-disabled="true"])';

/** Après Télécharger ou Retirer : où rendre le focus — la commande de la ligne du mod `cle` (`data-commande`, sinon sa
 * première commande active), sinon d'une ligne qui la suivait (`ordre` : les clés quand l'action a commencé), sinon
 * l'élément de secours. */
export function cibleDuFocus(lignes: HTMLElement[], cle: string, ordre: string[], entete: HTMLElement | null): HTMLElement | null {
  const avant = ordre.indexOf(cle);
  const rang = (l: HTMLElement) => {
    const r = ordre.indexOf(l.dataset.cle ?? '');
    return r < 0 ? Number.MAX_SAFE_INTEGER : r;
  };
  const ici = lignes.filter((l) => l.dataset.cle === cle);
  const suivantes = lignes.filter((l) => l.dataset.cle !== cle && rang(l) > avant).sort((a, b) => rang(a) - rang(b));
  for (const l of [...ici, ...suivantes]) {
    // La commande de la ligne (interrupteur, Télécharger…) d'abord : le nom, focalisable lui aussi, vient avant elle.
    const principale = l.querySelector<HTMLElement>('[data-commande]');
    if (principale?.matches(COMMANDES_DE_LIGNE)) return principale;
    const c = l.querySelector<HTMLElement>(COMMANDES_DE_LIGNE);
    if (c) return c;
  }
  return entete;
}

/** « le 18/11 à 14:05 » depuis des secondes (forme du cœur) ; un autre texte est rendu tel quel. */
export function dateVue(vueLe: string | null | undefined): string | null {
  if (!vueLe) return null;
  const n = Number(vueLe);
  if (!Number.isFinite(n) || n <= 0) return vueLe;
  const d = new Date(n * 1000);
  const p = (x: number) => String(x).padStart(2, '0');
  return `le ${p(d.getDate())}/${p(d.getMonth() + 1)} à ${p(d.getHours())}:${p(d.getMinutes())}`;
}

/** Ce qu'il faut dire de la liste elle-même (hors ligne, injoignable), ou `null`. */
export function ligneEtatListe(l: ModsSwitch | null | undefined): string | null {
  if (!l) return null;
  if (l.etat === 'hors_ligne') {
    const d = dateVue(l.vue_le);
    return `📴 Firehouse ne répond pas : voici la liste vue ${d ?? 'la dernière fois'}. Rien ne peut être téléchargé pour l’instant.`;
  }
  if (l.etat === 'injoignable') return '📴 Firehouse ne répond pas : la liste des mods n’a pas pu être lue.';
  return null;
}

/** Le bloc des mods se montre-t-il ? S'il y a des mods, ou quelque chose à dire (Firehouse injoignable, une remarque,
 * ce que la dernière partie a dit). */
export function blocModsVisible(l: ModsSwitch | null | undefined): boolean {
  if (!l || l.etat === 'rien') return false;
  return l.mods.length > 0 || l.etat === 'injoignable' || !!l.remarque?.trim() || !!l.derniere_pose?.raisons.length || !!ligneMusiques(l);
}

/** À la fin d'un téléchargement : le toast (combien sont prêts) ; `null` s'il n'y a rien de prêt. */
export function messageFinMods(su: SuiviModsSwitch | null | undefined): string | null {
  if (!su || su.actif || !su.faits) return null;
  return `🧩 ${su.faits} mod${s(su.faits)} prêt${s(su.faits)} : ${su.faits > 1 ? 'ils s’allument' : 'il s’allume'} d’un simple bouton.`;
}

/** Une partie refusée parce que des mods de ce jeu arrivent (sorte « refus » du cœur) ? */
export function refusModsEnRoute(e: unknown): boolean {
  return typeof e === 'object' && e !== null && (e as { sorte?: unknown }).sorte === 'refus' && /mods de ce jeu sont en train d.arriver/i.test(String((e as { motif?: unknown }).motif ?? ''));
}

/** Le message de ce refus ; au canapé, où le suivre (il n'y a pas de fiche du jeu à l'écran). */
export function messageModsEnRoute(motif: string, canape: boolean): string {
  return `⏳ ${motif}${canape ? ' Au canapé : Y, puis « 🧩 Mods ».' : ''}`;
}

// --- La page des mods d'un jeu (✅ Seb 26/11 : « dans plus de 300 mods, juste dans la petite bande de droite, faire
// défiler c'est ingérable ») : des onglets par sorte, des filtres d'état, une recherche (bureau), des paquets. ---

/** Le bouton de la bande de droite (toujours lui, même pour 2 mods). */
export function libelleOuvrirMods(n: number): string {
  return `🧩 Ouvrir les mods (${n.toLocaleString('fr-FR')})`;
}

/** Combien de mods sont allumés mais n'ont pas été mis en place à la dernière partie. */
export function nAllumesPasUtilises(l: ModSwitchVu[], dp: DernierePose | undefined): number {
  return l.filter((m) => allumePasPose(m, dp)).length;
}

/** « ⚠ Allumé, pas utilisé : 2 mods » (la bande de droite) ; `null` s'il n'y en a pas. */
export function ligneAllumesPasUtilises(n: number): string | null {
  return n ? `⚠ Allumé, pas utilisé : ${nMods(n)} (voir « À la dernière partie »).` : null;
}

export type Onglet = 'tout' | 'recommandes' | 'technique' | 'cosmetique' | 'contenu' | 'triche' | 'autre' | 'musiques' | 'pas_decrit';

/** Les onglets, dans l'ordre de l'écran (LB / RB au canapé). */
export const ONGLETS: { cle: Onglet; libelle: string }[] = [
  { cle: 'tout', libelle: 'Tout' },
  { cle: 'recommandes', libelle: '⭐ Recommandés' },
  { cle: 'technique', libelle: 'Technique' },
  { cle: 'cosmetique', libelle: 'Apparence' },
  { cle: 'contenu', libelle: 'Contenu' },
  { cle: 'triche', libelle: 'Triche' },
  { cle: 'autre', libelle: 'Autre' },
  { cle: 'musiques', libelle: '🎵 Musiques' },
  { cle: 'pas_decrit', libelle: '📝 Pas encore décrit' },
];

const TYPES_CONNUS = ['technique', 'contenu', 'cosmetique', 'triche'];

/** La sorte d'un mod pour les onglets : une sorte inconnue compte dans « Autre » (comme `libelleType`). */
function sorteDuMod(m: Pick<ModSwitchVu, 'type'>): Onglet | null {
  if (!m.type) return null;
  const t = m.type.toLowerCase();
  return TYPES_CONNUS.includes(t) ? (t as Onglet) : 'autre';
}

/** Le mod va-t-il dans cet onglet ? (Un mod peut aller dans plusieurs : sa sorte, ⭐, 🎵.) */
export function dansOnglet(m: ModSwitchVu, o: Onglet): boolean {
  if (o === 'tout') return true;
  if (o === 'recommandes') return recommande(m);
  if (o === 'musiques') return estMusique(m);
  if (o === 'pas_decrit') return sorteDuMod(m) === null;
  return sorteDuMod(m) === o;
}

/** Le nombre de mods de chaque onglet (calculé une fois par lecture de la liste). */
export function nombresParOnglet(l: ModSwitchVu[]): Record<Onglet, number> {
  const n = Object.fromEntries(ONGLETS.map((o) => [o.cle, 0])) as Record<Onglet, number>;
  for (const m of l) for (const o of ONGLETS) if (dansOnglet(m, o.cle)) n[o.cle]++;
  return n;
}

/** « Technique (12) ». */
export function libelleOnglet(o: Onglet, n: number): string {
  return `${ONGLETS.find((x) => x.cle === o)?.libelle ?? o} (${n.toLocaleString('fr-FR')})`;
}

export type FiltreEtat = 'allumes' | 'telecharges' | 'a_telecharger' | 'majs' | 'incompatibles' | 'plus_servis';

/** Les filtres d'état (un seul à la fois ; le même bouton l'enlève). */
export const FILTRES: { cle: FiltreEtat; libelle: string }[] = [
  { cle: 'allumes', libelle: 'Allumés' },
  { cle: 'telecharges', libelle: 'Téléchargés' },
  { cle: 'a_telecharger', libelle: 'À télécharger' },
  { cle: 'majs', libelle: 'Mises à jour' },
  { cle: 'incompatibles', libelle: 'Ne vont pas avec ta version' },
  { cle: 'plus_servis', libelle: 'Plus proposés par Firehouse' },
];

/** Le mod répond-il au filtre ? (Les mêmes règles que le résumé : `aUnInterrupteur`, `aTelecharger`, `aMettreAJour`.) */
export function repondAuFiltre(m: ModSwitchVu, f: FiltreEtat): boolean {
  if (f === 'allumes') return aUnInterrupteur(m) && m.allume;
  if (f === 'telecharges') return estInstalle(m);
  if (f === 'a_telecharger') return m.etat === 'pas_telecharge' && m.servi;
  if (f === 'majs') return m.etat === 'revision_dispo' && m.servi;
  if (f === 'incompatibles') return m.etat === 'incompatible';
  return !m.servi;
}

export function nombresParFiltre(l: ModSwitchVu[]): Record<FiltreEtat, number> {
  const n = Object.fromEntries(FILTRES.map((f) => [f.cle, 0])) as Record<FiltreEtat, number>;
  for (const m of l) for (const f of FILTRES) if (repondAuFiltre(m, f.cle)) n[f.cle]++;
  return n;
}

/** « Allumés (3) ». */
export function libelleFiltre(f: FiltreEtat, n: number): string {
  return `${FILTRES.find((x) => x.cle === f)?.libelle ?? f} (${n.toLocaleString('fr-FR')})`;
}

/** Le texte où l'on cherche (nom, description, auteur), sans majuscules ni accents : calculé une fois par lecture. */
export function indexRecherche(l: ModSwitchVu[]): { libelle: string; cle: string }[] {
  return l.map((m) => ({ libelle: simplifier(`${m.nom} ${m.description} ${m.auteur}`), cle: m.cle }));
}

/** Les clés des mods qui répondent au texte tapé (tous ses mots, comme les autres recherches de Frogtend). */
export function chercherMods(index: { libelle: string; cle: string }[], texte: string): Set<string> {
  return new Set(filtrerOptions(index, texte).map((o) => o.cle));
}

/** Le mod allumé avec lequel chacun ne va pas (`conflitAvec`, pour toute la liste d'un coup). */
export function tableConflits(l: ModSwitchVu[]): Map<string, ModSwitchVu | null> {
  return new Map(l.map((m) => [m.cle, conflitAvec(l, m.cle)]));
}

/** Le conflit dit AVANT l'appui : « ⚠ Ne va pas avec « X », allumé : l'allumer éteint l'autre. » ; `null` s'il n'y en
 * a pas ou si le mod est déjà allumé. */
export function ligneConflitAvant(m: ModSwitchVu, b: ModSwitchVu | null | undefined): string | null {
  if (!b || m.allume || m.etat === 'incompatible') return null;
  const suite = aUnInterrupteur(m) ? 'l’allumer éteint l’autre' : 'il arrivera éteint';
  return `⚠ Ne va pas avec « ${b.nom} », allumé : ${suite}.`;
}

/** Pourquoi la liste affichée est vide (jamais un écran muet). */
export function pourquoiVide(o: Onglet, f: FiltreEtat | null, recherche: string, total: number): string {
  if (!total) return 'Firehouse ne propose aucun mod pour ce jeu.';
  const ou = o === 'tout' ? '' : ` dans « ${ONGLETS.find((x) => x.cle === o)?.libelle ?? o} »`;
  if (recherche.trim()) return `Aucun mod ne correspond à « ${recherche.trim()} »${ou}${f ? ' avec ce filtre' : ''} : essaie un autre mot.`;
  if (f) return `Aucun mod « ${FILTRES.find((x) => x.cle === f)?.libelle ?? f} »${ou} : enlève le filtre pour tout voir.`;
  return `Aucun mod${ou}.`;
}

/** La liste se montre par paquets (pas de défilement de 363 lignes d'un coup) : 50 au bureau, 25 au canapé. */
export const PAQUET_BUREAU = 50;
export const PAQUET_CANAPE = 25;

/** « Voir 50 de plus » (le dernier paquet peut être plus petit). */
export function libelleVoirPlus(montres: number, total: number, paquet: number): string {
  return `Voir ${Math.min(paquet, total - montres).toLocaleString('fr-FR')} de plus`;
}

/** Le texte de l'accord « Tout télécharger » : ni la liste des mods (la fenêtre les montre en cases, le résumé dit le
 * total), ni les parties touchées (dites par `ligneTouchentParties`, selon les cases COCHÉES). */
export function messageToutTelecharger(l: ModSwitchVu[], musiques?: AnnonceMusiques | null): string {
  const lignes = [
    'Ils viennent de Firehouse, qui les a vérifiés. Ils valent pour toute la maison, avec Eden comme avec Ryubing.',
    'Une fois téléchargés, chacun s’allume ou s’éteint d’un simple bouton. Les ⭐ recommandés sont cochés d’office.',
  ];
  const m = ligneAccordMusiques(l, musiques);
  if (m) lignes.push(m);
  lignes.push('Le téléchargement se fait en arrière-plan ; il s’arrête si une partie commence.');
  return lignes.join('\n');
}

/** « 💾 2 mods cochés touchent à tes parties : … » ; `null` si aucun mod coché n'y touche. */
export function ligneTouchentParties(choisis: ModSwitchVu[]): string | null {
  const n = choisis.filter((m) => m.touche_sauvegardes).length;
  if (!n) return null;
  return `💾 ${n > 1 ? `${nMods(n)} cochés touchent` : '1 mod coché touche'} à tes parties : Frogtend les met à l’abri avant la prochaine partie.`;
}

/** « Tout télécharger » ne vaut que pour l'onglet ou le filtre affiché (✅ Seb 26/11) : le bouton le dit. */
export function libelleToutTelecharger(l: ModSwitchVu[], partiel: boolean): string {
  return `⬇ Tout télécharger${partiel ? ' ici' : ''} (${l.length.toLocaleString('fr-FR')}, ${taille(totalOctets(l))})`;
}
