#!/usr/bin/env python3
"""Render the QO-100 quick-start guide's UI-panel diagrams and PDFs.

The guide (`docs/qo100-quickstart.{en,tr}.md`) illustrates each step with a
small drawn mock-up of the relevant sdroxide panel — not a screenshot, so it
never goes stale when a control moves, and not ASCII art, so it reads well in
the rendered PDF. This script is *both* halves of that pipeline, driven
entirely off the Markdown, so the two are never out of sync:

  1. `--images` (re)draws the panel mock-ups as PNGs under
     `docs/images/qo100-quickstart/`, styled from the live palette in
     `crates/sdroxide-ui/src/theme.rs` (DEFAULT theme) and the app's own
     Chakra Petch / Share Tech Mono fonts. The Markdown embeds them with plain
     `![]()` image syntax, so they also render on GitHub.
  2. `--pdf` turns each Markdown file (images and all) into the themed A4 PDF
     next to it — `docs/qo100-quickstart.en.pdf` / `.tr.pdf`.

Run with no flags to do both. Needs `pip install markdown pillow` and a local
Chrome or Chromium (headless `--print-to-pdf` / `--screenshot`); pass
`--chrome` if it isn't found automatically.

    ./tools/gen_qo100_quickstart_pdf.py

Run it after editing either Markdown file or a diagram in this script, and
commit what it writes (the PNGs and the PDFs both).
"""

import argparse
import base64
import pathlib
import shutil
import subprocess
import sys
import tempfile

ROOT = pathlib.Path(__file__).resolve().parent.parent
FONTS = ROOT / "crates/sdroxide-ui/assets/fonts"
IMAGES = ROOT / "docs/images/qo100-quickstart"
DOCS = ROOT / "docs"

CHROME_CANDIDATES = [
    "google-chrome-stable", "google-chrome", "chromium", "chromium-browser",
    "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
    "/Applications/Chromium.app/Contents/MacOS/Chromium",
]


def find_chrome(explicit):
    if explicit:
        return explicit
    for c in CHROME_CANDIDATES:
        if shutil.which(c) or pathlib.Path(c).exists():
            return c
    sys.exit("No Chrome/Chromium found — pass --chrome /path/to/it")


def b64(p):
    return base64.b64encode(p.read_bytes()).decode()


# ---- palette (crates/sdroxide-ui/src/theme.rs, DEFAULT theme) — keep in sync ----
BG_DEEP = "#050810"
PANEL = "#0b111e"
INPUT_BG = "#04070e"
FILL = "#101a2c"
FILL_ACTIVE = "#1d2f4d"
LINE = "#1a2740"
LINE_LIT = "#2a4a66"
TEXT = "#b4c6da"
TEXT_STRONG = "#e8f4ff"
CYAN = "#00d0f4"
CYAN_DIM = "#1d9cbe"
PINK = "#ff2a55"
YELLOW = "#ffd23f"
GREEN = "#46e07d"
INK_ON_CYAN = "#021019"


def fonts_css():
    chakra_reg = b64(FONTS / "ChakraPetch-Regular.ttf")
    chakra_semi = b64(FONTS / "ChakraPetch-SemiBold.ttf")
    share_mono = b64(FONTS / "ShareTechMono-Regular.ttf")
    return f"""
@font-face {{ font-family:'Chakra'; src:url(data:font/ttf;base64,{chakra_reg}) format('truetype'); font-weight:400; }}
@font-face {{ font-family:'Chakra'; src:url(data:font/ttf;base64,{chakra_semi}) format('truetype'); font-weight:700; }}
@font-face {{ font-family:'ShareMono'; src:url(data:font/ttf;base64,{share_mono}) format('truetype'); }}
"""


PANEL_CSS = f"""
* {{ box-sizing:border-box; }}
.panel-mockup {{ font-family:'Chakra',sans-serif; color:{TEXT}; }}
.panel-mockup .panel {{
  position:relative; background:{PANEL}; border:2px solid {PINK};
  clip-path: polygon(0 0, calc(100% - 18px) 0, 100% 18px, 100% 100%, 18px 100%, 0 calc(100% - 18px));
}}
.panel-mockup .titlebar {{
  display:flex; align-items:center; gap:10px; padding:8px 16px;
  border-bottom:1px solid {LINE}; font-weight:700; font-size:14px;
  color:{TEXT_STRONG}; letter-spacing:.03em;
}}
.panel-mockup .titlebar .path {{ color:{CYAN_DIM}; font-weight:400; }}
.panel-mockup .body {{ padding:14px 18px; }}
.panel-mockup .tabs {{ display:flex; gap:4px; padding:8px 14px 0; }}
.panel-mockup .tab {{
  padding:6px 14px; font-size:12px; font-weight:700; letter-spacing:.03em;
  background:{FILL}; color:{TEXT}; border:1px solid {LINE};
  clip-path: polygon(0 0, calc(100% - 8px) 0, 100% 8px, 100% 100%, 0 100%);
}}
.panel-mockup .tab.active {{ background:{CYAN}; color:{INK_ON_CYAN}; border-color:{CYAN}; }}
.panel-mockup .row {{ display:flex; align-items:center; gap:10px; padding:7px 0; }}
.panel-mockup .label {{ width:98px; flex:0 0 auto; font-size:12.5px; color:{TEXT}; }}
.panel-mockup .field {{
  background:{INPUT_BG}; border:1px solid {LINE_LIT}; color:{TEXT_STRONG};
  padding:5px 10px; font-size:12.5px; font-family:'ShareMono',monospace; min-width:60px;
}}
.panel-mockup .field.grow {{ flex:1 1 auto; }}
.panel-mockup .field.select {{ display:flex; justify-content:space-between; align-items:center; }}
.panel-mockup .field.select::after {{ content:'\\25BE'; color:{CYAN}; font-family:'Chakra'; margin-left:8px; }}
.panel-mockup .hint {{ font-size:11px; color:{CYAN_DIM}; }}
.panel-mockup .btn {{
  background:{FILL}; border:1px solid {LINE_LIT}; color:{CYAN}; font-weight:700;
  font-size:12px; letter-spacing:.03em; padding:6px 14px;
  clip-path: polygon(0 0, calc(100% - 8px) 0, 100% 8px, 100% 100%, 0 100%);
}}
.panel-mockup .btn.on {{ background:{CYAN}; color:{INK_ON_CYAN}; border-color:{CYAN}; }}
.panel-mockup .btn.primary {{ background:{FILL_ACTIVE}; color:{TEXT_STRONG}; border-color:{CYAN_DIM}; }}
.panel-mockup .chk {{ display:flex; align-items:center; gap:6px; font-size:12.5px; }}
.panel-mockup .chk .box {{ width:14px; height:14px; border:1px solid {LINE_LIT}; background:{INPUT_BG}; position:relative; }}
.panel-mockup .chk .box.on::after {{ content:''; position:absolute; inset:2px; background:{CYAN}; }}
.panel-mockup .divider {{ border-top:1px dashed {LINE}; margin:6px 0; }}
.panel-mockup .badge {{
  display:inline-flex; align-items:center; justify-content:center;
  width:18px; height:18px; border-radius:50%; background:{PINK};
  color:#fff; font-family:'ShareMono',monospace; font-size:11px; font-weight:700; flex:0 0 auto;
}}
.panel-mockup .list {{ border:1px solid {LINE}; background:{INPUT_BG}; margin:6px 0; }}
.panel-mockup .list .item {{ padding:6px 10px; font-size:12.5px; display:flex; justify-content:space-between; border-bottom:1px solid {LINE}; }}
.panel-mockup .list .item:last-child {{ border-bottom:none; }}
.panel-mockup .list .item.sel {{ background:{FILL_ACTIVE}; color:{TEXT_STRONG}; }}
.panel-mockup .list .item .tag {{ color:{CYAN_DIM}; font-size:11px; }}
.panel-mockup .mono {{ font-family:'ShareMono',monospace; }}
.panel-mockup .readout {{ display:flex; justify-content:space-between; padding:4px 0; font-size:12.5px; }}
.panel-mockup .readout .k {{ color:{TEXT}; }}
.panel-mockup .readout .v {{ font-family:'ShareMono',monospace; color:{TEXT_STRONG}; }}
.panel-mockup .chip {{
  padding:6px 12px; font-size:11.5px; font-weight:700; letter-spacing:.03em;
  background:{FILL}; color:{TEXT}; border:1px solid {LINE};
}}
.panel-mockup .chip.active {{ background:{CYAN_DIM}; color:{INK_ON_CYAN}; border-color:{CYAN}; }}
.panel-mockup .arrow {{ display:flex; align-items:center; justify-content:center; color:{CYAN}; font-size:20px; padding:0 6px; }}
.panel-mockup .caption {{ font-size:11.5px; color:{TEXT}; font-style:italic; }}
.panel-mockup .mini-wf {{
  border:1px solid {LINE_LIT}; height:64px; position:relative; overflow:hidden; margin:8px 0;
  background: repeating-linear-gradient(90deg, #071322 0 10px, #081726 10px 20px);
}}
.panel-mockup .mini-wf .park {{
  position:absolute; left:0; top:0; bottom:0; width:38%; border-right:1px dashed {LINE_LIT};
  background:repeating-linear-gradient(45deg, #0d1c30 0 4px, #0a1526 4px 8px);
}}
.panel-mockup .mini-wf .target {{ position:absolute; left:19%; top:0; bottom:0; width:1px; background:{YELLOW}; }}
.panel-mockup .mini-wf .lobes {{
  position:absolute; left:14%; top:8px; bottom:8px; width:12%;
  display:flex; align-items:center; justify-content:space-between;
}}
.panel-mockup .mini-wf .lobe {{ width:5px; height:100%; background:{GREEN}; opacity:.85; }}
"""


def run_chrome(chrome, args):
    subprocess.run([chrome, "--headless", "--disable-gpu", *args],
                    stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=True)


def badge(n):
    return f'<span class="badge">{n}</span>'


def shoot(chrome, tmp, name, inner_html, scale=2, canvas=(1500, 1200), pad=20):
    """Render one panel mock-up on a generous canvas, then autocrop it."""
    from PIL import Image, ImageChops

    html = f"""<!doctype html><html><head><meta charset="utf-8"><style>
{fonts_css()}{PANEL_CSS}
html,body {{ margin:0; padding:40px; background:{BG_DEEP}; display:inline-block; }}
</style></head><body class="panel-mockup">{inner_html}</body></html>"""
    hpath = tmp / f"{name}.html"
    ppath = IMAGES / f"{name}.png"
    hpath.write_text(html, encoding="utf-8")
    run_chrome(chrome, [
        f"--force-device-scale-factor={scale}",
        f"--window-size={canvas[0]},{canvas[1]}",
        f"--screenshot={ppath}",
        f"file://{hpath}",
    ])
    im = Image.open(ppath).convert("RGB")
    bbox = ImageChops.difference(im, Image.new("RGB", im.size, BG_DEEP)).getbbox()
    if bbox:
        p = pad * scale
        l, t, r, b = bbox
        im.crop((max(0, l - p), max(0, t - p), min(im.width, r + p), min(im.height, b + p))).save(ppath)
    print("wrote", ppath.relative_to(ROOT))


# --------------------------------------------------------------------------
# The 7 panel mock-ups. 1 and 2 read the same in both languages; the rest
# carry a caption that is translated in the guide's diagram too.
# --------------------------------------------------------------------------

def diagram_top_bar(chrome, tmp):
    chips = ["BAND", "MODE", "FILTER", "AGC"]
    html = f"""
    <div class="panel" style="width:640px">
      <div class="body" style="padding:12px 16px;">
        <div class="row" style="gap:8px;">
          {''.join(f'<span class="chip">{c}</span>' for c in chips)}
          <span class="chip active">SETTINGS</span>{badge(2)}
          <span class="chip">SAT</span>
          <span class="chip">SCAN</span>
        </div>
      </div>
    </div>"""
    shoot(chrome, tmp, "01-top-bar", html)


def diagram_settings_general(chrome, tmp):
    html = f"""
    <div class="panel" style="width:460px">
      <div class="titlebar">Settings</div>
      <div class="tabs">
        <span class="tab active">General</span>
        <span class="tab">Radio</span>
        <span class="tab">Servers</span>
        <span class="tab">TLE</span>
      </div>
      <div class="body">
        <div class="row"><div class="label">Callsign</div><div class="field grow">TA1XYZ</div>{badge(3)}</div>
        <div class="row"><div class="label">Locator</div><div class="field grow">KN41GG</div>{badge(3)}</div>
      </div>
    </div>"""
    shoot(chrome, tmp, "02-settings-general", html)


def diagram_settings_radio(chrome, tmp, lang):
    offset_hint = "from preset, auto" if lang == "en" else "ön ayardan otomatik"
    html = f"""
    <div class="panel" style="width:560px">
      <div class="titlebar">Settings</div>
      <div class="tabs">
        <span class="tab">General</span>
        <span class="tab active">Radio</span>
        <span class="tab">Servers</span>
      </div>
      <div class="body">
        <div class="row"><div class="label">Interface</div><div class="field select grow">PlutoSDR</div>{badge(4)}</div>
        <div class="row"><div class="label">Converter</div><div class="field select grow">LNB, Ku low (-9750 MHz)</div>{badge(5)}</div>
        <div class="row"><div class="label">Offset</div><div class="field grow">-9 750 000 000 Hz</div><span class="hint">({offset_hint})</span></div>
        <div class="row">
          <div class="label">Transmit</div>
          <div class="field select" style="width:150px">Its own offset</div>
          <div class="field" style="width:60px">0</div>
          <span class="hint">Hz</span>{badge(6)}
        </div>
        <div class="divider"></div>
        <div class="row"><div class="label">Address</div><div class="field grow">192.168.2.1</div><span class="btn">Discover</span>{badge(7)}</div>
        <div class="row">
          <div class="label">Sample rate</div>
          <div class="field select" style="width:110px">1 Msps</div>
          <div class="chk"><span class="box on"></span>Full duplex</div>{badge(8)}
        </div>
        <div class="row" style="justify-content:flex-end;"><span class="btn primary">Apply</span>{badge(9)}</div>
      </div>
    </div>"""
    shoot(chrome, tmp, f"03-settings-radio.{lang}", html)


def diagram_sat_window_open(chrome, tmp, lang):
    caption = "SAT glows green while locked." if lang == "en" else "Kilit varken SAT yeşil yanar."
    html = f"""
    <div style="display:flex; align-items:center;">
      <div class="panel" style="width:110px">
        <div class="titlebar" style="font-size:11px;">System</div>
        <div class="body" style="display:flex; flex-direction:column; gap:6px; padding:10px;">
          <span class="chip active" style="text-align:center;">SAT</span>
          <span class="chip" style="text-align:center;">SCAN</span>
          <span class="chip" style="text-align:center;">MEM</span>
        </div>
      </div>
      <div class="arrow">&#8594;{badge(10)}</div>
      <div class="panel" style="width:420px">
        <div class="titlebar">
          <span class="tab active" style="font-size:11px;">SATELLITES</span>
          <span class="tab" style="font-size:11px;">QO-100</span>
        </div>
        <div class="body"><span class="caption">{caption}</span></div>
      </div>
    </div>"""
    shoot(chrome, tmp, f"04-sat-window-open.{lang}", html)


def diagram_sat_satellites(chrome, tmp, lang):
    search_label = "search:" if lang == "en" else "arama:"
    html = f"""
    <div class="panel" style="width:520px">
      <div class="titlebar">SAT <span class="path">SATELLITES</span></div>
      <div class="body">
        <div class="row"><div class="label">{search_label}</div><div class="field grow">QO-100</div></div>
        <div class="list">
          <div class="item sel">&gt; QO-100 (Es'hail-2) {badge(11)}<span class="tag">geostationary</span></div>
          <div class="item">&nbsp;&nbsp;ISS (ZARYA)</div>
          <div class="item">&nbsp;&nbsp;RS-44</div>
        </div>
        <div class="row"><span class="caption">NB transponder &mdash; beacon 10489.750 MHz</span></div>
        <div class="row" style="justify-content:flex-end; gap:10px;">
          <span class="btn">Tune</span>
          <span class="btn primary">Lock on</span>{badge(12)}
        </div>
      </div>
    </div>"""
    shoot(chrome, tmp, f"05-sat-satellites.{lang}", html)


def diagram_sat_qo100_tracker(chrome, tmp, lang):
    if lang == "en":
        wf_caption, target_caption = "mini waterfall — hatched = park lane", "target = 10489.750 MHz"
        lobe_caption, dbl_click = "beacon, two lobes", "double-click the beacon's centre to mark it"
    else:
        wf_caption, target_caption = "mini şelale — taralı = park lane", "hedef = 10489.750 MHz"
        lobe_caption, dbl_click = "beacon, iki lob", "beacon'ın ortasına çift tıklayıp işaretleyin"
    html = f"""
    <div class="panel" style="width:680px">
      <div class="titlebar">
        <span class="tab" style="font-size:11px;">SATELLITES</span>
        <span class="tab active" style="font-size:11px;">QO-100 *</span>
      </div>
      <div class="body">
        <div class="row" style="justify-content:space-between;">
          <div class="row" style="gap:8px;">
            <span class="btn on">ON</span>{badge(17)}
            <span class="btn">TELEMETRY</span>{badge(18)}
            <span class="btn on">AUTO</span>{badge(17)}
          </div>
          <div class="row" style="gap:6px;">
            <span class="caption">width</span>
            <span class="btn" style="padding:4px 9px;">-</span>
            <span class="mono">&plusmn;25 kHz</span>
            <span class="btn" style="padding:4px 9px;">+</span>{badge(14)}
          </div>
        </div>
        <div class="mini-wf">
          <div class="park"></div>
          <div class="target"></div>
          <div class="lobes"><span class="lobe"></span><span class="lobe"></span></div>
        </div>
        <div class="row" style="justify-content:space-between;">
          <span class="caption">{wf_caption}</span>
          <span class="caption">{target_caption} &middot; {lobe_caption}</span>
        </div>
        <div class="row"><span class="caption">{dbl_click}</span>{badge(16)}</div>
        <div class="divider"></div>
        <div class="readout"><span class="k">TRACKER</span><span class="v">+1.2 kHz &nbsp;(null 12 dB &middot; snr 15 dB)</span></div>
        <div class="readout"><span class="k">CONVERTER OFFSET</span><span class="v">-9 749 920 000 Hz</span></div>
        <div class="readout"><span class="k">MEASURED</span><span class="v">10489.751200 MHz &nbsp;DRIFT +1.2 kHz</span></div>
        <div class="row" style="justify-content:flex-end;"><span class="btn primary">Apply correction</span>{badge(16)}</div>
      </div>
    </div>"""
    shoot(chrome, tmp, f"06-sat-qo100-tracker.{lang}", html)


def diagram_main_window_mini(chrome, tmp, lang):
    last_label = "last" if lang == "en" else "son"
    caption = ("keep the panel open in a corner — it keeps correcting with the window closed"
               if lang == "en" else
               "paneli bir köşede açık bırakın — pencere kapalıyken de düzeltmeyi sürdürür")
    html = f"""
    <div class="panel" style="width:640px">
      <div class="titlebar">sdroxide <span class="path">main window</span></div>
      <div class="body" style="position:relative; height:140px;">
        <div style="position:absolute; inset:14px; border:1px dashed {LINE}; display:flex;
                    align-items:center; justify-content:center; color:{CYAN_DIM}; font-size:11px;">
          panadapter / waterfall
        </div>
        <div class="panel" style="position:absolute; right:8px; top:8px; width:190px;">
          <div class="titlebar" style="padding:5px 10px; font-size:10.5px;">SAT <span class="path">QO-100 *</span></div>
          <div class="body" style="padding:8px 10px;">
            <div class="row" style="gap:8px;">
              <span class="btn on" style="font-size:10px;">AUTO</span>
              <span class="caption">{last_label} -80 Hz</span>
            </div>
          </div>
        </div>
      </div>
      <div class="body" style="padding-top:0;"><span class="caption">{caption}</span></div>
    </div>"""
    shoot(chrome, tmp, f"07-main-window-mini-panel.{lang}", html)


def build_images(chrome):
    IMAGES.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory() as td:
        tmp = pathlib.Path(td)
        diagram_top_bar(chrome, tmp)
        diagram_settings_general(chrome, tmp)
        for lang in ("en", "tr"):
            diagram_settings_radio(chrome, tmp, lang)
            diagram_sat_window_open(chrome, tmp, lang)
            diagram_sat_satellites(chrome, tmp, lang)
            diagram_sat_qo100_tracker(chrome, tmp, lang)
            diagram_main_window_mini(chrome, tmp, lang)


# --------------------------------------------------------------------------
# PDF: the same Markdown, straight through python-markdown, themed and
# paginated for A4, printed by headless Chrome.
# --------------------------------------------------------------------------

DOC_CSS = f"""
@page {{ size: A4; margin: 18mm 16mm; }}
body {{
  font-family:'Chakra',sans-serif; color:{TEXT_STRONG}; background:{BG_DEEP};
  font-size:11.5px; line-height:1.55; margin:0;
}}
h1,h2,h3 {{ color:{TEXT_STRONG}; font-weight:700; letter-spacing:.02em; }}
h1 {{ font-size:22px; border-bottom:2px solid {PINK}; padding-bottom:8px; }}
h2 {{ font-size:16px; margin-top:26px; border-bottom:1px solid {LINE}; padding-bottom:5px; }}
h3 {{ font-size:13px; color:{CYAN}; margin-top:18px; }}
a {{ color:{CYAN}; }}
code {{ font-family:'ShareMono',monospace; background:{INPUT_BG}; color:{TEXT_STRONG}; padding:1px 5px; }}
pre {{ font-family:'ShareMono',monospace; background:{INPUT_BG}; color:{TEXT_STRONG};
       border:1px solid {LINE}; padding:10px 14px; font-size:11px; white-space:pre-wrap; }}
blockquote {{
  border-left:3px solid {CYAN_DIM}; margin:14px 0; padding:2px 14px; color:{TEXT};
  background:{PANEL};
}}
hr {{ border:none; border-top:1px solid {LINE}; margin:22px 0; }}
li {{ margin:3px 0; }}
img {{ max-width:100%; display:block; margin:10px 0 4px; page-break-inside:avoid; }}
strong {{ color:{TEXT_STRONG}; }}
"""


def build_pdf(chrome, lang):
    import markdown

    md_path = DOCS / f"qo100-quickstart.{lang}.md"
    md_text = md_path.read_text(encoding="utf-8")
    body = markdown.markdown(md_text, extensions=["extra", "sane_lists"])
    # The Markdown's image paths are relative (so they also render on
    # GitHub); rewrite them to absolute file:// URIs so headless Chrome can
    # load them regardless of where the rendered HTML ends up on disk.
    body = body.replace('src="images/', f'src="file://{DOCS / "images"}/')
    html = f"""<!doctype html><html><head><meta charset="utf-8"><style>
{fonts_css()}{DOC_CSS}
</style></head><body>{body}</body></html>"""
    with tempfile.TemporaryDirectory() as td:
        hpath = pathlib.Path(td) / "doc.html"
        hpath.write_text(html, encoding="utf-8")
        ppath = DOCS / f"qo100-quickstart.{lang}.pdf"
        run_chrome(chrome, [
            "--no-pdf-header-footer",
            f"--print-to-pdf={ppath}",
            "--print-to-pdf-no-header",
            f"file://{hpath}",
        ])
    print("wrote", ppath.relative_to(ROOT))


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--images", action="store_true", help="only regenerate the diagram PNGs")
    ap.add_argument("--pdf", action="store_true", help="only regenerate the PDFs")
    ap.add_argument("--chrome", help="path to Chrome/Chromium (auto-detected otherwise)")
    args = ap.parse_args()
    chrome = find_chrome(args.chrome)

    do_images = args.images or not (args.images or args.pdf)
    do_pdf = args.pdf or not (args.images or args.pdf)

    if do_images:
        build_images(chrome)
    if do_pdf:
        for lang in ("en", "tr"):
            build_pdf(chrome, lang)


if __name__ == "__main__":
    main()
