from llama_cpp import Llama
import time

SYSTEM_PROMPT = (
    "You are Veylo's intelligent speech post-processor. Clean the raw spoken transcription: "
    "remove filler words, fix self-corrections, punctuate properly, handle Hinglish naturally, "
    "and format code or emails cleanly. Output ONLY the polished text without explanation."
)

def test_model():
    print("[Veylo SLM Tester] Loading GGUF model into memory...")
    t0 = time.time()
    llm = Llama(
        model_path="models/veylo-qwen-0.5b-q4_k_m.gguf",
        n_ctx=512,
        n_threads=4,
        verbose=False
    )
    print(f"[Veylo SLM Tester] Model loaded in {time.time() - t0:.2f} seconds!")
    
    test_prompts = [
        "uh bhai sun kal meeting schedule kar dena shaam 5 baje client ke sath you know",
        "deploy to staging first wait no push directly to production",
        "create a function named get user by id taking user id as string and returning user",
        "so like basically um we should probably inspect the redis cache you know",
        "dear team new line we are excited to announce our release tomorrow at 4 pm thanks rahul"
    ]
    
    print("\n" + "=" * 65)
    print("        VEYLO SLM LOCAL CPU INFERENCE BENCHMARK")
    print("=" * 65)
    
    for raw in test_prompts:
        t_start = time.time()
        response = llm.create_chat_completion(
            messages=[
                {"role": "system", "content": SYSTEM_PROMPT},
                {"role": "user", "content": raw}
            ],
            max_tokens=64,
            temperature=0.1
        )
        elapsed = time.time() - t_start
        clean_text = response["choices"][0]["message"]["content"].strip()
        tok_speed = response["usage"]["completion_tokens"] / max(elapsed, 0.001)
        
        print(f"\nRAW SPEECH:  \"{raw}\"")
        print(f"POLISHED:    \"{clean_text}\"")
        print(f"LATENCY:     {elapsed * 1000:.1f} ms | SPEED: {tok_speed:.1f} tokens/sec")
        print("-" * 65)

if __name__ == "__main__":
    test_model()
