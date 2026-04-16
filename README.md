# 🍓 Redberry: The Contrarian Conversationalist Engine

> **"Okay, that's THREE vague prompts in a row. Are you just slapping the keyboard at this point? Take a breath, abstract your architecture, and try again when you have actual requirements."**

Welcome to **Redberry V2**, a high-performance local middleware engine built in purely compiled Rust. 
Its single purpose in life? To ruthlessly evaluate your prompts, mathematically map their geometric weaknesses, and relentlessly bully you until you learn how to write decent instructions for your Language Models.

We waste too many tokens and compute cycles dealing with vague, unconstrained prompts. Redberry sits locally between you and your LLM (via MCP), evaluates your prompt against a native local ML pipeline mapped to five distinct criteria (Vagueness, Syntax, Semantic Drift, Coherence, and Specificity), and gives you a direct, sarcastic "Verdict" before allowing the prompt to drain your wallet.

---

## ⚡ Core Features

### 1. Lightning-Fast Pure Rust ML Vectors
Redberry utilizes the `tract-onnx` inference CPU engine, meaning you do not need Python virtual environments or CUDA dependencies to run complex Machine Learning evaluations. It computes semantic multi-dimensional bounds totally natively.

### 2. Multi-Tier Semantic Embedding
During installation, Redberry interacts with you to download its localized embedding model directly to your system drive.
*   **Tier 1 (Standard):** Powered by `bge-small-en-v1.5` (~33MB). Lightning-fast evaluations.
*   **Tier 2 (Quality):** Powered by `bge-base-en-v1.5` (~110MB). Maximum reasoning for dense code abstractions.

### 3. Exhaustive 5-Point Geometric Constraint Analysis
Through an expansive SQLite Context Cache, Redberry tracks the localized mathematical execution flow of your prompts across a session, measuring them dynamically across 5 dimensions:
- **Lexical/Semantic Vagueness:** Do you use empty pronouns (`"do that thing"`) or hedge words?
- **Syntax Integrity:** Are there fragments, contradictions, or un-parsable run-on boundaries?
- **Context Drift:** Are you wildly pivoting between domains (e.g. asking for Rust server architectures, then instantly asking about basketball)?
- **Topic Coherence:** Is the execution matching the defined cosine semantic limits of the prior flow?
- **Code Specificity:** Did you actually formulate constraints over the architecture?

### 4. V2 Sarcasm Engine & Level-3 "Fatigue"
Why use a boring linter when you can have a digital entity professionally roast you? Redberry evaluates the arrays and returns dynamically shifting insult boundaries. New to V2 is the **Fatigue Engine**. If you consecutively ignore Redberry and write terrible prompts, it escalates its mockery:
- *"I'm cutting you off. You've sent three consecutive un-executable fragments. Go get some coffee and think about what you want."*
- *"My embeddings are hurting. Three strikes. Please sit back, draw a flowchart on a napkin, and figure out what software we are actually building."*
- *"Is this a joke? Because I'm not laughing. Stop sending me garbage and write a real prompt."*

### 5. Enterprise-Secured Architecture
This isn't a script; it's a natively hardened binary. In V2, Redberry's execution context is protected against raw payloads. We implement fixed local Tokenizer Tensor `TruncationParams (max 512)` to gracefully block arbitrary OOM (Out Of Memory) Payload vectors, while explicitly mapping all Axum API queries onto unbuffered localized C SQLite aggregators (`AVG`, limiting DB fetching) ensuring absolute protection against local 10GB+ Heap/DoS Memory attacks. 

---

## 🚀 Performance Metrics

Redberry brings virtually **zero overhead** to your LLM interactions.

| Execution Mode | Latency (Apple Silicon M-Series) | Description |
| :--- | :--- | :--- |
| **CLI (Cold Start)** | `~ 800 ms` | Spinning up the native binary, booting `tract`, loading the ONNX tensors into RAM, tokenizing, and evaluating. |
| **Server / MCP (Hot)**| `~ 20 - 50 ms` | Tensors remain hot. Tokenization, inference, SQLite cosine similarity matches, and snark generation execute instantly. |

---

## ⚙️ Installation & Usage

### 1. Install from Source & Compile Dashboard

```bash
# Clone the repository
git clone https://github.com/VidyaBodepudi/Redberry.git
cd Redberry

# Install the native telemetry engine and MCP daemon
cargo install --path crates/redberry-cli
cargo install --path crates/redberry-server

# Build the beautiful V2 Carmine WebUI
cd crates/redberry-ui/dashboard
npm install && npm run build
```

### 2. Interactive Setup
Run setup to load the ML Tensors:
```bash
redberry setup
```

### 3. Redberry Carmine Telemetry Dashboard (Web UI)
Redberry natively tracks your prompt evaluations over time in your local SQLite database. You can visualize this data natively by spinning up the Redberry UI module!

```bash
cargo run --bin redberry-ui
```
Navigate to `http://127.0.0.1:8443` in your browser.

> **Meet Carmine Elements**  
> Built natively with React 18, Vite, Recharts, and Tailwind V4, the beautifully responsive dashboard evaluates your prompt history across the massive **Geometric Sarcasm Radar Chart**, logs recent violations mapped dynamically against glass-morphic structural panes, and handles telemetry via the heavily audited Axum memory bindings!

### 4. Native Model Context Protocol (MCP) Interceptor
Provide the MCP path to your IDE, and Redberry will physically intercept *every single prompt*. If you type a vague prompt into Claude or Cursor, Redberry natively returns severe snark straight into your chat interface, unilaterally refusing to pass your code to the LLM until you fix it!

Ensure your IDE configuration (`claude_desktop_config.json` or `cline_mcp_settings.json`) looks like this:
```json
{
  "mcpServers": {
    "redberry": {
      "command": "redberry-server",
      "args": []
    }
  }
}
```

### 5. Manual CLI Analysis

```bash
redberry analyze "hello world"
```
*Output:*
```json
{
  "type": "ContextDrift",
  "mockery": "I was tracking our semantic workspace, but you just threw a flashbang into it. Is this a new topic?",
  "drift_score": 0.14598769,
  "prev_topic": "prior discussion"
}
```

```bash
redberry analyze "Write a highly concurrent Rust server using Axum and Tokio that handles incoming REST payloads."
```
*Output:*
```json
{
  "type": "Approved",
  "backhanded_compliment": "Whoa, slow down. If you keep writing prompts this clearly, I might actually lose my job as a professional hater."
}
```

---

## 🏗️ Architecture

- `redberry-core`: Unified configuration types, model presets, and validation mapping.
- `redberry-embed`: Localized caching, robust SQL limit bounds, HuggingFace Tokenizer (capped bounds), and `tract-onnx`.
- `redberry-analyze`: Lexical analyzers mapping vague hedge words and specific constraints.
- `redberry-pipeline`: Mathematics bindings for multi-dimensional contextual geometry vectors.
- `redberry-persona`: The contrarian semantic framework pushing TOML configuration template thresholds.
- `redberry-ui`: The Axum/React V2 Data Dashboard.
- `redberry-server`: The MCP bindings.
