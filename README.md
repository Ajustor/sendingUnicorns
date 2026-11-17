# 🦄 Sending Unicorns

Un client HTTP léger et rapide construit en Rust, l'équivalent de Postman ou
Bruno dans la suite Unicorns.

**Philosophie** : pas de navigateur embarqué, pas de compte, pas de nuage. Un
seul exécutable natif (egui), qui démarre en un instant ; vos collections sont
de simples fichiers JSON sur votre machine, faciles à partager et à versionner.

---

## Fonctionnalités

Tout ce qu'on attend de Postman ou de Bruno, dans l'allure de la suite
Unicorns : barre de titre en dégradé avec les onglets, barre d'adresse sombre,
thème clair ou sombre pour les panneaux.

- 🗂️ **Onglets** : plusieurs requêtes ouvertes, réordonnables, brouillons hors
  collection (« Untitled request ») que `Ctrl+S` range où l'on veut
- 📁 **Collections et dossiers** : arborescence, glisser-déposer entre dossiers
  et collections, renommer / dupliquer / supprimer, filtre, point rose sur ce
  qui n'est pas enregistré, la fermeture demande quoi faire
- ✉️ **Requêtes** : GET, POST, PUT, PATCH, DELETE, HEAD, OPTIONS ; paramètres
  synchronisés avec l'adresse, **variables de chemin** `/users/:id`, en-têtes
- 📦 **Corps** : JSON (bouton *Format*), formulaire URL-encoded, **multipart**
  (champs et fichiers), texte, XML, **GraphQL** (requête + variables), fichier
  binaire
- 🔐 **Auth** : Bearer, Basic, clé d'API (en-tête ou paramètre), héritée du
  dossier puis de la collection ; en-têtes communs par collection et par
  dossier
- 🌍 **Variables** `{{nom}}` partout : globales, de collection,
  d'environnement (secrètes masquées), *runtime*, de requête — vertes si
  définies, rouges sinon
- 🧪 **Tests sans script**, comme Bruno : assertions déclaratives
  (`res.status eq 200`, `res.body.items[0].id isNumber`,
  `res.headers.content-type contains json`…) et variables extraites de la
  réponse (`token = res.body.access_token`) pour les requêtes suivantes
- ▶️ **Runner** : une collection ou un dossier envoyé dans l'ordre, chaque
  requête voyant les variables posées par les précédentes, bilan des
  assertions
- 📨 **Réponses** : statut, durée, taille ; JSON indenté et coloré, recherche
  surlignée ; en-têtes, cookies, résultats des tests ; copie ou enregistrement
  du corps
- 👁️ **Preview** : pages HTML (CSS, scripts, images relatives), images, SVG et
  PDF rendus par la WebView du système, comme dans browsingUnicorns
- 🍪 **Cookies** gardés et renvoyés comme par un navigateur (désactivable,
  liste et oubli dans les réglages)
- 🕘 **Historique** des envois, rouvrable en un clic
- 🔁 **cURL** : coller une commande (même dans la barre d'adresse) crée la
  requête ; toute requête se copie en `curl`
- 📥 **Import** Postman v2 (dossiers, auth, variables, multipart, GraphQL) et
  anciennes versions ; **export** au format sendingUnicorns
- ⚙️ Délai, redirections, vérification TLS, **proxy** (ou `HTTPS_PROXY`),
  certificats du système
- ⬆️ Mises à jour intégrées, comme dans le reste de la suite

| Raccourci | Action |
|---|---|
| `Ctrl+Entrée` (ou `Entrée` dans l'adresse) | Envoyer |
| `Ctrl+S` | Enregistrer (choisir la collection d'un brouillon) |
| `Ctrl+T` | Nouvelle requête (brouillon) |
| `Ctrl+N` | Nouvelle requête dans la collection courante |
| `Ctrl+W` | Fermer l'onglet |
| `Ctrl+Tab` / `Ctrl+Maj+Tab` | Onglet suivant / précédent |

### Pourquoi pas de scripts JavaScript ?

Postman exécute du JavaScript avant et après chaque requête. Embarquer un
moteur JS alourdirait l'exécutable de plusieurs mégaoctets. Comme Bruno, les
besoins courants (vérifier une réponse, en extraire un jeton pour la suite)
sont couverts de façon déclarative : onglets **Assert** et **Vars**.

## Lancer

```
sendingUnicorns              Ouvre l'application
sendingUnicorns --version    Affiche la version
sendingUnicorns --help       Affiche l'aide
```

Les données sont dans `<dossier de données>/sending-unicorns/`
(`%APPDATA%` sous Windows, `~/Library/Application Support` sous macOS,
`~/.local/share` sous Linux) :

```
sending-unicorns/
  collections/<nom>.json   une collection par fichier (format sending-unicorns/2)
  globals.json             variables globales
  history.json             les 300 derniers envois
  settings.json            délai, redirections, TLS, proxy, cookies
```

`SU_DATA_DIR` fait pointer l'application vers un autre dossier (tests,
installation portable). Au premier lancement, les collections de la
version 0.2 (`.sending_unicorns/collections`) sont copiées, l'ancien dossier
n'est pas touché.

## Architecture

Clean architecture en workspace Cargo, comme browsingUnicorns : les
dépendances pointent vers le domaine, jamais l'inverse, ce que
`tests/architecture.rs` vérifie.

```
sending-unicorns (src/)       présentation egui + composition root (bootstrap.rs)
   │
   ├── infrastructure          adaptateurs : UreqHttpClient (cookies, multipart, proxy),
   │      │                    formats (natif v1/v2, Postman, 0.2), fichiers JSON
   │      ▼
   ├── application             un cas d'usage par intention (SendRequest, RunCollection,
   │      │                    ImportCollection, RecordHistory…) + ports
   │      ▼
   └── domain                  règles métier pures : Request, Collection (dossiers),
                               Auth, gabarits {{…}}, Workspace (portées de variables),
                               prepare, assertions, cURL, lecteur JSON
```

| Couche | Crate | Dépend de | Rôle |
|---|---|---|---|
| Domaine | `crates/domain` | rien | Entités et règles, sans I/O |
| Application | `crates/application` | domain | Cas d'usage, ports (traits) |
| Infrastructure | `crates/infrastructure` | domain, application, ureq, serde | Implémente les ports |
| Présentation | `src/` | toutes | UI egui, câblage |
| Mise à jour | `crates/updater` | ureq, sha2, self-replace | Partagé avec storingUnicorns et browsingUnicorns |

- Les widgets produisent des `Command`, exécutées une fois par frame.
- Une requête part sur un thread (`ureq`, bloquant) : pas de runtime async à
  embarquer. Le résultat revient par un canal ; *Cancel* l'ignore.
- L'éditeur travaille sur une copie de la requête : la collection n'est
  marquée modifiée que si quelque chose a vraiment changé.

### Légèreté

- Pas de moteur web embarqué : la preview utilise la WebView du système
  (WebView2, WKWebView, WebKitGTK) via wry, comme browsingUnicorns. Pas de
  Node, pas de tokio, pas d'OpenSSL (`deny.toml` y veille) : rustls avec les
  certificats du système, le même client HTTP que l'updater.
- Le domaine n'a aucune dépendance (même le JSON des corps est indenté et
  coloré par un petit tokenizer maison, tolérant aux `{{variables}}`).
- Profil release optimisé pour la taille (`opt-level = "s"`, LTO, strip).

## Compiler et tester

```bash
cargo run                  # debug
cargo run --release        # optimisé pour la taille
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings   # lints de l'équipe (Cargo.toml)
cargo fmt --all --check
```

Linux : `libwebkit2gtk-4.1-dev`, `libgtk-3-dev`, `libsoup-3.0-dev` (preview),
`libxkbcommon-dev` et les en-têtes xcb pour egui/winit (voir
`.github/workflows/ci.yml`). À l'exécution, la preview demande WebKitGTK
(`libwebkit2gtk-4.1-0`) et une session X11 (XWayland suffit).

## Icône

Même licorne que les autres logiciels de la suite, le sabot posé sur une
enveloppe d'où s'envole un avion en papier. Pour la régénérer (Pillow requis) :

```powershell
python scripts/draw-icon.py assets/icon.png 256
./scripts/New-Icon.ps1   # assets/icon.ico, intégrée à l'exe par build.rs
```

## Releases et mises à jour

Comme storingUnicorns et browsingUnicorns :

1. Monter `version` dans `Cargo.toml` et transformer `## [Unreleased]` en
   `## [X.Y.Z] - date` dans `CHANGELOG.md`.
2. Pousser un tag `vX.Y.Z` : `.github/workflows/release.yml` vérifie que le tag
   correspond à `Cargo.toml`, teste, construit `.msi`, `.exe` portable,
   AppImage, `.dmg` et les binaires seuls, crée la release GitHub puis publie
   sur GitHub Pages `latest.json` (version, notes, URL et SHA-256 de chaque
   fichier), la page de téléchargement (`pages/index.html`) et les scripts
   d'installation.
3. L'application lit `https://ajustor.github.io/sendingUnicorns/latest.json`
   une fois par jour, propose la mise à jour dans un bandeau, vérifie le
   SHA-256, puis remplace l'exécutable (ou lance le `.msi` à la fermeture).
   Un clic sur la version, en bas à gauche, force la vérification.

`.github/workflows/pages.yml` republie la page quand ses sources changent sur
`master`, sans attendre de release. GitHub Pages doit être réglé sur
« GitHub Actions » dans les paramètres du dépôt.

Installation en une ligne :

```bash
curl -fsSL https://ajustor.github.io/sendingUnicorns/install-macos.sh | bash   # macOS
curl -fsSL https://ajustor.github.io/sendingUnicorns/install.sh | sh           # Linux
```

```powershell
irm https://ajustor.github.io/sendingUnicorns/install.ps1 | iex                # Windows
```
