# Fix — TW-078 : audio muet après Stop/Reset (consommateur jamais relancé)

Date : 2026-11-20. Signalé par l'humain le 2026-11-19 pendant les tests de la pastille flottante :
la machine passe en `Playing` (jauge, badge), mais **aucun son**.

## Symptôme

- Lancement d'une lecture : annonce correcte, silence total.
- Aucune erreur côté machine d'états ni côté pipeline de synthèse : les segments sont
  bien synthétisés et poussés dans la file audio.
- Reproduit de façon intermittente, plus fréquent après un `stop` (arrêt) suivi d'une relance.

## Cause racine

`AudioPlayer::spawn_consumer()` (`src-tauri/src/audio/mod.rs`) était gardé par un drapeau
`consumer_alive: AtomicBool` (introduit pour TW-041 afin d'éviter la multiplication des threads) :

```rust
if self.consumer_alive.load(SeqCst) { return; }   // ← saute le spawn
```

Séquence fautive :

1. `stop()` incrémente la génération et réveille le consommateur, mais **son thread ne pose
   `alive = false` qu'en sortant de sa boucle** — de façon asynchrone.
2. `reset()` (nouvelle lecture) arrive dans la foulée sur le thread audio et appelle
   `spawn_consumer()`. Si l'ancien thread n'a pas encore été ordonnancé, `alive` est **encore
   `true`** : aucun consommateur n'est créé.
3. L'ancien thread sort ensuite et pose `alive = false`, **après** le spawn raté.
4. Résultat : plus aucun thread ne puise dans `SegmentQueue` ; `enqueue` remplit la file,
   le sink reste vide → `is_playing()` faux, zéro audio.

Variante aggravante : l'ancien consommateur pouvait puiser un segment de la **nouvelle**
session avant de constater sa génération périmée, ou l'ajouter au **nouveau** sink.

## Correction (`src-tauri/src/audio/mod.rs`)

Le drapeau `consumer_alive` est supprimé : la **génération** est désormais l'unique source
d'autorité, sans état intermédiaire à observer.

1. **`spawn_consumer()` spawn toujours** un consommateur neuf et incrémente la génération,
   puis `notify_all()` : les anciens threads voient un jeton périmé et sortent.
   Une relance ne peut donc plus être muette par construction.
2. **`invalidate_consumers()`** centralise l'arrêt d'un consommateur (bump + `notify_all`),
   utilisé par `stop()` et `reset()`.
3. **`reset()` invalide avant de remplacer le sink** ; le sink nouveau est créé d'abord
   (si le périphérique refuse, la session en cours reste intacte).
4. **Contrôle de génération avant toute ponction** : un consommateur périmé ne peut plus voler
   un segment destiné à la session suivante.
5. **Re-vérification de la génération sous verrou du sink** : aucun segment de l'ancienne
   session ne peut être ajouté au nouveau sink.
6. `enqueue()` conserve `notify_all()` (TW-041) : un thread périmé ne peut pas absorber le
   réveil destiné au consommateur courant.

## Vérification

- Test de régression `reset_apres_stop_relance_un_consommateur` : 5 cycles
  `stop → reset → enqueue` sur un vrai périphérique ; la file doit être consommée à chaque
  cycle. Sur une machine sans sortie audio (CI), le test s'auto-déclare non concluant.
- `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`.
- Validation terrain attendue (humain) : Lire A → arrêt → Lire B, répété 5×, sans silence.

## Traçabilité

- Carte N-078 (`doing` → PR), branche `fix/TW-078-audio-silence`.
- Antécédent : `docs/s1-41-second-read-mute.md` (TW-041) — le drapeau `alive` y avait été
  introduit ; il est ici remplacé par une invalidation par génération.
