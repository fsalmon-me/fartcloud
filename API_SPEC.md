# FartCloud — Platform API Specification

Ce document décrit l'API que la plateforme doit exposer pour communiquer avec le jeu FartCloud.
Le jeu fonctionne en **mode autonome** (anonyme) si aucune plateforme n'est configurée,
et en **mode connecté** lorsqu'un `PLATFORM_API_URL` est défini et qu'un token est fourni.

---

## Sommaire

1. [Architecture](#architecture)
2. [Authentification](#authentification)
3. [Endpoints](#endpoints)
   - [GET /api/game/auth/validate](#get-apigameauthvalidate)
   - [POST /api/game/auth/login](#post-apigameauthlogin)
   - [GET /api/game/config](#get-apigameconfig)
   - [POST /api/game/scores](#post-apigamescores)
   - [GET /api/game/leaderboard](#get-apigameleaderboard)
4. [Schémas de données](#schémas-de-données)
5. [Codes d'erreur](#codes-derreur)
6. [Modes de fonctionnement](#modes-de-fonctionnement)

---

## Architecture

```
┌─────────────────┐         ┌──────────────────┐
│   Navigateur    │         │    Plateforme    │
│                 │         │      (API)       │
│  ┌───────────┐  │  HTTP   │                  │
│  │  FartCloud │──┼────────►  /api/game/*     │
│  │   (WASM)  │◄─┼────────┤                  │
│  └───────────┘  │  JSON   │  Base de données │
│                 │         │  Auth, Scores,   │
│  JS Bridge      │         │  Config, etc.    │
│  (fetch API)    │         │                  │
└─────────────────┘         └──────────────────┘
```

- Le jeu est un fichier WASM statique servi par Caddy (Hetzner).
- Les appels API partent du **navigateur du joueur** vers la plateforme.
- Le JS bridge dans `index.html` fait les appels `fetch()` et expose les résultats au WASM via `extern "C"`.
- En mode anonyme, aucun appel API n'est effectué.

---

## Authentification

### Méthodes d'authentification

Le jeu supporte 3 modes :

| Mode | Déclencheur | Comportement |
|------|------------|--------------|
| **Anonyme** | Pas de `PLATFORM_API_URL` ou pas de token | Jeu complet, scores en mémoire de session uniquement |
| **Token URL** | `?token=xxx` dans l'URL | La plateforme redirige vers le jeu avec un token pré-authentifié |
| **Login** | Formulaire dans le jeu (placeholder) | Le jeu envoie username/password, reçoit un token |

### Header d'authentification

Tous les endpoints protégés nécessitent :

```
Authorization: Bearer <token>
Content-Type: application/json
```

### Flux d'authentification

```
1. Token URL:
   Plateforme → redirect → https://fartcloud.domain.com/?token=ABC123
   Jeu → GET /api/game/auth/validate (header: Bearer ABC123)
   Plateforme → { "valid": true, "username": "PlayerOne" }

2. Login (placeholder):
   Jeu → POST /api/game/auth/login { "username": "x", "password": "y" }
   Plateforme → { "token": "ABC123", "username": "PlayerOne" }

3. Anonyme:
   Pas d'appel API. Jeu jouable, scores en sessionStorage.
```

---

## Endpoints

### GET /api/game/auth/validate

Valide un token d'authentification et retourne les infos utilisateur.

**Request:**
```http
GET /api/game/auth/validate
Authorization: Bearer <token>
```

**Response 200:**
```json
{
  "valid": true,
  "username": "PlayerOne",
  "user_id": "usr_abc123"
}
```

**Response 401:**
```json
{
  "valid": false,
  "error": "Token expired or invalid"
}
```

---

### POST /api/game/auth/login

Authentifie un utilisateur par username/password. **(Placeholder — pas encore implémenté côté plateforme)**

**Request:**
```http
POST /api/game/auth/login
Content-Type: application/json
```

```json
{
  "username": "PlayerOne",
  "password": "secret123"
}
```

**Response 200:**
```json
{
  "token": "eyJhbGciOi...",
  "username": "PlayerOne",
  "user_id": "usr_abc123"
}
```

**Response 401:**
```json
{
  "error": "Invalid credentials"
}
```

---

### GET /api/game/config

Retourne les paramètres de jeu définis par la plateforme. Ces paramètres **surchargent** (override partiel) la config locale `config.json`. Seuls les champs présents dans la réponse sont appliqués.

**Request:**
```http
GET /api/game/config
Authorization: Bearer <token>
```

> Note : Cet endpoint peut aussi être appelé sans token pour obtenir une config publique.

**Response 200:**
```json
{
  "gravity_base": 600.0,
  "fart_power_base": 380.0,
  "player_size": 28.0,
  "cloud_speed_initial": 180.0,
  "cloud_speed_increment": 6.0,
  "spawn_interval_initial": 2.0,
  "spawn_interval_min": 0.8,
  "difficulty_increase_every": 10,
  "particle_count": 15,
  "particle_lifetime": 0.7,
  "shake_intensity": 12.0,
  "shake_decay": 0.88,
  "world_height": 2700.0,
  "camera_lerp": 0.08,
  "master_volume": 1.0,
  "sfx_volume": 0.8,
  "speed_transition_level": 10,
  "speed_slow_growth": 0.01,
  "cloud_density_factor": 0.008,
  "cloud_level_increment": 3,
  "spawn_interval_decay": 0.08,
  "hardcore_factor": 1.0
}
```

> **Override partiel** : La plateforme peut ne retourner qu'un sous-ensemble des champs.
> Exemple, pour forcer le mode hardcore :
> ```json
> { "hardcore_factor": 2.0 }
> ```
> Les champs non présents gardent leur valeur locale (`config.json`).

**Champs disponibles :**

| Champ | Type | Défaut | Description |
|-------|------|--------|-------------|
| `gravity_base` | f32 | 600.0 | Gravité de base (px/s²) |
| `fart_power_base` | f32 | 380.0 | Puissance de base du pet |
| `player_size` | f32 | 28.0 | Rayon du nuage joueur |
| `cloud_speed_initial` | f32 | 180.0 | Vitesse initiale des obstacles |
| `cloud_speed_increment` | f32 | 6.0 | Gain de vitesse par niveau |
| `spawn_interval_initial` | f32 | 2.0 | Intervalle initial entre spawns (s) |
| `spawn_interval_min` | f32 | 0.8 | Intervalle minimum entre spawns (s) |
| `difficulty_increase_every` | u32 | 10 | Secondes entre chaque montée de niveau |
| `particle_count` | u32 | 15 | Nombre de particules par pet |
| `particle_lifetime` | f32 | 0.7 | Durée de vie des particules (s) |
| `shake_intensity` | f32 | 12.0 | Intensité du tremblement d'écran |
| `shake_decay` | f32 | 0.88 | Décroissance du tremblement |
| `world_height` | f32 | 2700.0 | Hauteur verticale du monde |
| `camera_lerp` | f32 | 0.08 | Fluidité du suivi caméra |
| `master_volume` | f32 | 1.0 | Volume master audio |
| `sfx_volume` | f32 | 0.8 | Volume effets sonores |
| `speed_transition_level` | u32 | 10 | Niveau où la croissance de vitesse ralentit |
| `speed_slow_growth` | f32 | 0.01 | Taux de croissance après transition |
| `cloud_density_factor` | f32 | 0.008 | Facteur de densité des obstacles |
| `cloud_level_increment` | u32 | 3 | +1 obstacle minimum tous les N niveaux |
| `spawn_interval_decay` | f32 | 0.08 | Vitesse de décroissance de l'intervalle |
| `hardcore_factor` | f32 | 1.0 | Multiplicateur de difficulté (1.0=normal, 2.0=extrême) |

---

### POST /api/game/scores

Soumet le score d'une partie terminée. Nécessite un token valide.

**Request:**
```http
POST /api/game/scores
Authorization: Bearer <token>
Content-Type: application/json
```

```json
{
  "username": "PlayerOne",
  "score": 1523,
  "difficulty_level": 8,
  "combo_max": 5,
  "fart_count": 42,
  "duration_seconds": 87.5,
  "death_type": "cloud",
  "altitude_zone": "sky"
}
```

**Champs du payload :**

| Champ | Type | Requis | Description |
|-------|------|--------|-------------|
| `username` | string | oui | Pseudo du joueur |
| `score` | u32 | oui | Score total de la partie |
| `difficulty_level` | u32 | oui | Niveau de difficulté atteint |
| `combo_max` | u32 | oui | Combo maximum atteint durant la partie |
| `fart_count` | u32 | oui | Nombre total de pets |
| `duration_seconds` | f32 | oui | Durée de la partie en secondes |
| `death_type` | string | oui | Type de mort : `"splat"`, `"explode"`, `"cloud"` |
| `altitude_zone` | string | oui | Zone d'altitude à la mort : `"space"`, `"high_sky"`, `"sky"`, `"ground"` |

**Response 201:**
```json
{
  "success": true,
  "rank": 12,
  "personal_best": false
}
```

**Response 401:**
```json
{
  "error": "Authentication required"
}
```

---

### GET /api/game/leaderboard

Retourne le classement des meilleurs scores.

**Request:**
```http
GET /api/game/leaderboard
Authorization: Bearer <token>
```

> Note : Peut être appelé sans token pour un classement public.

**Query parameters (optionnels) :**

| Param | Type | Défaut | Description |
|-------|------|--------|-------------|
| `limit` | u32 | 10 | Nombre maximum d'entrées retournées |
| `offset` | u32 | 0 | Décalage pour la pagination |

**Response 200:**
```json
{
  "entries": [
    {
      "rank": 1,
      "username": "SpaceFarter",
      "score": 2500,
      "difficulty_level": 12,
      "date": "2026-02-25T14:30:00Z"
    },
    {
      "rank": 2,
      "username": "CloudKing",
      "score": 1800,
      "difficulty_level": 9,
      "date": "2026-02-24T10:15:00Z"
    }
  ],
  "total": 156,
  "player_rank": 12,
  "player_best": 1523
}
```

**Champs de chaque entrée :**

| Champ | Type | Description |
|-------|------|-------------|
| `rank` | u32 | Position dans le classement |
| `username` | string | Pseudo du joueur |
| `score` | u32 | Score |
| `difficulty_level` | u32 | Niveau de difficulté atteint |
| `date` | string (ISO 8601) | Date de la partie |

**Champs du wrapper :**

| Champ | Type | Description |
|-------|------|-------------|
| `entries` | array | Liste des entrées du classement |
| `total` | u32 | Nombre total de scores enregistrés |
| `player_rank` | u32 \| null | Rang du joueur connecté (null si anonyme) |
| `player_best` | u32 \| null | Meilleur score du joueur connecté (null si anonyme) |

---

## Schémas de données

### GameConfig (override partiel)

Le jeu charge toujours sa config locale `assets/config.json` au démarrage.
Si connecté à la plateforme, un appel à `GET /api/game/config` retourne un objet JSON partiel.
Seuls les champs présents dans la réponse écrasent les valeurs locales.

```
Config finale = config.json ← merge(platform_config)
```

### LeaderboardEntry

```json
{
  "rank": 1,
  "username": "string",
  "score": 0,
  "difficulty_level": 0,
  "date": "2026-01-01T00:00:00Z"
}
```

### ScoreSubmission

```json
{
  "username": "string",
  "score": 0,
  "difficulty_level": 0,
  "combo_max": 0,
  "fart_count": 0,
  "duration_seconds": 0.0,
  "death_type": "cloud",
  "altitude_zone": "sky"
}
```

### AuthResult

```json
{
  "token": "string",
  "username": "string",
  "user_id": "string"
}
```

---

## Codes d'erreur

| Code HTTP | Signification | Quand |
|-----------|--------------|-------|
| 200 | OK | Requête réussie |
| 201 | Created | Score soumis avec succès |
| 400 | Bad Request | Payload invalide (champs manquants, types incorrects) |
| 401 | Unauthorized | Token absent, expiré ou invalide |
| 403 | Forbidden | Token valide mais accès refusé |
| 404 | Not Found | Endpoint inexistant |
| 429 | Too Many Requests | Rate limiting (anti-triche) |
| 500 | Internal Server Error | Erreur serveur |

### Format d'erreur standard

```json
{
  "error": "Description lisible de l'erreur",
  "code": "ERROR_CODE"
}
```

Codes d'erreur possibles :

| Code | Description |
|------|-------------|
| `INVALID_TOKEN` | Token invalide ou expiré |
| `MISSING_TOKEN` | Header Authorization absent |
| `INVALID_PAYLOAD` | Corps de requête invalide |
| `RATE_LIMITED` | Trop de requêtes |
| `SCORE_REJECTED` | Score rejeté (anti-triche) |
| `SERVER_ERROR` | Erreur interne |

---

## Modes de fonctionnement

### Mode Anonyme (sans plateforme)

- `PLATFORM_API_URL` n'est pas défini ou vide
- Le jeu est 100% jouable
- Les scores sont conservés en `sessionStorage` (perdus à la fermeture de l'onglet)
- Pas de leaderboard
- Le menu affiche "Anonyme" comme pseudo
- Un bouton "Connexion" (placeholder) est visible mais désactivé

### Mode Connecté (avec plateforme)

- `PLATFORM_API_URL` est défini
- Un token est fourni (URL `?token=xxx` ou login)
- Le token est validé au démarrage via `GET /api/game/auth/validate`
- La config plateforme est chargée via `GET /api/game/config` et mergée
- Les scores sont envoyés à la plateforme via `POST /api/game/scores`
- Le leaderboard est récupéré via `GET /api/game/leaderboard`
- Le menu affiche le pseudo de la plateforme
- Le high score affiché est `player_best` du leaderboard

### Résumé des flux par mode

| Action | Anonyme | Connecté |
|--------|---------|----------|
| Démarrage | Charge config.json | Charge config.json + merge API config |
| Pseudo | "Anonyme" ou saisi localement | Depuis la plateforme |
| Score affiché | Session uniquement | High score plateforme |
| Fin de partie | Score en sessionStorage | POST /api/game/scores |
| Leaderboard | "Connectez-vous" | GET /api/game/leaderboard |
| High score | sessionStorage | player_best de l'API |

---

## CORS

La plateforme doit autoriser les requêtes cross-origin depuis le domaine du jeu :

```
Access-Control-Allow-Origin: https://fartcloud.ton-domaine.com
Access-Control-Allow-Headers: Authorization, Content-Type
Access-Control-Allow-Methods: GET, POST, OPTIONS
```

---

## Notes d'implémentation pour la plateforme

1. **Anti-triche** : Les scores sont envoyés côté client — la plateforme devrait valider la cohérence (score vs durée vs difficulté) et appliquer un rate limiting.
2. **Config override** : L'endpoint config peut servir à créer des "événements" (ex: mode hardcore temporaire pour tous les joueurs).
3. **Token** : JWT recommandé avec expiration. Le jeu ne gère pas le refresh — si le token expire, le joueur repasse en mode anonyme.
4. **Leaderboard** : Le champ `player_rank` dans la réponse permet au jeu de positionner le joueur dans le classement sans avoir à chercher dans la liste.
