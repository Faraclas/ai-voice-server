import asyncio
import json
import websockets
import urllib.request
import os

SERVER_WS = "ws://127.0.0.1:3000/stream"
SERVER_HTTP = "http://127.0.0.1:3000"
AUDIO_FILE = "../python-prototype/server/test_audio.wav"

async def test_health_endpoint():
    print("--- Testing /health Endpoint ---")
    try:
        req = urllib.request.Request(f"{SERVER_HTTP}/health")
        with urllib.request.urlopen(req) as response:
            data = json.loads(response.read().decode())
            print(f"Health check response: {data}")
            assert "status" in data
    except Exception as e:
        print(f"Health check failed: {e}")
    print("--------------------------------\n")

async def test_websocket_stream():
    print("--- Testing WebSocket /stream ---")
    
    if not os.path.exists(AUDIO_FILE):
        print(f"Test audio file missing at {AUDIO_FILE}")
        return

    async with websockets.connect(SERVER_WS) as websocket:
        print("Connected to WebSocket.")
        
        # 1. Test Dynamic Model Download
        print("Requesting model 'small.en'...")
        await websocket.send(json.dumps({"action": "set_model", "model": "small.en"}))
        
        while True:
            response = await websocket.recv()
            data = json.loads(response)
            if data.get("status") == "downloading":
                print(f"Downloading model: {data.get('progress_pct', 0)}%")
            elif data.get("status") == "success":
                print("Model loaded successfully!")
                break
            elif "error" in data:
                print(f"Error loading model: {data['error']}")
                return

        # 2. Test Audio Streaming
        print("Streaming audio binary chunks...")
        with open(AUDIO_FILE, "rb") as f:
            while chunk := f.read(4096):
                await websocket.send(chunk)
                await asyncio.sleep(0.01) # Simulate real-time streaming
        
        print("Sent all audio data. Sending end_stream signal...")
        await websocket.send(json.dumps({"action": "end_stream"}))
        
        # 3. Await Transcription
        response = await websocket.recv()
        data = json.loads(response)
        
        if "text" in data:
            print(f"Transcription Received: '{data['text']}'")
            print(f"Processing time: {data['processing_time_ms']}ms")
        else:
            print(f"Unexpected response: {data}")
            
    print("--------------------------------\n")

async def main():
    await test_health_endpoint()
    await test_websocket_stream()

if __name__ == "__main__":
    asyncio.run(main())
