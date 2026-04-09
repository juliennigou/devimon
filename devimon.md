# 🐾 Terminal Pet — MVP Vision

## 🎯 Vision

Créer un **compagnon vivant dans le terminal** qui évolue en fonction de l’activité réelle du développeur.

> Ton monstre grandit pendant que tu codes.

Le jeu doit être :

* léger
* non intrusif
* émotionnellement attachant
* aligné avec les sessions de travail réelles

---

## 🧠 Concept central

Contrairement à un Tamagotchi classique :

👉 Le monstre **ne progresse pas grâce aux actions du jeu**
👉 Il progresse grâce à **l’activité réelle du développeur**

---

## ⚙️ Core Gameplay Loop

### Boucle principale

1. Le joueur spawn un monstre
2. Il commence à travailler normalement
3. Le jeu détecte :

   * session active
   * modifications de fichiers
4. Le monstre gagne de l’XP
5. Les besoins du monstre diminuent lentement
6. Le joueur doit s’en occuper
7. Le monstre monte de niveau et évolue

---

## 🧩 Système de progression

### 🎯 Source principale d’XP

Le monstre progresse grâce à :

* ⏱️ **Temps de session active**
* 📝 **Modifications de fichiers**
* 📂 activité dans le projet (édition, création, suppression)

---

## 🤖 Rôle de l’IA (important)

Le jeu **n’a pas besoin de détecter directement l’IA**.

👉 Il détecte le **résultat de l’utilisation de l’IA** :

* modifications fréquentes
* itérations rapides
* flux de travail actif

💡 Donc :

> Plus tu utilises l’IA pour coder → plus tu modifies → plus ton monstre évolue

---

## 📊 Système d’XP (MVP)

Exemple simple :

* fichier modifié → +1 XP
* plusieurs fichiers → bonus
* activité régulière → bonus
* inactivité → rien

⚠️ Important :

* XP plafonnée par minute
* anti-spam (éviter triche)

---

## ❤️ Besoins du monstre (MVP)

Seulement 3 :

* 🍗 **Faim**
* ⚡ **Énergie**
* 😊 **Moral**

---

## 🎮 Actions du joueur

Commandes simples :

* `pet status`
* `pet feed`
* `pet play`
* `pet rest`

---

## 🔄 Dynamique gameplay

### Travail réel → progression

* XP augmente
* moral peut augmenter

### Négligence → ralentissement

* faim ↓
* énergie ↓
* moral ↓

Conséquence :

* progression ralentie
* pas de punition brutale

---

## 🧬 Évolution

3 stades :

1. Bébé
2. Jeune
3. Évolué

### Dépend de :

* niveau
* état général
* style de session

---

## 🎭 Personnalité du monstre

Le monstre réagit :

* “Session productive !”
* “Tu sembles fatigué…”
* “Il est fier de toi”

---

## 🧘 Philosophie du jeu

* pas stressant
* pas punitif
* accompagne le travail
* récompense la régularité

---

## 🚫 Hors scope MVP

* combats
* multijoueur
* inventaire
* plusieurs monstres
* stats complexes

---

## 🚀 Roadmap

### V1 (MVP)

* 1 monstre
* tracking fichiers
* 3 besoins
* XP simple
* évolution

### V2

* UI terminal (ratatui)
* espèces différentes
* personnalité

### V3

* PvP
* profils joueurs
* leaderboard

---

## 🧠 Résumé

> Un compagnon de terminal qui grandit grâce à ton travail réel et crée un lien émotionnel avec toi.

---

## 🔥 Core Insight

Le jeu ne récompense pas le joueur pour jouer.

👉 Il récompense le joueur pour travailler.

Et transforme son travail en progression ludique.
