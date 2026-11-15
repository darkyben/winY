# 15 novembre 2026 — redimensionnement et publication initiale

Sensibilité de redimensionnement augmentée de 40 % (0,25 → 0,35).

Le redimensionnement horizontal conserve maintenant le déplacement par bouton droit. La largeur utilise une cible fractionnaire, une interpolation temporelle et des changements de taille natifs différés ; le sens de l’axe a été inversé selon le retour utilisateur. Les pixels physiques limitent la granularité finale de la fenêtre, sans arrondir les deltas de molette à des pas fixes.

29 tests automatisés réussis, dont accumulation fractionnaire, changement de direction, convergence sans dépassement et taille minimale. Clippy sans avertissement. Aucun essai graphique supplémentaire ; le sens du geste est corrigé dans le code conformément au retour utilisateur, sans nouvelle vérification matérielle.

Le dépôt public contient les sources, ressources et notices. Les artefacts de compilation, journaux et sessions locales ne sont pas suivis. GPUI reste un sous-module public épinglé.

# Keyboard tasks, tab bar and community themes

Checkbox click selects the entire marker; Space toggles without replacing it, and Up/Down selects the next/previous line's checkbox. A regression test exercises the sequence across differently indented items and undo. Text/Markdown scope is unchanged.

Tab visibility is persisted, defaults to visible for old sessions, and is available through Alt → V → T and the palette. Community themes: 20 families / 98 additional variants fetched from upstream sources, with original notices and pinned provenance under assets/themes. Catalog tests assert theme presence and unique names; settings tests cover hidden-tab serialization and old-session defaults.

27 tests pass; release build, Clippy with warnings denied, and formatting succeed. The installed executable hash matches the release build. Session closed normally and app relaunched. No graphical verification was performed.

# Checkbox first-click alignment fix

All task marker forms reserve three spaces in the shaped text. Checkbox geometry and following text therefore remain stable when `[]` becomes `[x]`. Word wrapping also treats each marker as one fixed-width token. The source remains ordinary text and document offsets are preserved by the existing shaping map.

26 tests pass, including equal marker display widths and stable wrapped rows across checkbox states. Release build and Clippy succeed. Installed build hash checked; session closed normally and reopened. No graphical verification.

# Todo checkboxes and Win+Y options

Checkboxes are painted directly by GPUI for visible text/Markdown only. No UI framework or document-wide task index was added. Tests cover marker recognition, Unicode offsets, visible-range boundaries, preservation of carets/multiple selections, and undo/redo.

Win+Y has explicit enable/disable and Windows-startup options in the command palette. The launcher exposes a hidden message-only control window after successful hotkey registration, allowing status reporting and orderly shutdown. Installation preserves existing startup/launcher choices.

24 automated tests pass, including the three new checkbox tests. Clippy passes with warnings denied. No graphical verification was performed for these additions.

# Validation — mise à jour du 14 novembre 2026

La dernière mise à jour ajoute l’interface anglaise, View/Word Wrap, le panneau Unicode virtualisé, le logo Windows, le zoom fractionnaire animé, six modes de curseur, la transparence GPU, le mode toujours au-dessus et le dépôt de fichiers avec coloration CSV.

- **21 tests automatisés réussis** : document, persistance et compatibilité des anciens réglages, noms Unicode/codes Alt, segmentation du retour à la ligne (graphèmes, tabulations, CRLF), CSV (détection et champs multilignes avec guillemets échappés), convergence des animations.
- Compilation release réussie, Clippy sans avertissement et formatage Rust. Exécutable installé identique au build (SHA-256 contrôlé).
- Pas de contrôle graphique de cette mise à jour, conformément à la demande de réserver Computer Use aux bugs. Les nouveaux gestes Windows sont validés par lecture du routage d’événements et compilation, sans essai interactif.
- Pas de mesure de FPS ou de latence touche-écran pour cette version. Le rendu utilise le moteur GPU GPUI existant ; la précision de taille stockée est de 0,001 px, la rasterisation dépend du moteur et de l’écran.
- La liste Unicode est une sélection des catégories spéciales, pas un inventaire de tous les idéogrammes et de toutes les écritures. Les codes Alt indiqués sont historiques DOS / Windows-1252 ; les caractères restants sont toujours insérables au clic.
- Taille minimale de fenêtre : 10 × 10 unités logiques, donc ajustée au facteur DPI de Windows.

Les résultats des versions antérieures suivent ; leurs essais graphiques et mesures ne décrivent pas les nouveaux comportements.

# Validation — 13 novembre 2026

## Mise à jour du 14 novembre 2026

- Noms automatiques des onglets : numéro si la première ligne est vide, sinon premier mot de cette ligne (libellé limité à 48 caractères).
- Ctrl+D duplique les lignes ; Ctrl+Maj+D duplique la sélection. Déplacement de blocs de lignes, séquences Ctrl+K puis Ctrl+U/L, sélection multiple centrale et sélection rectangulaire, déplacement de fenêtre par bouton droit.
- Palette Ctrl+Maj+P : commandes, 12 thèmes (11 variantes Zed et winY Black), polices système, taille du texte, largeur de tabulation, langage et coloration. Aperçu réversible, réglages validés sauvegardés dans le manifeste de session.
- **14 tests automatisés passent**, dont titres, duplication de dernière ligne, CRLF, blocs de lignes sélectionnés, expansion Unicode lors d’un changement de casse et chargement des thèmes. Clippy sans avertissement et compilation optimisée réussis.
- Avant la demande de limiter Computer Use, essai graphique réussi : titre `fn` et onglet vide `2`, Ctrl+D, Ctrl+Maj+Bas, Ctrl+K puis Ctrl+U, filtre `theme`, aperçu One Dark, Échap restaurant le noir. Le manifeste est resté sur winY Black pendant l’aperçu.
- Le contrôle graphique a ensuite été arrêté par l’utilisateur. Aucune vérification graphique supplémentaire n’a été effectuée ; les gestes maintenus avec bouton central/droit et le sélecteur de polices restent à essayer à l’usage.
- Les commandes disponibles et les différences avec Sublime sont décrites dans `SHORTCUTS.md`. Les modules exclus restent exclus ; il ne s’agit pas d’une émulation complète de Sublime.

Les mesures et essais ci-dessous concernent la première version.

## Version

- Rust 1.100.0, MSVC x64 ; build `release`, optimisation 3, LTO thin.
- GPUI épinglé au commit Zed `dc93abce2fa6c0bf349d425310db4884aa953467`.
- Exécutable final : **8 443 904 octets** (~8,05 Mio).

## Contrôles réussis

- `cargo test --release --locked` : **7 tests réussis**. Unicode UTF-16, graphèmes traversant les chunks du rope, CRLF, recherche avec bouclage, remplacement/undo, Ctrl+D multiple, édition de 100 000 lignes et flush atomique à la fermeture.
- `cargo fmt --check` : réussi.
- `cargo clippy --release --locked -- -D warnings` : réussi.
- Arbre des dépendances de production : aucun module Zed Git, projet, agent, remote, Vim, debugger, terminal, LSP ou prédiction.
- Essai graphique d’un premier build : fenêtre noire sans barre de titre, saisie accents/emoji, Ctrl+T, menu Alt, choix du mode Rust, coloration, recherche, navigation entre les champs, remplacement global et Ctrl+Q.
- Vérification des fichiers après fermeture : deux onglets, texte Unicode et texte Rust modifié présents ; onglet actif et syntaxe enregistrés.
- Redémarrage du build final sur cette session : processus lancé sans erreur dans le journal. Le contrôle visuel de la restauration n’a pas été terminé : l’utilisateur a interrompu Computer Use avec Échap.

## Mesures indicatives

Le premier processus GUI observé utilisait **67 604 480 octets de working set** (~64,5 Mio), avec deux petites notes. Cette mesure inclut le moteur graphique et la coloration ; ce n’est pas une limite de mémoire.

Microbenchmark final via `scripts/measure.ps1` : buffer de **11 162 250 octets**, 1 000 insertions avec historique d’annulation. Médiane **1 500 ns**, p95 **1 900 ns**, maximum **77 500 ns**. Le benchmark mesure uniquement l’édition du rope et le snapshot d’historique, **sans le rendu, les entrées Windows ni les écritures disque**. Ce n’est donc pas une mesure de latence touche-écran.

## Installation et vérifications restantes

Installation dans `%LOCALAPPDATA%\Programs\winY`, raccourci du menu Démarrer et lanceur `--hotkey-launcher` dans le dossier de démarrage de Windows. Le lanceur enregistre Win+Y et attend les messages sans initialiser GPUI.

Le raccourci physique Win+Y et le geste bouton droit + molette restent à essayer manuellement. Pas de mesure comparative de FPS ou de latence avec Zed. Pas de campagne IME multilingue, de panne électrique ni de test prolongé sur des fichiers de plusieurs gigaoctets.
