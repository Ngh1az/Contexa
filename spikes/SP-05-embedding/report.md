# SP-05: Embedding Model Selection

**Date:** 2026-07-14  
**Owner:** —  
**Status:** Pass

## Summary

fastembed + all-MiniLM-L6-v2 (384-dim, in-process ONNX) clears every gate with wide margin: MRR@10 = 0.958 on a 20-query/100-chunk desktop-context dataset, 27 ms batch embed, +124 MB model memory. Ollama was not running on the test machine, so nomic-embed-text remains **documented** as the quality opt-in (ADR-0006) without fresh measurements — the gate only requires it documented.

## Results

| Metric | Target | Actual | Pass? |
|--------|--------|--------|-------|
| MRR@10 (fastembed default) | > 0.7 | **0.958** (hits@1: 19/20) | ✅ |
| Batch embed (10 chunks) | < 0.5 s | 27 ms (median of 5) | ✅ |
| Model memory (fastembed) | < 200 MB | +124 MB (10 → 134 MB RSS) | ✅ |
| MRR@10 (nomic quality) | > 0.75 | not measured — Ollama not installed | ⚠️ documented opt-in |
| Batch embed (nomic) | < 2 s | not measured | ⚠️ documented opt-in |

## Observations

- Model load (first run, includes HuggingFace download): 18.7 s; warm loads will be disk-bound only.
- Corpus embed 100 chunks: 205 ms (~2 ms/chunk batched).
- Peak RSS 258 MB includes corpus + query embeddings held in memory by the benchmark itself; steady-state model footprint is the 124 MB delta.
- Dataset: 20 (query, relevant-chunk) pairs with deliberately different wording + 80 templated distractors across the same domains. The single miss (rank #6) was the ramen-recipe query — short informal text.
- MRR here uses synthetic data; real-world MRR should be re-validated in Beta against actual timeline data.

## Recommendation

Proceed with fastembed all-MiniLM-L6-v2 as the default embedding path (confirms ADR-0006). Validate nomic-embed-text numbers when an Ollama install is available; not blocking.

## Raw Data

- `cargo run --release` in `spikes/SP-05-embedding/` — assert-based gate check built in.
