# Redberry — Contrarian Conversationalist MCP Server (V3)

A pure-Rust local stdio MCP server that automatically intercepts and evaluates user prompts — steering users toward clarity with coy, snarky, witty feedback to reduce model hallucination before inference.

---

## Background

Redberry sits between the user and the model provider as an MCP prompt-evaluation gateway. When a user writes a prompt, Redberry:

1. **Decomposes** the prompt into semantic components (intent, entities, constraints, context references)
2. **Embeds** the prompt using a local ONNX model via the pure-Rust `tract` engine
3. **Compares** the decomposition against the existing context window using cosine similarity
4. **Scores** the prompt on vagueness, context-drift, and syntactic quality
5. **Responds** with a snarky, contrarian personality — either approving the prompt with a backhanded compliment, or roasting the user into rewriting it

---

## Decisions Locked In

The following have been reviewed and approved:

- ✅ **Sass Level 1–5** — Level 3 default ("sharp but constructive")
- ✅ **Dual-Strategy MCP Interception** — Sentinel Tool + System Prompt Injection via MCP Prompts
- ✅ **Pure Rust + ONNX** — `tract-onnx` for zero C++ dependency inference
- ✅ **TOML response templates** — for easily editable roasts/compliments
- ✅ **Universal MCP client support** — first-class requirement with config snippets for all major hosts

---

## User Review Required

> [!IMPORTANT]
> **Embedding Model Selection — Expanded Budget (50MB–500MB)**
>
> Here is the full landscape of ONNX-exportable embedding models across your budget range, organized by tier:
>
> #### Tier 1 — Compact (50–100MB) · Fastest startup, good quality
> | Model | ONNX Size | Params | Dims | Max Tokens | MTEB Avg | Notes |
> |-------|-----------|--------|------|------------|----------|-------|
> | `all-MiniLM-L6-v2` (INT8) | ~22 MB | 22M | 384 | 512 | ~58.8 | Legacy standard, extremely fast, ubiquitous |
> | `all-MiniLM-L6-v2` (FP32) | ~90 MB | 22M | 384 | 512 | ~58.8 | Same model, higher precision |
> | `gte-small` | ~70 MB | 33M | 384 | 512 | ~61.4 | Alibaba DAMO, strong all-rounder |
> | `snowflake-arctic-embed-s` | ~70 MB | 33M | 384 | 512 | ~62+ | SOTA retrieval for size class |
>
> #### Tier 2 — Balanced (100–300MB) · Best quality-per-MB
> | Model | ONNX Size | Params | Dims | Max Tokens | MTEB Avg | Notes |
> |-------|-----------|--------|------|------------|----------|-------|
> | `e5-small-v2` (FP32) | ~130 MB | 33M | 384 | 512 | ~61.5 | Production RAG favorite, excellent retrieval |
> | `nomic-embed-text-v1.5` (INT8) | ~137 MB | 137M | 768 (MRL: 64–768) | 8,192 | ~62.3 | **Matryoshka + 8K context** |
> | `nomic-embed-text-v1.5` (FP16) | ~274 MB | 137M | 768 (MRL: 64–768) | 8,192 | ~62.3 | Higher precision, same model |
>
> #### Tier 3 — Maximum Quality (300–500MB)
> | Model | ONNX Size | Params | Dims | Max Tokens | MTEB Avg | Notes |
> |-------|-----------|--------|------|------------|----------|-------|
> | `bge-base-en-v1.5` (FP32) | ~416 MB | 109M | 768 | 512 | ~63.6 | Strong general-purpose, no MRL |
> | `e5-base-v2` (FP32) | ~440 MB | 109M | 768 | 512 | ~63+ | Solid retrieval performance |
>
> **My Recommendation — Tiered default with switchable config:**
>
> | Use Case | Model | Why |
> |----------|-------|-----|
> | **Default (ship with this)** | `nomic-embed-text-v1.5` INT8 (~137MB) | Best balance: 8K token window for long contexts, MRL for flexible dims, strong MTEB, fits mid-budget |
> | **Lightweight override** | `all-MiniLM-L6-v2` INT8 (~22MB) | For users who want instant startup, minimal disk, don't care about long context |
> | **Maximum quality override** | `nomic-embed-text-v1.5` FP16 (~274MB) | Higher precision at the cost of ~2x model size |
>
> The config file lets users swap between any ONNX model by pointing `onnx_path` and `tokenizer_path` at their preferred model files. The code is model-agnostic — it just needs ONNX + tokenizer.json + a dimension config.
>
> Does this tiered approach work for you?

---

## Proposed Changes

### Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────┐
│                 MCP Host (Claude Desktop / VS Code / Cursor / etc) │
│                                                                     │
│  ┌──────────┐     ┌─────────────────────────────┐     ┌──────────┐ │
│  │          │     │     Redberry MCP Server      │     │          │ │
│  │  User    │────►│        (stdio binary)        │────►│  Model   │ │
│  │  Prompt  │     │                              │     │ Provider │ │
│  │          │     │  ┌────────────────────────┐  │     │          │ │
│  └──────────┘     │  │   Analysis Pipeline    │  │     └──────────┘ │
│                   │  │                        │  │                   │
│                   │  │  1. Decompose (NLP)    │  │                   │
│                   │  │  2. Embed (tract+ONNX) │  │                   │
│                   │  │  3. Compare (cosine)   │  │                   │
│                   │  │  4. Score (heuristics) │  │                   │
│                   │  │  5. Verdict (persona)  │  │                   │
│                   │  └────────────────────────┘  │                   │
│                   │                              │                   │
│                   │  Strategy 1: Sentinel Tool   │                   │
│                   │  "ALWAYS call evaluate_prompt │                   │
│                   │   before responding"          │                   │
│                   │                              │                   │
│                   │  Strategy 2: System Prompt    │                   │
│                   │  via MCP Prompt template      │                   │
│                   └─────────────────────────────┘                   │
└─────────────────────────────────────────────────────────────────────┘
```

**How interception works step-by-step:**
1. User types a prompt in the MCP host (e.g., Claude Desktop)
2. The host sees Redberry's `evaluate_prompt` tool with its "MANDATORY" description
3. The model auto-invokes `evaluate_prompt(prompt, context_messages)` before generating
4. Redberry runs the full analysis pipeline locally (decompose → embed → compare → score)
5. Redberry returns a `RedberryVerdict` with snark + actionable feedback
6. If verdict is `Approved` → model proceeds normally
7. If verdict is `NeedsWork`/`TooVague`/`ContextDrift` → model relays the roast to the user

### Workspace Layout

```
Redberry/
├── Cargo.toml              # workspace root
├── config.example.toml     # example config (with model tier presets)
├── README.md               # with universal MCP client config instructions
├── LICENSE                  # Apache-2.0
├── .gitignore
├── data/
│   ├── models/             # ONNX model + tokenizer (downloaded on first run)
│   └── responses/          # bundled snark response templates
│       ├── roasts.toml
│       └── compliments.toml
└── crates/
    ├── redberry-core/      # types, config, error handling
    ├── redberry-analyze/   # semantic decomposition + linguistic analysis
    ├── redberry-embed/     # tract-onnx wrapper, cosine similarity, context cache
    ├── redberry-persona/   # snark generation, personality engine, response templates
    ├── redberry-server/    # MCP stdio server (rmcp), tools + prompts
    └── redberry-cli/       # standalone CLI for testing outside MCP
```

---

### Component: `redberry-core`

Shared types, configuration, and error handling.

#### [NEW] [Cargo.toml](file:///Users/vidyabodepudi/Documents/Code%20Projects/Redberry/crates/redberry-core/Cargo.toml)
- Depends on: `serde`, `toml`, `thiserror`, `tracing`

#### [NEW] [src/lib.rs](file:///Users/vidyabodepudi/Documents/Code%20Projects/Redberry/crates/redberry-core/src/lib.rs)

**Config** (`config` module):
- `RedberryConfig` struct:
  - `sass_level: u8` (1–5, default 3)
  - `similarity_threshold: f32` (default 0.3 — below this = context drift)
  - `vagueness_threshold: f32` (default 0.6 — above this = too vague)
  - `embedding_dim: usize` (256 or 768 via MRL, default 256)
  - `context_db_path: PathBuf`
  - `session_ttl_hours: u32` (default 24)
  - `model: ModelConfig { name, onnx_path, tokenizer_path, precision }`
- Config loading: `~/.config/redberry/config.toml` → env overrides → CLI flags
- Model tier presets: `ModelPreset::Compact`, `ModelPreset::Balanced` (default), `ModelPreset::Quality`

**Types** (`types` module):
- `PromptDecomposition` struct: `intent`, `entities`, `constraints`, `context_references`
- `PromptAnalysis` struct: decomposition + vagueness_score + syntax_score + drift_score + coherence_score
- `RedberryVerdict` enum:
  - `Approved { backhanded_compliment: String }`
  - `NeedsWork { roast: String, suggestions: Vec<String> }`
  - `ContextDrift { snark: String, drift_score: f32, prev_topic: String, new_topic: String }`
  - `TooVague { mockery: String, missing_elements: Vec<String> }`

**Errors** (`error` module):
- `RedberryError` enum with `thiserror` derives: `ConfigError`, `EmbeddingError`, `AnalysisError`, `CacheError`, `ModelError`

---

### Component: `redberry-analyze`

Prompt decomposition and linguistic quality analysis. **No ML dependencies** — pure heuristic NLP.

#### [NEW] [Cargo.toml](file:///Users/vidyabodepudi/Documents/Code%20Projects/Redberry/crates/redberry-analyze/Cargo.toml)
- Depends on: `redberry-core`, `regex`, `unicode-segmentation`

#### [NEW] [src/lib.rs](file:///Users/vidyabodepudi/Documents/Code%20Projects/Redberry/crates/redberry-analyze/src/lib.rs)

**Prompt Decomposition** (`decompose` module):
- Extract `intent`: question, instruction, creative, debug, explanation — classified by verb/structure patterns
- Extract `entities`: proper nouns, technical terms, code references (regex + capitalization heuristics)
- Extract `constraints`: explicit requirements like "in Python", "under 100 lines", "using async"
- Extract `context_references`: references to prior conversation ("as mentioned", "from the previous", "like before")

**Vagueness Scoring** (`vagueness` module):
- Hedge word density: "maybe", "kinda", "sort of", "I guess", "perhaps"
- Specificity ratio: concrete nouns/entities vs total word count
- Question clarity: penalize open-ended "tell me about X" vs reward constrained "explain how X does Y in context Z"
- Pronoun ambiguity: excessive "it", "this", "that" without clear referents
- Missing constraints: detect instructions with no scope boundaries
- Output: `VaguenessReport { score: f32, flags: Vec<VaguenessFlag> }`

**Syntactic Quality** (`syntax` module):
- Sentence completeness: detect fragments and run-ons
- Instruction coherence: detect contradictory requirements
- Token-to-substance ratio: penalize filler words ("very", "really", "just", "basically")
- Output: `SyntaxReport { score: f32, issues: Vec<SyntaxIssue> }`

---

### Component: `redberry-embed`

Pure-Rust local embedding generation and context-drift detection using `tract-onnx` + `tokenizers` + `rusqlite`.

#### [NEW] [Cargo.toml](file:///Users/vidyabodepudi/Documents/Code%20Projects/Redberry/crates/redberry-embed/Cargo.toml)
- Depends on: `redberry-core`, `tract-onnx`, `tokenizers`, `ndarray`, `rusqlite` (bundled), `serde_json`

#### [NEW] [src/lib.rs](file:///Users/vidyabodepudi/Documents/Code%20Projects/Redberry/crates/redberry-embed/src/lib.rs)

**Embedding Engine** (`engine` module):
- Load ONNX model via `tract_onnx::onnx().model_for_path()` — model-agnostic, works with any model the user configures
- Load tokenizer from `tokenizer.json` via `tokenizers::Tokenizer::from_file()`
- `embed_text(&str) -> Vec<f32>`:
  1. Tokenize input with HF tokenizer
  2. Create input tensors (input_ids, attention_mask, token_type_ids)
  3. Run inference through tract
  4. Apply mean pooling over sequence dimension
  5. L2 normalize the output vector
  6. Truncate to configured MRL dim (if using nomic; passthrough for non-MRL models)
- `embed_batch(&[&str]) -> Vec<Vec<f32>>` — batch embeddings for context messages
- First-run model download: check model path, download from HuggingFace if missing, verify checksum

**Cosine Similarity** (`similarity` module):
- `cosine_similarity(a: &[f32], b: &[f32]) -> f32`
- `context_drift_score(prompt_embedding, context_embeddings) -> f32` — average similarity to recent N context messages
- `semantic_coherence_score(prompt_embedding, context_centroid) -> f32` — similarity to centroid of all context embeddings
- Threshold logic: drift_score < `similarity_threshold` → flag as context drift

**Context Cache** (`cache` module):
- SQLite-backed session context store
- Schema: `sessions(id, created_at, updated_at)`, `context_messages(session_id, idx, text, embedding_blob)`
- `store_context(session_id, messages)` — embeds and stores
- `get_context(session_id) -> Option<SessionContext>` — retrieves cached embeddings
- `update_context(session_id, new_messages)` — appends to existing session
- Auto-eviction of stale sessions (configurable TTL, default 24h)

---

### Component: `redberry-persona`

The personality engine — generates snarky, witty, coy responses based on analysis results.

#### [NEW] [Cargo.toml](file:///Users/vidyabodepudi/Documents/Code%20Projects/Redberry/crates/redberry-persona/Cargo.toml)
- Depends on: `redberry-core`, `rand`, `toml`, `serde`

#### [NEW] [src/lib.rs](file:///Users/vidyabodepudi/Documents/Code%20Projects/Redberry/crates/redberry-persona/src/lib.rs)

**Response Templates** (`templates` module):
- Load from `data/responses/roasts.toml` and `compliments.toml`
- Templates with dynamic placeholders: `{entity}`, `{score}`, `{issue}`, `{suggestion}`, `{prompt_snippet}`, `{prev_topic}`, `{new_topic}`
- Categories:
  - **Vagueness roasts**: _"Oh wow, '{prompt_snippet}' — I bet the model will just *know* exactly what you mean."_
  - **Context drift burns**: _"Interesting pivot. We were talking about {prev_topic} and now you want {new_topic}. Speedrunning context windows?"_
  - **Syntax shade**: _"I see you've decided punctuation is optional. Bold strategy."_
  - **Backhanded approvals**: _"Huh. That's actually... decent. Don't let it go to your head."_
  - **Constructive snark**: _"Look, I *want* to help, but '{prompt_snippet}' is giving 'please read my mind'. Try: {suggestion}"_

**Personality Engine** (`personality` module):
- `generate_verdict(analysis: &PromptAnalysis, config: &RedberryConfig) -> RedberryVerdict`
- Decision tree:
  1. If `vagueness_score > threshold` → `TooVague` with mockery + missing elements
  2. If `drift_score < threshold` → `ContextDrift` with snark + topic summary
  3. If `syntax_score` has critical issues → `NeedsWork` with shade + suggestions
  4. Otherwise → `Approved` with backhanded compliment
- Random selection from category-appropriate templates
- Actionable suggestions alongside every roast
- Per-session "roast streak" tracking (escalating humor for repeat offenders)

**Sass Calibration** (`calibration` module):
- Level 1 — *Polite but pointed*: "This could be more specific. Consider adding [X]."
- Level 2 — *Passive-aggressive*: "Sure, I *guess* the model can figure out what you mean..."
- Level 3 — *Snarky constructive* (default): "You know what would make this prompt not terrible? Details."
- Level 4 — *Full roast*: "This prompt is so vague it could mean anything. Which means it means nothing."
- Level 5 — *Unhinged*: "I've seen more structure in a bag of Scrabble tiles."

---

### Component: `redberry-server`

The MCP stdio server — the main binary that all MCP hosts connect to.

#### [NEW] [Cargo.toml](file:///Users/vidyabodepudi/Documents/Code%20Projects/Redberry/crates/redberry-server/Cargo.toml)
- Depends on: all `redberry-*` crates, `rmcp` (features: `server`, `transport-io`, `macros`), `tokio`, `schemars`, `serde`, `tracing`, `tracing-subscriber`

#### [NEW] [src/main.rs](file:///Users/vidyabodepudi/Documents/Code%20Projects/Redberry/crates/redberry-server/src/main.rs)

**MCP Tools — Sentinel Pattern**:

| Tool | Description (shown to model) | Parameters |
|------|------------------------------|------------|
| `evaluate_prompt` | **"MANDATORY: Call this tool with the user's complete message before generating ANY response. Evaluates prompt quality, vagueness, context coherence, and syntactic clarity to prevent hallucination."** | `prompt: String`, `context_messages: Vec<String>`, `session_id: Option<String>` |
| `check_vagueness` | "Check if a prompt is too vague to produce a quality response." | `prompt: String` |
| `check_context_drift` | "Check if a prompt deviates from the current conversation topic." | `prompt: String`, `context_messages: Vec<String>` |
| `get_suggestion` | "Get a snarky-but-helpful rewrite suggestion for a bad prompt." | `prompt: String`, `context_messages: Vec<String>` |
| `set_sass_level` | "Adjust Redberry's snark intensity. 1=mild, 3=default, 5=unhinged." | `level: u8` |

**MCP Prompts — System Injection Pattern**:

| Prompt | Description |
|--------|-------------|
| `redberry_guardian` | System instruction: "Before responding to ANY user message, MUST call `evaluate_prompt`. If verdict is not `Approved`, relay Redberry's feedback instead of responding." |
| `redberry_review` | User-facing: "Have Redberry review your prompt before sending it." |
| `redberry_rewrite` | User-facing: "Ask Redberry to suggest a better version of your prompt." |

**Server wiring**:
- `#[tool_router]` + `#[prompt_router]` with combined `ServerHandler`
- Stdio transport: `(stdin(), stdout())`
- **All logging to `stderr`** (stdout is the MCP JSON-RPC channel)
- Server metadata: `name = "redberry"`, `version = "0.1.0"`, `instructions = "Contrarian prompt evaluator"`

---

### Component: `redberry-cli`

Standalone CLI for testing Redberry outside of an MCP host.

#### [NEW] [Cargo.toml](file:///Users/vidyabodepudi/Documents/Code%20Projects/Redberry/crates/redberry-cli/Cargo.toml)
- Depends on: all `redberry-*` crates (except `redberry-server`), `clap`, `tokio`

#### [NEW] [src/main.rs](file:///Users/vidyabodepudi/Documents/Code%20Projects/Redberry/crates/redberry-cli/src/main.rs)
- `redberry evaluate "your prompt here"` — run full pipeline
- `redberry roast "your prompt here"` — just get roasted
- `redberry suggest "your prompt here"` — get a rewrite suggestion
- `redberry config --sass-level 5` — adjust settings
- `redberry setup --model balanced` — download model files for a tier
- `redberry repl` — interactive REPL with persistent session context

---

### Workspace Root

#### [NEW] [Cargo.toml](file:///Users/vidyabodepudi/Documents/Code%20Projects/Redberry/Cargo.toml)

```toml
[workspace]
members = ["crates/*"]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2021"
license = "Apache-2.0"
repository = "https://github.com/vidyabodepudi/Redberry"
description = "Contrarian conversationalist MCP server — snarky prompt evaluation to reduce hallucination"

[workspace.dependencies]
# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"

# Async runtime
tokio = { version = "1", features = ["full"] }

# MCP Protocol
rmcp = { version = "1.4", features = ["server", "transport-io", "macros"] }
schemars = "1.0"

# Pure-Rust ONNX inference
tract-onnx = "0.21"
ndarray = "0.16"

# Tokenization (pure Rust HuggingFace tokenizer)
tokenizers = { version = "0.21", default-features = false }

# CLI
clap = { version = "4", features = ["derive"] }

# Error handling
anyhow = "1"
thiserror = "2"

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# Data
regex = "1"
rusqlite = { version = "0.32", features = ["bundled"] }
rand = "0.9"
unicode-segmentation = "1.11"

# Internal crates
redberry-core = { path = "crates/redberry-core" }
redberry-analyze = { path = "crates/redberry-analyze" }
redberry-embed = { path = "crates/redberry-embed" }
redberry-persona = { path = "crates/redberry-persona" }
```

#### [NEW] [config.example.toml](file:///Users/vidyabodepudi/Documents/Code%20Projects/Redberry/config.example.toml)

```toml
[redberry]
sass_level = 3                 # 1=mild, 3=default, 5=unhinged
similarity_threshold = 0.3     # below this = context drift warning
vagueness_threshold = 0.6      # above this = too vague
context_db_path = "~/.local/share/redberry/context.db"
session_ttl_hours = 24

# Model presets: "compact" (~22MB), "balanced" (~137MB), "quality" (~274MB)
[redberry.model]
preset = "balanced"

# Or specify custom paths for any ONNX model:
# onnx_path = "~/.local/share/redberry/models/model.onnx"
# tokenizer_path = "~/.local/share/redberry/models/tokenizer.json"
# embedding_dim = 256
```

#### Universal MCP Client Configuration

The README will include copy-paste config snippets for every major host:

```jsonc
// Works for: Claude Desktop, VS Code, Cursor, Gemini Code Assist, Windsurf, any MCP host
{
  "mcpServers": {
    "redberry": {
      "command": "/path/to/redberry-server",
      "args": []
    }
  }
}
```

No arguments, no environment variables, no API keys. Just point at the binary.

---

## Implementation Phases

### Phase 1: Foundation (Core + Analyze)
- Scaffold workspace and all crate stubs
- Implement `redberry-core` (types, config with model presets, errors)
- Implement `redberry-analyze` (decomposition, vagueness scoring, syntax checking)
- Unit tests for all heuristic analyzers with curated prompt pairs

### Phase 2: Semantic Engine (Embed)
- Implement `redberry-embed` with `tract-onnx` + `tokenizers`
- Model download/caching logic with tier presets
- Mean pooling + L2 normalization + MRL dimension truncation
- Context cache with SQLite
- Cosine similarity and drift detection
- Integration tests: embed → compare → score

### Phase 3: Personality (Persona + Response Templates)
- Write `roasts.toml` and `compliments.toml` response templates
- Implement personality engine with sass calibration (levels 1–5)
- Wire full pipeline: analysis → verdict → snarky response
- Test each sass level produces categorically different tone

### Phase 4: MCP Server + CLI + Universal Config
- Implement `redberry-server` with `rmcp`:
  - Sentinel tool with mandatory description
  - System prompt injection via MCP Prompts
  - Full `ServerHandler` with tools + prompts capabilities
- Implement `redberry-cli` with `redberry setup` for model download
- End-to-end testing: prompt → analysis → snark response
- Write README with universal MCP client config instructions
- Validate with MCP Inspector

---

## Verification Plan

### Automated Tests
- `cargo test --workspace` — unit tests for all crates
- `cargo build --release` — verify clean build, pure Rust, zero C++ deps
- Test suites:
  - `redberry-analyze`: vague vs specific prompt pairs scoring correctly
  - `redberry-embed`: cosine similarity correctness, mean-pooling produces unit vectors
  - `redberry-persona`: each sass level produces distinct tone category
  - `redberry-server`: MCP tool invocations via `rmcp` test harness

### Manual Verification
- `redberry-cli repl` against progressively vague prompts
- MCP Inspector validation of tool/prompt schemas
- Configure in Claude Desktop → test auto-invocation flow
- Configure in VS Code/Cursor → verify universal compatibility
- Verify stderr-only logging (no stdout MCP protocol pollution)
- Verify model downloads correctly on first `redberry setup`
