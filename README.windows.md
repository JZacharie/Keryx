# 🏁 Keryx - Guide d'exécution sous Windows (WSL2 + GPU)

Ce guide explique comment lancer et tester le pipeline complet de traitement Keryx sur votre poste de travail Windows avec accélération GPU Nvidia.

---

## 📋 Prérequis

Pour exécuter les services IA avec accélération matérielle GPU sur Windows, vous devez installer et configurer les éléments suivants :

1. **Pilote GPU NVIDIA** : Assurez-vous d'avoir les pilotes NVIDIA officiels récents installés sur votre Windows hôte.
2. **WSL 2 (Windows Subsystem for Linux)** : 
   - Installez WSL2 depuis Powershell en administrateur : `wsl --install`
   - Redémarrez la machine.
3. **Docker Desktop** :
   - Installez Docker Desktop pour Windows.
   - Dans les paramètres de Docker Desktop, sous **General**, assurez-vous que **Use the WSL 2 based engine** est coché.
   - Sous **Resources > WSL Integration**, activez l'intégration pour votre distribution par défaut (ex: `Ubuntu`).
4. **NVIDIA Container Toolkit** :
   - Docker Desktop intègre désormais directement le support GPU pour WSL 2 sans étape supplémentaire complexe. Vérifiez la disponibilité GPU avec la commande Powershell :
     ```bash
     docker run --rm --gpus all nvidia/cuda:12.0.0-base-ubuntu22.04 nvidia-smi
     ```

---

## 🚀 Lancement du Pipeline Local

Un fichier de configuration dédié `docker-compose.windows.yaml` a été créé pour orchestrer tous les services en mode local avec MinIO (S3 local) et Redis.

### 1. Démarrer les services

Exécutez la commande suivante dans votre terminal (dans le dossier `Keryx`) :

```bash
docker compose -f docker-compose.windows.yaml up --build
```

Cette commande va :
- Lancer un serveur **MinIO** local (port `9000` pour l'API, `9001` pour la console).
- Créer automatiquement les buckets S3 requis (`keryx` et `keryx-cache`) grâce au conteneur `create-buckets`.
- Lancer le serveur **Redis** interne.
- Démarrer l'**orchestrateur** Rust et tous les **microservices de traitement** (Whisper, Diffusion Engine, Voice Cloners, Dewatermark, etc.) connectés au GPU.
- Démarrer l'interface **Frontend** accessible sur `http://localhost:8000`.

---

## 🦙 Gestion d'Ollama (Traduction & Raffinement)

Pour la traduction (Phase 4), les conteneurs tentent de se connecter à Ollama via `http://host.docker.internal:11434`.

### Option A : Ollama installé nativement sur Windows (Recommandé 🚀)
Faire tourner Ollama sur l'hôte Windows permet de libérer de la mémoire VRAM pour Docker et offre de meilleures performances.
1. Téléchargez et installez [Ollama pour Windows](https://ollama.com/).
2. Lancez Ollama et téléchargez le modèle requis :
   ```cmd
   ollama run llama3
   ```
3. Docker communiquera automatiquement avec Ollama Windows via la passerelle `host.docker.internal`.

### Option B : Lancer Ollama dans Docker
Si vous préférez encapsuler Ollama dans votre environnement Docker, vous pouvez ajouter ce service dans votre fichier compose ou configurer la variable d'environnement `OLLAMA_URL` vers un conteneur Ollama externe.

---

## 🧪 Tester le bon fonctionnement (E2E)

Une fois tous les conteneurs démarrés, vous pouvez soumettre une requête de test depuis l'interface web à l'adresse `http://localhost:8000` ou exécuter le script de validation d'intégration :

```bash
python scripts/test_e2e.py
```

Vous pouvez suivre l'état des buckets et des fichiers générés en vous connectant à l'interface MinIO :
- **URL** : `http://localhost:9001`
- **Login** : `minioadmin`
- **Mot de passe** : `minioadmin`
