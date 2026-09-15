# Démarrage rapide CB (sdroxide — 11 m)

Un guide court et pratique pour mettre en route **sdroxide** sur la bande
**11 m (27 MHz)** : régler votre SDR ou votre poste CAT, utiliser les plans de
canaux par pays et suivre le trafic numérique WSJT-CB. Pour le détail de chaque
commande, voir [`USER_MANUAL.md`](USER_MANUAL.md) ; pour l'esprit de ce fork,
voir le [README](../README.md).

> Ceci est le fork CB/SWL de sdroxide. La bande 11 m et les bandes de
> radiodiffusion sont ajoutées par-dessus ; les bandes amateurs et tout le
> reste viennent d'upstream et sont inchangés.

*English: [cb-quickstart.en.md](cb-quickstart.en.md). Nederlands:
[cb-quickstart.nl.md](cb-quickstart.nl.md). Italiano:
[cb-quickstart.it.md](cb-quickstart.it.md). PDF :
[cb-quickstart.fr.pdf](cb-quickstart.fr.pdf).*

Les noms en gras comme **SETTINGS** et **Callsign** sont des boutons et des
champs tels qu'ils apparaissent à l'écran. `Settings > Radio` est un chemin de
menu.

---

## Ce qu'il vous faut

- **Un SDR ou un poste pris en charge.** Pour la 11 m, les choix habituels :
  - **RTL-SDR** (clé USB) — natif, pas besoin de SoapySDR.
  - **RX-888 / RX-888 Mk2** — natif ; le firmware est envoyé automatiquement au
    récepteur.
  - **Airspy HF+** (Dual / Discovery / Ranger) — natif, 0,5 kHz–31 MHz.
  - Également possible : HackRF, Airspy R2/Mini, SDRplay RSP, ELAD, PlutoSDR,
    ou un **poste CAT** (Icom/Yaesu/Xiegu) via port série + carte son, TCI,
    OpenHPSDR ou SoapySDR.
- **Une antenne 27 MHz** adaptée à votre récepteur.
- **sdroxide installé** (ci-dessous).

## Installation

- **Windows** — l'installeur (`.msi`) ou le `.zip` portable (qui contient
  `sdroxide.exe`) : voir la [page Releases](https://github.com/madmedicnl/sdroxide/releases/latest).
- **Linux** — l'**AppImage** (un seul fichier, `chmod +x` puis exécuter), le
  `.deb`, ou l'archive portable.
- **macOS** — le `.dmg`.

Vous pouvez aussi lancer sdroxide en **serveur** et l'ouvrir dans un
navigateur : `sdroxide --server`, puis `http://localhost:4950`. Pratique quand
l'antenne est ailleurs.

---

## Partie A — Réglage initial

Tout est enregistré sous `~/.config/sdroxide/`, donc à faire une seule fois.

### 1. Lancer sdroxide

La fenêtre principale a la barre de commandes en haut, et le panadapter et la
cascade en dessous.

### 2. Choisir le poste

Ouvrez **SETTINGS** puis l'onglet **Radio**. Choisissez l'interface (par
exemple **RTL-SDR**, **RX-888** ou **Airspy HF+**), puis la **fréquence
d'échantillonnage** et le **gain**. Les changements s'appliquent après
**Apply / reconnect**.

- **Linux — récepteurs USB :** installez les règles udev fournies, sinon
  sdroxide verra l'appareil mais ne pourra pas l'ouvrir :
  `sudo cp 60-sdroxide-*.rules /usr/lib/udev/rules.d/ && sudo udevadm control --reload`, puis rebranchez.
- **Windows — RX-888 :** liez l'appareil à **WinUSB** une fois avec
  [Zadig](https://zadig.akeo.ie/) — pour **les deux** identifiants USB
  (`04B4:00F3` et `04B4:00F1`).

### 3. Vos informations

Onglet **General** :

- **Callsign** — pour le CB/WSJT-CB, saisissez ici votre identifiant CB.
- **Locator / grid** — votre locator Maidenhead ; la carte et le décodage
  l'utilisent.
- **IARU region** — la région qui définit le plan de bandes.
- **CB plan** — le plan de canaux de votre pays : **World / freeband**,
  **CEPT/EU** (France, Belgique, Pays-Bas, …), **Germany 80 channels**,
  **UK 27/81**, **USA**, **Australia**. C'est lui qui détermine les canaux et
  le numéro de canal affiché sur la cascade.

---

## Partie B — Écouter la 11 m

1. Choisissez la bande **11 m** sur la barre de bandes. Elle va de **26,965 à
   27,860 MHz**.
2. Avec le plan **CEPT/EU**, **canal 1 = 26,965 MHz** et **canal 40 =
   27,405 MHz** (pas de 10 kHz). Le numéro de canal apparaît sur le
   panadapter.
3. Choisissez le **mode** : **AM** ou **FM** pour la voix, **USB/LSB** pour la
   BLU en freeband.
4. **Écoute seule ?** Activez **SWL mode** : toutes les commandes d'émission
   (PTT, TUNE, CALL CQ, …) disparaissent. Avec **Simple UI**, vous ne voyez
   que l'essentiel, avec **AM · FM · USB · LSB** en tête.

---

## Partie C — Numérique sur 27 MHz (WSJT-CB / famille FT8)

Ce fork parle le même échange WSJT haché que
[WSJT-CB](https://github.com/vash909/WSJT-CB) utilise sur 27 MHz.

1. Choisissez **FT8** (ou FT4/FT2).
2. Choisissez le canal numérique dans le plan / la liste de canaux de votre
   plan CB.
3. Dans la **liste de décodage** : cliquez une ligne pour amener votre audio
   sur ce signal, puis **REPLY** ou **Call CQ**.
4. Chaque station décodée affiche son **drapeau de pays** — sdroxide suit les
   conventions d'indicatif et la numérotation des pays de WSJT-CB.
5. En **SWL mode**, vous pouvez tout lire sans émettre.

---

## Partie D — Sauvegarder votre configuration (profils)

Sous **Settings > Profiles**, vous enregistrez toute une configuration sous un
nom et la remettez en un clic : fréquences et VFO, mode et filtres, gain,
puissance et antennes, l'identité numérique et les piles de bandes. Pratique
pour passer de « 11 m à la maison » à « écoute des ondes courtes ».

---

## Partie E — En plus

- **CW au clavier :** maintenez la **barre d'espace** — votre clavier sert de
  straight key.
- **Exporter la liste de décodage** en **CSV** ou en **ADIF**
  *received-report*.
- Le **client navigateur** peut **importer** des fichiers ADIF et **CHIRP**.
- **Alertes sonores** pour les appels et les nouveaux DXCC/grids.
- **Thèmes :** dix thèmes de couleurs supplémentaires.

---

## Remarque sur l'émission

- Hors des bandes amateurs, le **verrouillage d'émission** est actif par
  défaut. **La 11 m / CB n'est pas une bande amateur**, donc l'émission est
  bloquée sauf si vous levez le verrouillage avec `--oob-tx` pour la session.
- Le droit d'émettre en 11 m, et à quelle puissance/mode, varie selon le pays
  (en France et en Belgique, les canaux CEPT libres avec une puissance
  limitée). **Vérifiez la réglementation en vigueur — la responsabilité est la
  vôtre.**
- Ce fork est d'abord conçu pour **recevoir et décoder**.

---

## Aide et liens

- [README](../README.md) — l'aperçu complet du fork.
- [USER_MANUAL.md](USER_MANUAL.md) — le manuel, commande par commande.
- [WSJT-CB](https://github.com/vash909/WSJT-CB) — le projet numérique 11 m sur
  lequel ceci s'appuie.
- [Releases](https://github.com/madmedicnl/sdroxide/releases/latest) — la
  dernière version pour chaque plateforme.

À bientôt sur la 11 m ! 73
