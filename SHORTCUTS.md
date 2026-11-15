# Raccourcis winY

Base d’édition Sublime Windows, avec deux personnalisations demandées : **Ctrl+T crée un onglet**, **Ctrl+D duplique la ligne**. La sélection de l’occurrence suivante reste disponible avec **Ctrl+K, Ctrl+D** et dans la palette.

| Commande | Raccourci |
| --- | --- |
| Palette de commandes et paramètres | Ctrl+Maj+P |
| Sélecteur d’onglets | Ctrl+P |
| Aller à la ligne | Ctrl+G |
| Nouveau / fermer / rouvrir un onglet | Ctrl+T ou Ctrl+N / Ctrl+W / Ctrl+Maj+T |
| Onglet suivant / précédent | Ctrl+Tab / Ctrl+Maj+Tab, Ctrl+Page suivante/précédente |
| Codes de caractères Windows | Alt + chiffres du pavé numérique (ex. Alt+3 → ♥, Alt+0151 → —) |
| Word Wrap / panneau Unicode | Alt, V, W / Alt, V, U |
| Ouvrir / enregistrer / enregistrer sous | Ctrl+O / Ctrl+S / Ctrl+Maj+S |
| Dupliquer la ligne / la sélection | Ctrl+D / Ctrl+Maj+D |
| Déplacer les lignes sélectionnées | Ctrl+Maj+Haut/Bas |
| Sélectionner la ligne / séparer en sélections par ligne | Ctrl+L / Ctrl+Maj+L |
| Supprimer la ligne | Ctrl+Maj+K |
| Ajouter un curseur au-dessus / en dessous | Ctrl+Alt+Haut/Bas |
| Ajouter l’occurrence suivante / toutes les occurrences | Ctrl+K, Ctrl+D / Alt+F3 |
| Réduire les sélections multiples / annuler | Ctrl+U |
| MAJUSCULES / minuscules | Ctrl+K, Ctrl+U / Ctrl+K, Ctrl+L |
| Supprimer jusqu’à la fin de ligne | Ctrl+K, Ctrl+K |
| Joindre les lignes | Ctrl+J |
| Trier sans tenir compte de la casse / avec la casse | F9 / Maj+F9 |
| Commenter / décommenter les lignes | Ctrl+/ |
| Aller à la parenthèse, accolade ou crochet associé / sélectionner l’intérieur | Ctrl+M / Ctrl+Maj+M |
| Insérer une ligne après / avant | Ctrl+Entrée / Ctrl+Maj+Entrée |
| Indenter / désindenter | Tab / Maj+Tab, Ctrl+] / Ctrl+[ |
| Copier / couper / coller / tout sélectionner | Ctrl+C/X/V/A |
| Annuler / rétablir | Ctrl+Z / Ctrl+Maj+Z ou Ctrl+Y |
| Navigation et sélection | Flèches, Home/End, Page précédente/suivante, avec Maj |
| Navigation par mots / début ou fin de document | Ctrl+flèches / Ctrl+Home/End, avec Maj pour sélectionner |
| Supprimer un mot avant / après | Ctrl+Retour arrière / Ctrl+Suppr |
| Défiler sans déplacer le curseur | Ctrl+Haut/Bas |
| Recherche / remplacement | Ctrl+F / Ctrl+H |
| Occurrence suivante / précédente | F3 / Maj+F3 ; Entrée / Maj+Entrée dans la recherche |
| Remplacer une / toutes les occurrences | Ctrl+Entrée / Ctrl+Maj+Entrée dans les champs de recherche |
| Zoom du texte / réinitialiser | Ctrl++ / Ctrl+- / Ctrl+0 |
| Menu / plein écran / quitter | Alt / F11 / Ctrl+Q ou Alt+F4 |

Les séquences comme **Ctrl+K, Ctrl+U** se font successivement : maintenir Ctrl, appuyer sur K, puis U. Échap annule une séquence commencée.

## Souris

- Clic central : ajouter un curseur.
- Glissement central : sélectionner un rectangle de texte entre les lignes visibles. Le rectangle est une sélection par ligne, sans espaces virtuels ajoutés après les fins de lignes.
- Bouton droit maintenu + déplacement : déplacer la fenêtre ; un petit seuil évite de la bouger lors d’un simple clic.
- Bouton droit maintenu + molette verticale : changer d’onglet.
- Bouton droit maintenu + molette horizontale : modifier la largeur avec interpolation fluide ; droite = élargir, gauche = rétrécir. Le déplacement de la fenêtre reste actif pendant le geste. La cible conserve les fractions de pixel ; seule la taille native finale est alignée sur les pixels physiques.
- Ctrl+molette : zoom animé du texte. Ctrl+Maj+molette : ajustement fin de 0,001 px par événement de molette. Taille exacte saisissable dans la commande `Font — text size`.
- Bouton droit + Ctrl+molette : opacité de 0 à 100 %, haut = plus opaque.
- Bouton droit + clic central : toujours au-dessus ; notification fugace en haut à droite.
- Déposer des fichiers sur la fenêtre : ouverture dans de nouveaux onglets. CSV/TSV : colonnes colorées en arc-en-ciel, séparateur détecté entre virgule, point-virgule, tabulation et barre verticale, avec prise en compte des champs entre guillemets et multilignes.
- Maj+molette : défilement horizontal (hors Word Wrap).
- Clic central sur un onglet : fermer cet onglet.

## Palette et thèmes

**Ctrl+Maj+P → theme → Entrée** affiche les variantes embarquées de One, Ayu et Gruvbox provenant de Zed, ainsi que le noir d’origine. Les flèches ou le survol prévisualisent le thème général et la coloration ; Entrée ou un clic valide, Échap annule. La commande **font** offre la même prévisualisation pour les polices installées. La taille du texte, la largeur d’indentation, le langage et l’activation de la coloration sont aussi disponibles dans la palette.

Les thèmes et polices validés sont conservés au redémarrage. Un aperçu non validé ne remplace pas les réglages sauvegardés.

## Périmètre

Il s’agit des commandes applicables au noyau actuel, pas d’une émulation complète de Sublime : pas de builds, debugger, projet, LSP, macros/plugins, repliage ni sélection par arbre syntaxique. La navigation entre délimiteurs est textuelle ; elle n’analyse pas les chaînes/commentaires. Les options de recherche restent littérales et sensibles à la casse. Les raccourcis des modules exclus ne sont pas réintroduits.


## View et Unicode

Interface en anglais. Alt seul affiche le menu ; V ouvre View, W bascule le retour à la ligne visuel, U ouvre/ferme Unicode. Aucun saut de ligne n’est ajouté au fichier par Word Wrap. Son état est conservé au redémarrage.

Le panneau à droite donne accès à 14 716 symboles, signes, caractères accentués, marques, nombres et lettres latines/grecques/cyrilliques. Recherche anglaise par nom (`arrow`, `heart`, `em dash`), symbole ou point de code (`U+2192`). La recherche prend le focus à l’ouverture. Un clic insère au curseur de l’onglet actif, y compris les sélections multiples, et rend le focus au texte. Le survol affiche le nom et, s’il existe, le code Alt. Les codes avec zéro initial correspondent à Windows-1252 ; ceux sans zéro aux symboles DOS historiques. Les dispositions/pages de codes Windows peuvent affecter ces codes. Un point de code U+XXXX n’est pas présenté comme un raccourci Alt. Échap ferme le panneau.

## Animation du curseur

Ctrl+Maj+P → `Caret — animation style` :

| Réglage | Mouvement |
| --- | --- |
| Off | Instantané, valeur par défaut |
| Linear | Vitesse constante |
| Smooth | Accélération et décélération douces |
| Spring | Ressort amorti avec léger dépassement |
| Neovide | Curseur qui se déforme en suivant le mouvement |
| Trail | Traînée de curseurs atténués |

`Caret — animation duration` règle la durée de 30 à 500 ms (valeur numérique personnalisée acceptée). Les animations utilisent les frames GPUI et s’arrêtent une fois le mouvement achevé. Les réglages validés et l’opacité sont sauvegardés. Les variantes sont des implémentations propres à winY, inspirées des discussions [Zed #44770](https://github.com/zed-industries/zed/pull/44770) et [#4991](https://github.com/zed-industries/zed/issues/4991).

La fenêtre peut être réduite à 10 × 10 unités logiques Windows. Le contenu est alors masqué par les limites de la fenêtre. Le logo embarqué contient des tailles de 16 à 256 px, notamment 32 et 64 px.


## Todo lists

In plain text and Markdown tabs, standalone `[]` and `[ ]` render as empty checkboxes; `[x]` and `[X]` render as checked boxes. Markdown lists such as `- [ ] Buy milk` work too. Click a box to select the entire marker, then press Space to toggle it. Up/Down selects the checkbox on the adjacent line, including differently indented items; the selection remains after toggling. Checking `[]` writes `[x]`; unchecking writes `[ ]`. Changes use the normal undo history and automatic session save. Copying or saving still yields ordinary text markers. Shift-click extends the text selection. Left/Right leaves the selected checkbox. Code tabs, expressions such as `array[x]`, and Markdown links stay literal.

## Win+Y setup

**Ctrl+Shift+P → Win+Y** opens the global shortcut options, with current launcher and Windows startup status:

- **Enable Win+Y now**: start the lightweight background launcher for this session.
- **Enable Win+Y and start with Windows**: also register it for Windows login.
- **Disable Windows startup only**: leave the current launcher unchanged.
- **Disable Win+Y and Windows startup**: release the shortcut and disable automatic startup.

The launcher can bring back the existing window or open winY if it is closed. Conflicts with another application's Win+Y registration are reported. The installer preserves these choices on updates. Startup registration uses the current user's Run entry and replaces the older winY Startup shortcut when configured.


## Tab bar and community themes

**Alt → V → T** toggles the top tab bar. This setting survives restart. Ctrl+T, Ctrl+Tab, Ctrl+P and right-button + wheel keep working with the bar hidden. The palette also contains `View — toggle tab bar`.

**Ctrl+Shift+P → theme** now includes all 20 leading theme extension families from the [official Zed catalog sorted by downloads](https://zed.dev/extensions?filter=themes), retrieved on 2026-11-14: Catppuccin, macOS Classic, Tokyo Night, One Dark Pro, GitHub, Nightfox, VSCode Dark Modern, Dracula, Snazzy, Catppuccin Blur, JetBrains, Cutiepro, The Dark Side, Nord, Material Dark, GitHub Dark Default, Fleet, Rosé Pine, New Darcula and Zedokai. They add 98 variants alongside winY's 12 existing choices. Hover/arrow previews, Enter applies, Escape restores the previous theme.

Color definitions come from the upstream repositories, with revisions and notices recorded in `assets/themes/`. The theme adapter imports interface/syntax colors; blur, opacity and font settings remain controlled by winY rather than imported from theme extensions.


Les onglets liés à un fichier affichent son nom avec son extension, dans la barre et le sélecteur Ctrl+P. Les notes sans fichier conservent le premier mot de la première ligne, ou leur numéro si celle-ci est vide.
