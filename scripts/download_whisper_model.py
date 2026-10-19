import os
import urllib.request
import sys

MODELS = {
    "large-v3-turbo-q4": {
        "url": "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-turbo-q4_0.bin",
        "filename": "ggml-large-v3-turbo-q4_0.bin",
        "size_mb": 390
    },
    "base": {
        "url": "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin",
        "filename": "ggml-base.bin",
        "size_mb": 140
    }
}

def download_with_progress(url, dest_path):
    print(f"[Veylo Model Downloader] Downloading from: {url}")
    print(f"[Veylo Model Downloader] Destination: {dest_path}")
    
    def reporthook(block_num, block_size, total_size):
        read = block_num * block_size
        if total_size > 0:
            percent = min(100.0, read * 100.0 / total_size)
            mb_read = read / (1024 * 1024)
            mb_total = total_size / (1024 * 1024)
            sys.stdout.write(f"\rDownloading: {percent:.1f}% ({mb_read:.1f}/{mb_total:.1f} MB)")
            sys.stdout.flush()
            
    urllib.request.urlretrieve(url, dest_path, reporthook)
    print("\n[Veylo Model Downloader] Download complete!")

def main():
    os.makedirs("models", exist_ok=True)
    model_choice = "large-v3-turbo-q4" if len(sys.argv) < 2 else sys.argv[1]
    
    if model_choice not in MODELS:
        print(f"Unknown model choice: {model_choice}. Available: {list(MODELS.keys())}")
        return
        
    info = MODELS[model_choice]
    dest = os.path.join("models", info["filename"])
    
    if os.path.exists(dest):
        print(f"[Veylo Model Downloader] {dest} already exists ({os.path.getsize(dest)/(1024*1024):.1f} MB). Skipping.")
        return
        
    download_with_progress(info["url"], dest)

if __name__ == "__main__":
    main()
