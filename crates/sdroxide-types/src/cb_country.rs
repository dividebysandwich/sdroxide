//! The 11 m / CB country of origin, from a CB callsign.
//!
//! A CB callsign's leading digits are its international country number — `26`
//! for the Netherlands, `26AT715` for a Dutch station. The numbers are the
//! classic CB countries-of-the-world list, and the *names* here are the ones
//! WSJT-CB shows for them (the fork's `cb_NNN_to_country` table in
//! `logbook/AD1CCty.cpp`), which is what a user on the 11 m band is used to
//! seeing. Some are historical DXCC-looking names ("East Germany",
//! "Czechoslovakia") — that is what the reference calls them, so that is what
//! the row shows.
//!
//! Each entry also carries the DXCC *primary prefix* of the modern country
//! that flag belongs to. That is what the entity table keys on: it reuses the
//! same flag/continent/position machinery the amateur bands use, so the 11 m
//! decode list draws its flags from the selfsame set. A handful of entries
//! have no current DXCC entity (Geneva, Walvis Bay) and fall back to the
//! override table below.
//!
//! Shape checking mirrors WSJT-CB's `Radio::cb_callsign_*_re` exactly: a lead
//! of 1–3 digits, 1–2 letters, then digits (1–4, a 4-digit suffix only behind
//! a single-digit prefix) or a `/LL` split form.

/// CB country number → (WSJT-CB country name, DXCC primary prefix for its flag).
///
/// Codes are WSJT-CB's own 001–352 numbering — a right-justified three-digit
/// form of the callsign's leading digit run, so `26AT715` is `026` =
/// Netherlands. Seven and 295 have no entry in the reference.
pub(crate) static CB: &[(u16, &str, &str)] = &[
    (1, "Italy", "I"),
    (2, "U.S.A.", "K"),
    (3, "Brazil", "PY"),
    (4, "Argentina", "LU"),
    (5, "Venezuela", "YV"),
    (6, "Colombia", "HK"),
    (8, "Peru", "OA"),
    (9, "Canada", "VE"),
    (10, "Mexico", "XE"),
    (11, "Puerto Rico", "KP4"),
    (12, "Uruguay", "CX"),
    (13, "Germany", "DL"),
    (14, "France", "F"),
    (15, "Switzerland", "HB"),
    (16, "Belgium", "ON"),
    (17, "Hawaiian Islands", "KH6"),
    (18, "Greece", "SV"),
    (19, "Netherlands", "PA"),
    (20, "Norway", "LA"),
    (21, "Sweden", "SM"),
    (22, "French Guyana", "FY"),
    (23, "Jamaica", "6Y"),
    (24, "Panama", "HP"),
    (25, "Japan", "JA"),
    (26, "England", "G"),
    (27, "Iceland", "TF"),
    (28, "Honduras", "HR"),
    (29, "Ireland", "EI"),
    (30, "Spain", "EA"),
    (31, "Portugal", "CT"),
    (32, "Chile", "CE"),
    (33, "Alaska", "KL"),
    (34, "Canary Islands", "EA8"),
    (35, "Austria", "OE"),
    (36, "San Marino", "T7"),
    (37, "Dominican Republic", "HI"),
    (38, "Greenland", "OX"),
    (39, "Angola", "D2"),
    (40, "Liechtenstein", "HB0"),
    (41, "New Zealand", "ZL"),
    (42, "Liberia", "5L"),
    (43, "Australia", "VK"),
    (44, "South Africa", "ZS"),
    (45, "Serbia", "YU"),
    (46, "East Germany", "DL"),
    (47, "Denmark", "OZ"),
    (48, "Saudi Arabia", "HZ"),
    (49, "Balearic Islands", "EA6"),
    (50, "European Russia", "UA"),
    (51, "Andorra", "C3"),
    (52, "Faroe Islands", "OY"),
    (53, "El Salvador", "YS"),
    (54, "Luxembourg", "LX"),
    (55, "Gibraltar", "ZB"),
    (56, "Finland", "OH"),
    (57, "India", "VU"),
    (58, "East Malaysia", "9M6"),
    (59, "Dodecanese Islands", "SV5"),
    (60, "Hong Kong", "VR"),
    (61, "Ecuador", "HC"),
    (62, "Guam Island", "KH2"),
    (63, "St. Helena Island", "ZD7"),
    (64, "Senegal", "6W"),
    (65, "Sierra Leone", "9L"),
    (66, "Mauritania", "5T"),
    (67, "Paraguay", "ZP"),
    (68, "Northern Ireland", "GI"),
    (69, "Costa Rica", "TI"),
    (70, "American Samoa Islands", "KH8"),
    (71, "Midway Islands", "KH4"),
    (72, "Guatemala", "TG"),
    (73, "Suriname", "PZ"),
    (74, "Namibia", "V5"),
    (75, "Azores Islands", "CU"),
    (76, "Morocco", "CN"),
    (77, "Ghana", "9G"),
    (78, "Zambia", "9J"),
    (79, "Philippine Islands", "DU"),
    (80, "Bolivia", "CP"),
    (81, "San Andres Providencia", "HK0/a"),
    (82, "Guantanamo Bay", "KG4"),
    (83, "Tanzania", "5H"),
    (84, "Ivory Coast", "TU"),
    (85, "Zimbabwe", "Z2"),
    (86, "Nepal", "9N"),
    (87, "Yemen", "7O"),
    (88, "Cuba", "CM"),
    (89, "Nigeria", "5N"),
    (90, "Crete Island", "SV9"),
    (91, "Indonesia", "YB"),
    (92, "Libya", "5A"),
    (93, "Malta", "9H"),
    (94, "United Arab Emirates", "A6"),
    (95, "Mongolia", "JT"),
    (96, "Tonga Islands", "A3"),
    (97, "Israel", "4X"),
    (98, "Singapore", "9V"),
    (99, "Fiji Islands", "3D2"),
    (100, "Korea", "HL"),
    (101, "Papua – New Guinea", "P2"),
    (102, "Kuwait", "9K"),
    (103, "Haiti", "HH"),
    (104, "Corsica", "TK"),
    (105, "Botswana", "A2"),
    (106, "Ceuta & Melilla", "EA9"),
    (107, "Monaco", "3A"),
    (108, "Scotland", "GM"),
    (109, "Hungary", "HA"),
    (110, "Cyprus", "5B"),
    (111, "Jordan", "JY"),
    (112, "Lebanon", "OD"),
    (113, "West Malaysia", "9M2"),
    (114, "Pakistan", "AP"),
    (115, "Qatar", "A7"),
    (116, "Turkey", "TA"),
    (117, "Egypt", "SU"),
    (118, "The Gambia", "C5"),
    (119, "Madeira Island", "CT3"),
    (120, "Antigua & Barbuda Isl", "V2"),
    (121, "The Bahamas", "C6"),
    (122, "Barbados Island", "8P"),
    (123, "Bermuda Island", "VP9"),
    (124, "Amsterdam & St. Paul Isl", "FT/z"),
    (125, "Cayman Islands", "ZF"),
    (126, "Nicaragua", "YN"),
    (127, "Virgin Islands", "KP2"),
    (128, "British Virgin Isl.", "VP2V"),
    (129, "Macquarie Islands", "VK0M"),
    (130, "Norfolk Islands", "VK9N"),
    (131, "Guyana", "8R"),
    (132, "Marshall Islands", "V7"),
    (133, "Marianas Islands", "KH0"),
    (134, "Republic Of Palau", "T8"),
    (135, "Solomon Islands", "H4"),
    (136, "Martinique Island", "FM"),
    (137, "Isle Of Man", "GD"),
    (138, "Vatican City State", "HV"),
    (139, "Southern Yemen", "7O"),
    (140, "Antarctica", "CE9"),
    (141, "St. Pierre & Miquelon", "FP"),
    (142, "Lesotho", "7P"),
    (143, "St. Lucia Island", "J6"),
    (144, "Easter Island", "CE0Y"),
    (145, "Galapagos Islands", "HC8"),
    (146, "Algeria", "7X"),
    (147, "Tunisia", "3V"),
    (148, "Ascension Island", "ZD8"),
    (149, "Laccadive Islands", "VU7"),
    (150, "Bahrain", "A9"),
    (151, "Iraq", "YI"),
    (152, "Maldives Islands", "8Q"),
    (153, "Thailand", "HS"),
    (154, "Iran", "EP"),
    (155, "Taiwan", "BV"),
    (156, "Cameroon", "TJ"),
    (157, "Montserrat Island", "VP2M"),
    (158, "Trinidad & Tobago Isl", "9Y"),
    (159, "Somali Republic", "T5"),
    (160, "Sudan", "ST"),
    (161, "Poland", "SP"),
    (162, "Democratic Republic Of Congo", "9Q"),
    (163, "Wales", "GW"),
    (164, "Togo Republic", "5V"),
    (165, "Sardegna", "IS"),
    (166, "St. Maarten, Saba & St.Eustatius", "PJ5"),
    (167, "Jersey Island", "GJ"),
    (168, "Mauritius Islands", "3B8"),
    (169, "Guernsey Island", "GU"),
    (170, "Burkina Faso", "XT"),
    (171, "Svalbard Islands", "JW"),
    (172, "New Caledonia", "FK"),
    (173, "Reunion Island", "FR"),
    (174, "Uganda", "5X"),
    (175, "Chad Republic", "TT"),
    (176, "Central African Republic", "TL"),
    (177, "Sri Lanka", "4S"),
    (178, "Bulgaria", "LZ"),
    (179, "Czechoslovakia", "OK"),
    (180, "Oman", "A4"),
    (181, "Syria", "YK"),
    (182, "Republic Of Guinea", "3X"),
    (183, "Benin", "TY"),
    (184, "Burundi", "9U"),
    (185, "Comoros Islands", "D6"),
    (186, "Djibouti", "J2"),
    (187, "Kenya", "5Z"),
    (188, "Malagasy Republic", "5R"),
    (189, "Mayotte Island", "FH"),
    (190, "Seychelles Islands", "S7"),
    (191, "Kingdom of Eswatini", "3DA"),
    (192, "Cocos Islands", "VK9C"),
    (193, "Keeling Islands", "VK9C"),
    (194, "Dominica Island", "J7"),
    (195, "Grenada Island", "J3"),
    (196, "Guadeloupe Islands", "FG"),
    (197, "Vanuatu Islands", "YJ"),
    (198, "Falkland Islands", "VP8"),
    (199, "Equatorial Guinea", "3C"),
    (200, "South Shetland Islands", "VP8/h"),
    (201, "French Polynesia", "FO"),
    (202, "Bhutan", "A5"),
    (203, "China", "BY"),
    (204, "Mozambique", "C9"),
    (205, "Republic Of Cape Verde", "D4"),
    (206, "Ethiopia", "ET"),
    (207, "St. Martin Island", "FS"),
    (208, "Glorieuses Islands", "FT/e"),
    (209, "Juan De Nova Island", "FT/j"),
    (210, "Wallis & Futuna Islands", "FW"),
    (211, "Jan Mayen Island", "JX"),
    (212, "Aland Islands", "OH0"),
    (213, "Market Reef", "OJ0"),
    (214, "Congo Republic", "TN"),
    (215, "Gabon Republic", "TR"),
    (216, "Mali", "TZ"),
    (217, "Christmas Island", "VK9X"),
    (218, "Belize", "V3"),
    (219, "Anguilla Island", "VP2E"),
    (220, "St. Vincent Island", "J8"),
    (221, "South Orkney Islands", "VP8/o"),
    (222, "South Sandwich Islands", "VP8/s"),
    (223, "Western Samoa Islands", "5W"),
    (224, "Western Kiribati", "T30"),
    (225, "Brunei", "V8"),
    (226, "Malawi", "7Q"),
    (227, "Rwanda", "9X"),
    (228, "Chagos Islands", "VQ9"),
    (229, "Heard Island", "VK0H"),
    (230, "Federated States Of Micronesia", "V6"),
    (231, "St. Peter & St. Paul Rock", "PY0S"),
    (232, "Aruba Island", "P4"),
    (233, "Romania", "YO"),
    (234, "Afghanistan", "YA"),
    (235, "Geneva", "4U1GSC"),
    (236, "Bangladesh", "S2"),
    (237, "Union Of Myanmar", "XZ"),
    (238, "Cambodia", "XU"),
    (239, "Laos", "XW"),
    (240, "Macao", "XX9"),
    (241, "Spratly Island", "1S"),
    (242, "Vietnam", "3W"),
    (243, "Gleam & St.Brandon Isl", "3B6"),
    (244, "Pagalu Island", "3C0"),
    (245, "Niger Republic", "5U"),
    (246, "Sao Tome & Principe Isl", "S9"),
    (247, "Navassa Island", "KP1"),
    (248, "Turks & Caicos Islands", "VP5"),
    (249, "Northern Cook Islands", "E5/n"),
    (250, "Cook Islands", "E5/s"),
    (251, "Albania", "ZA"),
    (252, "Revillagigedo Islands", "XF4"),
    (253, "Andaman & Nicobar Island", "VU4"),
    (254, "Mount Athos", "SV/a"),
    (255, "Kerguelen Islands", "FT/x"),
    (256, "Prince Edward & Marion Islands", "ZS8"),
    (257, "Rodriguez Island", "3B9"),
    (258, "Tristan Da Cunha & Gough", "ZD9"),
    (259, "Tromelin Island", "FT/t"),
    (260, "Baker & Howland Islands", "KH1"),
    (261, "Chatham Islands", "ZL7"),
    (262, "Johnston Island", "KH3"),
    (263, "Kermadec Islands", "ZL8"),
    (264, "Kingman Reef", "KH5K"),
    (265, "Central Kiribati", "T31"),
    (266, "Eastern Kiribati", "T32"),
    (267, "Kure Island", "KH7K"),
    (268, "Lord Howe Islands", "VK9L"),
    (269, "Mellish Reef", "VK9M"),
    (270, "Minami Torishima Island", "JD/m"),
    (271, "Republic Of Nauru", "C2"),
    (272, "Niue Island", "E6"),
    (273, "Jarvis & Palmyra Islands", "KH5"),
    (274, "Pitcairn Island", "VP6"),
    (275, "Tokelau Islands", "ZK3"),
    (276, "Tuvalu Islands", "T2"),
    (277, "Sable Island", "CY0"),
    (278, "Wake Island", "KH9"),
    (279, "Willis Islets", "VK9W"),
    (280, "Aves Island", "YV0"),
    (281, "Ogasawara Islands", "JD/o"),
    (282, "Auckland & Campbell Islands", "ZL9"),
    (283, "St. Kitts & Nevis Island", "V4"),
    (284, "St. Paul Island", "CY9"),
    (285, "Fernando De Noronha Islands", "PY0F"),
    (286, "Juan Fernandez Islands", "CE0Z"),
    (287, "Malpelo Island", "HK0/m"),
    (288, "San Felix & San Ambrosio", "CE0X"),
    (289, "South Georgia Islands", "VP8/g"),
    (290, "Trindade & Martim Vaz Islands", "PY0T"),
    (291, "Dhekelia & Akrotiri", "ZC4"),
    (292, "Abu-ail & Jabal-al-tair", "7O"),
    (293, "Guinea Bissau", "J5"),
    (294, "Peter 1st Island", "3Y/p"),
    (296, "Clipperton Island", "FO/c"),
    (297, "Bouvet Island", "3Y/b"),
    (298, "Crozet Islands", "FT/w"),
    (299, "Desecheo Island", "KP5"),
    (300, "West Sahara-rio De Oro", "S0"),
    (301, "Armenia", "EK"),
    (302, "Asiatic Russia", "UA9"),
    (303, "Azerbaijan", "4J"),
    (304, "Estonia", "ES"),
    (305, "Franz Josef Land", "R1FJ"),
    (306, "Georgia", "4L"),
    (307, "Kaliningradsk", "UA2"),
    (308, "Kazakh", "UN"),
    (309, "Kyrgyzstan", "EX"),
    (310, "Latvia", "YL"),
    (311, "Lithuania", "LY"),
    (312, "Moldavia", "ER"),
    (313, "Tajikistan", "EY"),
    (314, "Turkoman", "EZ"),
    (315, "Ukraine", "UR"),
    (316, "Uzbek", "UK"),
    (317, "Belarus", "EU"),
    (318, "Survey Military Of Malta", "1A"),
    (319, "United Nations New York", "4U1UN"),
    (320, "Banaba Island", "T33"),
    (321, "Conway Reef", "3D2/c"),
    (322, "Walvis Bay", "ZS9"),
    (323, "Yemen Republic", "7O"),
    (324, "Penguin Islands", "ZS8"),
    (325, "Rotuma Island", "3D2/r"),
    (326, "Malyj Vytsotskj", "R1MV"),
    (327, "Slovenia", "S5"),
    (328, "Croatia", "9A"),
    (329, "Czech Republic", "OK"),
    (330, "Slovak Republic", "OM"),
    (331, "Bosnia", "E7"),
    (332, "North Macedonia", "Z3"),
    (333, "Eritrea", "E3"),
    (334, "North Korea", "P5"),
    (335, "Scarborough Reef", "BS7"),
    (336, "Pratas Island", "BV9P"),
    (337, "Austral Islands", "FO/a"),
    (338, "Marquesas Islands", "FO/m"),
    (339, "Temotu", "H40"),
    (340, "Palestina", "E4"),
    (341, "East Timor", "4W"),
    (342, "Chesterfields Islands", "FK/c"),
    (343, "Ducie Island", "VP6/d"),
    (344, "Republic Of Montenegro", "4O"),
    (345, "Swains Island", "KH8/s"),
    (346, "St. Barthelemy Island", "FJ"),
    (347, "Curacao", "PJ2"),
    (348, "Sint Maarten", "PJ7"),
    (349, "Saba & Sint-Eustatius", "PJ5"),
    (350, "Bonaire", "PJ4"),
    (351, "South Sudan", "Z8"),
    (352, "Republic of Kosovo", "Z6"),
];

/// Flag + continent for the CB entries that have no current DXCC entity, so
/// the cty lookup cannot name them. `(DXCC-lookalike prefix, flag code,
/// continent)`. Everything else resolves through the ordinary entity table.
static FALLBACK: &[(&str, &str, &str)] = &[
    // French overseas bits ISO 3166-1 leaves out (Glorieuses) — the flag they
    // fly is the French one, like the neighbouring districts.
    ("FT/e", "FR", "AF"),
    // Old DXCC entities long merged away — the flag of the country they became.
    ("VU7", "IN", "AS"),
    ("5L", "LR", "AF"),
    ("ZS9", "NA", "AF"),
    ("ZS8", "ZA", "AF"),
    ("R1MV", "RU", "EU"),
    // Kingman Reef and the Northern Cooks are real DXCC entities this country
    // file omits; Kingman flies the US flag, the Northern Cooks the Cook one.
    ("KH5K", "UM", "OC"),
    ("ZK1/n", "CK", "OC"),
    // UN offices are not countries; the flag shown is the host's.
    ("4U1GSC", "CH", "EU"),
    ("4U1UN", "US", "NA"),
];

/// The CB country number a callsign's leading digit run encodes, or `None`
/// when it is not a CB-shaped callsign. Matches WSJT-CB's own shape rules and
/// right-justifies the run to three digits (`26AT715` → `026`).
pub(crate) fn cb_country_number(call: &str) -> Option<u32> {
    let call = call.trim().to_ascii_uppercase();
    if !is_cb_shape(call.as_bytes()) {
        return None;
    }
    call[..call.bytes().take_while(|b| b.is_ascii_digit()).count()].parse().ok()
}

/// WSJT-CB country name + DXCC prefix for a CB country number.
pub(crate) fn name_prefix(code: u16) -> Option<(&'static str, &'static str)> {
    CB.iter().find(|(c, _, _)| *c == code).map(|(_, name, pfx)| (*name, *pfx))
}

/// `(primary prefix → (flag, continent))` for a CB entity.
pub(crate) fn fallback_cell(pfx: &str) -> Option<(&'static str, &'static str)> {
    FALLBACK.iter().find(|(p, _, _)| *p == pfx).map(|(_, f, c)| (*f, *c))
}

/// Whether the call matches WSJT-CB's CB-callsign shape (`Radio::cb_callsign_*_re`):
/// `N{1,3}L{1,2}N{1,3}` (or a 4-digit suffix behind a one-digit prefix), or the
/// split `N{1,3}L{1,2}/LL`.
fn is_cb_shape(b: &[u8]) -> bool {
    if b.is_empty() {
        return false;
    }
    let digits = b.iter().take_while(|&&c| c.is_ascii_digit()).count();
    if !(1..=3).contains(&digits) {
        return false;
    }
    // Split form `999ZZ/ZZ` — a single slash delimiting the final two letters.
    if let Some(sl) = b.iter().rposition(|&c| c == b'/') {
        if b[..sl].contains(&b'/') {
            return false;
        }
        let sfx = &b[sl + 1..];
        return sfx.len() == 2
            && sfx.iter().all(u8::is_ascii_uppercase)
            && digits_letters(&b[..sl]) == Some(digits);
    }
    let rest = &b[digits..];
    let letters = rest.iter().take_while(|&&c| c.is_ascii_uppercase()).count();
    if !(1..=2).contains(&letters) {
        return false;
    }
    let units = rest[letters..].iter().take_while(|&&c| c.is_ascii_digit()).count();
    if digits + letters + units != b.len() {
        return false;
    }
    (1..=4).contains(&units) && (units < 4 || digits == 1)
}

/// For a digit-then-letters run (split form's base) confirm the digit count.
fn digits_letters(b: &[u8]) -> Option<usize> {
    let digits = b.iter().take_while(|&&c| c.is_ascii_digit()).count();
    let letters = b[digits..].iter().take_while(|&&c| c.is_ascii_uppercase()).count();
    (1..=2).contains(&letters).then_some(digits)
}
