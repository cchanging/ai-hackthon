# Project Overview

This project adapts the open-source Rust application `codex` to run on Asterinas NixOS, enabling advanced AI agents to assist development, validation, and automated testing. It documents the adaptation process, design decisions, encountered challenges, and lessons learned when integrating AI agents with a Linux-compatible runtime.

# Motivation

- Asterinas provides Linux ABI compatibility, making it suitable for running many Linux applications. This project demonstrates how to bring a representative Linux application into the Asterinas environment and captures practical guidance for similar migrations.
- `codex` is an open-source Rust application that includes networking and process-management features, making it a realistic and non-trivial candidate for adaptation and agent-assisted verification.
- When `codex` runs outside the guest (on the host), interaction with the Asterinas guest is limited to serial interfaces, which constrains AI agents. Running `codex` inside Asterinas enables richer multi-agent collaboration, improved debugging workflows, and more comprehensive automated testing.

# Agent Setup and Deployment

The target application is `codex` and agents are powered by the GPT-5.4 model. To reduce interruptions from manual permission prompts, `codex` is configured with broad runtime access during experiments; to mitigate risk, it is deployed inside a container on Asterinas.

Recommended container deployment steps:

1. Install NVM:

```bash
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.7/install.sh | bash
```

2. Load NVM and verify:

```bash
export NVM_DIR="$HOME/.nvm"
[ -s "$NVM_DIR/nvm.sh" ] && . "$NVM_DIR/nvm.sh"
nvm --version
```

3. Install and select the latest LTS Node.js:

```bash
nvm install --lts
nvm use --lts
nvm alias default lts/*
```

4. Install Codex CLI/package:

```bash
npm install -g @openai/codex
```

# Repository structure

- `artifacts/`: Source code, build artifacts, reproducible examples, and supporting documentation necessary to reproduce the work.
- `experiment.md`: A record of interactions with agent tools — including models and tools used, experiment organization, workflows, and encountered issues.
- `lessons.md`: Concise, high-value takeaways and practical recommendations for other developers attempting similar integrations.
