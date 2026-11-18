# S1-035 — Nettoyage et normalisation du texte capturé (carte N-035)

Date : 2026-11-18. Carte : N-035 (P1, M). Module : `src-tauri/src/text/mod.rs`.
Exigence (revue TW-014, notée sur la carte) : exclure les éléments non lisibles ; une sélection sans contenu texte exploitable doit renvoyer une erreur claire.

## Pipeline

```
capture_selection() → format_used ("unicode"/"html"/"rtf"/"ansi") + texte brut
        │
        ▼
clean_captured_text(raw, format_hint)
   1. HTML ? → extract_fragment (StartFragment…EndFragment, sans en-têtes clipboard)
   2. commentaires <!--…--> retirés
   3. éléments bloqués retirés (script/style/svg/noscript/iframe/object/embed/video/audio/canvas/applet) — contenu compris
   4. balises restantes retirées ; balises de bloc → \n (p, div, br, li, h1-6, pre, table…)
   5. entités HTML décodées (&amp; &lt; &gt; &quot; &apos; &nbsp; &#NNN; &#xHHH;)
   6. RTF ? → table groups (fonttbl/colortbl/stylesheet/info/\*) retirées, \par/\line → \n, mots de contrôle supprimés
   7. URLs et data: URIs supprimées (http/https/www/data/ftp/mailto) — newlines conservées
   8. abréviations FR de base (M.→Monsieur, Mme→Madame, …, p.ex.→par exemple — avec frontières de mot pour éviter les faux positifs)
   9. normalisation : NBSP/contrôles retirés, espaces et newlines compactées (≤2)
        │
        ▼
   texte prêt pour espeak / Kokoro — ou CleanError::NoText (erreur FR claire) si rien d'exploitable
```

## Couverture (16 tests dans `text::tests`, suite 50 passed)

- HTML fragment réel (en-têtes clipboard ignorés) ; `<script>`/`<style>` supprimés avec contenu ; `<img src="data:image/png;base64,…">` supprimée (500 caractères base64) ; texte de lien conservé ; `<p>`/`<div>`/`<br>` → newlines ; entités décodées ; `"<div>×500×base64</div>"` → erreur `NoText` avec message clair.
- RTF : `{…\fonttbl…}` et mots de contrôle supprimés, `\par` → newline.
- Abréviations : frontières de mot ; « num.ro » non touché.
- Général : URLs nues supprimées sans détruire les newlines ; NBSP/contrôles ; texte plain.

## Limites documentées

1. **Le câblage pipeline capture→nettoyage→synthèse n'est pas fait ici** — il arrive avec N-039/N-041. `clean_captured_text` est exposée et testée ; `CaptureResult` reste brut (fidélité de la capture).
2. Le strip HTML est **maison** (pas de dépendance `scraper`/`html5ever`) : suffisant pour le clipboard réel (fragments Word/Chromium/Office), pas un parseur HTML général. Cas pathologiques (imbrication exotique, `>` dans attribut) → texte approximatif.
3. RTF : meilleur effort ; tableaux RTF complexes restent approximatifs ; pas de décodage sémantique des styles.
4. Abréviations : liste volontairement minimale (FR), extensible ; espeak gère partiellement les autres (ex. "Dr" non couvert dans tous les contextes).
5. Pas de détection de langue ici (c'est N-036) ; la phonémisation espeak sélectionne le voicepack via le paramètre `language` en amont.
6. La conservation du saut de ligne entre balises inline (`<b>mot</b>`) → pas de newline, texte concaténé naturellement — correct.

## Traçabilité

- Carte : N-035. Sources : `src-tauri/src/text/mod.rs`, `cargo test` (50 passed).
- Exigence (revue TW-014) : médias/markup exclus, `NoText` renvoyée avec message FR clair — couvert par `html_data_uri_supprimee` et `html_vide_donne_erreur_claire` ✓.
- Dépendances : N-032 (machine d'états) pour l'intégration pipeline (N-041) ; N-036 pour la langue ; N-045 pour le déclenchement.
