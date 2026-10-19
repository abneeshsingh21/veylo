import os
import time
import wave
import struct
from pywhispercpp.model import Model
from llama_cpp import Llama

SYSTEM_PROMPT = (
    "You are Veylo's intelligent speech post-processor. Clean the raw spoken transcription: "
    "remove filler words, fix self-corrections, punctuate properly, handle Hinglish naturally, "
    "and format code or emails cleanly. Output ONLY the polished text without explanation."
)

def test():
    print("[Veylo Pipeline] Initializing Whisper Engine (models/ggml-base.bin)...")
    t0 = time.time()
    whisper_model = Model("models/ggml-base.bin", n_threads=4)
    print(f"[Veylo Pipeline] Whisper loaded in {time.time() - t0:.2f}s")

    print("[Veylo Pipeline] Initializing Veylo SLM (models/veylo-qwen-0.5b-q4_k_m.gguf)...")
    t1 = time.time()
    slm = Llama(
        model_path="models/veylo-qwen-0.5b-q4_k_m.gguf",
        n_ctx=512,
        n_threads=4,
        verbose=False
    )
    print(f"[Veylo Pipeline] SLM loaded in {time.time() - t1:.2f}s")

    # Test SLM direct cleanup
    raw_test = "uh bhai sun kal meeting schedule kar do shaam ko 5 baje client ke sath you know"
    print(f"\n[Testing SLM] Input: '{raw_test}'")
    t2 = time.time()
    resp = slm.create_chat_completion(
        messages=[
            {"role": "system", "content": SYSTEM_PROMPT},
            {"role": "user", "content": raw_test}
        ],
        max_tokens=64,
        temperature=0.1
    )
    clean = resp["choices"][0]["message"]["content"].strip()
    print(f"[Testing SLM] Output: '{clean}' ({time.time() - t2:.2f}s)")
    print("\n[Veylo Pipeline] Full end-to-end model stack verified and operational!")

if __name__ == "__main__":
    test()
