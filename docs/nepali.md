# Nepali in Speak mode

Status: approved 2026-11-09; v1 built on branch `nepali`. Test set awaiting native-speaker correction.

## Goal

Type Nepali in Speak mode, as Devanagari or IPA, and hear it in standard Kathmandu
pronunciation with every phonemic contrast. English output stays bit-identical.

## Decisions

| Topic | Decision |
|---|---|
| Scope | Speak mode: Devanagari and IPA. Sing is out of scope for v1 |
| Repo | Personal fork `GauravRajSharma/chimera-vocal-synth`, `jamesondh` kept as `upstream`. No upstream PR planned |
| Language UI | English/Nepali picker in Speak. Devanagari input switches it to Nepali |
| English | All Nepali behaviour gated on the language. English goldens must pass unchanged, never re-blessed |
| Dialect | Standard Kathmandu: च=ts, छ=tsʰ, ज=dz, झ=dzʱ, श ष स=s, inherent vowel ʌ, व=w or b by context |
| Fidelity | Full: 4-way stops, dental vs retroflex, nasal vowels |
| Schwa | Rules plus a growing exception list |
| Prosody | Mild initial stress (second syllable when the first is light and the second heavy), existing contours |
| Mixed text | Latin words go through the English reader. Devanagari digits are read as Nepali numbers 0 to 99, larger numbers digit by digit |
| Judge | Your ears, plus faster-whisper `large-v3` on the GPU as a relative intelligibility score |

## Sound inventory work (engine)

| Sound | Change |
|---|---|
| t̪ t̪ʰ d̪ d̪ʱ n̪ | New dental entries using the unused `DENT_STOP` template. The tokenizer keeps the ̪ mark instead of dropping it |
| ʈ ɖ | Already present. Gain ʰ and ʱ forms |
| Cʰ (pʰ tʰ ʈʰ kʰ tsʰ) | `ʰ` becomes a token modifier. Nepali plan: long-VOT abduction gesture at release |
| C (p t ʈ k ts) | Nepali plan: short VOT always, no English aspiration rule |
| Cʱ (bʱ dʱ ɖʱ gʱ dzʱ) | `ʱ` (and ̤) becomes a modifier. Voiced closure, then a breathy abduction (about the size of ɦ's) through the release into the vowel |
| Ṽ (ã ĩ ũ ẽ õ ʌ̃) | The tilde becomes a modifier. A velum-lowering gesture over the vowel |
| Vowels i e a ʌ o u | Use the existing calibrated targets. Nepali a and ʌ are checked by ear, recalibrated only if wrong |

## Devanagari reader (`phonetics/nepali.rs`)

- Letters to IPA by table: consonants, matras, independent vowels, halant, conjuncts.
- Anusvara ं is a homorganic nasal before a stop, otherwise vowel nasalisation. Chandrabindu ँ nasalises the vowel. Visarga ः is mostly silent or lengthening, per modern speech.
- Schwa deletion: drop it word-finally, except after conjuncts and in listed verb forms (गर्छ). Drop it medially in VC_CV. An exception list in a plain file overrides both rules.
- Stress and syllables feed the existing planner.

## Planner

`plan_speech` gains a language option. With Nepali, ʰ/ʱ/̃ segments plan as described above and the English aspiration rule is off. With English, code paths are unchanged.

## UI

- Speak screen: a language picker. Devanagari input switches it to Nepali.
- The consonant chart shows the Nepali inventory when Nepali is selected.

## Testing

- English goldens unchanged (the gate).
- New goldens: Nepali plans and audio for a few phrases.
- Unit tests: Devanagari to IPA for the test set below, and tokenizer modifiers.
- Test set, `tools/nepali/testset.tsv`: about 40 words and 15 phrases with expected IPA. Covers minimal pairs (ताल/टाल, बाल/भाल, काल/खाल, गाल/घाल), nasal vowels, conjuncts and schwa cases. I draft it, you correct it before use.
- `tools/asr/nepali_wer.py`: renders the set, transcribes it with large-v3 (`language="ne"`, CUDA), and reports CER per item. The venv and model live under `~/.cache/chimera-asr`. The CUDA libraries come from pip wheels in that venv, so nothing system-wide changes.
- Listening page: you rate the renders and flag errors, which feed the exception list and tuning.

## Constraints

- Builds follow the memory limits: one cargo at a time, `-j 2`, `debug=0`.
- The repo rules from `AGENTS.md`: engine math contract, no allocation while rendering, gesture record changes made in both `lib.rs` and `wire.ts` with a `LAYOUT_VERSION` bump.

## Out of scope for v1

Sing mode lyrics, other languages, Vercel deploy of the fork, and Nepali judged against real recorded speakers.
