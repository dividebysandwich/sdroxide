//! JS8 message-layer golden vectors — **generated, do not edit**.
//!
//! Emitted by `tools/gen_js8_varicode_vectors.sh` from JS8Call's own
//! `varicode.cpp`, linked and executed. Expected values therefore come from
//! the reference implementation rather than from a second reading of it.
//! See `vendor/js8call/PROVENANCE.md` for the commit.
//!
//! In a subdirectory because cargo compiles every `.rs` directly under
//! `tests/` as its own test binary, and this one has no tests in it.

/// `(callsign, packed 28-bit value)`.
pub const CALLSIGNS: &[(&str, u32)] = &[
    ("KN4CRD", 146325342),
    ("N0JDS", 259625430),
    ("VK3ABC", 223657958),
    ("G0ABC", 258240989),
    ("JA1XYZ", 136637143),
    ("W1AW", 261410543),
    ("M0ABC", 259421969),
    ("9A1A", 65761631),
    ("3DA0XX", 23833844),
    ("3XY1AB", 190944836),
    ("OH2BH", 173447540),
    ("ZL4AA", 252217988),
    ("PY2ABC", 183878615),
    ("LU1DZ", 154730951),
    ("K7A", 259166789),
    ("VE3XYZ", 222494389),
    ("@ALLCALL", 262177562),
    ("@JS8NET", 262177563),
    ("@DX/EU", 262177566),
    ("@REGION/1", 262177571),
    ("@GROUP/0", 262177574),
    ("@NTS", 262177587),
    ("@APRSIS", 262177593),
    ("@EMCOMM", 262177596),
    ("@AMRRON", 262177599),
    ("@QRO", 262177614),
];

/// `(grid, packed 15-bit value)`.
pub const GRIDS: &[(&str, u16)] = &[
    ("EM73", 23883),
    ("QF22", 3112),
    ("IO91", 16341),
    ("FN42", 22632),
    ("JJ00", 16110),
    ("AA00", 32220),
    ("RR99", 179),
    ("JO31", 15621),
    ("PM95", 3725),
    ("GF15", 21295),
    ("BP51", 29671),
];

/// `(from, text, parsed to, parsed cmd, parsed num, frame chars)`.
pub const DIRECTED: &[(&str, &str, &str, &str, &str, &str)] = &[
    ("KN4CRD", "N0JDS SNR?", "N0JDS", " SNR?", "", "SN5-lUyoEi00"),
    ("KN4CRD", "N0JDS GRID?", "N0JDS", " GRID?", "", "SN5-lUyoEiG0"),
    ("KN4CRD", "N0JDS STATUS?", "N0JDS", " STATUS?", "", "SN5-lUyoEiO0"),
    ("KN4CRD", "@ALLCALL HEARING?", "@ALLCALL", " HEARING?", "", "SN5-lVGGOqC0"),
    ("KN4CRD", "N0JDS SNR -05", "N0JDS", " SNR", " -05", "SN5-lUyoEjaQ"),
    ("KN4CRD", "N0JDS SNR +15", "N0JDS", " SNR", " +15", "SN5-lUyoEjak"),
    ("KN4CRD", "VK3ABC QSL", "VK3ABC", " QSL", "", "SN5-lQgN+DS0"),
    ("KN4CRD", "VK3ABC QSL?", "VK3ABC", " QSL?", "", "SN5-lQgN+DO0"),
    ("KN4CRD", "N0JDS RR", "N0JDS", " RR", "", "SN5-lUyoEjK0"),
    ("KN4CRD", "N0JDS SK", "N0JDS", " SK", "", "SN5-lUyoEjG0"),
    ("KN4CRD", "N0JDS FB", "N0JDS", " FB", "", "SN5-lUyoEj80"),
    ("KN4CRD", "N0JDS HW CPY?", "N0JDS", " HW CPY?", "", "SN5-lUyoEjC0"),
    ("KN4CRD", "@ALLCALL CQ CQ CQ", "@ALLCALL", " ", "", "SN5-lVGGOry0"),
    ("KN4CRD", "N0JDS INFO?", "N0JDS", " INFO?", "", "SN5-lUyoEj00"),
    ("VK3ABC", "KN4CRD GRID?", "KN4CRD", " GRID?", "", "Ugb+pHSNwyG0"),
    ("G0ABC", "@JS8NET STATUS?", "@JS8NET", " STATUS?", "", "ViZZk+GGOsO0"),
];

/// `(from, text, frame chars)`.
pub const HEARTBEATS: &[(&str, &str, &str)] = &[
    ("KN4CRD", "@HB HEARTBEAT EM73", "2Y-pe-ukukfO"),
    ("VK3ABC", "@HB HEARTBEAT QF22", "3vM8S1qrO650"),
    ("N0JDS", "@HB HEARTBEAT FN42", "2s0AYI0iOiD0"),
    ("G0ABC", "@HB HEARTBEAT IO91", "1-byv4ieOVwe"),
];

/// `(mycall, mygrid, text, frames as "chars/flags" joined by spaces)`.
pub const MESSAGES: &[(&str, &str, &str, &str)] = &[
    ("KN4CRD", "EM73", "@EMCOMM HELLO WORLD", "SN5-lVGGPvy0/1 tYuMzoX+++++/2"),
    ("KN4CRD", "EM73", "@AMRRON SNR?", "SN5-lVGGP-00/3"),
    ("KN4CRD", "EM73", "@EMCOMM QRV?", "SN5-lVGGPvy0/1 wr4iV+++++++/2"),
    ("KN4CRD", "EM73", "@EMCOMM SNR -10", "SN5-lVGGPvaL/3"),
    ("KN4CRD", "EM73", "@MAGNET STATUS AT HOME", "AY-pe-ukukfO/1 KnAJIbbiW+K8/0 o3taF+++++++/2"),
    ("KN4CRD", "EM73", "W1AW/4 GRID?", "AY-pe-ukukfO/1 JzLmyzdFO+Jm/2"),
    ("KN4CRD/QRP", "EM73", "N0JDS SNR?", "AY-pe+CJMkfO/1 Is0AYI0iO+JG/2"),
    ("KN4CRD/QRP", "EM73", "@EMCOMM HELLO", "AY-pe+CJMkfO/1 KlZxdXure+N8/0 tYuK++++++++/2"),
    ("KN4CRD", "EM73", "@DX/NA HELLO", "SN5-lVGGOvy0/1 tYuK++++++++/2"),
];

/// `(character, code bits as a string of '0'/'1')`.
pub const HUFF: &[(&str, &str)] = &[
    (" ", "01"),
    ("!", "1011000"),
    ("\"", "1010101"),
    ("+", "1100000"),
    ("-", "1100001"),
    (".", "1110100"),
    ("/", "11101010"),
    ("0", "0010101"),
    ("1", "0010001"),
    ("2", "0001001"),
    ("3", "0000101"),
    ("4", "11110101"),
    ("5", "0000100"),
    ("6", "11110000"),
    ("7", "11101011"),
    ("8", "11110001"),
    ("9", "11110100"),
    ("?", "1011001"),
    ("A", "0011"),
    ("B", "1111001"),
    ("C", "110001"),
    ("D", "111011"),
    ("E", "100"),
    ("F", "001001"),
    ("G", "000101"),
    ("H", "00011"),
    ("I", "11100"),
    ("J", "0010100"),
    ("K", "1100100"),
    ("L", "110011"),
    ("M", "101011"),
    ("N", "10111"),
    ("O", "11111"),
    ("P", "1111011"),
    ("Q", "0010000"),
    ("R", "00000"),
    ("S", "10100"),
    ("T", "1101"),
    ("U", "101101"),
    ("V", "1100101"),
    ("W", "001011"),
    ("X", "1010100"),
    ("Y", "000011"),
    ("Z", "0001000"),
];

/// `(text, characters consumed, frame chars)`.
pub const DATA_FRAMES: &[(&str, i32, &str)] = &[
    ("HELLO WORLD", 11, "tYuMzoX+++++"),
    ("TEST", 4, "vqZ+++++++++"),
    ("THE QUICK BROWN FOX", 16, "nE0ETvVbsl++"),
    ("CQ CQ CQ DE KN4CRD", 18, "tuhyL-A+DTrF"),
    ("GOOD MORNING", 12, "zqCJO7++++++"),
    ("73 AND THANKS", 13, "xiUM7w8+++++"),
    ("QSL TNX FER QSO", 15, "wmCyc8BLiV++"),
    ("E", 1, "mF++++++++++"),
    ("EEEEEEEEEE", 10, "m00000001+++"),
    ("HELLO", 5, "tYuK++++++++"),
    ("WEATHER IS FINE HERE TODAY", 26, "+nA6xPTuhyvd"),
    ("ABC123", 6, "tZdLOBDR7+++"),
];
