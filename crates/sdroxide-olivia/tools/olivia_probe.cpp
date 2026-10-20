// olivia_probe -- dump the internals of Pawel Jalocha's MFSK demodulator.
//
// This exists so a port does not have to be written from a *reading* of
// pj_mfsk.h. Three attempts failed that way. This prints the two stages that
// matter, for the same audio:
//
//   SYM  <n> <slice> <foff> <soft bits...>   what the demodulator hands the
//                                            decoder for symbol n
//   CHR  <n> <char codes...>                  what the decoder makes of a block
//
// A Rust implementation can then be diffed against it symbol by symbol instead
// of argued about.
//
// Build:
//   g++ -O2 -std=c++14 -I ../cxx olivia_probe.cpp -o olivia_probe
//
// Run (audio must be raw little-endian f32, mono, at the rate given):
//   ./olivia_probe capture.f32 16 500 8000 1718 1 0 0 0
//                  file      tones bw rate centre rev slice foff phase
//
// Convert a WAV with:
//   ffmpeg -i capture.wav -f f32le -acodec pcm_f32le -ac 1 capture.f32

#include <cmath>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <vector>

#include "pj_mfsk.h"

static size_t Log2u(size_t x) {
    size_t y = 0;
    while (x > 1) {
        x >>= 1;
        y += 1;
    }
    return y;
}

int main(int argc, char **argv) {
    if (argc < 6) {
        fprintf(stderr,
                "usage: %s <raw f32> <tones> <bandwidth> <rate> <centre> "
                "[reverse] [slice] [freq offset] [block phase] [max symbols]\n",
                argv[0]);
        return 2;
    }
    const char *path = argv[1];
    size_t tones = (size_t)atoi(argv[2]);
    size_t bandwidth = (size_t)atoi(argv[3]);
    double rate = atof(argv[4]);
    double centre = atof(argv[5]);
    int reverse = argc > 6 ? atoi(argv[6]) : 0;
    size_t want_slice = argc > 7 ? (size_t)atoi(argv[7]) : 0;
    int want_foff = argc > 8 ? atoi(argv[8]) : 0;
    size_t want_phase = argc > 9 ? (size_t)atoi(argv[9]) : 0;
    size_t max_sym = argc > 10 ? (size_t)atoi(argv[10]) : 400;

    FILE *f = fopen(path, "rb");
    if (!f) {
        perror(path);
        return 1;
    }
    std::vector<float> audio;
    {
        float buf[4096];
        size_t n;
        while ((n = fread(buf, sizeof(float), 4096, f)) > 0) audio.insert(audio.end(), buf, buf + n);
    }
    fclose(f);
    fprintf(stderr, "read %zu samples (%0.1f s at %0.0f Hz)\n", audio.size(),
            audio.size() / rate, rate);

    // The same arithmetic fldigi's olivia_modem does in whf_dsp_olivia::restart().
    size_t bits = Log2u(tones);
    size_t symlen = (size_t)1 << (bits + 7 - Log2u(bandwidth / 125));
    double fc_offset = bandwidth * (1.0 - 0.5 / tones) / 2.0;
    float multiplier = (float)((reverse ? centre + fc_offset : centre - fc_offset) / 500.0);

    MFSK_Demodulator<float> dem;
    dem.Default();
    dem.BitsPerSymbol = bits;
    dem.SymbolLen = symlen;
    dem.FirstCarrier = (size_t)((symlen / 16) * multiplier) + 1;
    dem.DecodeMargin = 8;
    dem.Reverse = reverse;
    if (dem.Preset() < 0) {
        fprintf(stderr, "demodulator preset failed\n");
        return 1;
    }
    fprintf(stderr,
            "SymbolLen %zu, SymbolSepar %zu, FirstCarrier %zu (%.1f Hz), "
            "carriers %zu, spacing %.2f Hz\n",
            dem.SymbolLen, dem.SymbolSepar, dem.FirstCarrier,
            dem.FirstCarrier * rate / dem.SymbolLen, dem.Carriers,
            (double)dem.CarrierSepar * rate / dem.SymbolLen);

    MFSK_SoftDecoder<float, float> dec;
    dec.bContestia = false;
    dec.BitsPerSymbol = bits;
    if (dec.Preset() < 0) {
        fprintf(stderr, "decoder preset failed\n");
        return 1;
    }

    size_t sep = dem.SymbolSepar;
    size_t n_sym = 0;
    for (size_t off = 0; off + sep <= audio.size() && n_sym < max_sym; off += sep, n_sym++) {
        dem.Process(audio.data() + off);

        float soft[8] = {0};
        dem.SoftDecode(soft, want_slice, want_foff);
        printf("SYM %zu %zu %d", n_sym, want_slice, want_foff);
        for (size_t b = 0; b < bits; b++) printf(" %.6f", soft[b]);
        printf("\n");

        // Feed only the wanted slice/offset into the decoder, so the printed
        // characters belong to the stream above them.
        dec.Input(soft);
        if ((n_sym + 1) % dec.SymbolsPerBlock == 0) {
            size_t first = (n_sym + 1) - dec.SymbolsPerBlock;
            if (first >= want_phase && ((first - want_phase) % dec.SymbolsPerBlock) == 0) {
                dec.Process();
                unsigned char chars[8] = {0};
                dec.Output(chars);
                printf("CHR %zu", first);
                for (size_t b = 0; b < bits; b++) printf(" %u", (unsigned)chars[b]);
                printf("\n");
            }
        }
    }
    fprintf(stderr, "%zu symbols dumped\n", n_sym);
    return 0;
}
