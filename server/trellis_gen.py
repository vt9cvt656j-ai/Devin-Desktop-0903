#!/usr/bin/env python3
"""
Free neural image-to-3D via public Hugging Face ZeroGPU Spaces (TRELLIS).

Usage:  trellis_gen.py <image_url_or_path> <out_glb_path>
Env:    HF_API_KEY        required — your HF token (applies your ZeroGPU quota)
        TRELLIS_SPACES    optional — comma list of Spaces to try, in order
                          default: trellis-community/TRELLIS

Prints "OK <bytes> <space>" to stdout on success (exit 0).
Prints diagnostics to stderr; exits 1 on failure, 2 on config error.

Why a subprocess: gradio_client transparently handles the ZeroGPU session
handshake, file upload, SSE result stream and token→quota binding that raw
HTTP against a Gradio Space cannot do reliably (the Space writes its GLB into
a per-session temp dir created by start_session / demo.load).
"""
import os
import sys
import shutil


def log(msg):
    print(msg, file=sys.stderr, flush=True)


def _find_glb(x):
    """Recursively locate a .glb filepath anywhere in a Gradio result."""
    if isinstance(x, str) and x.endswith(".glb"):
        return x
    if isinstance(x, dict):
        for v in x.values():
            g = _find_glb(v)
            if g:
                return g
    if isinstance(x, (list, tuple)):
        for i in x:
            g = _find_glb(i)
            if g:
                return g
    return None


def generate(space, image_url, out_path, token):
    from gradio_client import Client, handle_file

    client = Client(space, token=token, verbose=False)

    # TRELLIS writes its GLB into TMP_DIR/<session_hash>, a directory created by
    # the Space's start_session handler (normally fired by demo.load on page
    # open). An API call skips that, so we trigger it explicitly first; the same
    # Client reuses one session_hash across calls, so the dir then exists.
    try:
        client.predict(api_name="/start_session")
    except Exception as e:  # non-fatal — some mirrors don't expose it
        log(f"start_session note: {e}")

    res = client.predict(
        handle_file(image_url),  # image prompt
        [],                       # multiimages
        0,                        # seed
        7.5,                      # ss_guidance_strength
        12,                       # ss_sampling_steps
        3.0,                      # slat_guidance_strength
        12,                       # slat_sampling_steps
        "stochastic",            # multiimage_algo
        0.95,                     # mesh_simplify
        1024,                     # texture_size
        api_name="/generate_and_extract_glb",
    )

    glb = _find_glb(res)
    if not glb or not os.path.exists(glb):
        raise RuntimeError(f"no GLB in result: {str(res)[:200]}")
    shutil.copy(glb, out_path)
    return os.path.getsize(out_path)


def main():
    if len(sys.argv) < 3:
        log("usage: trellis_gen.py <image_url> <out_glb>")
        sys.exit(2)
    image_url, out_path = sys.argv[1], sys.argv[2]

    token = os.environ.get("HF_API_KEY", "").strip()
    if not token:
        log("ERR: HF_API_KEY not set")
        sys.exit(2)

    spaces = [
        s.strip()
        for s in os.environ.get("TRELLIS_SPACES", "trellis-community/TRELLIS").split(",")
        if s.strip()
    ]

    last_err = ""
    for sp in spaces:
        try:
            log(f"neural3d: trying {sp}")
            size = generate(sp, image_url, out_path, token)
            print(f"OK {size} {sp}")
            sys.exit(0)
        except Exception as e:
            last_err = str(e)
            log(f"neural3d: {sp} failed: {e}")

    log(f"neural3d: ALL SPACES FAILED: {last_err}")
    sys.exit(1)


if __name__ == "__main__":
    main()
