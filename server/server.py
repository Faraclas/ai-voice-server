import os
import time
import tempfile
from contextlib import asynccontextmanager
from fastapi import FastAPI, UploadFile, File, HTTPException
from pydantic import BaseModel
from faster_whisper import WhisperModel

model = None
current_model_size = None

@asynccontextmanager
async def lifespan(app: FastAPI):
    global model, current_model_size
    current_model_size = os.environ.get("WHISPER_MODEL", "small.en")
    
    print(f"Loading default model '{current_model_size}' on startup...")
    model = WhisperModel(current_model_size, device="cuda", compute_type="float16")
    print("Ready for requests!")
    yield
    print("Shutting down...")

app = FastAPI(title="Voice-to-Text API", lifespan=lifespan)

class ModelRequest(BaseModel):
    model_size: str

@app.post("/set_model")
async def set_model(request: ModelRequest):
    """Dynamically switch the loaded model over the network without restarting the server."""
    global model, current_model_size
    try:
        print(f"API Request received: Switching model to '{request.model_size}'...")
        start_time = time.time()
        
        # Load the new model into VRAM (this overwrites the old one, freeing its memory)
        model = WhisperModel(request.model_size, device="cuda", compute_type="float16")
        current_model_size = request.model_size
        
        return {
            "status": "success",
            "message": f"Successfully loaded {request.model_size}",
            "load_time_seconds": round(time.time() - start_time, 2)
        }
    except Exception as e:
        raise HTTPException(status_code=500, detail=f"Failed to load model: {str(e)}")

@app.get("/status")
async def get_status():
    """Check which model is currently loaded in VRAM."""
    return {"current_model": current_model_size, "status": "running"}

@app.post("/transcribe")
async def transcribe_audio(file: UploadFile = File(...)):
    """Transcribe an uploaded audio file using the currently loaded model."""
    if not file.filename.endswith((".wav", ".mp3", ".ogg")):
        raise HTTPException(status_code=400, detail="Unsupported file format.")
        
    start_time = time.time()
    
    with tempfile.NamedTemporaryFile(delete=False, suffix=".wav") as tmp:
        content = await file.read()
        tmp.write(content)
        tmp_path = tmp.name

    try:
        segments, info = model.transcribe(tmp_path, beam_size=5)
        full_text = " ".join([segment.text.strip() for segment in segments])
        
        return {
            "text": full_text,
            "language": info.language,
            "model_used": current_model_size,
            "processing_time_seconds": round(time.time() - start_time, 3)
        }
    except Exception as e:
        raise HTTPException(status_code=500, detail=str(e))
    finally:
        if os.path.exists(tmp_path):
            os.remove(tmp_path)
