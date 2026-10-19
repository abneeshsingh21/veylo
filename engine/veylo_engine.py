import os
import sys
import json
import time
import io
import tempfile
from http.server import HTTPServer, BaseHTTPRequestHandler
from pywhispercpp.model import Model
from llama_cpp import Llama

HOST = "127.0.0.1"
PORT = 48123

SYSTEM_PROMPT = (
    "You are Veylo's intelligent speech post-processor. Clean the raw spoken transcription: "
    "remove filler words, fix self-corrections, punctuate properly, handle Hinglish naturally, "
    "and format code or emails cleanly. Output ONLY the polished text without explanation."
)

class VeyloEngine:
    def __init__(self):
        print("[Veylo Engine] Initializing models...")
        
        # 1. Load Whisper STT Model (fast CPU base model by default, or large-v3-turbo if set)
        model_name = os.environ.get("VEYLO_WHISPER_MODEL", "ggml-base.bin")
        whisper_model_path = os.path.join("models", model_name)
        if not os.path.exists(whisper_model_path):
            whisper_model_path = os.path.join("models", "ggml-large-v3-turbo-q5_0.bin")
        if not os.path.exists(whisper_model_path):
            whisper_model_path = os.path.join("models", "ggml-base.bin")
            
        print(f"[Veylo Engine] Loading Whisper model from: {whisper_model_path}")
        t0 = time.time()
        self.whisper = Model(whisper_model_path, n_threads=4)
        print(f"[Veylo Engine] Whisper loaded in {time.time() - t0:.2f}s")
        
        # 2. Load Fine-Tuned Veylo Qwen SLM
        slm_model_path = os.path.join("models", "veylo-qwen-0.5b-q4_k_m.gguf")
        print(f"[Veylo Engine] Loading Veylo SLM from: {slm_model_path}")
        t1 = time.time()
        self.slm = Llama(
            model_path=slm_model_path,
            n_ctx=512,
            n_threads=4,
            verbose=False
        )
        print(f"[Veylo Engine] Veylo SLM loaded in {time.time() - t1:.2f}s")
        print("[Veylo Engine] All AI models initialized and ready!")

    def process_audio(self, wav_bytes, is_code=False, is_email=False, is_chat=False):
        t_start = time.time()
        
        # Save temporary WAV to disk for pywhispercpp
        with tempfile.NamedTemporaryFile(suffix=".wav", delete=False) as tmp:
            tmp.write(wav_bytes)
            tmp_path = tmp.name
            
        try:
            # Stage 1: Whisper Acoustic Transcription (language="" triggers auto-detect)
            t_w = time.time()
            segments = self.whisper.transcribe(tmp_path, language="")
            raw_parts = []
            for seg in segments:
                t = seg.text.replace("[BLANK_AUDIO]", "").strip()
                if t:
                    raw_parts.append(t)
            raw_text = " ".join(raw_parts).strip()
            whisper_dur = time.time() - t_w
            
            if not raw_text:
                return {"status": "empty", "raw": "", "text": "", "duration_ms": 0}
                
            print(f"[Veylo Engine] Whisper ({whisper_dur:.2f}s): \"{raw_text}\"")
            
            # Stage 2: Context-Aware Post-Processing via Fine-Tuned SLM
            t_s = time.time()
            context_note = ""
            if is_code:
                context_note = "\nContext: Code Editor (convert spoken words to syntax, camelCase/snake_case)."
            elif is_email:
                context_note = "\nContext: Business Email (format paragraphs and sign-offs)."
            elif is_chat:
                context_note = "\nContext: Chat Message (preserve natural conversational Hinglish)."
                
            prompt = f"{raw_text}{context_note}" if context_note else raw_text
            
            response = self.slm.create_chat_completion(
                messages=[
                    {"role": "system", "content": SYSTEM_PROMPT},
                    {"role": "user", "content": prompt}
                ],
                max_tokens=128,
                temperature=0.1
            )
            clean_text = response["choices"][0]["message"]["content"].strip()
            slm_dur = time.time() - t_s
            print(f"[Veylo Engine] Veylo SLM ({slm_dur:.2f}s): \"{clean_text}\"")
            
            total_dur = (time.time() - t_start) * 1000
            return {
                "status": "ok",
                "raw": raw_text,
                "text": clean_text,
                "duration_ms": round(total_dur, 1)
            }
        finally:
            if os.path.exists(tmp_path):
                os.remove(tmp_path)

ENGINE = None

class EngineHandler(BaseHTTPRequestHandler):
    def log_message(self, format, *args):
        # Suppress noisy HTTP request logging
        pass

    def do_GET(self):
        if self.path == "/health":
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.end_headers()
            self.wfile.write(b'{"status": "ready"}\n')
        else:
            self.send_response(404)
            self.end_headers()

    def do_POST(self):
        if self.path.startswith("/transcribe"):
            content_length = int(self.headers.get("Content-Length", 0))
            wav_data = self.rfile.read(content_length)
            
            # Parse context headers
            is_code = self.headers.get("X-Context-Code", "0") == "1"
            is_email = self.headers.get("X-Context-Email", "0") == "1"
            is_chat = self.headers.get("X-Context-Chat", "0") == "1"
            
            result = ENGINE.process_audio(
                wav_data,
                is_code=is_code,
                is_email=is_email,
                is_chat=is_chat
            )
            
            resp_body = json.dumps(result).encode("utf-8")
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(resp_body)))
            self.end_headers()
            self.wfile.write(resp_body)
        else:
            self.send_response(404)
            self.end_headers()

def main():
    global ENGINE
    print("======================================================")
    print("           Veylo AI Local Inference Engine            ")
    print("      Persistent Whisper + Qwen SLM Loopback          ")
    print("======================================================")
    
    ENGINE = VeyloEngine()
    server = HTTPServer((HOST, PORT), EngineHandler)
    print(f"\n[Veylo Engine] Server listening on http://{HOST}:{PORT}")
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        print("\n[Veylo Engine] Stopping server...")
        server.server_close()

if __name__ == "__main__":
    main()
