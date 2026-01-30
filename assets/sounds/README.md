# 🔊 FartCloud Sound System

## Vue d'ensemble

Ce dossier contient les fichiers audio du jeu. Le système charge automatiquement les sons `.ogg` s'ils existent.
Si un fichier est manquant, le jeu continue sans ce son (pas d'erreur).

## Fichiers requis

| Fichier | Description | Durée | Volume par défaut | Priorité |
|---------|-------------|-------|-------------------|----------|
| `fart_1.ogg` | Pet court classique | 0.3-0.5s | 0.8 | Non |
| `fart_2.ogg` | Pet variant (aigu) | 0.3-0.5s | 0.8 | Non |
| `fart_3.ogg` | Pet variant (grave) | 0.3-0.5s | 0.8 | Non |
| `mega_fart.ogg` | Gros pet combo (≥5) | 0.8-1.2s | 1.0 | ✅ Oui |
| `splat.ogg` | Écrasement au sol | 0.5-0.8s | 1.0 | ✅ Oui |
| `boom.ogg` | Explosion dans l'espace | 0.5-1.0s | 1.0 | ✅ Oui |
| `combo_up.ogg` | Montée de combo | 0.2-0.3s | 0.6 | Non |
| `alert_beep.ogg` | Bip d'alerte danger | 0.1-0.15s | 0.4 | Non |
| `game_over.ogg` | Fin de partie | 1.0-1.5s | 1.0 | ✅ Oui |

## Comment créer les sons

### Conversion en OGG avec FFmpeg

```bash
# Installer ffmpeg (Windows)
winget install ffmpeg

# Convertir un WAV en OGG (qualité standard)
ffmpeg -i mon_son.wav -c:a libvorbis -q:a 6 mon_son.ogg

# Batch convert tous les WAV
Get-ChildItem *.wav | ForEach-Object { ffmpeg -i $_.Name -c:a libvorbis -q:a 6 ($_.BaseName + ".ogg") }

# Qualité plus basse (fichiers plus petits, pour effets courts)
ffmpeg -i mon_son.wav -c:a libvorbis -q:a 3 mon_son.ogg
```

### Qualité recommandée

- `-q:a 3-4` : Pour les sons courts (pets, beeps) 
- `-q:a 5-6` : Pour les sons moyens (splat, boom)
- `-q:a 7-8` : Pour les sons longs (game over, musique)

## Où trouver des sons gratuits

- [freesound.org](https://freesound.org) - Chercher "fart", "whoosh", "splat", "explosion"
- [pixabay.com/sound-effects](https://pixabay.com/sound-effects/)
- [zapsplat.com](https://zapsplat.com)

## Système de ducking (priorité)

Les sons marqués "Priorité" font baisser le volume des autres sons à 50% pendant leur lecture.
Cela permet d'entendre clairement les événements importants (mort, gros combo).

## Activer les sons

1. Créer les fichiers `.ogg` avec les noms exacts ci-dessus
2. Les placer dans ce dossier (`assets/sounds/`)
3. Relancer le jeu - les sons seront automatiquement détectés et chargés

## Contrôles volume en jeu

- `M` : Mute/Unmute toggle
- Volume global configurable dans `config.json` : `"master_volume": 1.0`
