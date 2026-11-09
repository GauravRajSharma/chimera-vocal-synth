//! Nepali text (Devanagari) → segments, in standard Kathmandu pronunciation.
//!
//! Letters map to IPA by table; the inherent vowel ʌ is dropped word-finally and in VC_CV
//! (scanning right to left, as for Hindi), except where Nepali keeps it: after a conjunct, in a
//! one-syllable word, after छ (गर्छ), in negative verbs in -एन/-ऐन, and when nasalised. Postpositions
//! and the plural written onto a word (घरमा, कलमहरू) are split off first, so the stem's schwas
//! are decided as a word's. `nepaliExceptions.tsv` overrides all of it per word.
//!
//! Stress is mild and initial: the first syllable, or the second when the first is light (open)
//! and the second heavy (closed, or a diphthong). The stressed vowel gets 1, the others 2 (no
//! English-style reduction), a diphthong's offglide 0.

use super::english::{EnglishG2P, Phrase};
use super::inventory::{ASPIRATED, NASALISED};
use super::ipa::tokenize_ipa_lang;
use super::{Intonation, Lang, Segment};
use std::collections::HashMap;
use std::sync::OnceLock;

/// Whether text is Nepali (any Devanagari letter in it).
pub fn is_devanagari(text: &str) -> bool {
    text.chars().any(is_deva)
}

/// a Devanagari letter or sign (the dandas are punctuation)
fn is_deva(c: char) -> bool {
    ('\u{900}'..='\u{97F}').contains(&c) && !matches!(c, '।' | '॥' | '\u{970}')
}

fn deva_digit(c: char) -> Option<u32> {
    match c {
        '\u{966}'..='\u{96F}' => Some(c as u32 - 0x966),
        '0'..='9' => Some(c as u32 - '0' as u32),
        _ => None,
    }
}

fn is_phrase_punct(c: char) -> bool {
    matches!(c, '।' | '॥' | '.' | '!' | '?' | ';' | ':' | ',' | '\n')
}

/// Nepali text → phrases. Latin words go through `english`; digits (Devanagari or ASCII) are
/// read as Nepali numbers.
pub fn nepali_phrases(text: &str, english: &EnglishG2P) -> Vec<Phrase> {
    let mut phrases = Vec::new();
    let mut words: Vec<String> = Vec::new();
    let mut flush = |delim: Option<char>, words: &mut Vec<String>| {
        let mut segs: Vec<Segment> = Vec::new();
        let mut said = Vec::new();
        for w in words.drain(..) {
            let mut ws = if w.chars().any(is_deva) {
                word(&w)
            } else if w.chars().all(|c| deva_digit(c).is_some()) {
                number(&w)
            } else {
                english.word(&w.to_lowercase(), true)
            };
            let Some(last) = ws.last_mut() else { continue };
            last.word_end = Some(true);
            segs.append(&mut ws);
            said.push(w);
        }
        if segs.is_empty() {
            return;
        }
        let pause = match delim {
            Some('।' | '॥' | '.' | '!' | '?' | '\n') => 0.35,
            Some(_) => 0.18,
            None => 0.3,
        };
        let intonation = if delim == Some('?') { Intonation::Question } else { Intonation::Statement };
        phrases.push(Phrase { segments: segs, intonation, pause, words: said });
    };
    let mut cur = String::new();
    // a run is Devanagari, digits, or Latin letters; anything else separates words
    let kind = |c: char| {
        if deva_digit(c).is_some() {
            1
        } else if is_deva(c) || c == '\u{200C}' || c == '\u{200D}' {
            2
        } else if c.is_ascii_alphabetic() || c == '\'' {
            3
        } else {
            0
        }
    };
    let mut cur_kind = 0;
    for c in text.chars() {
        let k = kind(c);
        if k != 0 && (k == cur_kind || cur.is_empty()) {
            cur.push(c);
            cur_kind = k;
            continue;
        }
        if !cur.is_empty() {
            words.push(std::mem::take(&mut cur));
        }
        if k != 0 {
            cur.push(c);
            cur_kind = k;
        } else if is_phrase_punct(c) {
            flush(Some(c), &mut words);
        }
        // spaces and other symbols only separate words
    }
    if !cur.is_empty() {
        words.push(cur);
    }
    flush(None, &mut words);
    phrases
}

// ------------------------------------------------------------------ letters

fn consonant(c: char) -> Option<&'static str> {
    Some(match c {
        'क' | '\u{958}' => "k",
        'ख' | '\u{959}' => "kʰ",
        'ग' | '\u{95A}' => "g",
        'घ' => "gʱ",
        'ङ' => "ŋ",
        'च' => "ts",
        'छ' => "tsʰ",
        'ज' | '\u{95B}' => "dz",
        'झ' => "dzʱ",
        'ञ' => "n",
        'ट' => "ʈ",
        'ठ' => "ʈʰ",
        'ड' => "ɖ",
        'ढ' => "ɖʱ",
        '\u{95C}' => "ɽ",
        '\u{95D}' => "ɽʱ",
        'ण' => "n",
        'त' => "t̪",
        'थ' => "t̪ʰ",
        'द' => "d̪",
        'ध' => "d̪ʱ",
        'न' => "n",
        'प' => "p",
        'फ' | '\u{95E}' => "pʰ",
        'ब' => "b",
        'भ' => "bʱ",
        'म' => "m",
        'य' | '\u{95F}' => "j",
        'र' => "ɾ",
        'ल' | 'ळ' => "l",
        // व is [b] on its own and [w] after a consonant (स्व, द्व): see `parse`
        'व' => "b",
        'श' | 'ष' | 'स' => "s",
        'ह' => "ɦ",
        _ => return None,
    })
}

/// A vowel sign (matra) or independent vowel → its vowel parts. ऋ/ृ is [ɾi]: its ɾ is pushed as
/// a consonant by `parse`.
fn vowel(c: char) -> Option<&'static [&'static str]> {
    Some(match c {
        'अ' => &["ʌ"],
        'आ' | 'ा' => &["a"],
        'इ' | 'ई' | 'ि' | 'ी' => &["i"],
        'उ' | 'ऊ' | 'ु' | 'ू' => &["u"],
        'ऋ' | 'ृ' => &["i"],
        'ए' | 'े' | 'ऍ' | 'ॅ' | 'ॆ' => &["e"],
        'ऐ' | 'ै' => &["ʌ", "i"],
        'ओ' | 'ो' | 'ऑ' | 'ॉ' | 'ॊ' => &["o"],
        'औ' | 'ौ' => &["ʌ", "u"],
        _ => return None,
    })
}

fn is_matra(c: char) -> bool {
    ('\u{93E}'..='\u{94C}').contains(&c) || c == '\u{962}' || c == '\u{963}'
}

#[derive(Clone, Debug, PartialEq)]
enum Tok {
    C { ipa: &'static str, ch: char },
    V { parts: &'static [&'static str], inherent: bool, nasal: bool, deleted: bool },
}

impl Tok {
    fn is_c(&self) -> bool {
        matches!(self, Tok::C { .. })
    }
    fn is_live_v(&self) -> bool {
        matches!(self, Tok::V { deleted: false, .. })
    }
}

const INHERENT: Tok = Tok::V { parts: &["ʌ"], inherent: true, nasal: false, deleted: false };

/// Letters → consonants and vowels (inherent vowels included, not yet deleted). Anusvara before a
/// stop becomes its homorganic nasal; otherwise it and chandrabindu nasalise the vowel.
fn parse(w: &str) -> Vec<Tok> {
    let mut out: Vec<Tok> = Vec::new();
    let mut pending = false; // a consonant still waiting for its vowel
    let mut anusvara_at: Vec<usize> = Vec::new();
    let chars: Vec<char> = w.chars().collect();
    for (i, &c) in chars.iter().enumerate() {
        if let Some(ipa) = consonant(c) {
            if pending {
                out.push(INHERENT);
            }
            // व after a consonant (a conjunct) is [w]
            let after_halant = i > 0 && chars[i - 1] == '्';
            let ipa = if c == 'व' && after_halant { "w" } else { ipa };
            out.push(Tok::C { ipa, ch: c });
            pending = true;
        } else if c == '\u{93C}' {
            // nukta: ड़ ढ़ written as two code points
            if let Some(Tok::C { ipa, ch }) = out.last_mut() {
                match ch {
                    'ड' => *ipa = "ɽ",
                    'ढ' => *ipa = "ɽʱ",
                    _ => {}
                }
            }
        } else if c == '्' {
            pending = false;
        } else if let Some(parts) = vowel(c) {
            if is_matra(c) {
                if c == 'ृ' {
                    out.push(Tok::C { ipa: "ɾ", ch: 'र' });
                }
            } else {
                if pending {
                    out.push(INHERENT);
                }
                if c == 'ऋ' {
                    out.push(Tok::C { ipa: "ɾ", ch: 'र' });
                }
            }
            out.push(Tok::V { parts, inherent: false, nasal: false, deleted: false });
            pending = false;
        } else if c == 'ं' || c == 'ँ' || c == 'ः' {
            if pending {
                out.push(INHERENT);
                pending = false;
            }
            if c == 'ं' {
                anusvara_at.push(out.len());
            } else if c == 'ँ'
                && let Some(Tok::V { nasal, .. }) = out.last_mut()
            {
                *nasal = true;
            }
        } else if c == 'ॐ' {
            out.push(Tok::V { parts: &["o"], inherent: false, nasal: false, deleted: false });
            out.push(Tok::C { ipa: "m", ch: 'म' });
        }
    }
    if pending {
        out.push(INHERENT);
    }
    // anusvara: a nasal at the next stop's place, else a nasal vowel (back to front so the
    // insertions don't shift the positions still to do)
    for &at in anusvara_at.iter().rev() {
        let next = out.get(at).and_then(|t| if let Tok::C { ipa, .. } = t { Some(*ipa) } else { None });
        let base = next.map(|n| n.trim_end_matches([ASPIRATED, 'ʱ']));
        let nasal = match base {
            Some("k" | "g") => Some("ŋ"),
            Some("ts" | "dz" | "t̪" | "d̪" | "n") => Some("n"),
            Some("ʈ" | "ɖ") => Some("ɳ"),
            Some("p" | "b" | "m") => Some("m"),
            _ => None,
        };
        match nasal {
            Some(n) => out.insert(at, Tok::C { ipa: n, ch: 'ं' }),
            None => {
                if let Some(Tok::V { nasal, .. }) = out[..at].last_mut() {
                    *nasal = true;
                }
            }
        }
    }
    clusters(&mut out);
    out
}

/// क्ष is [tsʰ] (+[j] before a back or open vowel), ज्ञ is [gj].
fn clusters(t: &mut Vec<Tok>) {
    let mut i = 0;
    while i + 1 < t.len() {
        match (&t[i], &t[i + 1]) {
            (Tok::C { ch: 'क', .. }, Tok::C { ch: 'ष', .. }) => {
                let glide = matches!(t.get(i + 2), Some(Tok::V { parts, .. }) if matches!(parts[0], "a" | "ʌ" | "o" | "u"));
                t[i] = Tok::C { ipa: "tsʰ", ch: 'छ' };
                if glide {
                    t[i + 1] = Tok::C { ipa: "j", ch: 'य' };
                } else {
                    t.remove(i + 1);
                }
            }
            (Tok::C { ch: 'ज', .. }, Tok::C { ch: 'ञ', .. }) => {
                t[i] = Tok::C { ipa: "g", ch: 'ग' };
                t[i + 1] = Tok::C { ipa: "j", ch: 'य' };
            }
            _ => {}
        }
        i += 1;
    }
}

// ------------------------------------------------------------------ schwa deletion

/// Postpositions and the plural, written onto the word they follow (longest first).
const SUFFIXES: [&str; 16] = ["हरूलाई", "हरूको", "हरूले", "हरूमा", "हरू", "सँगै", "सँग", "देखि", "सम्म", "भन्दा", "बाट", "लाई", "ले", "को", "का", "मा"];

fn delete_schwas(t: &mut [Tok]) {
    let n = t.len();
    let vowels = t.iter().filter(|x| matches!(x, Tok::V { .. })).count();
    // word-final
    if let Some(Tok::V { inherent: true, nasal: false, .. }) = t.last() {
        let cluster = n >= 3 && t[n - 2].is_c() && t[n - 3].is_c();
        let after_chha = matches!(t.get(n.wrapping_sub(2)), Some(Tok::C { ch: 'छ', .. }));
        let negative = n >= 3 && matches!(t[n - 2], Tok::C { ch: 'न', .. }) && matches!(t[n - 3], Tok::V { parts, inherent: false, .. } if matches!(*parts, ["e"] | ["ʌ", "i"]));
        if !(vowels == 1 || cluster || after_chha || negative)
            && let Tok::V { deleted, .. } = &mut t[n - 1]
        {
            *deleted = true;
        }
    }
    // medial VC_CV, right to left
    for i in (2..n.saturating_sub(2)).rev() {
        let medial = matches!(t[i], Tok::V { inherent: true, nasal: false, deleted: false, .. });
        if medial && t[i - 1].is_c() && t[i - 2].is_live_v() && t[i + 1].is_c() && t[i + 2].is_live_v()
            && let Tok::V { deleted, .. } = &mut t[i]
        {
            *deleted = true;
        }
    }
}

// ------------------------------------------------------------------ words

fn exceptions() -> &'static HashMap<String, Vec<Segment>> {
    static E: OnceLock<HashMap<String, Vec<Segment>>> = OnceLock::new();
    E.get_or_init(|| {
        include_str!("nepaliExceptions.tsv")
            .lines()
            .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
            .filter_map(|l| l.split_once('\t'))
            .map(|(w, ipa)| (w.trim().to_string(), tokenize_ipa_lang(ipa.trim(), Lang::Nepali)))
            .collect()
    })
}

/// One Devanagari word → segments with stress (and word-final ʌ etc. decided).
pub fn word(w: &str) -> Vec<Segment> {
    let w: String = w.chars().filter(|c| *c != '\u{200C}' && *c != '\u{200D}').collect();
    if let Some(segs) = exceptions().get(&w) {
        return segs.clone();
    }
    // the stem's schwas are a word's; a suffix keeps its own
    for suf in SUFFIXES {
        if let Some(stem) = w.strip_suffix(suf)
            && stem.chars().filter(|c| consonant(*c).is_some() || vowel(*c).is_some()).count() >= 2
        {
            let mut t = parse(stem);
            delete_schwas(&mut t);
            let mut s = parse(suf);
            delete_schwas(&mut s);
            t.extend(s);
            return segments(&t);
        }
    }
    let mut t = parse(&w);
    delete_schwas(&mut t);
    segments(&t)
}

/// Tokens → segments, with stress.
fn segments(t: &[Tok]) -> Vec<Segment> {
    let mut out: Vec<Segment> = Vec::new();
    // syllable nuclei: (index into out of the first part, heavy)
    let mut nuclei: Vec<(usize, bool)> = Vec::new();
    let live: Vec<&Tok> = t.iter().filter(|x| !matches!(x, Tok::V { deleted: true, .. })).collect();
    for (k, tok) in live.iter().enumerate() {
        match tok {
            Tok::C { ipa, .. } => out.push(Segment::new(ipa)),
            Tok::V { parts, nasal, .. } => {
                // closed: a consonant follows that ends the word or precedes another consonant
                let closed = live.get(k + 1).is_some_and(|x| x.is_c()) && live.get(k + 2).is_none_or(|x| x.is_c());
                nuclei.push((out.len(), closed || parts.len() > 1));
                for (j, p) in parts.iter().enumerate() {
                    let mut ipa = p.to_string();
                    if *nasal {
                        ipa.push(NASALISED);
                    }
                    let mut s = Segment::new(&ipa);
                    s.stress = Some(if j == 0 { 2 } else { 0 });
                    out.push(s);
                }
            }
        }
    }
    let stressed = match nuclei.as_slice() {
        [(_, false), (b, true), ..] => Some(*b),
        [(a, _), ..] => Some(*a),
        [] => None,
    };
    if let Some(i) = stressed {
        out[i].stress = Some(1);
    }
    for s in out.iter_mut() {
        if s.stress.is_none() {
            s.stress = Some(0);
        }
    }
    out
}

// ------------------------------------------------------------------ numbers

/// 0–99 (Nepali has a word for each).
const NUMBERS: [&str; 100] = [
    "शून्य", "एक", "दुई", "तीन", "चार", "पाँच", "छ", "सात", "आठ", "नौ", //
    "दस", "एघार", "बाह्र", "तेह्र", "चौध", "पन्ध्र", "सोह्र", "सत्र", "अठार", "उन्नाइस", //
    "बीस", "एक्काइस", "बाइस", "तेइस", "चौबीस", "पच्चीस", "छब्बीस", "सत्ताइस", "अट्ठाइस", "उनन्तीस", //
    "तीस", "एकतीस", "बत्तीस", "तेत्तीस", "चौँतीस", "पैँतीस", "छत्तीस", "सैँतीस", "अठतीस", "उनन्चालीस", //
    "चालीस", "एकचालीस", "बयालीस", "त्रिचालीस", "चवालीस", "पैँतालीस", "छयालीस", "सतचालीस", "अठचालीस", "उनन्चास", //
    "पचास", "एकाउन्न", "बाउन्न", "त्रिपन्न", "चवन्न", "पचपन्न", "छपन्न", "सन्ताउन्न", "अन्ठाउन्न", "उनन्साठी", //
    "साठी", "एकसट्ठी", "बयसट्ठी", "त्रिसट्ठी", "चौसट्ठी", "पैँसट्ठी", "छयसट्ठी", "सतसट्ठी", "अठसट्ठी", "उनन्सत्तरी", //
    "सत्तरी", "एकहत्तर", "बहत्तर", "त्रिहत्तर", "चौहत्तर", "पचहत्तर", "छयहत्तर", "सतहत्तर", "अठहत्तर", "उनासी", //
    "असी", "एकासी", "बयासी", "त्रियासी", "चौरासी", "पचासी", "छयासी", "सतासी", "अठासी", "उनान्नब्बे", //
    "नब्बे", "एकानब्बे", "बयानब्बे", "त्रियानब्बे", "चौरानब्बे", "पन्चानब्बे", "छयानब्बे", "सन्तानब्बे", "अन्ठानब्बे", "उनान्सय",
];

/// A digit string → segments: 0–99 as its word, longer numbers digit by digit.
pub fn number(digits: &str) -> Vec<Segment> {
    let d: Vec<u32> = digits.chars().filter_map(deva_digit).collect();
    let words: Vec<&str> = if d.len() <= 2 {
        vec![NUMBERS[d.iter().fold(0, |a, x| a * 10 + x) as usize]]
    } else {
        d.iter().map(|x| NUMBERS[*x as usize]).collect()
    };
    let mut out = Vec::new();
    for (i, w) in words.iter().enumerate() {
        let mut s = word(w);
        if i + 1 < words.len()
            && let Some(last) = s.last_mut()
        {
            last.word_end = Some(true);
        }
        out.append(&mut s);
    }
    out
}

/// A word's segments written as IPA ("ˈgʱʌɾ"): for tests and the test set.
pub fn transcribe(segs: &[Segment]) -> String {
    segs.iter().map(|s| format!("{}{}{}", if s.stress == Some(1) { "ˈ" } else { "" }, s.ipa, if s.word_end == Some(true) { " " } else { "" })).collect::<String>().trim().to_string()
}
