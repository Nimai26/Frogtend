// La suite complète, à lancer avant chaque livraison : types et composants, tests de l'interface, tests du cœur.
// S'arrête au premier échec.
import { execSync } from 'node:child_process';

const etapes = [
  ['Types et composants (svelte-check)', 'pnpm check'],
  ['Tests de l’interface (Vitest)', 'pnpm test'],
  ['Tests du cœur (cargo test)', 'pnpm test:rust'],
];

for (const [nom, commande] of etapes) {
  console.log(`\n=== ${nom}`);
  try {
    execSync(commande, { stdio: 'inherit' });
  } catch {
    console.error(`\nÉchec : ${nom}. La suite s'arrête ici.`);
    process.exit(1);
  }
}
console.log('\n✅ Suite complète : tout est vert.');
