# Privacy Policy

_Last updated: 7 November 2026_

sdroxide is free, open-source software (GPL-3.0-or-later). It contains **no
telemetry, no analytics, no crash reporting and no advertising**. It does not
collect information about you or your computer, and the sdroxide project runs
no server that receives any.

This page lists every case where sdroxide connects to the internet, so you can
see what leaves your computer and when.

## What happens without you doing anything

On a fresh install, sdroxide makes only these requests. Each is a plain
download. Apart from the standard request information any web client sends
(your IP address and a `User-Agent: sdroxide/<version>` header), none of them
sends anything about you, your station or your computer.

| Request | Purpose | How often | Turn it off |
|---|---|---|---|
| `https://sdroxide.com/script.js` | Checks whether a newer sdroxide release exists | Once per start | Settings → UI → update check |
| `https://www.hamqsl.com/solarxml.php` | Downloads the published HF band-conditions forecast | At most once an hour (cached) | Settings → UI → band-conditions forecast |
| `http://www.eibispace.de/dx/sked-<season>.csv` | Downloads the EiBi shortwave broadcast schedule (about 1 MB) | Once per broadcast season (cached) | Settings → UI → broadcast schedule |

With all three switched off, a fresh install makes no network requests at
all. The features below only connect once you set them up. Note that entering
your callsign is enough to start the Reverse Beacon Network feed, which is on
by default.

## Features that send data, only once you set them up

Some features are about sharing with the amateur radio community. Each one
sends data **directly to the service it names**, never via the sdroxide
project, and only after you have entered your callsign or account details or
switched the feature on.

| Feature | Service | What is sent | Default |
|---|---|---|---|
| Reverse Beacon Network spots | `telnet.reversebeacon.net` | Your callsign, as the login | On, but nothing connects until you set a callsign |
| WSPR spot upload | wsprnet.org | Your callsign, grid locator, dial frequency and the WSPR signals you decoded | On, but only while decoding WSPR with a callsign and grid set |
| PSK Reporter reception reports | pskreporter.info | Your callsign, grid locator, antenna description (optional), and the stations you decoded | Off |
| FreeDV Reporter | qso.freedv.org | Your callsign, grid locator, frequency, transmit state, status message, and callsigns you decoded | Off |
| DX cluster | The cluster node you choose | Your callsign and any commands you configure | Off |
| Callsign lookup | QRZ.com or HamQTH | Your account credentials and the callsign being looked up | Off |
| Logbook upload / sync | LoTW, eQSL, QRZ Logbook, Club Log, HamQTH, World Radio League, Wavelog | Your account credentials or API key, and the QSOs you choose to upload | Off |
| Winlink | Winlink CMS | Your callsign, password and the messages you send | Only when you use it |

## Data that is only downloaded

These fetch public data on request and send nothing beyond the standard
request information described above:

- POTA and SOTA spot feeds (api.pota.app, api-db2.sota.org.uk), and PSK
  Reporter spot retrieval. All off by default.
- Space weather and solar imagery (NOAA SWPC, NASA SDO, NASA CCMC, KC2G),
  fetched while the solar view is open.
- Satellite orbital elements (CelesTrak or the sources you add), fetched for
  the satellite tracker.
- Public SDR directories (e.g. KiwiSDR and sdr-list.xyz listings), fetched
  when you browse for a public receiver.

## Radios and receivers you connect to

When you use a network radio or receiver (KiwiSDR, SpyServer, rtl_tcp, TCI,
Icom LAN, FlexRadio, HPSDR, PlutoSDR, flrig, rigctld, and so on), sdroxide
talks to the address you entered. A public KiwiSDR receiver shows its owner a
user name for you, which is your callsign unless you set a different one. To find HPSDR and PlutoSDR radios,
sdroxide lists your computer's network adapters and searches the local network.
That information stays on your computer.

sdroxide's own remote-operation server and TCI server only run if you start
them, and they listen only on the addresses you configure.

## Data stored on your computer

Settings, logbooks, recordings, decoded messages and cached downloads are
stored only on your computer, in your user profile (on Windows under
`%APPDATA%\sdroxide`, on Linux under `~/.config/sdroxide` and
`~/.local/share/sdroxide`). Account passwords and API keys you enter are
stored there **in plain text**, so protect that folder as you would any
other file containing passwords. Uninstalling sdroxide leaves this folder in
place; delete it to remove everything.

## Third-party services

When you use a feature that talks to a third-party service, that service's
own privacy policy applies to what it receives. The sdroxide project does
not control or receive that data.

## Changes and contact

Changes to this policy are tracked in this file's history in the
[sdroxide repository](https://github.com/dividebysandwich/sdroxide).
Questions can be raised as an issue there.
