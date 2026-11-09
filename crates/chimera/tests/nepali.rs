//! Nepali: the Devanagari reader, the tokenizer's modifiers, and how the planner voices them.

use chimera::anatomy::human::BodyMix;
use chimera::control::score::ControlKey;
use chimera::phonetics::english::EnglishG2P;
use chimera::phonetics::ipa::{tokenize_ipa, tokenize_ipa_lang};
use chimera::phonetics::nepali::{nepali_phrases, number, word};
use chimera::phonetics::planner::{PlanOptions, plan_utterance};
use chimera::phonetics::{Lang, Segment};

fn ipa(segs: &[Segment]) -> String {
    segs.iter().map(|s| format!("{}{}", s.ipa, if s.word_end == Some(true) { " " } else { "" })).collect::<String>().trim().to_string()
}

#[test]
fn devanagari_reads_in_kathmandu_pronunciation() {
    let cases = [
        // the four-way stops and dental/retroflex
        ("ताल", "t̪al"),
        ("टाल", "ʈal"),
        ("बाल", "bal"),
        ("भाल", "bʱal"),
        ("काल", "kal"),
        ("खाल", "kʰal"),
        ("गाल", "gal"),
        ("घाल", "gʱal"),
        // schwa: final deleted, kept after a conjunct, after छ, in a one-syllable word, in -एन/-ऐन
        ("घर", "gʱʌɾ"),
        ("नेपाल", "nepal"),
        ("गर्छ", "gʌɾtsʰʌ"),
        ("हुन्छ", "ɦuntsʰʌ"),
        ("छ", "tsʰʌ"),
        ("म", "mʌ"),
        ("छैन", "tsʰʌinʌ"),
        ("गरेन", "gʌɾenʌ"),
        ("कृष्ण", "kɾisnʌ"),
        ("चलचित्र", "tsʌltsit̪ɾʌ"),
        // schwa: medial VC_CV
        ("सरकार", "sʌɾkaɾ"),
        ("जनता", "dzʌnt̪a"),
        ("बदलियो", "bʌd̪lijo"),
        ("कमल", "kʌmʌl"),
        ("नमस्ते", "nʌmʌst̪e"),
        ("नमस्कार", "nʌmʌskaɾ"),
        ("धन्यवाद", "d̪ʱʌnjʌbad̪"),
        // suffixes keep the stem's schwas a word's
        ("कलमहरू", "kʌlʌmɦʌɾu"),
        ("घरमा", "gʱʌɾma"),
        // nasals
        ("तपाईं", "t̪ʌpai\u{303}"),
        ("पाँच", "pa\u{303}ts"),
        ("संसार", "sʌ\u{303}saɾ"),
        ("पंखा", "pʌŋkʰa"),
        // clusters
        ("क्षेत्री", "tsʰet̪ɾi"),
        ("ज्ञान", "gjan"),
        ("ऋषि", "ɾisi"),
        ("स्वर्ग", "swʌɾgʌ"),
    ];
    let mut bad = Vec::new();
    for (w, want) in cases {
        let got = ipa(&word(w));
        if got != want {
            bad.push(format!("{w}: got {got}, want {want}"));
        }
    }
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

#[test]
fn stress_is_initial_unless_a_light_syllable_precedes_a_heavy_one() {
    let stressed = |w: &str| word(w).iter().filter(|s| s.stress == Some(1)).map(|s| s.ipa.clone()).collect::<Vec<_>>();
    assert_eq!(stressed("नेपाल"), ["a"]); // ne.pal: light then heavy
    assert_eq!(stressed("सरकार"), ["ʌ"]); // sʌɾ.kaɾ: heavy first
    assert_eq!(stressed("खाना"), ["a"]);
    assert_eq!(word("खाना").iter().filter(|s| s.stress == Some(1)).count(), 1);
}

#[test]
fn numbers_are_nepali_words() {
    assert_eq!(ipa(&number("५")), "pa\u{303}ts");
    assert_eq!(ipa(&number("12")), ipa(&word("बाह्र")));
    assert_eq!(ipa(&number("१००")), ["एक", "शून्य", "शून्य"].map(|w| ipa(&word(w))).join(" "));
}

#[test]
fn phrases_split_on_danda_and_read_latin_as_english() {
    let en = EnglishG2P::new();
    let p = nepali_phrases("मेरो नाम राम हो। तपाईंको नाम के हो?", &en);
    assert_eq!(p.len(), 2);
    assert_eq!(p[0].words, ["मेरो", "नाम", "राम", "हो"]);
    assert_eq!(p[1].intonation, chimera::phonetics::Intonation::Question);
    let mixed = nepali_phrases("मलाई computer मन पर्छ", &en);
    assert_eq!(mixed[0].words, ["मलाई", "computer", "मन", "पर्छ"]);
    assert!(mixed[0].segments.iter().any(|s| s.ipa == "ɝ" || s.ipa == "ɚ"), "English vowels in the Latin word: {}", ipa(&mixed[0].segments));
}

#[test]
fn nepali_ipa_keeps_aspiration_breath_dental_and_nasal_vowels() {
    let segs = tokenize_ipa_lang("pʰa bʱa b̤a t̪a d̪ʱa tsʰa ã ʌ̃ kʰʌ", Lang::Nepali);
    let got: Vec<&str> = segs.iter().map(|s| s.ipa.as_str()).collect();
    assert_eq!(got, ["pʰ", "a", "bʱ", "a", "bʱ", "a", "t̪", "a", "d̪ʱ", "a", "tsʰ", "a", "a\u{303}", "ʌ\u{303}", "kʰ", "ʌ"]);
    // modifiers a segment can't carry are dropped: aspirated vowel, nasal stop, dental k
    let odd: Vec<String> = tokenize_ipa_lang("aʰ k̪ m\u{303}", Lang::Nepali).into_iter().map(|s| s.ipa).collect();
    assert_eq!(odd, ["a", "k", "m"]);
    // English is untouched: modifiers dropped, a precomposed ã dropped whole, as before
    let en: Vec<String> = tokenize_ipa("pʰa t̪a ã").into_iter().map(|s| s.ipa).collect();
    assert_eq!(en, ["p", "a", "t", "a"]);
}

fn abduction_peak_after(plan: &chimera::phonetics::Plan, release: f64) -> f64 {
    plan.gestures.iter().filter(|g| g.targets.get(ControlKey::ABDUCTION).is_some() && g.t0 < release && g.t1 + g.ramp_out > release).map(|g| g.targets.get(ControlKey::ABDUCTION).unwrap()).fold(0.0, f64::max)
}

#[test]
fn nepali_plain_stops_are_unaspirated_and_aspirates_are_wider() {
    let o = |lang| PlanOptions { body: BodyMix::MALE, t0: 0.0, f0: Some(110.0), lang, ..Default::default() };
    let plain = plan_utterance(&tokenize_ipa_lang("pal", Lang::Nepali), &o(Some(Lang::Nepali)));
    let asp = plan_utterance(&tokenize_ipa_lang("pʰal", Lang::Nepali), &o(Some(Lang::Nepali)));
    let english = plan_utterance(&tokenize_ipa("pal"), &o(None));
    let rel = |p: &chimera::phonetics::Plan| p.segments[0].t1;
    let (a_plain, a_asp, a_en) = (abduction_peak_after(&plain, rel(&plain)), abduction_peak_after(&asp, rel(&asp)), abduction_peak_after(&english, rel(&english)));
    assert!(a_plain < a_en && a_en < a_asp, "plain {a_plain}, English {a_en}, aspirated {a_asp}");
    assert_eq!(asp.segments[0].ipa, "pʰ");
}

#[test]
fn nepali_breathy_stops_murmur_into_the_vowel_and_nasal_vowels_lower_the_velum() {
    let o = PlanOptions { body: BodyMix::MALE, t0: 0.0, f0: Some(110.0), lang: Some(Lang::Nepali), ..Default::default() };
    let b = plan_utterance(&tokenize_ipa_lang("bʱal", Lang::Nepali), &o);
    assert!(b.gestures.iter().any(|g| g.label.as_ref().is_some_and(|l| l.as_str() == "breathy release") && g.t0 < b.segments[0].t1 && g.t1 > b.segments[0].t1));
    let n = plan_utterance(&tokenize_ipa_lang("pãts", Lang::Nepali), &o);
    let v = &n.segments[1];
    assert!(n.gestures.iter().any(|g| g.label.as_ref().is_some_and(|l| l.as_str() == "nasal vowel") && g.t0 <= v.t0 && g.t1 >= v.t1));
}

/// The reader against `tools/nepali/testset.tsv`'s expected IPA (the words a native speaker
/// checked). A mismatch is a reader bug or a word for `nepaliExceptions.tsv`.
#[test]
fn reader_matches_the_test_set() {
    let set = include_str!("../../../tools/nepali/testset.tsv");
    let mut bad = Vec::new();
    let mut n = 0;
    for line in set.lines().filter(|l| !l.starts_with('#') && !l.trim().is_empty()) {
        let f: Vec<&str> = line.split('\t').collect();
        let (Some(text), Some(want)) = (f.get(1), f.get(2)) else { continue };
        n += 1;
        let got = ipa(&word(text));
        let want = ipa(&tokenize_ipa_lang(want, Lang::Nepali));
        if got != want {
            bad.push(format!("{text}: reader {got}, test set {want}"));
        }
    }
    assert!(n > 30, "{n} words");
    assert!(bad.is_empty(), "{} of {n} differ:\n{}", bad.len(), bad.join("\n"));
}

/// The dental closure (its template was never used before Nepali) seals like the alveolar and
/// retroflex ones: the middle of a voiceless closure is as quiet as theirs.
#[test]
fn the_dental_stop_seals() {
    use chimera::anatomy::human::{BODY_FEMALE, BODY_MALE};
    use chimera::voice::human_voice::{HumanVoice, SourcePatch};
    const FS: f64 = 48000.0;
    let rms = |x: &[f32]| (x.iter().map(|v| (*v as f64).powi(2)).sum::<f64>() / x.len() as f64).sqrt();
    for (body, name, f0) in [(BODY_MALE, "male", 112.0), (BODY_FEMALE, "female", 205.0)] {
        let quiet = |c: &str| {
            let o = PlanOptions { body: name.into(), t0: 0.05, f0: Some(f0), lang: Some(Lang::Nepali), ..Default::default() };
            let plan = plan_utterance(&tokenize_ipa_lang(&format!("ˈa{c}a"), Lang::Nepali), &o);
            let mut v = HumanVoice::new(FS, body, 2);
            v.set_source(&SourcePatch::f0(f0));
            v.schedule(&plan.gestures);
            let mut out = vec![0.0f32; (FS * (plan.end + 0.2)).round() as usize];
            for chunk in out.chunks_mut(128) {
                v.render(chunk);
            }
            let (a, cl) = (&plan.segments[0], &plan.segments[1]);
            let at = |t: f64| (t * FS).round() as usize;
            let mid = (cl.t0 + cl.t1) / 2.0;
            rms(&out[at(mid - 0.012)..at(mid + 0.012)]) / rms(&out[at(a.t0 + 0.02)..at(a.t1 - 0.01)])
        };
        let (dental, alveolar, retroflex) = (quiet("t̪"), quiet("t"), quiet("ʈ"));
        println!("{name}: closure/vowel rms dental {dental:.4}, alveolar {alveolar:.4}, retroflex {retroflex:.4}");
        assert!(dental < 2.0 * alveolar.max(retroflex) && dental < 0.05, "{name}: dental {dental}, alveolar {alveolar}, retroflex {retroflex}");
    }
}
