// Prépare une livraison : suite complète, installateur signé, fichier latest.json des mises à jour.
// Le dépôt des fichiers dans la « release » GitHub reste une étape à part, faite à la main (rien n'est publié ici).
//
// La clé privée de signature n'est JAMAIS dans le dépôt. Elle est lue dans le fichier désigné par
// FROGTEND_CLE_SIGNATURE, sinon dans %USERPROFILE%\.tauri\frogtend.key. Son mot de passe éventuel :
// TAURI_SIGNING_PRIVATE_KEY_PASSWORD.
import { execSync } from 'node:child_process';
import { existsSync, readFileSync, statSync, writeFileSync } from 'node:fs';
import { homedir } from 'node:os';
import { join } from 'node:path';

const DEPOT = 'https://github.com/Nimai26/Frogtend';

const version = readFileSync('VERSION', 'utf8').trim();
const cheminCle = process.env.FROGTEND_CLE_SIGNATURE ?? join(homedir(), '.tauri', 'frogtend.key');
if (!existsSync(cheminCle)) {
  console.error(`Refusé : clé de signature introuvable (${cheminCle}). Sans elle, les mises à jour ne sont pas signées.`);
  process.exit(1);
}

execSync('node scripts/tests.mjs', { stdio: 'inherit' });

console.log(`\n=== Construction de Frogtend ${version}`);
execSync('pnpm tauri build', {
  stdio: 'inherit',
  env: {
    ...process.env,
    TAURI_SIGNING_PRIVATE_KEY: readFileSync(cheminCle, 'utf8'),
    TAURI_SIGNING_PRIVATE_KEY_PASSWORD: process.env.TAURI_SIGNING_PRIVATE_KEY_PASSWORD ?? '',
  },
});

const dossier = 'src-tauri/target/release/bundle/nsis';
const installateur = `Frogtend_${version}_x64-setup.exe`;
const signature = `${installateur}.sig`;
for (const f of [installateur, signature]) {
  if (!existsSync(join(dossier, f))) {
    console.error(`Échec : ${f} n'a pas été produit dans ${dossier}.`);
    process.exit(1);
  }
}

/** Les notes de cette version, tirées de docs/NOTES-DE-VERSION.md. */
function notes() {
  const texte = readFileSync('docs/NOTES-DE-VERSION.md', 'utf8');
  const debut = texte.indexOf(`## ${version}`);
  if (debut < 0) return `Frogtend ${version}`;
  const suite = texte.indexOf('\n## ', debut + 1);
  return texte.slice(debut, suite < 0 ? undefined : suite).trim();
}

const latest = {
  version,
  notes: notes(),
  pub_date: new Date().toISOString(),
  platforms: {
    'windows-x86_64': {
      signature: readFileSync(join(dossier, signature), 'utf8').trim(),
      url: `${DEPOT}/releases/download/v${version}/${installateur}`,
    },
  },
};
writeFileSync(join(dossier, 'latest.json'), `${JSON.stringify(latest, null, 2)}\n`);

const taille = (f) => `${(statSync(join(dossier, f)).size / 1e6).toFixed(1)} Mo`;
console.log(`
✅ Livraison ${version} prête dans ${dossier} :
   - ${installateur} (${taille(installateur)})
   - latest.json

À faire pour publier : créer la release « v${version} » sur ${DEPOT}/releases/new
et y joindre ces DEUX fichiers (les noms doivent rester exacts).`);
