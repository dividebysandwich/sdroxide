#!/usr/bin/env bash
# Generate golden message-layer vectors by compiling JS8Call's own varicode.
#
# The JS8 message layer is too large and too arbitrary to transcribe on faith —
# base-37 callsign packing with per-country workarounds, a 15-bit grid encoding,
# a command table with reduced-fidelity SNR packing, and a Huffman free-text
# codec. So rather than reimplement and hope, we link the reference
# implementation and diff against it.
#
#   ./tools/gen_js8_varicode_vectors.sh ~/Development/js8call \
#       > crates/sdroxide-digi/tests/js8_varicode_vectors/mod.rs
#
# Needs Qt6 development files and a C++17 compiler. Only ever run when the
# upstream commit in vendor/js8call/PROVENANCE.md changes; the output is
# committed so an ordinary build needs neither.
set -euo pipefail

SRC="${1:?usage: $0 <path-to-js8call-checkout>}"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

MOC="$(pkg-config --variable=libexecdir Qt6Core 2>/dev/null || true)/moc"
[ -x "$MOC" ] || MOC=/usr/lib/qt6/moc

cat > "$WORK/gen.cpp" <<'CPP'
#include "varicode.h"
#include <QCoreApplication>
#include <QStringList>
#include <cstdio>

// Callsigns chosen to exercise the odd corners of the base-37 packing: two- to
// six-character calls, a numeric prefix, the Swaziland (3DA0) and Guinea (3X)
// special cases, and the reserved group names.
static const char *CALLS[] = {
    "KN4CRD", "N0JDS", "VK3ABC", "G0ABC", "JA1XYZ", "W1AW", "M0ABC", "9A1A",
    "3DA0XX", "3XY1AB", "OH2BH", "ZL4AA", "PY2ABC", "LU1DZ", "K7A", "VE3XYZ",
    "@ALLCALL", "@JS8NET", "@DX/EU", "@REGION/1", "@GROUP/0", "@NTS", "@APRSIS",
    "@EMCOMM", "@AMRRON", "@QRO", nullptr,
};

static const char *GRIDS[] = {
    "EM73", "QF22", "IO91", "FN42", "JJ00", "AA00", "RR99", "JO31",
    "PM95", "GF15", "BP51", nullptr,
};

// One line per directed message: the text as an operator would type it after
// their own callsign, plus the callsign sending it.
static const char *DIRECTED[][2] = {
    {"KN4CRD", "N0JDS SNR?"},        {"KN4CRD", "N0JDS GRID?"},
    {"KN4CRD", "N0JDS STATUS?"},     {"KN4CRD", "@ALLCALL HEARING?"},
    {"KN4CRD", "N0JDS SNR -05"},     {"KN4CRD", "N0JDS SNR +15"},
    {"KN4CRD", "VK3ABC QSL"},        {"KN4CRD", "VK3ABC QSL?"},
    {"KN4CRD", "N0JDS RR"},          {"KN4CRD", "N0JDS SK"},
    {"KN4CRD", "N0JDS FB"},          {"KN4CRD", "N0JDS HW CPY?"},
    {"KN4CRD", "@ALLCALL CQ CQ CQ"}, {"KN4CRD", "N0JDS INFO?"},
    {"VK3ABC", "KN4CRD GRID?"},      {"G0ABC", "@JS8NET STATUS?"},
    {nullptr, nullptr},
};

static const char *HEARTBEATS[][2] = {
    {"KN4CRD", "@HB HEARTBEAT EM73"}, {"VK3ABC", "@HB HEARTBEAT QF22"},
    {"N0JDS", "@HB HEARTBEAT FN42"},  {"G0ABC", "@HB HEARTBEAT IO91"},
    {nullptr, nullptr},
};

int main(int argc, char **argv) {
    QCoreApplication app(argc, argv);
    printf("//! JS8 message-layer golden vectors — **generated, do not edit**.\n");
    printf("//!\n");
    printf("//! Emitted by `tools/gen_js8_varicode_vectors.sh` from JS8Call's own\n");
    printf("//! `varicode.cpp`, linked and executed. Expected values therefore come from\n");
    printf("//! the reference implementation rather than from a second reading of it.\n");
    printf("//! See `vendor/js8call/PROVENANCE.md` for the commit.\n");
    printf("//!\n");
    printf("//! In a subdirectory because cargo compiles every `.rs` directly under\n");
    printf("//! `tests/` as its own test binary, and this one has no tests in it.\n\n");

    printf("/// `(callsign, packed 28-bit value)`.\n");
    printf("pub const CALLSIGNS: &[(&str, u32)] = &[\n");
    for (int i = 0; CALLS[i]; ++i) {
        bool portable = false;
        quint32 p = Varicode::packCallsign(CALLS[i], &portable);
        printf("    (\"%s\", %u),\n", CALLS[i], p);
    }
    printf("];\n\n");

    printf("/// `(grid, packed 15-bit value)`.\n");
    printf("pub const GRIDS: &[(&str, u16)] = &[\n");
    for (int i = 0; GRIDS[i]; ++i)
        printf("    (\"%s\", %u),\n", GRIDS[i], Varicode::packGrid(GRIDS[i]));
    printf("];\n\n");

    printf("/// `(from, text, parsed to, parsed cmd, parsed num, frame chars)`.\n");
    printf("pub const DIRECTED: &[(&str, &str, &str, &str, &str, &str)] = &[\n");
    for (int i = 0; DIRECTED[i][0]; ++i) {
        QString to, cmd, num;
        bool toCompound = false;
        int n = 0;
        QString f = Varicode::packDirectedMessage(DIRECTED[i][1], DIRECTED[i][0],
                                                  &to, &toCompound, &cmd, &num, &n);
        if (f.isEmpty()) continue;
        printf("    (\"%s\", \"%s\", \"%s\", \"%s\", \"%s\", \"%s\"),\n",
               DIRECTED[i][0], DIRECTED[i][1], qPrintable(to),
               qPrintable(cmd), qPrintable(num), qPrintable(f));
    }
    printf("];\n\n");

    printf("/// `(from, text, frame chars)`.\n");
    printf("pub const HEARTBEATS: &[(&str, &str, &str)] = &[\n");
    for (int i = 0; HEARTBEATS[i][0]; ++i) {
        int n = 0;
        QString f = Varicode::packHeartbeatMessage(HEARTBEATS[i][1], HEARTBEATS[i][0], &n);
        if (f.isEmpty()) continue;
        printf("    (\"%s\", \"%s\", \"%s\"),\n", HEARTBEATS[i][0], HEARTBEATS[i][1],
               qPrintable(f));
    }
    printf("];\n\n");

    // Whole messages as `buildMessageFrames` lays them out. Chosen for the
    // compound cases (`varicode.cpp:2100`, cases 1-3): a custom group or a
    // slashed callsign as the addressee, and a compound callsign as the
    // sender. Each needs a compound frame pair where an ordinary message has
    // one directed frame. A reserved group (@DX/NA) is the control case,
    // because it still fits a plain directed frame.
    static const char *MESSAGES[][3] = {
        {"KN4CRD", "EM73", "@EMCOMM HELLO WORLD"},
        {"KN4CRD", "EM73", "@AMRRON SNR?"},
        {"KN4CRD", "EM73", "@EMCOMM QRV?"},
        {"KN4CRD", "EM73", "@EMCOMM SNR -10"},
        {"KN4CRD", "EM73", "@MAGNET STATUS AT HOME"},
        {"KN4CRD", "EM73", "W1AW/4 GRID?"},
        {"KN4CRD/QRP", "EM73", "N0JDS SNR?"},
        {"KN4CRD/QRP", "EM73", "@EMCOMM HELLO"},
        {"KN4CRD", "EM73", "@DX/NA HELLO"},
        {nullptr, nullptr, nullptr},
    };
    printf("/// `(mycall, mygrid, text, frames as \"chars/flags\" joined by spaces)`.\n");
    printf("pub const MESSAGES: &[(&str, &str, &str, &str)] = &[\n");
    for (int i = 0; MESSAGES[i][0]; ++i) {
        auto frames = Varicode::buildMessageFrames(MESSAGES[i][0], MESSAGES[i][1], "",
                                                   MESSAGES[i][2], false, false,
                                                   Varicode::JS8CallNormal, nullptr);
        QStringList out;
        for (auto const &f : frames) out << QString("%1/%2").arg(f.first).arg(f.second);
        printf("    (\"%s\", \"%s\", \"%s\", \"%s\"),\n", MESSAGES[i][0], MESSAGES[i][1],
               MESSAGES[i][2], qPrintable(out.join(' ')));
    }
    printf("];\n\n");

    // The Huffman table for uncompressed free text. Emitted rather than
    // transcribed: 44 entries of variable-length codes is exactly the sort of
    // table a human copies one row wrong.
    printf("/// `(character, code bits as a string of '0'/'1')`.\n");
    printf("pub const HUFF: &[(&str, &str)] = &[\n");
    {
        auto const table = Varicode::defaultHuffTable();
        for (auto it = table.constBegin(); it != table.constEnd(); ++it) {
            QString ch = it.key();
            // Escape the two characters that would otherwise break the literal.
            QString esc = ch == "\\" ? "\\\\" : (ch == "\"" ? "\\\"" : ch);
            printf("    (\"%s\", \"%s\"),\n", qPrintable(esc), qPrintable(it.value()));
        }
    }
    printf("];\n\n");

    // Free-text frames. `packDataMessage` picks Huffman or dictionary
    // compression per message, whichever fits more characters, so these cover
    // both paths and record how many characters actually made it in.
    static const char *TEXTS[] = {
        "HELLO WORLD", "TEST", "THE QUICK BROWN FOX", "CQ CQ CQ DE KN4CRD",
        "GOOD MORNING", "73 AND THANKS", "QSL TNX FER QSO", "E", "EEEEEEEEEE",
        "HELLO", "WEATHER IS FINE HERE TODAY", "ABC123", nullptr,
    };
    printf("/// `(text, characters consumed, frame chars)`.\n");
    printf("pub const DATA_FRAMES: &[(&str, i32, &str)] = &[\n");
    for (int i = 0; TEXTS[i]; ++i) {
        int n = 0;
        QString f = Varicode::packDataMessage(TEXTS[i], &n);
        if (f.isEmpty() || n <= 0) continue;
        printf("    (\"%s\", %d, \"%s\"),\n", TEXTS[i], n, qPrintable(f));
    }
    printf("];\n");
    return 0;
}
CPP

"$MOC" "$SRC/varicode.h" -o "$WORK/moc_varicode.cpp"
g++ -std=c++17 -fPIC -O1 -w -o "$WORK/gen" "$WORK/gen.cpp" "$WORK/moc_varicode.cpp" \
    "$SRC/varicode.cpp" "$SRC/decodedtext.cpp" "$SRC/jsc.cpp" "$SRC/jsc_list.cpp" \
    "$SRC/jsc_map.cpp" -I"$SRC" $(pkg-config --cflags --libs Qt6Core)

"$WORK/gen"
