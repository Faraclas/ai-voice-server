import os
import urllib.request
import time
from faster_whisper import WhisperModel

# A famous short audio clip of JFK speaking
AUDIO_URL = "https://raw.githubusercontent.com/ggerganov/whisper.cpp/master/samples/jfk.wav"
AUDIO_FILE = "test_audio.wav"

def download_sample_audio():
    if not os.path.exists(AUDIO_FILE):
        print(f"Downloading sample audio to {AUDIO_FILE}...")
        urllib.request.urlretrieve(AUDIO_URL, AUDIO_FILE)
        print("Download complete.\n")
    else:
        print(f"Using existing {AUDIO_FILE}.\n")

def test_transcription():
    print("Initializing Faster-Whisper...")
    print("The first run will download the 'base' model to your cache.")
    
    # device="cuda" forces it to use the NVIDIA GPU
    # compute_type="float16" is highly optimized for RTX 3000 series cards
    model_size = "small.en"
    
    load_start = time.time()
    model = WhisperModel(model_size, device="cuda", compute_type="float16")
    print(f"Model loaded into VRAM in {time.time() - load_start:.2f} seconds.\n")

    print(f"Transcribing {AUDIO_FILE}...")
    transcribe_start = time.time()
    
    # The transcribe method returns a generator for the segments
    segments, info = model.transcribe(AUDIO_FILE, beam_size=5)

    print(f"Detected language '{info.language}' with probability {info.language_probability:.2f}")
    
    print("\n--- Transcription ---")
    for segment in segments:
        print(f"[{segment.start:.2f}s -> {segment.end:.2f}s] {segment.text}")
    print("---------------------\n")
    
    print(f"Transcription completed in {time.time() - transcribe_start:.2f} seconds.")

if __name__ == "__main__":
    download_sample_audio()
    test_transcription()
