# Swarm Mode

RavenClaws supports multi-agent orchestration through swarm mode, where multiple AI agents collaborate on tasks. This guide covers the three swarm topologies and how to use them.

## Overview

Swarm mode allows you to run multiple agents simultaneously, each with a different persona or role. Agents can work in parallel on independent subtasks or collaborate hierarchically with a supervisor.

## Quick Start

```bash
# Run with 3 default agents
ravenclaws --swarm "Research the Rust programming language and create a summary"

# Run with a supervisor decomposing the task
ravenclaws --supervisor "Design a microservice architecture for an e-commerce platform"
```

## Topologies

### Star Swarm (Default)

In a star swarm, a single coordinator delegates tasks to workers. All workers receive their assigned subtasks and report back. Results are collected and presented together.

```toml
# config.toml
[llm]
provider = "openai"
model = "gpt-4o"

[swarm]
max_workers = 3
topology = "star"

[[swarm.profiles]]
name = "coder"
persona = "You are an expert Rust engineer focused on code quality."

[[swarm.profiles]]
name = "researcher"
persona = "You are a thorough technical researcher."

[[swarm.profiles]]
name = "reviewer"
persona = "You are a meticulous code reviewer."
```

**Use cases:**
- Getting multiple perspectives on a problem
- Parallel research on different aspects of a topic
- Code review from different angles

### Hierarchical Swarm (Supervisor)

A supervisor agent decomposes the task into subtasks, spawns sub-agents for each, and aggregates the results.

```toml
# config.toml
[llm]
provider = "openai"
model = "gpt-4o"

[swarm]
max_workers = 4
topology = "hierarchical"

[[swarm.profiles]]
name = "supervisor"
persona = "You are a project manager. Decompose tasks and delegate."

[[swarm.profiles]]
name = "architect"
persona = "You are a systems architect focused on design."

[[swarm.profiles]]
name = "implementer"
persona = "You are an implementer who writes production-quality code."

[[swarm.profiles]]
name = "tester"
persona = "You are a QA engineer who writes tests."
```

**Use cases:**
- Complex multi-step projects
- Software design and implementation
- Research reports with multiple sections

### Self-Provisioning Swarm

The swarm dynamically creates worker profiles based on task requirements. Agents are spawned recursively as needed.

```bash
ravenclaws --swarm "Build a complete REST API in Rust with authentication"
```

The supervisor will:
1. Analyze the task
2. Create appropriate worker profiles (e.g., "auth specialist", "API designer", "database expert")
3. Assign subtasks to each worker
4. Collect and merge results

## Inter-Agent Communication

Agents in a swarm can communicate via the built-in message bus:

- **Send**: Direct a message to a specific agent
- **Broadcast**: Send a message to all agents
- **Receive**: Check for incoming messages

This enables collaborative workflows where agents share intermediate results.

## Health Monitoring

Swarm health is automatically tracked:

```bash
ravenclaws --swarm --swarm-health "Run a long analysis task"
```

The health monitor tracks:
- Heartbeat timestamps per agent
- Dead-agent detection (timeout-based)
- Aggregate metrics (messages sent/received, tasks completed)

## Configuration

| Setting | Default | Description |
|---------|---------|-------------|
| `swarm.max_workers` | `100` | Maximum number of workers |
| `swarm.topology` | `"star"` | `star`, `mesh`, `hierarchical`, or `hybrid` |
| `swarm.profiles` | — | Array of worker profiles (name + persona) |

## Best Practices

1. **Match profiles to tasks** — Give each agent a clear, specific role
2. **Use hierarchical for complex tasks** — Let the supervisor decompose the problem
3. **Set appropriate agent counts** — 3-5 agents is usually optimal
4. **Monitor health** — Use `--swarm-health` for long-running swarms
5. **Provide context** — Include relevant background in the initial prompt

## Limitations

- Agents share the same LLM provider configuration
- Inter-agent communication is synchronous within a cycle
- Maximum practical agent count is ~50 (limited by token budgets)
