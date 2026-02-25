---
sidebar_position: 1
---

# Installation avec Docker

Ygégé est disponible sous forme d'image Docker officielle multi-architecture. Ce guide explique comment déployer et configurer le service.

## Prérequis

- [Docker](https://docs.docker.com/get-docker/) installé sur votre système
- [Docker Compose](https://docs.docker.com/compose/install/) (recommandé pour une gestion simplifiée)
- Un compte YGG Torrent valide

## Installation rapide

### Avec Docker Run

```bash
docker run -d \
  --name ygege \
  -p 9876:9876 \
  -v ./config:/config \
  -e YGG_USERNAME="votre_nom_utilisateur" \
  -e YGG_PASSWORD="votre_mot_de_passe" \
  uwucode/ygege:latest
```

### Avec Docker Compose

Créez un fichier `compose.yml`:

```yaml
services:
  ygege:
    image: uwucode/ygege:latest
    container_name: ygege
    restart: unless-stopped
    ports:
      - "9876:9876"
    volumes:
      - ygege_sessions:/app/sessions           # Volume nommé (recommandé)
    environment:
      YGG_USERNAME: "votre_nom_utilisateur"
      YGG_PASSWORD: "votre_mot_de_passe"
      LOG_LEVEL: "debug"
    healthcheck:
      test: ["CMD-SHELL", "curl --fail http://localhost:$${BIND_PORT:-9876}/health || exit 1"]
      interval: 1m30s
      timeout: 20s
      retries: 3
      start_period: 10s

volumes:
  ygege_sessions:
    driver: local
```

Puis démarrez le service:

```bash
docker compose up -d
```

## Configuration

### Avec fichier config.json

Créez un fichier `config/config.json`:

```json
{
    "username": "votre_nom_utilisateur_ygg",
    "password": "votre_mot_de_passe",
    "bind_ip": "0.0.0.0",
    "bind_port": 9876,
    "log_level": "debug"
}
```

### Avec variables d'environnement

Les variables suivantes sont supportées:

| Variable | Description | Défaut |
|----------|-------------|--------|
| `YGG_USERNAME` | Nom d'utilisateur YGG | - |
| `YGG_PASSWORD` | Mot de passe YGG | - |
| `BIND_IP` | Adresse IP d'écoute | `0.0.0.0` |
| `BIND_PORT` | Port d'écoute | `9876` |
| `LOG_LEVEL` | Niveau de log (trace, debug, info, warn, error) | `info` |
| `TMDB_TOKEN` | Token API TMDB (optionnel) | - |
| `YGG_DOMAIN` | Domaine YGG personnalisé (optionnel) | - |
| `TURBO_ENABLED` | Activer le mode turbo (true/false) | `false` |

## Tags Docker disponibles

| Tag | Description |
|-----|-------------|
| `latest` | Dernière version stable |
| `stable` | Alias de latest |
| `noupx` | Version sans compression UPX (pour Synology) |
| `0.6.2` | Version spécifique |
| `develop` | Version de développement |

### Pour les systèmes avec architectures anciennes

Si vous rencontrez des erreurs de segmentation (segfault) sur des architectures anciennes ou certains NAS (comme Synology), utilisez l'image `noupx`:

```yaml
services:
  ygege:
    image: uwucode/ygege:noupx
    # ... reste de la configuration
```

## Vérification

Une fois le conteneur démarré, vérifiez qu'il fonctionne:

```bash
curl http://localhost:9876/health
```

Vous devriez recevoir une réponse `OK`.

## Sécurité

### Utilisateur non-root

L'image Docker Ygégé s'exécute par défaut avec un utilisateur non-root (UID 10001) pour des raisons de sécurité. Cela garantit:

- ✅ Compatibilité avec les politiques de sécurité Docker et Kubernetes
- ✅ Protection contre les escalades de privilèges
- ✅ Conformité aux meilleures pratiques de sécurité des conteneurs

### Gestion des permissions

:::warning Erreur "Permission denied"
Si vous obtenez `Error: Os { code: 13, kind: PermissionDenied }` après mise à jour, c'est lié aux permissions des volumes.
:::

**Solution recommandée**: Utilisez des **volumes nommés** (déjà dans l'exemple ci-dessus):

```yaml
volumes:
  - ygege_sessions:/app/sessions  # Gestion automatique des permissions
```

**Alternative avec bind mounts**: Si vous devez monter un dossier local:

```yaml
services:
  ygege:
    image: uwucode/ygege:latest
    user: "10001:10001"  # UID/GID du container
    volumes:
      - ./ygege/sessions:/app/sessions
```

Puis définissez les permissions:

**Linux/macOS**:
```bash
sudo chown -R 10001:10001 ./ygege/sessions
sudo chmod -R 755 ./ygege/sessions
```

**Windows** (PowerShell en administrateur):
```powershell
icacls ".\ygege\sessions" /grant Everyone:(OI)(CI)F /T
```

### Exécution avec un UID personnalisé

Si vous souhaitez exécuter le conteneur avec un UID/GID spécifique (par exemple pour correspondre à votre utilisateur hôte):

```bash
docker run -d \
  --name ygege \
  --user 1000:1000 \
  -p 9876:9876 \
  -v ./config:/app/sessions \
  -v ./config.json:/app/config.json \
  uwucode/ygege:latest
```

Ou avec Docker Compose:

```yaml
services:
  ygege:
    image: uwucode/ygege:latest
    user: "1000:1000"  # Votre UID:GID
    # ... reste de la configuration
```

:::tip
Assurez-vous que les volumes montés ont les permissions appropriées pour l'utilisateur spécifié:
```bash
sudo chown -R 1000:1000 ./config ./sessions
```
:::

### Exécution en root (non recommandé)

:::danger Avertissement de sécurité
L'exécution en root n'est **pas recommandée** et peut présenter des risques de sécurité. Utilisez cette option uniquement si vous comprenez les implications.
:::

Si vous devez absolument exécuter le conteneur en root:

**Docker Run**:
```bash
docker run -d \
  --name ygege \
  --user 0:0 \
  -p 9876:9876 \
  -v ./ygege/sessions:/app/sessions \
  -e YGG_USERNAME="votre_nom" \
  -e YGG_PASSWORD="votre_mdp" \
  uwucode/ygege:latest
```

**Docker Compose**:
```yaml
services:
  ygege:
    image: uwucode/ygege:latest
    container_name: ygege
    user: "0:0"  # Root
    restart: unless-stopped
    environment:
      YGG_USERNAME: "votre_nom_utilisateur"
      YGG_PASSWORD: "votre_mot_de_passe"
    volumes:
      - ./ygege/sessions:/app/sessions
    ports:
      - "9876:9876"
```

Avec cette configuration, vous n'aurez plus de problèmes de permissions, mais vous perdez les avantages de sécurité du mode non-root.

## Prochaines étapes

- [Configuration avancée](../configuration)
- [Intégration avec Prowlarr](../integrations/prowlarr)
- [Intégration avec Jackett](../integrations/jackett)
