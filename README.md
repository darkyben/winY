# winY

Éditeur Windows minimal en Rust, rendu GPU via **GPUI de Zed**, fond noir et fenêtre sans barre de titre. Les sources de Zed sont épinglées dans le sous-module `upstream` ; seul le moteur GPUI Windows et ses dépendances sont liés. Ce n’est pas l’exécutable Zed avec des panneaux cachés.

## Lancer et compiler

```powershell
.\scripts\setup.ps1          # Rust, MSVC et SDK Windows, une seule fois
.\scripts\build.ps1          # compilation optimisée
.\target\release\winy.exe
.\scripts\install.ps1 -SkipBuild
```

L’installation copie l’exécutable dans `%LOCALAPPDATA%\Programs\winY` et installe un petit lanceur au démarrage de la session Windows. **Win+Y** ouvre winY, ou ramène la fenêtre existante. Fermer l’éditeur laisse le lanceur en attente du raccourci ; il n’initialise ni GPU ni moteur de texte. L’installation signale un conflit si Win+Y est déjà réservé.

Pour récupérer le dépôt : `git clone --recurse-submodules --shallow-submodules https://github.com/darkyben/winY.git`. Pour une copie déjà clonée : `git submodule update --init --depth 1`.

## Commandes

Voir [SHORTCUTS.md](SHORTCUTS.md) pour la liste complète des commandes prises en charge, les gestes souris et les thèmes.

**Ctrl+Maj+P → theme** : thèmes Zed en aperçu direct ; **font** : choix de police.

| Raccourci | Action |
| --- | --- |
| Alt seul | Afficher/masquer le menu |
| Ctrl+T / Ctrl+N | Nouvel onglet |
| Ctrl+W / clic molette sur un onglet | Fermer l’onglet |
| Ctrl+Maj+T | Rouvrir un onglet fermé dans la session courante |
| Bouton droit + molette verticale | Onglet précédent/suivant |
| Bouton droit + molette horizontale | Largeur animée ; déplacement simultané possible |
| Alt → V → T | Afficher/masquer les onglets |
| Ctrl+Tab / Ctrl+Maj+Tab | Onglet suivant/précédent |
| Ctrl+Page suivante/précédente | Changer d’onglet |
| Ctrl+O | Ouvrir des fichiers UTF-8 |
| Ctrl+S / Ctrl+Maj+S | Enregistrer / enregistrer sous |
| Ctrl+F / Ctrl+H | Recherche / remplacement |
| Entrée, Maj+Entrée dans la recherche ; F3, Maj+F3 | Occurrence suivante/précédente |
| Ctrl+Entrée / Ctrl+Maj+Entrée dans la recherche | Remplacer un / tout |
| Ctrl+D | Dupliquer la ligne |
| Ctrl+Maj+D | Dupliquer la sélection ou la ligne |
| Ctrl+L / Ctrl+Maj+K | Sélectionner / supprimer une ligne |
| Ctrl+Entrée / Ctrl+Maj+Entrée dans le texte | Insérer une ligne après / avant |
| Tab / Maj+Tab, Ctrl+] / Ctrl+[ | Indenter / désindenter |
| Ctrl+Z / Ctrl+Maj+Z / Ctrl+Y | Annuler / rétablir |
| Ctrl+C/X/V/A | Copier, couper, coller, tout sélectionner |
| Ctrl+flèches, Maj+flèches, Home/End, Ctrl+Home/End | Navigation et sélection |
| Maj+molette | Défilement horizontal |
| Échap | Quitter la recherche/le menu ou réduire les sélections multiples |
| F11 | Plein écran |
| Alt+F4 / Ctrl+Q | Quitter l’éditeur |

Le menu **Move** permet de déplacer la fenêtre par glissement. Les bords restent redimensionnables. La police `.ZedMono` correspond à **Lilex** dans cette révision de Zed ; elle est embarquée, sans installation système.

## Données

Les onglets sont sauvegardés dans `%LOCALAPPDATA%\winY\temp` : un fichier UTF-8 par onglet et un `session.json` pour l’ordre, l’onglet actif, les positions, le langage et les réglages d’apparence. Ce dossier est persistant malgré son nom `temp` ; il n’est pas nettoyé à la fermeture. Les fichiers des onglets fermés sont conservés pour récupération manuelle.

Les écritures sont réalisées sur un thread séparé, regroupées sur 250 ms, synchronisées sur disque puis remplacées atomiquement. La fermeture normale attend les écritures restantes. Une interruption brutale peut perdre la dernière fraction de seconde. Un manifeste illisible ou un fichier manquant produit une erreur et n’est pas remplacé par une session vide.

Les fichiers ouverts ont aussi une copie de travail dans ce dossier. **Ctrl+S** écrit le fichier d’origine ; la sauvegarde de session ne le modifie pas automatiquement. Pour isoler une session de test, définir `WINY_DATA_DIR` avant le lancement.

## Périmètre et performances

- Inclus : texte, onglets, édition multiple, coloration syntaxique, recherche/remplacement littéral sensible à la casse, ouverture et enregistrement.
- Exclus : Git, agents/IA, prédictions, remote dev, debugger, Vim, LSP, terminal, explorateur, compte utilisateur, télémétrie et gestionnaire d’extensions.
- Rendu DirectX de GPUI, sans plafond logiciel de cadence ajouté par winY. Le moteur présente selon l’écran et ne reconstruit pas le texte lorsque rien ne change.
- Stockage du texte par rope, historique par snapshots persistants, rendu limité aux lignes et colonnes du viewport, sauvegardes/recherche/remplacement global/coloration hors du thread graphique.
- Coloration **Syntect** indépendante des services de langage de Zed. Détection par extension après ouverture/enregistrement ; le menu Language permet de choisir un mode pour une note. Pas de Tree-sitter dans cette première version.

Ce noyau reprend les raccourcis d’édition courants de Sublime ; ce n’est pas une reproduction exhaustive de ses commandes, macros ou plugins. Recherche regex, minimap, repliage et navigation projet sont absents. La coloration des lignes de plus de 32 Kio est ignorée pour préserver la réactivité. La composition IME utilise les interfaces natives GPUI ; les sélections multiples restent une fonction d’édition, pas plusieurs compositions IME simultanées.

L’objectif de performances est celui de Zed, mais aucune équivalence de latence ou de FPS avec Zed n’est affirmée sans comparaison mesurée. Les résultats vérifiés sont consignés dans `VALIDATION.md`.

## Validation

```powershell
cargo test --release --locked
cargo fmt --check
cargo clippy --release --locked -- -D warnings
```

## Organisation

`src/document.rs` : buffer, Unicode, historique, sélections et recherche. `src/editor.rs` : rendu et saisie GPUI, coloration en tâche de fond. `src/app.rs` : onglets/menu/fichiers/recherche. `src/session.rs` : persistance. `src/native.rs` : instance unique, gestes fenêtre et raccourci global. `src/palette.rs` et `src/appearance.rs` : commandes, réglages et thèmes Zed.

winY est sous GPL-3.0-or-later. GPUI est sous Apache-2.0, Lilex sous SIL OFL ; les textes correspondants sont dans `upstream/LICENSE-APACHE` et `upstream/assets/fonts/lilex/OFL.txt`.


## Nouveautés : View, rendu et fenêtre

Interface en anglais. `Alt → V → W` active le retour à la ligne visuel ; `Alt → V → U` ouvre le panneau Unicode avec recherche par nom et insertion au clic. Les chiffres avec Alt sont réservés à la saisie Windows, et ne changent plus d’onglet.

`Ctrl+molette` zoome avec interpolation ; `Ctrl+Maj+molette` ajuste par 0,001 px. La palette propose une taille numérique exacte et six styles d’animation du curseur (`caret`) avec durée réglable. `Bouton droit + Ctrl+molette` ajuste l’opacité, `bouton droit + clic central` bascule le mode toujours au-dessus. La taille minimale est de 10 × 10 unités logiques Windows.

Les fichiers déposés ouvrent de nouveaux onglets. Les CSV/TSV ont des colonnes colorées et un séparateur détecté automatiquement, y compris les champs cités multilignes. Le logo inspiré de ▧ est embarqué dans l’exécutable et les raccourcis Windows ; les PNG et l’ICO multirésolution sont dans `assets/`.

Détails dans [SHORTCUTS.md](SHORTCUTS.md). Les sources et la licence des noms de caractères sont dans `assets/UNICODE-SOURCE.md` et `assets/LICENSE-Unicode.txt`.


## Todo lists et thèmes

Dans les notes texte et Markdown, `[]`, `[ ]` et `[x]` sont rendus comme des cases. Un clic sélectionne la case entière ; Espace la coche/décoche et Haut/Bas passe à la case de la ligne voisine. Les marqueurs restent du texte ordinaire et les modifications sont annulables.

Le sélecteur `theme` contient 20 familles populaires de thèmes Zed et leurs variantes, en plus des thèmes intégrés. Provenance et notices : [assets/themes/NOTICE.md](assets/themes/NOTICE.md). Les licences des ressources tierces restent distinctes de celle du code de winY.

Les onglets associés à un fichier portent son nom. Les notes sans fichier utilisent le premier mot de leur première ligne, ou un numéro si elles sont vides. `Ctrl+Maj+P → Win+Y` configure le raccourci global et son démarrage avec Windows.
