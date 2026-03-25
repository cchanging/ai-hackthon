Lessons for effective bug fixing with AI

# Enable autonomous, iterative workflows

Define the expected final state for the issue up front and instruct the agent to reproduce the bug automatically.
Otherwise you end up as the agent’s tester: you wait for the AI to propose a fix, validate it manually, and then feed back results.
That loop is inefficient and prevents true parallelism between developer and agent.

# Build environments, not just patches

Shift engineering effort toward preparing environments that let agents run fast and repeatedly.
When an agent stalls,
diagnose which capability it lacks and provide that capability (for example, a prebuilt binary or cached build artifacts) rather than repeatedly rebuilding from scratch.
This reduces idle time and speeds iteration.

# Tell the agent what you know

Anticipate likely failure modes and share relevant prior experience with the agent.
For example, when debugging the networking issue, providing a past workaround guided the agent toward the root cause after many failed attempts.
Likewise, if a guest environment will struggle to install compilers (e.g., slow or unreliable builds in Nix),
preinstalling or declaring those toolchains prevents the agent from wasting time attempting expensive or impossible actions.