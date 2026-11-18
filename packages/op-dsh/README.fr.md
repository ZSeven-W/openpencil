<p align="center">
  <img src="./docs/images/dsh-openpencil-logo.png" alt="DSH OpenPencil" width="120" />
</p>

<h1 align="center">DSH OpenPencil</h1>

<p align="center">
  <strong>Le plugin DeepSeek Harness pour OpenPencil — prévisualisez, inspectez et modifiez de véritables documents <code>.op</code> directement dans une conversation.</strong><br />
  <sub>Aperçus multi-images exacts &bull; Canevas interactif &bull; Éditeur géré &bull; Outils de conception natifs pour agents</sub>
</p>

<p align="center">
  <sub>npm: <a href="https://www.npmjs.com/package/@zseven-w/dsh-openpencil"><code>@zseven-w/dsh-openpencil</code></a> · Version actuelle du plugin : <code>0.1.0-rc.7</code> · Testé jusqu'à DSH <code>0.1.1-rc.2</code></sub>
</p>

<p align="center">
  <a href="./README.md">English</a> · <a href="./README.zh.md">简体中文</a> · <a href="./README.zh-TW.md">繁體中文</a> · <a href="./README.ja.md">日本語</a> · <a href="./README.ko.md">한국어</a> · <a href="./README.fr.md"><b>Français</b></a> · <a href="./README.es.md">Español</a> · <a href="./README.de.md">Deutsch</a> · <a href="./README.pt.md">Português</a> · <a href="./README.ru.md">Русский</a> · <a href="./README.hi.md">हिन्दी</a> · <a href="./README.tr.md">Türkçe</a> · <a href="./README.th.md">ไทย</a> · <a href="./README.vi.md">Tiếng Việt</a> · <a href="./README.id.md">Bahasa Indonesia</a>
</p>

<p align="center">
  <a href="https://www.npmjs.com/package/@zseven-w/dsh-openpencil"><img src="https://img.shields.io/npm/v/%40zseven-w%2Fdsh-openpencil?style=flat&color=cfb537" alt="npm" /></a>
  <a href="https://github.com/ZSeven-W/openpencil/actions/workflows/dsh-check.yml"><img src="https://img.shields.io/github/actions/workflow/status/ZSeven-W/openpencil/dsh-check.yml?label=CI" alt="CI" /></a>
  <a href="https://github.com/ZSeven-W/openpencil/stargazers"><img src="https://img.shields.io/github/stars/ZSeven-W/openpencil?style=flat&color=cfb537" alt="Stars" /></a>
  <a href="https://github.com/ZSeven-W/openpencil/blob/main/LICENSE"><img src="https://img.shields.io/github/license/ZSeven-W/openpencil?color=64748b" alt="License" /></a>
  <a href="https://discord.gg/h9Fmyy6pVh"><img src="https://img.shields.io/badge/Discord-Join%20chat-5865F2?logo=discord&logoColor=white" alt="Discord" /></a>
</p>

<br />

<p align="center">
  <img src="./docs/images/dsh-openpencil-overview.png" alt="DSH OpenPencil — aperçu multi-images et éditeur latéral" width="100%" />
</p>
<p align="center"><sub>Aperçus <code>.op</code> multi-images exacts avec un canevas interactif et l'espace de travail de l'éditeur géré</sub></p>

## Pourquoi DSH OpenPencil

DSH OpenPencil connecte [DeepSeek Harness](https://github.com/deepseek-ai/DSH) à [OpenPencil](https://github.com/ZSeven-W/openpencil) afin qu'un agent pilote un véritable canevas de conception modifiable et interactif, au lieu de renvoyer une image générée.

<table>
<tr>
<td width="50%">

### 🖼️ Aperçus multi-images exacts

L'exportateur sans interface d'OpenPencil installé produit des aperçus fidèles à la conception : la première image de premier niveau en grand PNG sûr pour la relecture, plus une piste de vignettes défilant horizontalement, la sélection au clic et la navigation précédent/suivant pour les documents multi-images.

</td>
<td width="50%">

### 🗺️ Canevas interactif

« Ouvrir le canevas interactif » monte paresseusement le SDK web en lecture seule d'OpenPencil avec déplacement, zoom et ajustement — inspectez n'importe quelle page, nœud imbriqué ou page inactive sans quitter la conversation.

</td>
</tr>
<tr>
<td width="50%">

### ✏️ Éditeur géré

Avec `editable: true`, l'action d'édition ouvre l'éditeur géré d'OpenPencil — sélection, calques, propriétés, outils de dessin, annuler/rétablir et sémantique d'enregistrement explicite — dans un espace de travail latéral droit redimensionnable, avec une option plein écran.

</td>
<td width="50%">

### 🤖 Outils de conception natifs pour agents

Cinq outils directs de canevas et six outils `openpencil_pipeline_*` permettent à l'agent de créer, inspecter, affiner, publier, modifier et lire un véritable canevas via les runtimes OpenPencil gérés.

</td>
</tr>
<tr>
<td width="50%">

### 🔐 Autorisations contrôlées par capacités

Les autorisations d'image et de document sont des capacités signées, liées à un hachage. Les métadonnées du navigateur n'exposent jamais un chemin d'hôte arbitraire, et les capacités signées d'aperçu/éditeur n'entrent jamais dans le résultat canonique de l'outil ni dans le contexte du modèle.

</td>
<td width="50%">

### ⚡ Sécurité transactionnelle

Un document du pipeline complet reste un brouillon privé non publié jusqu'à la réussite de tous les contrôles qualité natifs et DSH. La publication n'écrase aucun chemin existant, et un abandon ou un lot échoué ne laisse aucune cible vide.

</td>
</tr>
<tr>
<td width="50%">

### 🌍 Suit l'apparence de DSH

La carte de l'outil et l'éditeur géré suivent la locale chinois/anglais et le thème clair/sombre de DSH sans recharger la session d'édition.

</td>
<td width="50%">

### 🎯 Un flux de travail complet

« Exigence → brouillon privé → lots sémantiques → aperçus live pour l'utilisateur → validation déterministe de la structure, de la mise en page et de la qualité → publication atomique avec intégrité du PNG final » — un flux complet dans DSH.

</td>
</tr>
</table>

## Installation dans DSH

DSH est un paquet distinct. Installez-le une fois si vous ne l'avez pas déjà :

```sh
npm install -g @deepseek-ai/dsh@latest
```

Ajoutez ensuite le plugin à un profil et lancez l'application web :

```sh
dsh plugin --profile web add @zseven-w/dsh-openpencil@next
dsh web
```

Pour le développement local, compilez ce checkout, liez son chemin absolu au profil Web, puis redémarrez complètement DSH :

```sh
pnpm run build
dsh plugin --profile web add link:/absolute/path/to/dsh-openpencil
dsh web
```

La dépendance `link:` rend les recompilations ultérieures visibles depuis ce checkout. DSH doit toutefois être entièrement redémarré après le remplacement de la dépendance du profil, car le profil Web fourni ne recharge pas à chaud les bundles hôte par défaut.

Vous préférez ne pas installer DSH globalement ? Exécutez les deux mêmes étapes via `pnpm dlx` :

```sh
pnpm dlx --package=@deepseek-ai/dsh@latest dsh plugin --profile web add @zseven-w/dsh-openpencil@next
pnpm dlx --package=@deepseek-ai/dsh@latest dsh web
```

> Le plugin OpenPencil est public et ne requiert aucun jeton npm. Si la pré-version DSH elle-même requiert une authentification auprès du registre, conservez cet identifiant dans une configuration npm au niveau utilisateur ou temporaire, hors du dépôt. Ce dépôt ne contient délibérément aucun identifiant de registre.

## Outils de conception

Le choix facultatif suit `openpencil_style_plan` → `openpencil_style_confirm` → `openpencil_pipeline_begin`, avec le brief original et `style_selection`. Catalogue et guide exact fonctionnent sans recommandation ; le fournisseur est désactivé par défaut. Ce transfert prend en charge script, pas app-agent ni commerce. La génération app-agent ordinaire reste disponible.

| Outil | Rôle |
| --- | --- |
| `openpencil_new` | Voie rapide compatible pour les tâches simples : exécute un script QuickJS transactionnel `batch_design`, ne publie que si la cible n'existe pas et renvoie une présentation modifiable. Préférez le pipeline complet pour un design de production. |
| `openpencil_pipeline_begin` | Démarre un brouillon privé appartenant à la session, crée en interne son unique racine, renvoie le contrat canvas/build compact correspondant au runtime et ouvre immédiatement ce même brouillon live dans la barre latérale ; le `.op` cible reste non publié. |
| `openpencil_pipeline_context` | Charge uniquement un détail ciblé de consigne, style, thème ou UI kit réellement absent du contrat begin ; ce n'est pas une étape de démarrage systématique. |
| `openpencil_pipeline_batch` | Exécute au plus deux scripts de génération natifs directs `I(...)`/`K(...)` : d'abord le premier viewport borné par `begin.next`, puis toutes les régions restantes dans un second et dernier lot. Chaque transaction réussie tente d'afficher un PNG exact à l'utilisateur. |
| `openpencil_pipeline_inspect` | Fournit uniquement un diagnostic manuel explicitement demandé par l'utilisateur ; la génération normale ne l'utilise ni comme aperçu ni comme examen visuel par le modèle. |
| `openpencil_pipeline_finish` | En un appel sain, finalise et valide le brouillon, rend automatiquement l'aperçu PNG exact de la racine finale et publie atomiquement avec `createIfAbsent`. Une seule réparation exclusivement `U(...)` et un dernier finish ne sont autorisés que si le résultat vaut `needs_correction`, `canContinue: true`, contient des `repairTargets` complets et non vides, avec `omitted: 0` ; tout autre résultat non publié est terminal. |
| `openpencil_pipeline_abort` | Supprime le brouillon non publié sans créer le fichier cible. |
| `openpencil_create` | Applique un programme transactionnel `batch_design` pour générer ou restructurer des nœuds sur un canevas existant. |
| `openpencil_edit` | Modifie un nœud explicite ou le nœud unique sélectionné par l'utilisateur. |
| `openpencil_render` | Crée un instantané `.op` immuable, adressé par contenu, et rend chaque image de premier niveau de la page active — `scale` et `editable` en option. |
| `openpencil_selection` | Lit les nœuds exacts sélectionnés dans le canevas de l'éditeur actif. |

## Flux de conception piloté par l'agent

Pour une demande courte ordinaire, appelez immédiatement `openpencil_pipeline_begin` et suivez le chemin rapide fixe : le premier lot JS direct borné par `begin.next` → un second et dernier lot JS direct contenant toutes les régions restantes → un appel à `openpencil_pipeline_finish`. Begin crée l'unique racine et ouvre aussitôt le canevas live privé dans la barre latérale ; le chemin demandé dans l'espace de travail reste absent jusqu'à la publication atomique. Après chaque begin ou lot réussi, lancez immédiatement l'appel requis suivant, sans narration, planification, comparaison, inspection ni outil sans rapport. Par défaut, le design entier n'utilise qu'une seule image ; chaque frame Hero/Product/Art/Media possède exactement un visuel principal, jamais une image accompagnée d'une icône factice.

Après chaque lot validé, l'outil tente seulement de présenter à l'utilisateur un aperçu PNG exact. Dès que l'appel revient, suivez immédiatement son `next` ; n'attendez pas une auto-inspection visuelle du modèle. Si `next` signale `previewUnavailable`, le script est déjà validé sur le canevas live : ne relancez pas le lot et n'appelez ni `openpencil_pipeline_inspect` ni `read_image`. `openpencil_pipeline_inspect` est réservé exclusivement à un diagnostic explicitement demandé par l'utilisateur.

Finish exécute les contrôles déterministes natifs de finalisation, lint, contraste et mise en page, ainsi que les garde-fous qualité stricts de DSH. Un appel sain rend le PNG exact de la racine finale et publie dans la même étape. Une réparation n'est autorisée que si le résultat contient simultanément `stage: "needs_correction"`, `canContinue: true`, des `repairTargets` complets et non vides avec `omitted: 0`, et si chaque cible possède `operation: "U"`, un `nodeId` exact non vide et un `patch` non vide. Appliquez toutes les cibles ensemble dans un unique lot exclusivement `U(...)`, puis appelez finish exactement une dernière fois sans narration. Tout autre résultat non publié, erreur ou `canContinue: false` est terminal : signalez-le une fois, sans réessayer, inspecter, lire image/contexte, abandonner ni démarrer un autre brouillon. Avec `published: true`, le PNG final et l'éditeur live sont déjà attachés : renvoyez le résultat et terminez immédiatement. Un garde-fou en échec ou `openpencil_pipeline_abort` laisse la cible absente. L'autorisation de modification liée au document n'ouvre automatiquement la barre latérale que si elle est libre et conserve **Modifier le canevas** pour un changement explicite.

Au sein du même service DSH en cours d'exécution, un changement de navigateur ou un rechargement permet de restaurer une publication durable et strictement analysée de `openpencil_new` ou `openpencil_pipeline_finish` sous forme de PNG exact avec l'action explicite **Modifier le canevas**. Une carte historique n'ouvre jamais automatiquement la barre latérale : l'utilisateur doit cliquer sur cette action. Un `openpencil_render` historique ordinaire reste en lecture seule, et les connexions non-loopback ne reçoivent jamais d'autorisation d'éditeur.

La skill `openpencil-design` incluse reste une référence légère facultative ; la génération ordinaire commence directement avec le contrat begin compact et complet. Le schéma natif actuel fait autorité et le runtime géré ne dépend pas du binaire de bureau. `openpencil_new` demeure une voie rapide compatible à lot unique.

Utilisez `openpencil_create` et `openpencil_edit` uniquement pour un canevas existant. Leurs modifications restent non enregistrées tant que l'action Enregistrer de l'éditeur n'a pas été déclenchée.

## Ressources de la visionneuse web

DSH ne sert que `client.js` pour un plugin client ; le SDK ESM d'OpenPencil, son WASM et CanvasKit sont donc préparés comme des ressources explicites de même origine :

```sh
pnpm run sync:viewer-assets
```

La commande de synchronisation utilise par défaut le monorepo OpenPencil englobant (`../..`), avec repli sur un dépôt voisin `../openpencil`. Remplacez-la avec `OPENPENCIL_ROOT` ou `--openpencil-root`. Un répertoire de ressources précompilées complet peut être sélectionné avec `DSHPLUGIN_OPENPENCIL_VIEWER_SOURCE`. La recherche au runtime peut être remplacée avec `DSHPLUGIN_OPENPENCIL_VIEWER_ASSET_DIR`.

Les ressources de la visionneuse sont chargées paresseusement uniquement après que l'utilisateur a ouvert le canevas. Si elles sont absentes ou invalides, l'aperçu PNG reste disponible et aucun bouton de canevas n'est proposé.

## Éditeur géré

Les sessions modifiables utilisent l'hôte web géré d'OpenPencil — la même architecture que celle d'`op-vscode`. Le plugin ne démarre l'hôte qu'après une action utilisateur autorisée, conserve le jeton du démon en mémoire, valide la source et l'origine de l'iframe, et ferme le processus lorsque la session d'édition se termine. La surface de l'éditeur est sélectionnée progressivement : les détails natifs de l'outil lorsque l'hôte déclare ce point d'intégration, sinon l'espace de travail latéral droit du plugin, avec des contrôles de redimensionnement et de plein écran.

Le démarrage utilise un listening handshake sûr avec les montages lents : les sondes de disponibilité ne commencent qu'après que l'hôte intégré a annoncé son adresse d'écoute. Aucune installation de bureau d'OpenPencil n'est requise.

Les installations publiées prennent en charge six cibles natives : `darwin-arm64`, `darwin-x64`, `linux-arm64`, `linux-x64`, `win32-arm64` et `win32-x64` ; les paquets Linux requièrent glibc. Le paquet racine sélectionne le paquet de plateforme adapté au système d'exploitation et au processeur au moyen d'`optionalDependencies` aux versions exactes (par exemple `@zseven-w/dsh-openpencil-darwin-arm64`). Ce paquet fournit `op-host-web-server`, le bundle web de l'éditeur et CanvasKit sous la forme d'un même runtime cohérent. L'éditeur géré ne dépend donc ni de `/Applications/OpenPencil.app`, ni d'`openpencil-desktop` dans le `PATH`, ni d'un checkout des sources d'OpenPencil.

Si DSH recharge ou décharge le plugin alors que le canevas contient des modifications non enregistrées, l'hôte conserve un brouillon de récupération local opaque pendant sept jours au maximum. Rouvrir la même source demande confirmation avant de la restaurer dans le canevas actif ; la récupération n'écrase jamais le fichier `.op` tant que l'utilisateur n'a pas explicitement enregistré.

Les paquets officiels des six plateformes reçoivent leurs points de terminaison bootstrap de collaboration Chine/Monde pendant le build de release protégé, qui valide les valeurs injectées avant publication. Un build local autonome dépourvu de cette injection peut remplacer le bootstrap avant de démarrer DSH avec `OPENPENCIL_COLLAB_BOOTSTRAP_URL=https://<your-host>/api/v1/collaboration/bootstrap` ; la valeur doit utiliser `https` et exactement le chemin `/api/v1/collaboration/bootstrap`.

La synchronisation du canevas entre appareils exige que le runtime natif PC/DSH et l'application mobile soient tous deux mis à jour vers la même ligne de publication OpenPencil contenant le correctif actuel de la file de collaboration. Associer une ancienne application mobile à un runtime PC plus récent peut encore afficher les curseurs distants sans recevoir les commits du canevas.

Pour développer depuis ce dépôt, construisez d'abord le bundle Web de l'éditeur, puis l'hôte natif, et préparez enfin ce runtime cohérent avant de lancer DSH.

`pnpm run build:editor-web` exécute le gate de bundle WASM officiellement pris en charge par OpenPencil. Il nécessite Bash, Cargo/Rust avec la cible `wasm32-unknown-unknown`, la CLI `wasm-bindgen`, `wasm-opt` de Binaryen, Node.js et `gzip` ; CanvasKit ne nécessite pas EMSDK. Le build Web n'utilise pas les variables de build du bootstrap de collaboration. Avant `pnpm run build:editor-runtime`, définissez à la fois `OPENPENCIL_BUILD_COLLAB_BOOTSTRAP_URL_CN` et `OPENPENCIL_BUILD_COLLAB_BOOTSTRAP_URL_GLOBAL`. Elles sont utilisées uniquement par le build Cargo natif, qui échoue en mode fail closed si l'une manque. Une fois les deux builds réussis, préparez le runtime avec la dernière commande.

```sh
pnpm run build:editor-web
pnpm run build:editor-runtime
pnpm run stage:editor-runtime
```

Les remplacements explicites du runtime ne sont acceptés que sous la forme d'un ensemble complet et cohérent :

- `DSHPLUGIN_OPENPENCIL_EDITOR_BINARY` pour `op-host-web-server` ;
- `DSHPLUGIN_OPENPENCIL_EDITOR_WEB_BUNDLE_DIR` pour le bundle web compilé de l'éditeur ;
- `DSHPLUGIN_OPENPENCIL_EDITOR_CANVASKIT_DIR` pour les ressources CanvasKit.

Ne fournir qu'une partie de cet ensemble constitue une configuration invalide ; le plugin ne mélange pas les chemins personnalisés avec les ressources du runtime fourni dans le paquet.

Les enregistrements utilisent un hachage de source optimiste, un remplacement atomique et une capacité successeur. Si la source change en dehors de l'éditeur, le plugin signale un conflit au lieu de l'écraser.

## Métadonnées de résultat

Le résultat visible par le modèle reste un simple JSON. `presentationMeta.$dshOpenPencil`, réservé au navigateur, porte des autorisations additionnelles pour :

- `image` : chemin PNG, URL d'aperçu/téléchargement, et largeur/hauteur réelles ;
- `frames` : chaque image de premier niveau rendue exactement, dans l'ordre de la page active, avec son identifiant/nom/index de nœud et ses URL PNG signées ;
- `document` : chemin de l'action source plus URL d'instantané immuable, octets et SHA-256 ;
- `viewer` : URL SDK/WASM/CanvasKit versionnées lorsque la route des ressources est rattachée ;
- `editor` : capacités de lancement/actualisation limitées lorsque `editable: true` est autorisé.

Le résultat enregistre également `renderer`, `rendererBinary`, `fidelity` et d'éventuels avertissements. Les messages existants de schéma v1, uniquement PNG, restent rendables.

DSH `0.1.1-rc.2` ne persiste pas les métadonnées de présentation du navigateur pour les outils imbriqués sous PTC/Code Mode. Le plugin récupère cette projection UI-only via un point de terminaison de même origine, lié à la session : le navigateur n'envoie que le session id, le call id et le SHA-256 immuable du document, tandis que l'hôte résout le résultat de référence à partir du journal de session DSH durable et utilise un marqueur éphémère en mémoire uniquement pour autoriser l'édition en direct récente. Les capacités signées d'aperçu/éditeur n'entrent jamais dans le résultat canonique de l'outil ni dans le contexte du modèle. L'historique durable d'un `openpencil_render` ordinaire reste en lecture seule. Une publication durable et strictement analysée de `openpencil_new` ou `openpencil_pipeline_finish` ne peut recevoir une autorisation d'éditeur que via loopback et après un clic explicite de l'utilisateur ; l'ouverture automatique de la barre latérale est réservée aux résultats en direct récents et fiables.

Pour une relecture bornée, la récupération des métadonnées imbriquées accepte jusqu'à 128 images de premier niveau ; les résultats Code Mode plus volumineux restent disponibles via leur repli JSON canonique.

## Limites actuelles

- Les modifications ultérieures d'un canevas existant nécessitent un éditeur géré déjà ouvert. Les changements restent non enregistrés jusqu'à ce que l'utilisateur déclenche son action Enregistrer.
- Le canevas léger du SDK web est en lecture seule ; l'édition complète passe par la surface d'éditeur géré distincte. Sur DSH `0.1.1-rc.2`, le plugin utilise l'espace de travail droit redimensionnable avec une option plein écran.
- La galerie exacte couvre les images de premier niveau de la page active ; le canevas interactif reste le moyen d'inspecter les pages inactives et les nœuds imbriqués.
- Les caches de rendu et d'instantanés ont encore besoin d'une politique de conservation au niveau du produit.

## Structure du projet

```text
packages/op-dsh/
├── src/                       Plugin sources (TypeScript)
│   ├── index.ts               Host plugin entry — Cordis service, tools, assets
│   ├── tool.ts / design-tools.ts / new-tool.ts   Host-side design tools
│   ├── renderer.ts            Exact OpenPencil renderer + Jian fallback
│   ├── editor-host.ts / editor-recovery.ts       Managed editor lifecycle + drafts
│   ├── viewer-assets.ts       Web SDK / WASM / CanvasKit asset staging
│   ├── mcp-client.ts          OpenPencil MCP connection
│   └── client/                Browser client — React workbench, gallery, selection dock
├── lib/                       Compiled output (published to npm)
├── scripts/                   Build helpers — viewer asset sync, client build, host tests
├── tests/                     Node test suites (client, host API, MCP, viewer assets)
├── docs/images/               Documentation screenshots
├── cordis.patch.yml           DSH bundle patch that mounts the plugin
├── tsconfig.json              Host / Node TypeScript config
└── tsconfig.client.json       Browser client TypeScript config
```

## Compilation et vérification

```sh
pnpm run sync:viewer-assets
pnpm run build
pnpm run test:viewer-assets
pnpm run test:client
pnpm run test:host /absolute/path/to/design.op 375 1091
```

Les compilations nécessitent Node 24.11 ou plus récent et pnpm. Les paquets hôte/client de DSH sont des dépendances de pairs fournies par le profil DSH cible. Les outils de compilation sont résolus à partir des dépendances de développement locales, du dépôt DSH lié actif ou d'un bundle source DSH installé ; `DSH_SOURCE_ROOT` permet de sélectionner explicitement un dépôt source. Le lockfile épingle les outils de compilation publics autonomes lorsque cet environnement est provisionné séparément.

Pour une pré-version DSH privée, conservez l'identifiant npm délivré hors de ce dépôt (par exemple dans un `.npmrc` au niveau utilisateur ou temporaire) et exécutez directement la version demandée :

```sh
pnpm dlx --package=@deepseek-ai/dsh@latest dsh web
```

Ne validez jamais `.npmrc`, `NPM_TOKEN` ou des identifiants de registre copiés. Ce dépôt ignore la configuration npm locale par défaut.

`test:host` effectue un véritable rendu exact, valide la géométrie IHDR et le SHA-256 des PNG, exerce les capacités immuables image/document via HTTP et vérifie que les ressources de la visionneuse peuvent être octroyées. Les dimensions attendues sont propres aux fixtures.

## Écosystème

DSH OpenPencil est le plugin DeepSeek Harness pour **[OpenPencil](https://github.com/ZSeven-W/openpencil)** — le premier outil de conception vectorielle open source et natif IA au monde — et fait partie de la famille **[ZSeven-W](https://github.com/ZSeven-W)** d'outils 100 % Rust et natifs IA.

| Projet | Présentation |
| ------- | ---------- |
| **[OpenPencil](https://github.com/ZSeven-W/openpencil)** | L'outil de conception piloté par ce plugin — génération de canevas à partir d'une invite, équipes d'agents concurrentes, fichiers `.op` au design-as-code et serveur MCP intégré. Les aperçus exacts, le canevas interactif et l'éditeur géré présentés ici sont propulsés par OpenPencil lui-même. |
| **[agent-rs](https://github.com/ZSeven-W/agent-rs)** | Un runtime asynchrone 100 % Rust pour déployer des agents LLM — multi-fournisseur, capable d'utiliser des outils de bout en bout, permissions structurées, véritable MCP, zéro `unsafe`. Alimente le runtime d'agent intégré d'OpenPencil. |
| **[jian](https://github.com/ZSeven-W/jian)** | Framework d'interface purement Rust sur GPU-Skia — widgets, disposition, événements et rechargement à chaud dans une seule pile. Le framework d'interface d'OpenPencil, et la source du moteur de rendu de secours de ce plugin. |
| **[Zode](https://github.com/ZSeven-W/zode)** | Assistant de codage open source et natif IA pour votre terminal — lit votre code, exécute des commandes et pilote OpenPencil via MCP. |
| **[noema](https://github.com/ZSeven-W/noema)** | Système de mémoire local-first, non vectoriel, pour les agents de codage — une mémoire durable sous forme de fichiers inspectables, fonctionnant sur tous les runtimes. |
| **[openpencil-skill](https://github.com/ZSeven-W/openpencil-skill)** | Le plugin de compétence LLM qui apprend aux agents IA à concevoir avec `op` — un compagnon de ce plugin DSH. |

Les autres plugins DSH :

- [DSH Android](https://github.com/ZSeven-W/dsh-android) — un émulateur Android ou un appareil USB, en direct dans la conversation, piloté entièrement via adb
- [DSH Crew](https://github.com/ZSeven-W/dsh-crew) — déléguer des tâches aux agents DSH depuis Claude Code / Codex
- [DSH iOS](https://github.com/ZSeven-W/dsh-ios) — un simulateur iOS — et un iPhone en USB — vivants dans la conversation
- [DSH Noema](https://github.com/ZSeven-W/dsh-noema) — mémoire à long terme pour DSH

## Contribuer

Les contributions sont les bienvenues ! Forkez et clonez, créez une branche, exécutez `pnpm run build` et les suites de tests, validez avec des [Conventional Commits](https://www.conventionalcommits.org/) et ouvrez une PR vers `main`.

## Communauté

<a href="https://discord.gg/h9Fmyy6pVh">
  <img src="https://raw.githubusercontent.com/ZSeven-W/openpencil/main/screenshot/logo-discord.svg" alt="Discord" width="16" />
  <strong> Rejoignez notre Discord</strong>
</a>
— Posez vos questions, partagez vos créations, proposez des fonctionnalités.

**Communauté reconnue : [LINUX DO](https://linux.do/)**

## Licence

[MIT](./LICENSE) — Copyright (c) 2026 ZSeven-W

Les composants tiers sont répertoriés dans [THIRD_PARTY_NOTICES.md](./THIRD_PARTY_NOTICES.md).
