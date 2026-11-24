// Signature ed25519 du manifeste des modèles (carte N-098).
//
//   node tools/sign-manifest.mjs keygen <dossier hors dépôt>      crée la paire de clés (affiche la clé PUBLIQUE seulement)
//   node tools/sign-manifest.mjs sign <clé privée.pem> <manifeste> écrit <manifeste>.sig (hexadécimal)
//
// La clé privée ne doit jamais entrer dans le dépôt ni être partagée.
import { createPrivateKey, createPublicKey, generateKeyPairSync, sign, verify } from 'node:crypto';
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const [command, ...args] = process.argv.slice(2);

// Les 32 octets de la clé publique brute terminent l'encodage SPKI d'une clé ed25519.
const rawPublic = (key) => key.export({ type: 'spki', format: 'der' }).subarray(-32);

if (command === 'keygen' && args.length === 1) {
  const privatePath = join(args[0], 'navi-models-ed25519.pem');
  if (existsSync(privatePath)) throw new Error(`${privatePath} existe déjà : refus d'écraser.`);
  mkdirSync(args[0], { recursive: true });
  const { privateKey, publicKey } = generateKeyPairSync('ed25519');
  writeFileSync(privatePath, privateKey.export({ type: 'pkcs8', format: 'pem' }), { flag: 'wx' });
  console.log(`Clé privée écrite dans ${privatePath} (ne pas afficher, ne pas partager).`);
  console.log(
    `Clé publique (base64, à fixer dans le code) : ${rawPublic(publicKey).toString('base64')}`,
  );
} else if (command === 'sign' && args.length === 2) {
  const [keyPath, manifestPath] = args;
  const privateKey = createPrivateKey(readFileSync(keyPath));
  const data = readFileSync(manifestPath);
  const signature = sign(null, data, privateKey);
  if (!verify(null, data, createPublicKey(privateKey), signature))
    throw new Error('Signature invalide.');
  writeFileSync(`${manifestPath}.sig`, signature.toString('hex') + '\n');
  console.log(
    `Signature écrite dans ${manifestPath}.sig (clé publique : ${rawPublic(createPublicKey(privateKey)).toString('base64')})`,
  );
} else {
  console.error('Usage : keygen <dossier> | sign <clé privée.pem> <manifeste>');
  process.exit(2);
}
