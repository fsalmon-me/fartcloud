# Fartcloud 🌥️💨

Un jeu style Flappy Bird avec un nuage péteur.

## Setup du projet

### 1. Créer le site Firebase Hosting (sous-site)

```bash
firebase hosting:sites:create fartcloud --project myitproject-site
```

→ URL: **https://fartcloud.web.app**

### 2. Créer le repo GitHub (sur fsalmon-me)

```bash
cd fartcloud
git init
gh repo create fsalmon-me/fartcloud --public --source=. --push
```

### 3. Token Firebase pour CI

```bash
firebase login:ci
# Copier le token généré

gh secret set FIREBASE_TOKEN --body 'LE_TOKEN_GENERE'
```

### 4. Configuration Firebase

`.firebaserc`:
```json
{
  "projects": {
    "default": "myitproject-site"
  }
}
```

`firebase.json`:
```json
{
  "hosting": {
    "site": "fartcloud",
    "public": "dist",
    "ignore": ["firebase.json", "**/.*", "**/node_modules/**"]
  }
}
```

### 5. GitHub Actions

Créer `.github/workflows/deploy.yml` (adapter selon stack Rust/WASM ou JS)

---

## Stack recommandée

| Option | Pros | Cons |
|--------|------|------|
| **JS/Canvas + Vite** | Rapide à prototyper | Moins performant |
| **Rust/WASM (macroquad)** | Performant, apprentissage | Plus de setup |

## Commandes utiles

```bash
# Déployer manuellement
firebase deploy --only hosting:fartcloud --project myitproject-site

# Lister les sites
firebase hosting:sites:list --project myitproject-site
```

## Liens

- Firebase: https://console.firebase.google.com/project/myitproject-site
- Site principal: https://myitproject-site.web.app
- Ce jeu: https://fartcloud.web.app
