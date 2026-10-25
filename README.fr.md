# plMail Desktop

[![CI](https://github.com/Gamso/pl_mail_desktop/actions/workflows/ci.yml/badge.svg)](https://github.com/Gamso/pl_mail_desktop/actions/workflows/ci.yml)

[English](README.md) | **Français**

Client de bureau Windows pour [pl_mail](https://github.com/karatektus/pl_mail), la messagerie
auto-hébergée. Il affiche votre boîte pl_mail dans une fenêtre dédiée et ajoute ce qu'un navigateur
ne fait pas bien : notifications Windows, nombre de messages non lus sur l'icône, démarrage avec
Windows, icône dans la barre système.

> Nom de travail : « plMail Desktop ». Ce projet est indépendant de pl_mail.

## Fonctionnalités

- **Fenêtre mail** : l'interface web de votre serveur pl_mail, avec sa propre session (2FA et SSO
  fonctionnent comme dans un navigateur).
- **Notifications Windows** pour chaque nouveau mail non lu de la boîte de réception, tous comptes
  confondus. Au-delà de trois mails d'un coup, une seule notification groupée. Un clic sur la
  notification ouvre le message concerné (ou la boîte de réception pour une notification groupée).
- **Pastille de non-lus** façon Thunderbird, sur l'icône de la barre des tâches (le nombre figure
  aussi dans l'infobulle de l'icône de la barre système).
- **Icône dans la barre système** : clic gauche pour ouvrir le mail, clic droit pour le menu
  (ouvrir, paramètres, quitter). Fermer une fenêtre ne quitte pas l'application.
- **Barre de menus avec Alt** : un appui seul sur `Alt` affiche ou masque la barre, comme dans
  Thunderbird. Elle se referme après chaque choix.
  - *Fichier* : nouveau message, synchroniser maintenant, imprimer, fermer la fenêtre,
    quitter.
  - *Affichage* : actualiser, page précédente et suivante, zoom (50 % à 300 %), plein écran.
  - *Aller à* : boîte de réception, suivis, reportés, brouillons, envoyés, archives,
    indésirables, corbeille, tous les messages, agenda, contacts.
  - *Outils* : filtres, modèles, intégrations, mots de passe d'application, réglages de plMail,
    administration (réservée aux administrateurs : un compte ordinaire reçoit le refus du
    serveur) et paramètres du client.
  - *Aide* : raccourcis clavier et nouveautés de pl_mail, ouvrir le dossier de données, à
    propos.
  - Le menu *Message* de Thunderbird (répondre, archiver, étiqueter…) n'existe pas ici : il agit
    sur le message sélectionné, qui vit dans la page web ; utilisez les boutons et les raccourcis
    de pl_mail.
- **Page « serveur injoignable »** avec la cause et les boutons *Réessayer* / *Modifier les
  paramètres*, au lieu de la page d'erreur du navigateur.
- **Quatre langues**, celles de plMail : English, Deutsch, Français et Pirate English. Par défaut
  l'application suit la langue de Windows ; elle se change dans les paramètres.
- **Liens dans les mails** : ouverts dans votre navigateur par défaut (les liens vers votre propre
  serveur pl_mail restent dans la fenêtre, et les liens `mailto:` ouvrent un nouveau message).
- **Fenêtres bloquantes** : paramètres, à propos et erreur verrouillent la fenêtre mail tant qu'elles
  sont ouvertes.
- **Mises à jour** : l'application cherche une version plus récente sur GitHub au lancement (au plus
  une fois par jour) et depuis *Aide → Rechercher des mises à jour…*. Les pré-versions ne sont jamais
  proposées. Une copie installée télécharge l'installeur signé et se met à jour sur place, en gardant
  ses raccourcis, son épinglage et ses réglages, puis redémarre ; on peut aussi ignorer une version.
- **Liens `mailto:`** (optionnel) : une fois activé dans les paramètres, l'application est proposée
  à Windows comme application de messagerie. Choisissez-la une fois dans *Paramètres → Applications
  → Applications par défaut* : un clic sur un lien `mailto:` n'importe où ouvre alors un nouveau
  message dans pl_mail. pl_mail ne lit pour l'instant que le destinataire (`/compose/new?to=…`) ;
  l'objet et le corps sont transmis et serviront dès que le serveur les lira.

## Prérequis

- Windows 10 ou 11 avec WebView2 (déjà présent sur Windows 11).
- Un serveur pl_mail joignable (adresse HTTPS, ou HTTP sur un réseau de confiance).
- Un **mot de passe d'application** pl_mail : *Réglages → Mots de passe d'application*
  (*Settings → App passwords*), à créer et copier tout de suite, il n'est affiché qu'une fois.

## Installation

Lancer l'installeur `plMail Desktop_<version>_x64-setup.exe` (voir [Compiler](#compiler)). Il
s'installe pour l'utilisateur courant, sans droits administrateur, et crée un raccourci dans le menu
Démarrer.

Au premier lancement, la page de bienvenue demande l'adresse du serveur et le mot de passe
d'application. La connexion est testée avant d'être enregistrée.

## Paramètres

| Réglage | Rôle |
|---|---|
| Adresse du serveur | `https://mail.exemple.fr` |
| Mot de passe d'application | Stocké dans le Gestionnaire d'identifiants de Windows, jamais dans un fichier. Laisser vide pour conserver l'existant. |
| Langue | Automatique (Windows), English, Deutsch, Français, Pirate English |
| Fréquence de vérification | 60 s par défaut, 15 s minimum |
| Certificat auto-signé | À n'activer que pour un serveur de confiance (réseau local) |
| Démarrer avec Windows | L'application démarre à l'ouverture de session et ouvre la fenêtre mail (elle attend le réseau jusqu'à une minute) |
| Démarrer uniquement dans la barre système | Avec *Démarrer avec Windows* : reste dans la barre système au lieu d'ouvrir la fenêtre |
| Gérer les liens `mailto:` | Déclare l'application à Windows comme application de messagerie (utilisateur courant uniquement) |
| Rechercher les mises à jour automatiquement | Activé par défaut ; contacte github.com une fois par jour |

Emplacements des données :

- réglages : `%APPDATA%\fr.gamso.plmailclient\settings.json`
- mot de passe : Gestionnaire d'identifiants, entrée `app-password.plmail-client`
- session de la fenêtre mail : `%LOCALAPPDATA%\fr.gamso.plmailclient\webview-plmail`

Pour repartir de zéro : quitter l'application, supprimer ces trois éléments.

## Comment ça marche

- L'application parle à pl_mail en **JMAP** (`/.well-known/jmap`), authentifiée par le mot de passe
  d'application en `Bearer`. C'est l'API prévue pour les clients tiers.
- *Synchroniser maintenant* (menu Fichier) appuie sur le bouton de
  synchronisation de pl_mail, qui va relever les boîtes chez Gmail, IMAP, etc. ; l'application relit
  ensuite le serveur pour mettre à jour la pastille et les notifications.
- Toutes les 60 secondes (réglable) : `Email/changes` donne les mails apparus depuis le dernier
  état, `Email/get` en lit l'expéditeur, l'objet et l'aperçu, et `Mailbox/get` donne le nombre de
  non-lus de la boîte de réception.
- Au démarrage, l'état courant est simplement mémorisé : les mails déjà présents ne déclenchent
  pas de notification.
- Pas de connexion permanente (SSE) : chaque connexion occuperait un worker PHP du serveur, ce que
  le guide des clients de pl_mail déconseille sur un petit serveur personnel.

## Mises à jour

- La recherche lit `latest.json` dans la dernière release GitHub qui n'est ni un brouillon ni une
  pré-version (`…/releases/latest/download/latest.json`) : les bêtas ne sont donc jamais proposées.
- L'installeur est signé avec la clé de mise à jour du projet, et l'application refuse une mise à jour
  dont la signature ne correspond pas à la clé publique qu'elle contient.
- La mise à jour lance l'installeur en mode « mise à jour » : l'ancienne version n'est **pas**
  désinstallée d'abord, donc raccourcis, épinglage dans la barre des tâches, réglages et démarrage
  avec Windows restent en place.
- Une copie **portable** (le fichier `…-portable.exe`, ou une version de développement) ne peut pas
  s'installer une mise à jour elle-même : elle signale qu'une nouvelle version existe et ouvre la page
  de la release.
- Les versions antérieures à la première qui contient ce mécanisme ne peuvent pas se mettre à jour
  seules : installez celle-là à la main.

## Sécurité et vie privée

- La fenêtre mail charge une page distante : elle n'a **aucun accès** aux commandes internes de
  l'application.
- La touche Alt est lue grâce aux événements clavier de WebView2, uniquement quand la fenêtre mail
  a le focus. Aucun crochet clavier global.
- L'application parle à votre serveur, et à github.com pour chercher les mises à jour (désactivable
  dans les paramètres). Elle ouvre GitHub dans votre navigateur si vous cliquez sur le lien de la
  page À propos.

## Limites connues

- Windows uniquement.
- Pas de mode hors ligne : la fenêtre mail est l'interface web.
- Un lien `mailto:` ne remplit que le destinataire : pl_mail ne lit pas encore l'objet, le corps, Cc ni Cci.
- Si le serveur devient injoignable alors que la fenêtre mail est déjà ouverte, c'est la page
  d'erreur du navigateur qui s'affiche.
- Détection des nouveaux mails par interrogation : jusqu'à une minute de délai.

## Développement

Prérequis : Node.js LTS, Rust (toolchain MSVC) et les Build Tools C++ de Visual Studio.

```powershell
npm install
npm run tauri dev
```

### Compiler

```powershell
npm run tauri build
```

L'installeur est produit dans `src-tauri\target\release\bundle\nsis\`. La compilation le signe aussi
pour la mise à jour, ce qui demande la clé privée dans l'environnement (`TAURI_SIGNING_PRIVATE_KEY`
et `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`). Pour compiler en local sans la clé :

```powershell
npm run tauri build -- --config '{"bundle":{"createUpdaterArtifacts":false}}'
```

### Tests et intégration continue

```powershell
npm test            # tests Rust : traductions, client JMAP (serveur simulé), pastille
npm run typecheck   # TypeScript
```

GitHub Actions (`.github/workflows/`) :

- `ci.yml` : à chaque push sur `main` et chaque pull request, compile le frontend, vérifie le
  formatage (`cargo fmt`), les avertissements (`cargo clippy`) et lance les tests, sur Windows.
- `release.yml` : pousser un tag `vX.Y.Z` lance les tests, compile l'installeur Windows, le signe
  pour la mise à jour, et joint l'installeur, sa signature, `latest.json` et un exe portable à une
  **release en brouillon** à publier à la main. Il s'arrête tôt si le tag diffère de la version de
  `src-tauri/tauri.conf.json`, si la clé de signature manque, ou si `CHANGELOG.md` n'a pas de section
  `## [X.Y.Z]` : cette section devient les notes de la release, que la mise à jour montre aux
  utilisateurs. Un tag avec un tiret (`v0.3.0-beta.1`) est publié comme pré-version et jamais proposé
  par la mise à jour.

Mise en place, une seule fois : générer la paire de clés, mettre la clé publique dans
`plugins.updater.pubkey` de `src-tauri/tauri.conf.json`, et stocker la clé privée et son mot de passe
dans les secrets du dépôt `TAURI_SIGNING_PRIVATE_KEY` et `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`.

```powershell
npm run tauri signer generate -- -w "$HOME/.tauri/plmail-desktop.key"
```

Gardez la clé privée hors du dépôt, avec une sauvegarde : **sans elle, les copies installées ne
peuvent plus être mises à jour**.

Pour publier une version : ajouter une section dans `CHANGELOG.md`, mettre à jour la version dans
`src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml` et `package.json`, fusionner, puis :

```powershell
git tag -a v0.2.0 -m "plMail Desktop 0.2.0"
git push origin v0.2.0
```

L'interface graphique (fenêtres, tray, barre de menus) n'est pas testée automatiquement : elle se
vérifie à la main.

### Structure

| Chemin | Contenu |
|---|---|
| `index.html`, `src/main.ts`, `src/styles.css` | Fenêtre de paramètres, à propos et erreur |
| `src/i18n.ts` | Textes de ces fenêtres (en, de, fr, pirate) |
| `src-tauri/src/lib.rs` | Application : fenêtres, tray, menus, réglages, boucle de vérification |
| `src-tauri/src/jmap.rs` | Client JMAP minimal |
| `src-tauri/src/i18n.rs` | Textes côté Rust : tray, menus, notifications, erreurs |
| `src-tauri/src/badge.rs` | Dessin de la pastille de non-lus |
| `src-tauri/src/altkey.rs` | Détection de la touche Alt dans la fenêtre mail |
| `src-tauri/src/*_tests.rs` | Tests (traductions, client JMAP avec serveur simulé) |
| `assets-src/` | Logo source |

Pour changer le nom de l'application : `APP_NAME` dans `src-tauri/src/lib.rs` et `productName` dans
`src-tauri/tauri.conf.json`.

## Crédits et licence

- Construit avec [Tauri](https://tauri.app/) 2.
- Le logo est celui de [pl_mail](https://github.com/karatektus/pl_mail), distribué sous licence
  AGPL-3.0.
- Licence : [AGPL-3.0-or-later](LICENSE), la même que pl_mail.
