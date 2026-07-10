#!/usr/bin/env python3
"""
Local CPU text-to-music via MusicGen (transformers). Unlimited, free, no quota,
no external dependency — runs entirely inside the backend container.

Usage:  music_gen.py <prompt> <out_path> [duration_seconds]
Env:    MUSICGEN_MODEL    default facebook/musicgen-small
        MUSICGEN_THREADS  default 8   (CPU threads for torch)

Prints "OK <bytes>" to stdout on success (exit 0); diagnostics to stderr.
Writes MP3 if ffmpeg is available, else WAV at the same path.
"""
import os
import sys
import subprocess


def log(msg):
    print(msg, file=sys.stderr, flush=True)


def main():
    if len(sys.argv) < 3:
        log("usage: music_gen.py <prompt> <out> [seconds]")
        sys.exit(2)
    prompt, out = sys.argv[1], sys.argv[2]
    seconds = float(sys.argv[3]) if len(sys.argv) > 3 else 10.0
    seconds = max(2.0, min(30.0, seconds))

    import numpy as np
    import torch
    from transformers import AutoProcessor, MusicgenForConditionalGeneration
    import scipy.io.wavfile as wav

    torch.set_num_threads(int(os.environ.get("MUSICGEN_THREADS", "8")))
    model_id = os.environ.get("MUSICGEN_MODEL", "facebook/musicgen-small")

    log(f"music_gen: loading {model_id}")
    proc = AutoProcessor.from_pretrained(model_id)
    model = MusicgenForConditionalGeneration.from_pretrained(model_id)

    inputs = proc(text=[prompt], padding=True, return_tensors="pt")
    tokens = int(seconds * 50)  # MusicGen emits ~50 audio tokens/sec
    log(f"music_gen: generating ~{seconds:.0f}s ({tokens} tokens)")
    audio = model.generate(**inputs, max_new_tokens=tokens, do_sample=True, guidance_scale=3.0)

    sr = model.config.audio_encoder.sampling_rate
    arr = audio[0, 0].cpu().numpy()
    arr16 = np.clip(arr, -1.0, 1.0)
    arr16 = (arr16 * 32767.0).astype(np.int16)

    wav_path = out + ".tmp.wav"
    wav.write(wav_path, rate=sr, data=arr16)

    # Transcode to MP3 when ffmpeg is present; otherwise keep the WAV bytes.
    ok_mp3 = False
    try:
        r = subprocess.run(
            ["ffmpeg", "-y", "-loglevel", "error", "-i", wav_path, "-b:a", "192k", out],
            capture_output=True,
        )
        ok_mp3 = r.returncode == 0 and os.path.exists(out) and os.path.getsize(out) > 100
        if not ok_mp3:
            log(f"music_gen: ffmpeg fallback ({r.stderr.decode('utf-8','replace')[-200:]})")
    except FileNotFoundError:
        log("music_gen: ffmpeg not found, writing WAV")

    if not ok_mp3:
        wav.write(out, rate=sr, data=arr16)  # out path holds WAV bytes
    try:
        os.remove(wav_path)
    except OSError:
        pass

    print(f"OK {os.path.getsize(out)}")


if __name__ == "__main__":
    main()
