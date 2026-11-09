"""Nepali intelligibility, scored by Whisper: render each item of tools/nepali/testset.tsv with
`chimera render text` (male and female), transcribe it with faster-whisper large-v3 on the GPU
(language "ne", greedy, so the same audio gives the same text) and count character errors
against the Devanagari that was said.

usage: nepali_cer.py <outdir> [--against earlier/cer.json] [--cpu]

Whisper is weak on Nepali even for real speakers, so the score is a relative signal (did a
change help?), not an absolute one. Characters are compared with spaces and punctuation removed.
Set up once with `tools/asr/setup.sh --gpu`.
"""

import json
import os
import re
import subprocess
import sys
import wave
from concurrent.futures import ThreadPoolExecutor

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(os.path.dirname(HERE))
CACHE = os.path.expanduser("~/.cache/chimera-asr/models")
BODIES = ("male", "female")
# character error rate (points) a change may add before it counts as a regression, and the fall
# that counts as an improvement
SLACK = 3.0
MARGIN = 2.0


def cuda_libs():
    """The pip CUDA wheels' library dirs (ctranslate2 dlopens cuBLAS and cuDNN)."""
    import site
    dirs = []
    for sp in site.getsitepackages():
        for lib in ("cublas", "cudnn"):
            d = os.path.join(sp, "nvidia", lib, "lib")
            if os.path.isdir(d):
                dirs.append(d)
    return dirs


def reexec_with_cuda():
    dirs = cuda_libs()
    have = os.environ.get("LD_LIBRARY_PATH", "").split(":")
    if dirs and not all(d in have for d in dirs):
        os.environ["LD_LIBRARY_PATH"] = ":".join(dirs + [p for p in have if p])
        os.execv(sys.executable, [sys.executable] + sys.argv)


def chars(text):
    """Devanagari letters and signs only (no spaces, dandas, digits or Latin)."""
    return [c for c in text if "ऀ" <= c <= "ॣ" or "ॱ" <= c <= "ॿ"]


def edits(ref, hyp):
    d = list(range(len(hyp) + 1))
    for i, r in enumerate(ref, 1):
        prev, d[0] = d[0], i
        for j, h in enumerate(hyp, 1):
            prev, d[j] = d[j], min(d[j] + 1, d[j - 1] + 1, prev + (r != h))
    return d[len(hyp)]


def load16k(path):
    import numpy as np
    with wave.open(path) as w:
        ch, rate = w.getnchannels(), w.getframerate()
        x = np.frombuffer(w.readframes(w.getnframes()), dtype="<i2").astype(np.float64) / 32768.0
    x = x.reshape(-1, ch).mean(axis=1)
    n = int(round(len(x) * 16000 / rate))
    return (np.fft.irfft(np.fft.rfft(x)[: n // 2 + 1], n) * (n / len(x))).astype(np.float32)


def listening_page(out, rows, jobs):
    """<outdir>/index.html: every render with what was said and what Whisper heard."""
    import html
    trs = []
    for r, (_, _, _, stem) in zip(rows, jobs):
        wav = os.path.basename(stem) + ".wav"
        trs.append(f"<tr><td>{html.escape(r['kind'])}</td><td>{r['body']}</td><td class=t>{html.escape(r['text'])}</td>"
                   f"<td><audio controls preload=none src='{wav}'></audio></td><td class=t>{html.escape(r['heard'])}</td>"
                   f"<td>{r['errors']}/{r['chars']}</td></tr>")
    page = ("<!doctype html><meta charset=utf-8><title>CHIMERA Nepali</title><style>body{font:14px system-ui;margin:2em}"
            "td{padding:4px 10px;border-bottom:1px solid #ddd}.t{font-size:18px}</style><h1>CHIMERA Nepali renders</h1>"
            "<table><tr><th>kind<th>body<th>said<th>listen<th>Whisper heard<th>errors</tr>" + "".join(trs) + "</table>")
    with open(os.path.join(out, "index.html"), "w", encoding="utf-8") as f:
        f.write(page)


def main():
    args = sys.argv[1:]
    cpu = "--cpu" in args
    if cpu:
        args.remove("--cpu")
    else:
        reexec_with_cuda()
    from faster_whisper import WhisperModel

    against = None
    if "--against" in args:
        i = args.index("--against")
        against = args[i + 1]
        del args[i : i + 2]
    out = args[0]
    os.makedirs(out, exist_ok=True)
    items = []
    for line in open(os.path.join(REPO, "tools/nepali/testset.tsv"), encoding="utf-8"):
        if line.startswith("#") or not line.strip():
            continue
        kind, text = line.rstrip("\n").split("\t")[:2]
        items.append((kind, text))
    chimera = os.path.join(REPO, "target/release/chimera")
    jobs = [(kind, text, body, os.path.join(out, f"{kind}-{body}-{i + 1:02d}"))
            for i, (kind, text) in enumerate(items) for body in BODIES]

    def render(job):
        _, text, body, stem = job
        subprocess.run([chimera, "render", "text", stem, text, body], check=True, capture_output=True)

    with ThreadPoolExecutor(4) as pool:
        list(pool.map(render, jobs))

    model = WhisperModel("large-v3", device="cpu" if cpu else "cuda", compute_type="int8" if cpu else "float16",
                         download_root=CACHE)
    rows, totals = [], {}
    for kind, text, body, stem in jobs:
        segs, _ = model.transcribe(load16k(stem + ".wav"), language="ne", beam_size=1, temperature=0.0,
                                   condition_on_previous_text=False, vad_filter=False)
        heard = " ".join(s.text.strip() for s in segs)
        ref = chars(text)
        e = edits(ref, chars(heard))
        rows.append({"kind": kind, "body": body, "text": text, "heard": heard, "errors": e, "chars": len(ref)})
        for key in (f"{kind}-{body}", "all"):
            t = totals.setdefault(key, [0, 0])
            t[0] += e
            t[1] += len(ref)
    cer = {k: round(100.0 * e / n, 2) for k, (e, n) in totals.items()}
    with open(os.path.join(out, "cer.json"), "w", encoding="utf-8") as f:
        json.dump({"cer": cer, "items": rows}, f, indent=1, ensure_ascii=False)
    listening_page(out, rows, jobs)
    for r in rows:
        mark = "  " if r["errors"] == 0 else "✗ "
        print(f"{mark}{r['kind']:7s} {r['body']:6s} {r['errors']}/{r['chars']}  {r['text']}  →  {r['heard']}")
    print("character error rate (%): " + ", ".join(f"{k} {v}" for k, v in sorted(cer.items())))
    if against:
        old, new = json.load(open(against, encoding="utf-8"))["cer"]["all"], cer["all"]
        verdict = "WORSE" if new > old + SLACK else "BETTER" if new <= old - MARGIN else "NO CLEAR CHANGE"
        print(f"nepali verdict against {against}: {verdict} ({old} → {new} %, better from -{MARGIN}, worse past +{SLACK})")


main()
