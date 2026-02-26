# Fartcloud 🌥️💨

Un jeu style Flappy Bird avec un nuage péteur, écrit en Rust/Macroquad et compilé en WebAssembly.

## Setup du projet

### 1. Prérequis

```bash
# Installer Rust (si pas déjà installé)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Ajouter la cible WASM
rustup target add wasm32-unknown-unknown
```

### 2. Développement local

```bash
# Lancer en mode desktop (natif, sans WASM)
cargo run

# Build WASM pour test local
cargo build --release --target wasm32-unknown-unknown
mkdir -p dist
cp target/wasm32-unknown-unknown/release/fartcloud.wasm dist/
cp -r assets dist/
cp web/index.html dist/

# Servir localement
python3 -m http.server -d dist 8080
# → Ouvrir http://localhost:8080
```

En mode anonyme (par défaut), le jeu est 100% jouable sans aucun service externe.

### 3. Créer le repo GitHub

```bash
cd fartcloud
git init
gh repo create fsalmon-me/fartcloud --public --source=. --push
```

### 4. Déploiement — Hetzner Server

Le jeu est déployé comme fichiers statiques sur un serveur Hetzner via Caddy.
Voir `AGENTS.md` pour les détails complets du déploiement.

#### Secrets GitHub Actions (repo Settings → Secrets → Actions)

| Secret | Description |
|--------|-------------|
| `HETZNER_SSH_KEY` | Contenu de la clé privée SSH (`~/.ssh/hetzner`) |
| `HETZNER_SSH_HOST` | IP du serveur (`46.225.85.7`) |
| `HETZNER_SSH_USER` | Utilisateur SSH (`admin`) |

```bash
gh secret set HETZNER_SSH_KEY < ~/.ssh/hetzner
gh secret set HETZNER_SSH_HOST --body '46.225.85.7'
gh secret set HETZNER_SSH_USER --body 'admin'
```

#### Première mise en place sur le serveur

```bash
ssh -i ~/.ssh/hetzner admin@46.225.85.7 \
  "sudo mkdir -p /var/www/fartcloud && sudo chown admin:admin /var/www/fartcloud"
```

Puis ajouter le bloc Caddy (voir `AGENTS.md` section "Adding this app to the server").

### 5. Connexion plateforme (optionnel)

Le jeu peut se connecter à une plateforme externe pour l'authentification,
les scores, le leaderboard et les paramètres de jeu. Voir `API_SPEC.md`.

Pour activer la connexion plateforme, modifier la balise meta dans `web/index.html` :

```html
<meta name="platform-api-url" content="https://votre-plateforme.com">
```

Si vide ou absent, le jeu fonctionne en mode autonome (anonyme).

## Stack technique

| Composant | Technologie |
|-----------|-------------|
| Langage | Rust |
| Framework jeu | Macroquad 0.4 |
| Compilation | WASM (`wasm32-unknown-unknown`) |
| Hébergement | Fichiers statiques (Caddy/Hetzner) |
| CI/CD | GitHub Actions |
| API (optionnel) | Plateforme externe via REST |

## Commandes utiles

```bash
# Build natif
cargo run

# Build WASM
cargo build --release --target wasm32-unknown-unknown

# Vérifier les erreurs
cargo check

# Lancer les tests
cargo test
```

## Liens

- Serveur : `https://fartcloud.ton-domaine.com`
- API Spec : `API_SPEC.md`
- Déploiement : `AGENTS.md`
