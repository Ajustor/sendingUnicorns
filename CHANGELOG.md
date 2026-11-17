# Changelog

Toutes les évolutions notables de sendingUnicorns. Format :
[Keep a Changelog](https://keepachangelog.com/fr/1.1.0/), versions
[SemVer](https://semver.org/lang/fr/).

## [0.3.0] - 2026-11-17

Réécriture complète, avec les méthodes des autres logiciels de la suite
Unicorns : une application Rust native (egui), sans Tauri ni Node, sans
moteur web embarqué (la preview utilise la WebView du système).

### Ajouté

- Une interface en trois volets : collections, requête, réponse, aux couleurs
  de la suite (thème clair et sombre, suit le système).
- Méthodes GET, POST, PUT, PATCH, DELETE, HEAD et OPTIONS ; corps JSON (avec
  bouton *Format*), formulaire ou texte libre, le contenu de chaque type est
  gardé quand on change de type.
- Paramètres synchronisés dans les deux sens avec l'adresse ; un paramètre
  décoché reste dans le tableau sans être envoyé.
- Environnements par collection, gérés dans une fenêtre dédiée ; une variable
  peut en utiliser une autre (`{{base}}/v1`). Les `{{variables}}` définies
  s'affichent en vert, les inconnues en rouge, et la réponse signale celles
  envoyées telles quelles.
- Réponse : statut coloré, durée, taille, en-têtes, JSON indenté et coloré,
  copie et enregistrement du corps dans un fichier.
- Import des collections Postman v2 (dossiers, variables, jeton Bearer) et
  export dans le format de sendingUnicorns.
- Renommer, dupliquer et supprimer requêtes et collections ; filtre des
  collections ; un point signale les collections non enregistrées et la
  fermeture demande quoi faire des modifications.
- Raccourcis : `Ctrl+Entrée` envoie, `Ctrl+S` enregistre, `Ctrl+T` ouvre un
  brouillon, `Ctrl+N` crée une requête, `Ctrl+W` ferme l'onglet, `Ctrl+Tab`
  passe d'un onglet à l'autre.
- L'allure de la suite : barre de titre en dégradé violet-cyan avec le logo et
  les onglets des requêtes ouvertes, barre d'adresse sombre, bouton Send en
  dégradé, pastilles pour les méthodes, les statuts et les sections.
- Onglets : plusieurs requêtes ouvertes, réordonnables par glisser-déposer ;
  brouillons hors collection, rangés par `Ctrl+S`.
- Dossiers dans les collections, avec glisser-déposer des requêtes et des
  dossiers, entre collections aussi.
- Auth Bearer, Basic et clé d'API (en-tête ou paramètre), héritée du dossier
  puis de la collection ; en-têtes communs par collection et par dossier.
- Variables de chemin (`/users/:id`), variables globales, de collection, de
  requête et *runtime* ; variables secrètes masquées.
- Corps multipart (champs et fichiers), XML, GraphQL et fichier binaire.
- Assertions déclaratives et variables extraites de la réponse, comme dans
  Bruno ; onglet Tests dans la réponse.
- Runner de collection ou de dossier, avec le bilan des assertions.
- Historique des envois ; import et export cURL ; recherche dans le corps de
  la réponse ; onglet Cookies.
- Cookies gardés entre les requêtes (désactivable) ; proxy réglable.
- Documentation par requête et par collection (onglet Docs).
- Onglet Preview de la réponse : pages HTML (avec leurs ressources relatives),
  images, SVG et PDF rendus par la WebView du système (WebView2, WKWebView,
  WebKitGTK), comme dans browsingUnicorns ; les images et PDF s'y ouvrent
  directement.
- Réglages : délai d'expiration, suivi des redirections, vérification des
  certificats (désactivable pour les serveurs locaux).
- Mises à jour intégrées, comme dans les autres logiciels de la suite :
  vérification quotidienne, bandeau, contrôle SHA-256, redémarrage.
- Installateur `.msi`, `.exe` portable, AppImage, `.dmg` et scripts
  d'installation en une ligne, publiés avec la page de téléchargement sur
  GitHub Pages.

### Modifié

- Les collections sont des fichiers JSON lisibles, un par collection, dans
  le dossier de données `sending-unicorns/collections`. Celles de la
  version 0.2 sont reprises au premier lancement (l'ancien dossier n'est pas
  modifié) et les anciens fichiers restent importables.
- Les certificats du système sont utilisés, et le proxy des variables
  `HTTP_PROXY` / `HTTPS_PROXY`.
- Les certificats sont vérifiés par défaut (ils ne l'étaient jamais).

## [0.2.11] - 2025-01-29

### Bug Fixes

- Fixing issue with debug config

## [0.2.10] - 2025-01-28

### Bug Fixes

- Fixing crash when updater cannot check status

## [0.2.9] - 2025-01-27

### Enhancements

- Add about button
- Disable default environment

### Bug Fixes

- Fixing issues with front
- Fix bug when save file


## [0.2.8] - 2025-01-27

### Enhancements

- Add logs to app

## [0.2.7] - 2025-01-27

### Enhancements

- Add modal to manage app update

### Bug fixes

- Fix app restarting loop

### Bug fixes

- Issues with file storage may be resolved

## [0.2.6] - 2025-01-27

### Bug fixes

- Issues with file storage may be resolved

## [0.2.3] - 2025-01-20

### Enhancements

- Start reworking environment system

### Bug fixes

- Fixing renderer issues
- Use new method to update collections

## [0.2.2] - 2025-01-20

### Bug fixes

- Fixing renderer issues

## [0.2.1] - 2025-01-19

Third candidate to beta testing

### Added features

- Collections export
- Collection import
- Auto updater system

### Enhancements

- Migrate to tauri 2
- Migrate to svelte 5

### Bug fixes

- Rendering issues

## [0.2.0] - 2024-07-19

Second candidate, allow users to make put, patch and delete request

### Added

- Add body
- Add params
- Add params parser

## [0.1.0] - 2024-07-02

First candidate, allow you to make get request and post but without body

### Added

- Set body headers
- Get request
- Save request to collections with Ctrl+S
- Create collections
