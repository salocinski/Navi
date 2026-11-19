# Fix — TW-041 : seconde sélection muette (consommateurs audio concurrents)

Date : 2026-11-19. Soulevé par l'humain : « le premier texte se lit ; le second rien, même après arrêt explicite ».

## Symptôme

- 1ʳᵉ sélection : ok. `Badge: Lecture → Inactif`, jauge s'efface, voix entendue.
- 2ⁿᵉ sélection : jauge/indicateur corrects, **aucun son**. Reproduit sur plusieurs cycles.

## Cause racine (double bug)

1. **Consommateurs concurrents** : `AudioPlayer::reset()` (envoi via `AudioHandle::reset()` à chaque nouvelle session) spawn un nouveau `thread::spawn` à chaque appel sans vérifier qu'un consommateur existe déjà ; le consommateur initial de `new()` reste vivant. Tous se partagent la même `SegmentQueue` via `pending.clone()` + `condvar`.
   - La `condvar.notify_one()` réveille **un seul** consommateur, choisi arbitrairement ; s'il est déjà obsolète (génération incrémentée par `stop`), il sort de sa boucle *sans pop* et l'échantillon est perdu.
   - Sous certaines séquences d'appels, plus aucun consommateur vivant ne poppe.

2. **Reset au rallye statique** : le consommateur initial persiste indéfiniment (l'incrément de génération était déclenché uniquement à `stop`, pas à `reset`), mais son état `Sink` est consommé par `stop` et non recréé pour `reset` — le thread restait attaché à l'ancien `Sink` mort.

## Correction (fichier `src-tauri/src/audio/mod.rs`)

1. **Alive-flag** sur `AudioPlayer` (`consumer_alive: AtomicBool`). `spawn_consumer()` n'agit que si `consumer_alive` est faux.
2. **`reset()` ne crée plus de consommateur supplémentaire** — le consommateur d'origine reste en vie. `stop()` garde son bump de génération (sortie propre pour couper), puis `reset()` re-spawn si besoin.
3. **`enqueue` passe à `notify_all()`** : tous les consommateurs se réveillent ; avec le test de génération courant, le consommateur obsolète sort proprement sans consommer d'échantillons.
4. **Boucles `break` uniformes** ; chaque sortie du thread consommateur passe par `consumer_alive.store(false)` — aucune fuite de consommateurs.
5. **Bonus** : `set_idle()` remet texte/progression/zéro — jauge visible uniquement quand une lecture est réellement active.

## Vérification

```
cargo test → 88 passed, 0 failed (int + unit + corpus)
```

Validation terrain attendue (un humain) : Lire A → arrêt → Lire B → OK, répéter 5×.

## Traçabilité

- Source : signalement humain + lecture `audio/mod.rs`.
- Les corrections `lib.rs` (jauge → disparaît en Inactif) et `state/mod.rs` (set_idle remet à zéro) ne dépendent pas des raccourcis — PR #57 (suivi TW-045) les complète naturellement.
