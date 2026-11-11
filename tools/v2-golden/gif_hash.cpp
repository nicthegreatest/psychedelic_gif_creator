// Encodes v2 golden RGBA dumps with v2's vendored gif-h, exactly as
// v2's gif_worker.cpp does (whole-frame zeroed palette, then threshold or
// Floyd-Steinberg against the previous output frame, delay 8 cs), and prints
// the FNV-1a 64 hash and length of each GIF. crates/gifcore/tests/export.rs
// checks the Rust encoder against these numbers, so no GIF bytes are stored.
//
//   g++ -O2 -std=c++17 -I/home/nic/Software/gif_creator_v2/third_party/gif-h \
//       tools/v2-golden/gif_hash.cpp -o /tmp/gif_hash
//   /tmp/gif_hash crates/gifcore/tests/golden

#include "gif.h"

#include <cstdint>
#include <cstdio>
#include <fstream>
#include <iterator>
#include <string>
#include <vector>

static std::vector<uint8_t> readFile(const std::string& path) {
    std::ifstream in(path, std::ios::binary);
    return {std::istreambuf_iterator<char>(in), std::istreambuf_iterator<char>()};
}

static uint64_t fnv1a(const std::vector<uint8_t>& bytes) {
    uint64_t h = 0xcbf29ce484222325ull;
    for (uint8_t b : bytes) {
        h ^= b;
        h *= 0x100000001b3ull;
    }
    return h;
}

int main(int argc, char** argv) {
    if (argc != 2) {
        std::fprintf(stderr, "usage: gif_hash <golden-dir>\n");
        return 2;
    }
    const std::string root = argv[1];
    struct Config {
        const char* dir;
        uint32_t w, h;
    };
    const Config configs[] = {{"defaults", 160, 120}, {"everything on, letterboxed", 64, 64}};
    const char* frames[] = {"0", "6", "12"};
    const uint32_t delay = 8;
    for (const Config& c : configs) {
        for (int dither = 0; dither < 2; ++dither) {
            const std::string out = "/tmp/gif_hash_out.gif";
            GifWriter w{};
            if (!GifBegin(&w, out.c_str(), c.w, c.h, delay, 8, dither)) return 1;
            for (const char* f : frames) {
                std::vector<uint8_t> rgba = readFile(root + "/" + c.dir + "/" + f + ".rgba");
                if (rgba.size() != size_t(c.w) * c.h * 4) return 1;
                GifPalette pal{};
                GifMakePalette(nullptr, rgba.data(), c.w, c.h, 8, dither, &pal);
                const uint8_t* previous = w.firstFrame ? nullptr : w.oldImage;
                w.firstFrame = false;
                if (dither)
                    GifDitherImage(previous, rgba.data(), w.oldImage, c.w, c.h, &pal);
                else
                    GifThresholdImage(previous, rgba.data(), w.oldImage, c.w, c.h, &pal);
                GifWriteLzwImage(w.f, w.oldImage, 0, 0, c.w, c.h, delay, &pal);
            }
            GifEnd(&w);
            const std::vector<uint8_t> bytes = readFile(out);
            std::printf("%s dither=%d len=%zu fnv=0x%016llx\n", c.dir, dither, bytes.size(),
                        static_cast<unsigned long long>(fnv1a(bytes)));
        }
    }
    return 0;
}
