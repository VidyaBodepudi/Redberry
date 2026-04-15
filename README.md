# 🍓 Redberry: The Contrarian Conversationalist Engine

> **"A coherent instruction? Are you feeling okay? Let me just write that script for you before you revert to fragments."**

Welcome to **Redberry**, a high-performance local middleware engine built in purely compiled Rust. 
Its single purpose in life? To ruthlessly evaluate your prompts, analyze their vagueness, and relentlessly bully you until you learn how to write decent instructions for your Language Models.

We waste too many tokens and compute cycles dealing with vague, unconstrained prompts. Redberry sits between you and your LLM, evaluates your prompt against multiple lexical and machine-learning heuristic criteria, and gives you a direct, sarcastic "Verdict" before allowing the prompt to drain your wallet.

---

## ⚡ Features

### 1. Lightning-Fast Pure Rust ML
Redberry utilizes the `tract-onnx` inference engine, meaning you do not need oversized C++ installations, Python virtual environments, or CUDA dependencies to run complex Machine Learning checks. It compiles directly to native machine code.

### 2. Multi-Tier Semantic Embedding
During installation, Redberry interacts with you to download its localized embedding model directly to your system drive.
*   **Tier 1 (Standard):** Powered by `bge-small-en-v1.5` (INT8 Quantized, ~33MB). Lightning-fast document evaluations.
*   **Tier 2 (Quality):** Powered by `bge-base-en-v1.5` (INT8 Quantized, ~110MB). Superior reasoning for dense code abstractions.

### 3. Context Drift & Vagueness Evaluation
Through an expansive SQLite Context Cache, Redberry remembers the vector embeddings of your past queries. If it detects a severe "Context Drift" (e.g., asking about fixing a CSS gradient right after designing a Python compiler), it will roast your lack of focus and prompt you to start a new session. It checks for:
- Lexical Vagueness (Hedge words, ambiguous pronouns)
- Syntactical Errors (Fragments, run-on sentences)
- Constraint Identification (Language limits, size guidelines)

### 4. The Sarcastic Persona Engine
Why use a boring linter when you can have a digital entity mock you? Redberry evaluates all extracted features and calculates a unified viability score.
- **Below Threshold:** You get told exactly why your prompt is garbage ("Too Vague", "Context Drift"), accompanied by a delightfully passive-aggressive insult.
- **Above Threshold:** Redberry reluctantly concedes that you did a good job and approves the prompt for execution.

---

## 🚀 Performance Metrics

Redberry brings virtually **zero overhead** to your LLM interactions.

| Execution Mode | Latency (Apple Silicon M-Series) | Description |
| :--- | :--- | :--- |
| **CLI (Cold Start)** | `~ 800 ms` | Spinning up the native binary, parsing configuration, booting the `tract` engine, loading the 33-110MB ONNX tensors from NVMe storage to RAM, tokenizing, and evaluating. |
| **Server / MCP (Hot)**| `~ 20 - 50 ms` | The tensors remain pre-loaded in memory. Tokenization, inference, SQLite cosine similarity matches, and snark generation are nearly instant. |

Compare this to the standard 2,000ms - 5,000ms latency of pinging cloud providers like Claude or GPT-4. Redberry catches your terrible prompts faster than you can hit the enter key.

---

## ⚙️ Installation & Usage

Ensure you have Rust and Cargo installed on your system.

### 1. Install from Source
Since Redberry is a native Rust binary, you can install it globally to your system directly from the source repository:

```bash
# Clone the repository
git clone https://github.com/your-username/redberry.git
cd redberry

# Install the binary globally to your ~/.cargo/bin
cargo install --path crates/redberry-cli
cargo install --path crates/redberry-server
```

### 2. Interactive Setup
Once installed, run the setup command from anywhere. Redberry will interactively guide you to download your preferred embedding tier.

```bash
redberry setup
```

### 3. Evaluate a Prompt
Now that Redberry is running natively, you can test it:

```bash
redberry analyze "Fix it."
```
*Output:*
```json
{
  "type": "TooVague",
  "mockery": "I'll pass this along, but let the record show I warned you about the lack of specific constraints.",
  "missing_elements": [
    "More words. Effort."
  ]
}
```

```bash
redberry analyze "Create a Rust CLI application using the clap crate that accepts a file path as an argument and outputs its contents."
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

Redberry is meticulously organized across several localized crates:
- `redberry-core`: Unified configuration types, model presets, and validation mapping.
- `redberry-embed`: Localized caching, SQLite context vectors, Model Setup, and `tract-onnx` execution.
- `redberry-analyze`: Regex, POS tagging equivalents, constraint parsing, and drift analytics.
- `redberry-persona`: The contrarian framework. Variable "sass levels" injected via TOML configuration templates.
- `redberry-cli` & `redberry-server`: Binary wrappers allowing simple ad-hoc checks or persistent MCP background execution.
