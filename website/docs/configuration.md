---
sidebar_position: 5
sidebar_label: Configuration
---

# Configuration

Ce guide détaille toutes les options de configuration disponibles pour Ygégé.

## Fichier config.json

Le fichier de configuration principal est `config.json`. Il doit être placé dans le dossier `/config` (Docker) ou à la racine du projet (installation manuelle).

### Structure complète

```json
{
    "username": "votre_nom_utilisateur_ygg",
    "password": "votre_mot_de_passe",
    "bind_ip": "0.0.0.0",
    "bind_port": 8715,
    "log_level": "debug",
    "tmdb_token": null,
    "ygg_domain": null,
    "turbo_enabled": null,
    "anti_bot_provider": "flaresolverr",
    "flaresolverr_url": "http://flaresolverr:8191"
}
```

## Options disponibles

### Authentification YGG

| Paramètre | Type | Requis | Description |
|-----------|------|--------|-------------|
| `username` | string | ✅ | Nom d'utilisateur YGG Torrent |
| `password` | string | ✅ | Mot de passe YGG Torrent |

:::warning Attention
YGG Torrent est un tracker privé. Des identifiants valides sont **obligatoires** pour que Ygégé puisse se connecter.
:::

### Configuration réseau

| Paramètre | Type | Défaut | Description |
|-----------|------|--------|-------------|
| `bind_ip` | string | `0.0.0.0` | Adresse IP d'écoute |
| `bind_port` | number | `8715` | Port d'écoute du serveur |

:::tip Personnaliser le port
Pour éviter les conflits de ports (ex: sur Windows), changez simplement `BIND_PORT` :
```yaml
environment:
  BIND_PORT: "3000"  # Utilise le port 3000 au lieu de 8715
ports:
  - "3000:3000"
```
Le healthcheck s'adapte automatiquement grâce à `$${BIND_PORT:-8715}`.
:::

### Logging

| Paramètre | Type | Défaut | Description |
|-----------|------|--------|-------------|
| `log_level` | string | `info` | Niveau de verbosité des logs |

Niveaux disponibles:
- `trace` : Maximum de détails (développement)
- `debug` : Informations de débogage
- `info` : Informations générales
- `warn` : Avertissements uniquement
- `error` : Erreurs uniquement

### Métadonnées TMDB/IMDB

| Paramètre | Type | Défaut | Description |
|-----------|------|--------|-------------|
| `tmdb_token` | string | `null` | Token API TMDB (optionnel) |

:::info
Lorsque `tmdb_token` est configuré, les résolveurs **TMDB et IMDB** sont automatiquement activés ensemble.
:::

Pour configurer TMDB/IMDB, consultez le [guide d'intégration TMDB/IMDB](./tmdb-imdb).

### Configuration du domaine YGG

| Paramètre | Type | Défaut | Description |
|-----------|------|--------|-------------|
| `ygg_domain` | string | `null` | Domaine YGG personnalisé (optionnel) |
| `anti_bot_provider` | string | `flaresolverr` | Provider anti-bot (`flaresolverr` ou `native`) |
| `flaresolverr_url` | string | `http://flaresolverr:8191` | URL du service Flaresolverr |

:::tip Quand utiliser YGG_DOMAIN ?
Par défaut, Ygégé détecte automatiquement le domaine YGG actuel via une redirection depuis `ygg.re`. Si cette auto-détection échoue (erreurs 307, "Session expired..."), vous pouvez spécifier manuellement le domaine :
```
YGG_DOMAIN=www.yggtorrent.org
```
:::

## Variables d'environnement

Toutes les options peuvent également être définies via des variables d'environnement:

| Variable | Équivalent config.json |
|----------|------------------------|
| `YGG_USERNAME` | `username` |
| `YGG_PASSWORD` | `password` |
| `BIND_IP` | `bind_ip` |
| `BIND_PORT` | `bind_port` |
| `LOG_LEVEL` | `log_level` |
| `TMDB_TOKEN` | `tmdb_token` |
| `YGG_DOMAIN` | `ygg_domain` |
| `TURBO_ENABLED` | `turbo_enabled` |
| `ANTI_BOT_PROVIDER` | `anti_bot_provider` |
| `FLARESOLVERR_URL` | `flaresolverr_url` |


:::tip Priorité
Les variables d'environnement ont **priorité** sur le fichier config.json.
:::

## Exemple de configuration complète

### Pour Docker Compose

```yaml
services:
  flaresolverr:
    image: ghcr.io/flaresolverr/flaresolverr:latest
    container_name: flaresolverr
    restart: unless-stopped
    ports:
      - "8191:8191"
    healthcheck:
      test: ["CMD-SHELL", "curl --fail http://localhost:8191/ || exit 1"]
      interval: 30s
      timeout: 10s
      retries: 5

  ygege:
    image: uwucode/ygege:latest
    container_name: ygege
    restart: unless-stopped
    depends_on:
      flaresolverr:
        condition: service_healthy
    ports:
      - "8715:8715"
    volumes:
      - ./config:/config
    environment:
      YGG_USERNAME: "mon_username"
      YGG_PASSWORD: "mon_password"
      LOG_LEVEL: "debug"
      TMDB_TOKEN: "votre_token_tmdb"
      ANTI_BOT_PROVIDER: "flaresolverr"
      FLARESOLVERR_URL: "http://flaresolverr:8191"
      # YGG_DOMAIN: "www.yggtorrent.org"  # Optionnel : forcer un domaine spécifique
      # TURBO_ENABLED: true  # Optionnel : Si turbo est actif, pas besoin d'attendre 30 secondes entre le token et le torrent
```

### Pour fichier config.json

```json
{
    "username": "mon_username",
    "password": "mon_password",
    "bind_ip": "0.0.0.0",
    "bind_port": 8715,
    "log_level": "debug",
    "tmdb_token": "votre_token_tmdb",
    "ygg_domain": null,
    "turbo_enabled": true,
    "anti_bot_provider": "flaresolverr",
    "flaresolverr_url": "http://flaresolverr:8191"
}
```


## Dépannage Flaresolverr

### Erreur de connexion à Flaresolverr
- Vérifiez que le service est démarré: `curl http://localhost:8191/`
- Vérifiez que `FLARESOLVERR_URL` cible la bonne URL (`http://flaresolverr:8191` en Docker Compose).
- Vérifiez que les deux services sont sur le même réseau Docker.

### Ygégé démarre mais les recherches échouent
- Consultez les logs Ygégé: `docker logs ygege`
- Consultez les logs Flaresolverr: `docker logs flaresolverr`
- En dernier recours, testez le fallback natif: `ANTI_BOT_PROVIDER=native`

### Timeouts ou réponses Cloudflare intermittentes
- Relancez Flaresolverr (`docker restart flaresolverr`).
- Réduisez la charge en limitant les requêtes parallèles côté clients (Prowlarr/Jackett).
- Vérifiez que l'horloge système (hôte + conteneurs) est correcte.

## Validation de la configuration

Pour vérifier que votre configuration est correcte, consultez les logs au démarrage:

```bash
docker logs ygege
```

Vous devriez voir:
```
[INFO] Configuration chargée avec succès
[INFO] Connexion à YGG Torrent...
[INFO] Authentification réussie
[INFO] Serveur démarré sur 0.0.0.0:8715
```

## Prochaines étapes

- [API Documentation](./api)
- [Intégration Prowlarr](./integrations/prowlarr)
