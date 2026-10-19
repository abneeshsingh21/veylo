import os
import sys
import requests
from kaggle.api.kaggle_api_extended import KaggleApi
from kagglesdk.kernels.services.kernels_api_service import ApiListKernelSessionOutputRequest

def download_gguf():
    print("[Veylo Model Pipeline] Authenticating with Kaggle API...")
    api = KaggleApi()
    api.authenticate()
    
    os.makedirs("models", exist_ok=True)
    target_path = os.path.join("models", "veylo-qwen-0.5b-q4_k_m.gguf")
    
    print("[Veylo Model Pipeline] Locating GGUF Q4_K_M artifact in kernel outputs...")
    gguf_url = None
    gguf_filename = None
    
    with api.build_kaggle_client() as kaggle:
        token = None
        while True:
            req = ApiListKernelSessionOutputRequest()
            req.user_name = "abneeshsingh"
            req.kernel_slug = "veylo-slm-cleaner"
            if token:
                req.page_token = token
            resp = kaggle.kernels.kernels_api_client.list_kernel_session_output(req)
            for item in resp.files:
                if item.file_name.lower().endswith(".gguf"):
                    gguf_url = item.url
                    gguf_filename = item.file_name
                    break
            if gguf_url:
                break
            token = getattr(resp, "next_page_token", None)
            if not token:
                break

    if not gguf_url:
        print("[Veylo Model Pipeline] Error: Could not find .gguf file in kernel output.")
        return

    print(f"[Veylo Model Pipeline] Found: {gguf_filename}")
    print(f"[Veylo Model Pipeline] Streaming directly to {target_path}...")
    
    response = requests.get(gguf_url, stream=True)
    total_size = int(response.headers.get("content-length", 0))
    chunk_size = 1024 * 1024 # 1 MB chunks
    downloaded = 0
    
    with open(target_path, "wb") as f:
        for chunk in response.iter_content(chunk_size=chunk_size):
            if chunk:
                f.write(chunk)
                downloaded += len(chunk)
                if total_size > 0:
                    pct = min(100.0, downloaded * 100.0 / total_size)
                    mb_cur = downloaded / (1024 * 1024)
                    mb_tot = total_size / (1024 * 1024)
                    sys.stdout.write(f"\rDownloading Veylo GGUF: {pct:.1f}% ({mb_cur:.1f}/{mb_tot:.1f} MB)")
                    sys.stdout.flush()

    print(f"\n[Veylo Model Pipeline] Download complete! Saved to {target_path} ({os.path.getsize(target_path)/(1024*1024):.1f} MB)")

if __name__ == "__main__":
    download_gguf()
