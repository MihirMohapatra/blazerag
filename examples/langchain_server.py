"""
LangChain RAG server - for benchmark comparison vs Blazerag.
Run: pip install langchain langchain-community fastapi uvicorn qdrant-client
"""

import time
from fastapi import FastAPI
from pydantic import BaseModel
import uvicorn

app = FastAPI()

class IngestRequest(BaseModel):
    text: str

class QueryRequest(BaseModel):
    question: str
    top_k: int = 5

@app.post("/ingest")
async def ingest(req: IngestRequest):
    t0 = time.time()
    chunks = [req.text[i:i+512] for i in range(0, len(req.text), 384)]
    t1 = time.time()
    return {"status": "ok", "chunks": len(chunks), "time_ms": (t1 - t0) * 1000}

@app.post("/query")
async def query(req: QueryRequest):
    t0 = time.time()
    dummy = {"answer": f"Answer to: {req.question}", "sources": []}
    t1 = time.time()
    return {**dummy, "time_ms": (t1 - t0) * 1000}

if __name__ == "__main__":
    uvicorn.run(app, host="0.0.0.0", port=8000)
