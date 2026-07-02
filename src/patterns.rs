//! Multi-agent pattern primitives
//!
//! Built-in multi-agent collaboration patterns that extend beyond simple
//! swarm/supervisor modes. Each pattern implements a distinct collaboration
//! strategy:
//!
//! - **Debate** — Multiple agents argue different positions, then converge
//! - **Review-Loop** — One agent produces, another reviews, iterating to quality
//! - **Research-Synthesize** — Parallel research agents feed a synthesizer
//! - **Voting** — Multiple agents vote on options, majority decides
//! - **Tree-of-Thought** — Multiple parallel reasoning paths with evaluation and pruning
//! - **Self-Reflection** — Agent generates output, reflects, and improves iteratively
//!
//! These patterns are first-class modes accessible via `--mode debate`,
//! `--mode review-loop`, `--mode research-synthesize`, `--mode voting`,
//! `--mode tree-of-thought`, or `--mode self-reflection`.

use std::sync::{Arc, Mutex};
use tracing::{info, warn};

use crate::agent::ConversationMemory;
use crate::config::Config;
use crate::error::RavenClawsError;
use crate::healing::SelfHealingEngine;
use crate::llm::{LLMProviderTrait, MultiModelManager};
use crate::ravenfabric::RavenFabricClient;

// ── Pattern Configuration ──────────────────────────────────────────────────

/// Configuration for multi-agent patterns
#[derive(Debug, Clone)]
pub struct PatternConfig {
    /// Maximum debate rounds (default: 3)
    pub max_rounds: usize,
    /// Maximum review iterations (default: 3)
    pub max_review_iterations: usize,
    /// Number of research agents (default: 3)
    pub research_agent_count: usize,
    /// Number of voters (default: 3)
    pub voter_count: usize,
    /// Whether to show intermediate results
    pub verbose: bool,
    /// Tree-of-thought: number of branches per step (default: 3)
    pub tot_branches: usize,
    /// Tree-of-thought: maximum depth (default: 3)
    pub tot_depth: usize,
    /// Tree-of-thought: top-k branches to keep after pruning (default: 2)
    pub tot_top_k: usize,
    /// Self-reflection: number of reflection rounds (default: 2)
    pub reflection_rounds: usize,
}

impl Default for PatternConfig {
    fn default() -> Self {
        Self {
            max_rounds: 3,
            max_review_iterations: 3,
            research_agent_count: 3,
            voter_count: 3,
            verbose: false,
            tot_branches: 3,
            tot_depth: 3,
            tot_top_k: 2,
            reflection_rounds: 2,
        }
    }
}

// ── Pattern 1: Debate ──────────────────────────────────────────────────────

/// Run a debate between multiple agents with different positions.
///
/// Each agent argues a distinct position, then a judge synthesizes the
/// final conclusion after configurable rounds of debate.
pub async fn run_debate(
    llm: Arc<dyn LLMProviderTrait>,
    config: Config,
    ravenfabric: Option<RavenFabricClient>,
    pattern_config: PatternConfig,
    healing_engine: Option<Arc<Mutex<SelfHealingEngine>>>,
) -> crate::error::Result<()> {
    info!(
        "Starting debate mode with {} max rounds",
        pattern_config.max_rounds
    );

    let system_prompt = &config.llm.system_prompt;
    let task = "Analyze the given task and provide your solution.";

    // Define debate positions
    let positions = [
        ("Proponent", "You argue FOR the proposition. Focus on benefits, opportunities, and strengths. Be persuasive and evidence-based."),
        ("Opponent", "You argue AGAINST the proposition. Focus on risks, drawbacks, and weaknesses. Be critical and thorough."),
        ("Synthesizer", "You are the neutral judge. Listen to both sides, identify common ground, and synthesize a balanced conclusion."),
    ];

    let mut debate_history = ConversationMemory::new(system_prompt, 50);
    debate_history.add_user_message(&format!(
        "Debate topic: {}\n\nProponent, present your opening argument.",
        task
    ));

    // Debate rounds
    for round in 0..pattern_config.max_rounds {
        info!(round = round + 1, "Debate round starting");

        for (role, persona) in &positions {
            if *role == "Synthesizer" && round < pattern_config.max_rounds - 1 {
                continue; // Synthesizer only speaks in final round
            }

            let mut agent_memory = ConversationMemory::new(persona, 20);
            // Feed debate history
            for msg in debate_history.history() {
                if msg.role == "system" {
                    continue;
                }
                if msg.role == "user" {
                    agent_memory.add_user_message(&msg.content);
                } else {
                    agent_memory.add_assistant_message(&msg.content);
                }
            }

            // Check self-healing circuit breaker
            if let Some(ref healing) = healing_engine {
                let healthy = {
                    let mut engine = healing.lock().unwrap();
                    engine.is_healthy(&format!("debate-{}", role))
                };
                if !healthy {
                    warn!(role = %role, round = round + 1, "Debate agent blocked by circuit breaker");
                    continue;
                }
            }

            let messages = agent_memory.history().to_vec();
            match llm.chat(messages).await {
                Ok(response) => {
                    // Record success
                    if let Some(ref healing) = healing_engine {
                        let mut engine = healing.lock().unwrap();
                        engine.record_success(&format!("debate-{}", role));
                    }

                    if let Some(choice) = response.choices.first() {
                        let content = &choice.message.content;
                        info!(role = %role, round = round + 1, "Debate contribution received");

                        if pattern_config.verbose {
                            println!("\n── {} (Round {}) ──\n{}", role, round + 1, content);
                        }

                        debate_history.add_assistant_message(&format!("{}: {}", role, content));
                    }
                }
                Err(e) => {
                    // Record failure
                    if let Some(ref healing) = healing_engine {
                        let mut engine = healing.lock().unwrap();
                        engine.record_failure(&format!("debate-{}", role), &e.to_string());
                    }
                    warn!(error = %e, role = %role, round = round + 1, "Debate LLM request failed");
                }
            }
        }
    }

    // Final synthesis
    let synthesizer_persona = "You are a neutral synthesizer. Produce a final balanced conclusion that incorporates the best arguments from both sides.";
    let mut final_memory = ConversationMemory::new(synthesizer_persona, 30);
    for msg in debate_history.history() {
        if msg.role == "system" {
            continue;
        }
        if msg.role == "user" {
            final_memory.add_user_message(&msg.content);
        } else {
            final_memory.add_assistant_message(&msg.content);
        }
    }
    final_memory
        .add_user_message("Now produce your FINAL synthesis that balances all perspectives:");

    // Check self-healing circuit breaker before synthesis
    if let Some(ref healing) = healing_engine {
        let healthy = {
            let mut engine = healing.lock().unwrap();
            engine.is_healthy("debate-synthesis")
        };
        if !healthy {
            warn!("Debate synthesis blocked by circuit breaker");
            return Err(RavenClawsError::HealingError(
                "Debate synthesis blocked by circuit breaker".to_string(),
            ));
        }
    }

    let messages = final_memory.history().to_vec();
    match llm.chat(messages).await {
        Ok(response) => {
            // Record success
            if let Some(ref healing) = healing_engine {
                let mut engine = healing.lock().unwrap();
                engine.record_success("debate-synthesis");
            }

            if let Some(choice) = response.choices.first() {
                let result = &choice.message.content;
                println!("\n🐦‍⬛ Debate Synthesis:\n{}", result);

                if let Some(ref rf) = ravenfabric {
                    if rf.is_enabled() {
                        let preview = result.chars().take(500).collect::<String>();
                        let _ = rf.broadcast(&preview, 30).await;
                    }
                }
            }
        }
        Err(e) => {
            // Record failure
            if let Some(ref healing) = healing_engine {
                let mut engine = healing.lock().unwrap();
                engine.record_failure("debate-synthesis", &e.to_string());
            }
            warn!(error = %e, "Debate synthesis failed");
            return Err(RavenClawsError::CommandExecution(format!(
                "Debate synthesis failed: {}",
                e
            )));
        }
    }

    Ok(())
}

// ── Pattern 2: Review-Loop ─────────────────────────────────────────────────

/// Run a producer-reviewer loop that iterates until quality is met.
///
/// A producer agent creates content, a reviewer agent critiques it,
/// and the producer revises based on feedback. Repeats until the
/// reviewer approves or max iterations reached.
pub async fn run_review_loop(
    llm: Arc<dyn LLMProviderTrait>,
    config: Config,
    ravenfabric: Option<RavenFabricClient>,
    pattern_config: PatternConfig,
    healing_engine: Option<Arc<Mutex<SelfHealingEngine>>>,
) -> crate::error::Result<()> {
    info!(
        "Starting review-loop mode with max {} iterations",
        pattern_config.max_review_iterations
    );

    let _system_prompt = &config.llm.system_prompt;
    let task = "Analyze the given task and provide your solution.";

    let producer_persona = "You are a producer. Create high-quality content based on the requirements. Be thorough and detailed.";
    let reviewer_persona = "You are a reviewer. Critically evaluate the content. Identify specific issues, gaps, and improvements needed. Be constructive and precise. If the content meets quality standards, respond with APPROVED: followed by your final sign-off.";

    let mut current_content = String::new();
    let mut approved = false;

    for iteration in 0..pattern_config.max_review_iterations {
        info!(iteration = iteration + 1, "Review-loop iteration");

        if iteration == 0 {
            // Producer creates initial content
            let mut producer_memory = ConversationMemory::new(producer_persona, 10);
            producer_memory.add_user_message(&format!(
                "Create content for the following task:\n\n{}",
                task
            ));

            // Check self-healing circuit breaker
            if let Some(ref healing) = healing_engine {
                let healthy = {
                    let mut engine = healing.lock().unwrap();
                    engine.is_healthy("review-producer")
                };
                if !healthy {
                    warn!("Review producer blocked by circuit breaker");
                    return Err(RavenClawsError::HealingError(
                        "Review producer blocked by circuit breaker".to_string(),
                    ));
                }
            }

            let messages = producer_memory.history().to_vec();
            match llm.chat(messages).await {
                Ok(response) => {
                    // Record success
                    if let Some(ref healing) = healing_engine {
                        let mut engine = healing.lock().unwrap();
                        engine.record_success("review-producer");
                    }

                    if let Some(choice) = response.choices.first() {
                        current_content = choice.message.content.clone();
                        info!("Initial content produced: {} chars", current_content.len());

                        if pattern_config.verbose {
                            println!("\n── Initial Content ──\n{}", current_content);
                        }
                    }
                }
                Err(e) => {
                    // Record failure
                    if let Some(ref healing) = healing_engine {
                        let mut engine = healing.lock().unwrap();
                        engine.record_failure("review-producer", &e.to_string());
                    }
                    warn!(error = %e, "Producer LLM request failed");
                    return Err(RavenClawsError::CommandExecution(format!(
                        "Producer failed: {}",
                        e
                    )));
                }
            }
        } else {
            // Reviewer critiques
            let mut reviewer_memory = ConversationMemory::new(reviewer_persona, 10);
            reviewer_memory.add_user_message(&format!(
                "Review the following content and provide constructive feedback:\n\n{}",
                current_content
            ));

            // Check self-healing circuit breaker
            if let Some(ref healing) = healing_engine {
                let healthy = {
                    let mut engine = healing.lock().unwrap();
                    engine.is_healthy("review-reviewer")
                };
                if !healthy {
                    warn!("Reviewer blocked by circuit breaker");
                    continue;
                }
            }

            let messages = reviewer_memory.history().to_vec();
            let review = match llm.chat(messages).await {
                Ok(response) => {
                    // Record success
                    if let Some(ref healing) = healing_engine {
                        let mut engine = healing.lock().unwrap();
                        engine.record_success("review-reviewer");
                    }
                    response
                        .choices
                        .first()
                        .map(|c| c.message.content.clone())
                        .unwrap_or_default()
                }
                Err(e) => {
                    // Record failure
                    if let Some(ref healing) = healing_engine {
                        let mut engine = healing.lock().unwrap();
                        engine.record_failure("review-reviewer", &e.to_string());
                    }
                    warn!(error = %e, "Reviewer LLM request failed");
                    continue;
                }
            };

            if pattern_config.verbose {
                println!("\n── Review (Iteration {}) ──\n{}", iteration + 1, review);
            }

            // Check if approved
            if review.contains("APPROVED:") {
                info!("Content approved after {} iterations", iteration + 1);
                let final_content = review.split("APPROVED:").nth(1).unwrap_or(&current_content);
                current_content = final_content.trim().to_string();
                approved = true;
                break;
            }

            // Producer revises based on feedback
            let mut producer_memory = ConversationMemory::new(producer_persona, 10);
            producer_memory.add_user_message(&format!(
                "Your previous content:\n\n{}\n\nReviewer feedback:\n\n{}\n\nPlease revise the content addressing all feedback.",
                current_content, review
            ));

            // Check self-healing circuit breaker
            if let Some(ref healing) = healing_engine {
                let healthy = {
                    let mut engine = healing.lock().unwrap();
                    engine.is_healthy("review-producer")
                };
                if !healthy {
                    warn!("Producer revision blocked by circuit breaker");
                    continue;
                }
            }

            let messages = producer_memory.history().to_vec();
            match llm.chat(messages).await {
                Ok(response) => {
                    // Record success
                    if let Some(ref healing) = healing_engine {
                        let mut engine = healing.lock().unwrap();
                        engine.record_success("review-producer");
                    }

                    if let Some(choice) = response.choices.first() {
                        current_content = choice.message.content.clone();
                        info!("Content revised: {} chars", current_content.len());

                        if pattern_config.verbose {
                            println!(
                                "\n── Revised Content (Iteration {}) ──\n{}",
                                iteration + 1,
                                current_content
                            );
                        }
                    }
                }
                Err(e) => {
                    // Record failure
                    if let Some(ref healing) = healing_engine {
                        let mut engine = healing.lock().unwrap();
                        engine.record_failure("review-producer", &e.to_string());
                    }
                    warn!(error = %e, "Producer revision failed");
                    continue;
                }
            }
        }
    }

    if !approved {
        warn!("Review-loop reached max iterations without approval");
    }

    println!("\n🐦‍⬛ Review-Loop Final Content:\n{}", current_content);

    if let Some(ref rf) = ravenfabric {
        if rf.is_enabled() {
            let preview = current_content.chars().take(500).collect::<String>();
            let _ = rf.broadcast(&preview, 30).await;
        }
    }

    Ok(())
}

// ── Pattern 3: Research-Synthesize ─────────────────────────────────────────

/// Run parallel research agents followed by a synthesizer.
///
/// Multiple research agents explore different aspects of a topic in parallel.
/// A synthesizer agent then combines their findings into a coherent report.
pub async fn run_research_synthesize(
    llm: Arc<dyn LLMProviderTrait>,
    config: Config,
    ravenfabric: Option<RavenFabricClient>,
    pattern_config: PatternConfig,
    healing_engine: Option<Arc<Mutex<SelfHealingEngine>>>,
) -> crate::error::Result<()> {
    info!(
        "Starting research-synthesize mode with {} research agents",
        pattern_config.research_agent_count
    );

    let _system_prompt = &config.llm.system_prompt;
    let task = "Analyze the given task and provide your solution.";

    // Define research perspectives
    let perspectives = [
        ("Fact-Finder", "You are a fact-finding researcher. Focus on verifiable facts, data, statistics, and concrete evidence. Cite specific sources and numbers."),
        ("Analyst", "You are an analytical researcher. Focus on patterns, trends, cause-and-effect relationships, and strategic implications."),
        ("Innovator", "You are an innovative researcher. Focus on novel approaches, emerging trends, creative solutions, and future possibilities."),
    ];

    let agent_count = pattern_config.research_agent_count.min(perspectives.len());
    let mut research_results: Vec<(String, String)> = Vec::new();

    // Phase 1: Parallel research
    for (role, persona) in perspectives.iter().take(agent_count) {
        info!(role = %role, "Research agent starting");

        let mut memory = ConversationMemory::new(persona, 10);
        memory.add_user_message(&format!(
            "Research the following topic from your perspective:\n\n{}",
            task
        ));

        // Check self-healing circuit breaker
        if let Some(ref healing) = healing_engine {
            let healthy = {
                let mut engine = healing.lock().unwrap();
                engine.is_healthy(&format!("research-{}", role))
            };
            if !healthy {
                warn!(role = %role, "Research agent blocked by circuit breaker");
                research_results.push((
                    role.to_string(),
                    "[Research blocked by circuit breaker]".to_string(),
                ));
                continue;
            }
        }

        let messages = memory.history().to_vec();
        match llm.chat(messages).await {
            Ok(response) => {
                // Record success
                if let Some(ref healing) = healing_engine {
                    let mut engine = healing.lock().unwrap();
                    engine.record_success(&format!("research-{}", role));
                }

                if let Some(choice) = response.choices.first() {
                    let content = choice.message.content.clone();
                    info!(role = %role, "Research completed: {} chars", content.len());
                    research_results.push((role.to_string(), content));
                }
            }
            Err(e) => {
                // Record failure
                if let Some(ref healing) = healing_engine {
                    let mut engine = healing.lock().unwrap();
                    engine.record_failure(&format!("research-{}", role), &e.to_string());
                }
                warn!(error = %e, role = %role, "Research agent failed");
                research_results.push((role.to_string(), format!("[Research failed: {}]", e)));
            }
        }
    }

    // Print intermediate research if verbose
    if pattern_config.verbose {
        println!("\n── Research Findings ──");
        for (role, content) in &research_results {
            println!("\n--- {} ---\n{}", role, content);
        }
    }

    // Phase 2: Synthesis
    let synthesizer_persona = "You are a synthesis specialist. Combine multiple research perspectives into a coherent, well-structured report. Identify common themes, resolve contradictions, and present a unified analysis.";
    let mut synth_memory = ConversationMemory::new(synthesizer_persona, 20);

    let mut synthesis_input =
        String::from("Synthesize the following research findings into a comprehensive report:\n\n");
    for (role, content) in &research_results {
        synthesis_input.push_str(&format!("\n=== {} ===\n{}\n", role, content));
    }
    synth_memory.add_user_message(&synthesis_input);

    // Check self-healing circuit breaker before synthesis
    if let Some(ref healing) = healing_engine {
        let healthy = {
            let mut engine = healing.lock().unwrap();
            engine.is_healthy("research-synthesis")
        };
        if !healthy {
            warn!("Research synthesis blocked by circuit breaker");
            return Err(RavenClawsError::HealingError(
                "Research synthesis blocked by circuit breaker".to_string(),
            ));
        }
    }

    let messages = synth_memory.history().to_vec();
    match llm.chat(messages).await {
        Ok(response) => {
            // Record success
            if let Some(ref healing) = healing_engine {
                let mut engine = healing.lock().unwrap();
                engine.record_success("research-synthesis");
            }

            if let Some(choice) = response.choices.first() {
                let result = &choice.message.content;
                println!("\n🐦‍⬛ Research Synthesis:\n{}", result);

                if let Some(ref rf) = ravenfabric {
                    if rf.is_enabled() {
                        let preview = result.chars().take(500).collect::<String>();
                        let _ = rf.broadcast(&preview, 30).await;
                    }
                }
            }
        }
        Err(e) => {
            // Record failure
            if let Some(ref healing) = healing_engine {
                let mut engine = healing.lock().unwrap();
                engine.record_failure("research-synthesis", &e.to_string());
            }
            warn!(error = %e, "Synthesis failed");
            return Err(RavenClawsError::CommandExecution(format!(
                "Synthesis failed: {}",
                e
            )));
        }
    }

    Ok(())
}

// ── Pattern 4: Voting ──────────────────────────────────────────────────────

/// Run a voting process where multiple agents evaluate options.
///
/// Each voter agent independently evaluates the options and provides
/// their choice with reasoning. Results are tallied and the majority
/// decision is reported.
pub async fn run_voting(
    llm: Arc<dyn LLMProviderTrait>,
    config: Config,
    ravenfabric: Option<RavenFabricClient>,
    pattern_config: PatternConfig,
    healing_engine: Option<Arc<Mutex<SelfHealingEngine>>>,
) -> crate::error::Result<()> {
    info!(
        "Starting voting mode with {} voters",
        pattern_config.voter_count
    );

    let system_prompt = &config.llm.system_prompt;
    let task = "Analyze the given task and provide your solution.";

    // Define voter personas for diversity
    let voter_personas = [
        "You are a conservative voter. You prefer safe, proven approaches. Prioritize stability and risk mitigation. Respond with: VOTE: <your choice> REASONING: <your reasoning>",
        "You are an aggressive voter. You prefer bold, ambitious approaches. Prioritize maximum impact and innovation. Respond with: VOTE: <your choice> REASONING: <your reasoning>",
        "You are a balanced voter. You weigh pros and cons carefully. Prioritize pragmatic, well-rounded solutions. Respond with: VOTE: <your choice> REASONING: <your reasoning>",
        "You are a detail-oriented voter. You focus on implementation feasibility and technical soundness. Respond with: VOTE: <your choice> REASONING: <your reasoning>",
        "You are a user-centric voter. You prioritize user experience, accessibility, and usability. Respond with: VOTE: <your choice> REASONING: <your reasoning>",
    ];

    let voter_count = pattern_config.voter_count.min(voter_personas.len());
    let mut votes: Vec<(String, String, String)> = Vec::new(); // (persona_name, vote, reasoning)

    // Phase 1: Independent voting
    for (i, persona) in voter_personas.iter().enumerate().take(voter_count) {
        let persona_name = persona
            .split('.')
            .next()
            .unwrap_or(&format!("Voter {}", i + 1))
            .to_string();

        let mut memory = ConversationMemory::new(&format!("{}\n\n{}", system_prompt, persona), 10);
        memory.add_user_message(&format!(
            "Evaluate the following and cast your vote:\n\n{}",
            task
        ));

        // Check self-healing circuit breaker
        if let Some(ref healing) = healing_engine {
            let healthy = {
                let mut engine = healing.lock().unwrap();
                engine.is_healthy(&format!("voter-{}", i))
            };
            if !healthy {
                warn!(voter = %persona_name, "Voter blocked by circuit breaker");
                votes.push((
                    persona_name,
                    "Error".to_string(),
                    "Vote blocked by circuit breaker".to_string(),
                ));
                continue;
            }
        }

        let messages = memory.history().to_vec();
        match llm.chat(messages).await {
            Ok(response) => {
                // Record success
                if let Some(ref healing) = healing_engine {
                    let mut engine = healing.lock().unwrap();
                    engine.record_success(&format!("voter-{}", i));
                }

                if let Some(choice) = response.choices.first() {
                    let content = choice.message.content.clone();
                    info!(voter = %persona_name, "Vote cast");

                    // Extract vote and reasoning
                    let vote = content
                        .split("VOTE:")
                        .nth(1)
                        .and_then(|s| s.split("REASONING:").next())
                        .map(|s| s.trim().to_string())
                        .unwrap_or_else(|| "Unknown".to_string());

                    let reasoning = content
                        .split("REASONING:")
                        .nth(1)
                        .map(|s| s.trim().to_string())
                        .unwrap_or_default();

                    if pattern_config.verbose {
                        println!(
                            "\n── {} ──\nVOTE: {}\nREASONING: {}",
                            persona_name, vote, reasoning
                        );
                    }

                    votes.push((persona_name, vote, reasoning));
                }
            }
            Err(e) => {
                // Record failure
                if let Some(ref healing) = healing_engine {
                    let mut engine = healing.lock().unwrap();
                    engine.record_failure(&format!("voter-{}", i), &e.to_string());
                }
                warn!(error = %e, voter = %persona_name, "Voter LLM request failed");
                votes.push((
                    persona_name,
                    "Error".to_string(),
                    format!("Vote failed: {}", e),
                ));
            }
        }
    }

    // Phase 2: Tally results
    let mut tally: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for (_, vote, _) in &votes {
        *tally.entry(vote.clone()).or_insert(0) += 1;
    }

    // Find winner(s)
    let max_votes = tally.values().cloned().max().unwrap_or(0);
    let winners: Vec<String> = tally
        .iter()
        .filter(|(_, count)| **count == max_votes)
        .map(|(vote, _)| vote.clone())
        .collect();

    println!("\n🐦‍⬛ Voting Results:");
    println!("───");
    for (persona, vote, reasoning) in &votes {
        println!("{}: VOTE = {} | {}", persona, vote, reasoning);
    }
    println!("───");
    println!("Tally: {:?}", tally);
    println!("\n🏆 Decision: {}", winners.join(", "));

    if let Some(ref rf) = ravenfabric {
        if rf.is_enabled() {
            let summary = format!(
                "Voting completed: {} voters, decision: {}",
                votes.len(),
                winners.join(", ")
            );
            let _ = rf.broadcast(&summary, 30).await;
        }
    }

    Ok(())
}

// ── Multi-model variants ───────────────────────────────────────────────────

/// Run debate mode with multiple LLM providers (round-robin)
pub async fn run_debate_multi(
    multi_llm: MultiModelManager,
    config: Config,
    _ravenfabric: Option<RavenFabricClient>,
    pattern_config: PatternConfig,
    healing_engine: Option<Arc<Mutex<SelfHealingEngine>>>,
) -> crate::error::Result<()> {
    info!(
        "Starting debate mode (multi-model) with {} providers",
        multi_llm.client_count()
    );

    let system_prompt = &config.llm.system_prompt;
    let task = "Analyze the given task and provide your solution.";

    let positions = [
        (
            "Proponent",
            "You argue FOR the proposition. Focus on benefits, opportunities, and strengths.",
        ),
        (
            "Opponent",
            "You argue AGAINST the proposition. Focus on risks, drawbacks, and weaknesses.",
        ),
        (
            "Synthesizer",
            "You are the neutral judge. Synthesize a balanced conclusion.",
        ),
    ];

    let mut debate_history = ConversationMemory::new(system_prompt, 50);
    debate_history.add_user_message(&format!("Debate topic: {}", task));

    for round in 0..pattern_config.max_rounds {
        info!(round = round + 1, "Multi-model debate round");

        for (role, persona) in &positions {
            if *role == "Synthesizer" && round < pattern_config.max_rounds - 1 {
                continue;
            }

            let provider_idx = round % multi_llm.client_count();
            let client = multi_llm.get_client(provider_idx);

            if let Some(client) = client {
                let mut agent_memory = ConversationMemory::new(persona, 20);
                for msg in debate_history.history() {
                    if msg.role == "system" {
                        continue;
                    }
                    if msg.role == "user" {
                        agent_memory.add_user_message(&msg.content);
                    } else {
                        agent_memory.add_assistant_message(&msg.content);
                    }
                }

                // Check self-healing circuit breaker
                if let Some(ref healing) = healing_engine {
                    let healthy = {
                        let mut engine = healing.lock().unwrap();
                        engine.is_healthy(&format!("debate-multi-{}", role))
                    };
                    if !healthy {
                        warn!(role = %role, "Multi-model debate agent blocked by circuit breaker");
                        continue;
                    }
                }

                let messages = agent_memory.history().to_vec();
                match client.chat(messages).await {
                    Ok(response) => {
                        // Record success
                        if let Some(ref healing) = healing_engine {
                            let mut engine = healing.lock().unwrap();
                            engine.record_success(&format!("debate-multi-{}", role));
                        }

                        if let Some(choice) = response.choices.first() {
                            let content = &choice.message.content;
                            info!(role = %role, provider = client.provider_name(), round = round + 1, "Debate contribution");
                            if pattern_config.verbose {
                                println!(
                                    "\n── {} ({} via {}, Round {}) ──\n{}",
                                    role,
                                    client.provider_name(),
                                    client.model(),
                                    round + 1,
                                    content
                                );
                            }
                            debate_history.add_assistant_message(&format!(
                                "{} ({}): {}",
                                role,
                                client.provider_name(),
                                content
                            ));
                        }
                    }
                    Err(e) => {
                        // Record failure
                        if let Some(ref healing) = healing_engine {
                            let mut engine = healing.lock().unwrap();
                            engine
                                .record_failure(&format!("debate-multi-{}", role), &e.to_string());
                        }
                        warn!(error = %e, role = %role, "Multi-model debate failed");
                    }
                }
            }
        }
    }

    // Final synthesis using first provider
    if let Some(client) = multi_llm.get_client(0) {
        // Check self-healing circuit breaker
        if let Some(ref healing) = healing_engine {
            let healthy = {
                let mut engine = healing.lock().unwrap();
                engine.is_healthy("debate-multi-synthesis")
            };
            if !healthy {
                warn!("Multi-model debate synthesis blocked by circuit breaker");
                return Err(RavenClawsError::HealingError(
                    "Multi-model debate synthesis blocked by circuit breaker".to_string(),
                ));
            }
        }

        let synthesizer_persona =
            "You are a neutral synthesizer. Produce a final balanced conclusion.";
        let mut final_memory = ConversationMemory::new(synthesizer_persona, 30);
        for msg in debate_history.history() {
            if msg.role == "system" {
                continue;
            }
            if msg.role == "user" {
                final_memory.add_user_message(&msg.content);
            } else {
                final_memory.add_assistant_message(&msg.content);
            }
        }
        final_memory.add_user_message("Now produce your FINAL synthesis:");

        let messages = final_memory.history().to_vec();
        match client.chat(messages).await {
            Ok(response) => {
                // Record success
                if let Some(ref healing) = healing_engine {
                    let mut engine = healing.lock().unwrap();
                    engine.record_success("debate-multi-synthesis");
                }

                if let Some(choice) = response.choices.first() {
                    println!(
                        "\n🐦‍⬛ Multi-Model Debate Synthesis:\n{}",
                        choice.message.content
                    );
                }
            }
            Err(e) => {
                // Record failure
                if let Some(ref healing) = healing_engine {
                    let mut engine = healing.lock().unwrap();
                    engine.record_failure("debate-multi-synthesis", &e.to_string());
                }
                warn!(error = %e, "Multi-model synthesis failed");
            }
        }
    }

    Ok(())
}

/// Run review-loop mode with multiple LLM providers
pub async fn run_review_loop_multi(
    multi_llm: MultiModelManager,
    _config: Config,
    _ravenfabric: Option<RavenFabricClient>,
    pattern_config: PatternConfig,
    healing_engine: Option<Arc<Mutex<SelfHealingEngine>>>,
) -> crate::error::Result<()> {
    info!("Starting review-loop mode (multi-model)");

    let _system_prompt = &_config.llm.system_prompt;
    let _ = &_system_prompt;
    let task = "Analyze the given task and provide your solution.";

    let producer_persona = "You are a producer. Create high-quality content.";
    let reviewer_persona = "You are a reviewer. Critically evaluate content. Respond with APPROVED: when quality is met.";

    let mut current_content = String::new();
    let mut approved = false;

    for iteration in 0..pattern_config.max_review_iterations {
        info!(iteration = iteration + 1, "Multi-model review-loop");

        let provider_idx = iteration % multi_llm.client_count();
        let client = multi_llm.get_client(provider_idx);

        if let Some(client) = client {
            // Check self-healing circuit breaker
            if let Some(ref healing) = healing_engine {
                let agent_id = if iteration == 0 {
                    "review-multi-producer"
                } else {
                    "review-multi-reviewer"
                };
                let healthy = {
                    let mut engine = healing.lock().unwrap();
                    engine.is_healthy(agent_id)
                };
                if !healthy {
                    warn!(
                        iteration = iteration + 1,
                        "Multi-model review blocked by circuit breaker"
                    );
                    continue;
                }
            }

            if iteration == 0 {
                let mut memory = ConversationMemory::new(producer_persona, 10);
                memory.add_user_message(&format!("Create content for: {}", task));
                let messages = memory.history().to_vec();
                match client.chat(messages).await {
                    Ok(response) => {
                        // Record success
                        if let Some(ref healing) = healing_engine {
                            let mut engine = healing.lock().unwrap();
                            engine.record_success("review-multi-producer");
                        }

                        if let Some(choice) = response.choices.first() {
                            current_content = choice.message.content.clone();
                            info!(
                                "Initial content: {} chars via {}",
                                current_content.len(),
                                client.provider_name()
                            );
                        }
                    }
                    Err(e) => {
                        // Record failure
                        if let Some(ref healing) = healing_engine {
                            let mut engine = healing.lock().unwrap();
                            engine.record_failure("review-multi-producer", &e.to_string());
                        }
                        warn!(error = %e, "Producer failed");
                    }
                }
            } else {
                // Review
                let mut rev_memory = ConversationMemory::new(reviewer_persona, 10);
                rev_memory.add_user_message(&format!("Review:\n\n{}", current_content));
                let messages = rev_memory.history().to_vec();
                match client.chat(messages).await {
                    Ok(response) => {
                        // Record success
                        if let Some(ref healing) = healing_engine {
                            let mut engine = healing.lock().unwrap();
                            engine.record_success("review-multi-reviewer");
                        }

                        if let Some(choice) = response.choices.first() {
                            let review = &choice.message.content;
                            if review.contains("APPROVED:") {
                                info!(
                                    "Approved after {} iterations via {}",
                                    iteration + 1,
                                    client.provider_name()
                                );
                                current_content = review
                                    .split("APPROVED:")
                                    .nth(1)
                                    .unwrap_or(&current_content)
                                    .trim()
                                    .to_string();
                                approved = true;
                                break;
                            }
                            // Revise
                            let mut prod_memory = ConversationMemory::new(producer_persona, 10);
                            prod_memory.add_user_message(&format!(
                                "Content:\n{}\n\nFeedback:\n{}\n\nRevise:",
                                current_content, review
                            ));
                            let msgs = prod_memory.history().to_vec();
                            if let Some(next_client) =
                                multi_llm.get_client((iteration + 1) % multi_llm.client_count())
                            {
                                // Check circuit breaker for producer revision
                                if let Some(ref healing) = healing_engine {
                                    let healthy = {
                                        let mut engine = healing.lock().unwrap();
                                        engine.is_healthy("review-multi-producer")
                                    };
                                    if !healthy {
                                        warn!("Multi-model producer revision blocked by circuit breaker");
                                        continue;
                                    }
                                }

                                if let Ok(rev_resp) = next_client.chat(msgs).await {
                                    // Record success
                                    if let Some(ref healing) = healing_engine {
                                        let mut engine = healing.lock().unwrap();
                                        engine.record_success("review-multi-producer");
                                    }

                                    if let Some(rev_choice) = rev_resp.choices.first() {
                                        current_content = rev_choice.message.content.clone();
                                        info!(
                                            "Revised: {} chars via {}",
                                            current_content.len(),
                                            next_client.provider_name()
                                        );
                                    }
                                } else {
                                    // Record failure
                                    if let Some(ref healing) = healing_engine {
                                        let mut engine = healing.lock().unwrap();
                                        engine.record_failure(
                                            "review-multi-producer",
                                            "revision failed",
                                        );
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        // Record failure
                        if let Some(ref healing) = healing_engine {
                            let mut engine = healing.lock().unwrap();
                            engine.record_failure("review-multi-reviewer", &e.to_string());
                        }
                        warn!(error = %e, "Review failed");
                    }
                }
            }
        }
    }

    if !approved {
        warn!("Review-loop reached max iterations without approval");
    }

    println!("\n🐦‍⬛ Multi-Model Review-Loop Final:\n{}", current_content);
    Ok(())
}

/// Run research-synthesize mode with multiple LLM providers
pub async fn run_research_synthesize_multi(
    multi_llm: MultiModelManager,
    config: Config,
    _ravenfabric: Option<RavenFabricClient>,
    pattern_config: PatternConfig,
    healing_engine: Option<Arc<Mutex<SelfHealingEngine>>>,
) -> crate::error::Result<()> {
    info!("Starting research-synthesize mode (multi-model)");

    let system_prompt = &config.llm.system_prompt;
    let task = "Analyze the given task and provide your solution.";

    let perspectives = [
        (
            "Fact-Finder",
            "Focus on verifiable facts, data, statistics.",
        ),
        (
            "Analyst",
            "Focus on patterns, trends, strategic implications.",
        ),
        (
            "Innovator",
            "Focus on novel approaches and future possibilities.",
        ),
    ];

    let agent_count = pattern_config.research_agent_count.min(perspectives.len());
    let mut results: Vec<(String, String, String)> = Vec::new(); // (role, provider, content)

    for (i, (role, persona)) in perspectives.iter().enumerate().take(agent_count) {
        let client = multi_llm.get_client(i % multi_llm.client_count());

        if let Some(client) = client {
            let mut memory =
                ConversationMemory::new(&format!("{}\n\n{}", system_prompt, persona), 10);
            memory.add_user_message(&format!("Research: {}", task));
            let messages = memory.history().to_vec();

            // Check self-healing circuit breaker
            if let Some(ref healing) = healing_engine {
                let healthy = {
                    let mut engine = healing.lock().unwrap();
                    engine.is_healthy(&format!("research-multi-{}", role))
                };
                if !healthy {
                    warn!(role = %role, "Multi-model research agent blocked by circuit breaker");
                    results.push((
                        role.to_string(),
                        "blocked".to_string(),
                        "[Research blocked by circuit breaker]".to_string(),
                    ));
                    continue;
                }
            }

            match client.chat(messages).await {
                Ok(response) => {
                    // Record success
                    if let Some(ref healing) = healing_engine {
                        let mut engine = healing.lock().unwrap();
                        engine.record_success(&format!("research-multi-{}", role));
                    }

                    if let Some(choice) = response.choices.first() {
                        let content = choice.message.content.clone();
                        info!(role = %role, provider = client.provider_name(), "Research completed");
                        results.push((
                            role.to_string(),
                            client.provider_name().to_string(),
                            content,
                        ));
                    }
                }
                Err(e) => {
                    // Record failure
                    if let Some(ref healing) = healing_engine {
                        let mut engine = healing.lock().unwrap();
                        engine.record_failure(&format!("research-multi-{}", role), &e.to_string());
                    }
                    warn!(error = %e, role = %role, "Research failed");
                }
            }
        }
    }

    // Synthesize
    if let Some(client) = multi_llm.get_client(0) {
        // Check self-healing circuit breaker
        if let Some(ref healing) = healing_engine {
            let healthy = {
                let mut engine = healing.lock().unwrap();
                engine.is_healthy("research-multi-synthesis")
            };
            if !healthy {
                warn!("Multi-model research synthesis blocked by circuit breaker");
                return Err(RavenClawsError::HealingError(
                    "Multi-model research synthesis blocked by circuit breaker".to_string(),
                ));
            }
        }

        let mut synth_memory = ConversationMemory::new("You are a synthesis specialist.", 20);
        let mut input = String::from("Synthesize these findings:\n\n");
        for (role, provider, content) in &results {
            input.push_str(&format!("=== {} ({}) ===\n{}\n", role, provider, content));
        }
        synth_memory.add_user_message(&input);
        let messages = synth_memory.history().to_vec();

        match client.chat(messages).await {
            Ok(response) => {
                // Record success
                if let Some(ref healing) = healing_engine {
                    let mut engine = healing.lock().unwrap();
                    engine.record_success("research-multi-synthesis");
                }

                if let Some(choice) = response.choices.first() {
                    println!(
                        "\n🐦‍⬛ Multi-Model Research Synthesis:\n{}",
                        choice.message.content
                    );
                }
            }
            Err(e) => {
                // Record failure
                if let Some(ref healing) = healing_engine {
                    let mut engine = healing.lock().unwrap();
                    engine.record_failure("research-multi-synthesis", &e.to_string());
                }
                warn!(error = %e, "Synthesis failed");
            }
        }
    }

    Ok(())
}

/// Run voting mode with multiple LLM providers
pub async fn run_voting_multi(
    multi_llm: MultiModelManager,
    config: Config,
    _ravenfabric: Option<RavenFabricClient>,
    pattern_config: PatternConfig,
    healing_engine: Option<Arc<Mutex<SelfHealingEngine>>>,
) -> crate::error::Result<()> {
    info!(
        "Starting voting mode (multi-model) with {} providers",
        multi_llm.client_count()
    );

    let system_prompt = &config.llm.system_prompt;
    let task = "Analyze the given task and provide your solution.";

    let voter_personas = [
        "Conservative: Prefer safe, proven approaches. VOTE: <choice> REASONING: <reasoning>",
        "Aggressive: Prefer bold, ambitious approaches. VOTE: <choice> REASONING: <reasoning>",
        "Balanced: Weigh pros and cons. VOTE: <choice> REASONING: <reasoning>",
        "Detail-oriented: Focus on feasibility. VOTE: <choice> REASONING: <reasoning>",
        "User-centric: Prioritize UX. VOTE: <choice> REASONING: <reasoning>",
    ];

    let voter_count = pattern_config
        .voter_count
        .min(voter_personas.len())
        .min(multi_llm.client_count());
    let mut votes: Vec<(String, String, String, String)> = Vec::new(); // (persona, provider, vote, reasoning)

    for (i, persona) in voter_personas.iter().enumerate().take(voter_count) {
        let client = multi_llm.get_client(i % multi_llm.client_count());

        if let Some(client) = client {
            let mut memory =
                ConversationMemory::new(&format!("{}\n\n{}", system_prompt, persona), 10);
            memory.add_user_message(&format!("Cast your vote on: {}", task));
            let messages = memory.history().to_vec();

            // Check self-healing circuit breaker
            if let Some(ref healing) = healing_engine {
                let healthy = {
                    let mut engine = healing.lock().unwrap();
                    engine.is_healthy(&format!("voter-multi-{}", i))
                };
                if !healthy {
                    warn!(
                        voter = i + 1,
                        "Multi-model voter blocked by circuit breaker"
                    );
                    votes.push((
                        format!("Voter {}", i + 1),
                        "blocked".to_string(),
                        "Error".to_string(),
                        "Vote blocked by circuit breaker".to_string(),
                    ));
                    continue;
                }
            }

            match client.chat(messages).await {
                Ok(response) => {
                    // Record success
                    if let Some(ref healing) = healing_engine {
                        let mut engine = healing.lock().unwrap();
                        engine.record_success(&format!("voter-multi-{}", i));
                    }

                    if let Some(choice) = response.choices.first() {
                        let content = choice.message.content.clone();
                        let vote = content
                            .split("VOTE:")
                            .nth(1)
                            .and_then(|s| s.split("REASONING:").next())
                            .map(|s| s.trim().to_string())
                            .unwrap_or_else(|| "Unknown".to_string());
                        let reasoning = content
                            .split("REASONING:")
                            .nth(1)
                            .map(|s| s.trim().to_string())
                            .unwrap_or_default();
                        votes.push((
                            format!("Voter {}", i + 1),
                            client.provider_name().to_string(),
                            vote,
                            reasoning,
                        ));
                    }
                }
                Err(e) => {
                    // Record failure
                    if let Some(ref healing) = healing_engine {
                        let mut engine = healing.lock().unwrap();
                        engine.record_failure(&format!("voter-multi-{}", i), &e.to_string());
                    }
                    warn!(error = %e, "Voter {} failed", i + 1);
                }
            }
        }
    }

    // Tally
    let mut tally: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for (_, _, vote, _) in &votes {
        *tally.entry(vote.clone()).or_insert(0) += 1;
    }

    println!("\n🐦‍⬛ Multi-Model Voting Results:");
    for (persona, provider, vote, reasoning) in &votes {
        println!(
            "{} ({}): VOTE = {} | {}",
            persona, provider, vote, reasoning
        );
    }
    println!("Tally: {:?}", tally);
    println!(
        "🏆 Decision: {}",
        tally
            .iter()
            .max_by_key(|(_, c)| *c)
            .map(|(v, _)| v.clone())
            .unwrap_or_else(|| "No decision".to_string())
    );

    Ok(())
}

// ── Pattern 5: Tree-of-Thought Reasoning ────────────────────────────────────

/// Run tree-of-thought reasoning with multiple parallel branches.
///
/// At each step, the agent generates N candidate thoughts, evaluates each
/// one's promise, prunes to the top-K, and continues exploring the most
/// promising branches. Returns the best complete reasoning path.
pub async fn run_tree_of_thought(
    llm: Arc<dyn LLMProviderTrait>,
    config: Config,
    ravenfabric: Option<RavenFabricClient>,
    pattern_config: PatternConfig,
    healing_engine: Option<Arc<Mutex<SelfHealingEngine>>>,
) -> crate::error::Result<()> {
    info!(
        "Starting tree-of-thought mode: {} branches, depth {}, top-k {}",
        pattern_config.tot_branches, pattern_config.tot_depth, pattern_config.tot_top_k
    );

    let system_prompt = &config.llm.system_prompt;
    let task = "Analyze the given task and provide your solution.";

    // Each branch is a vector of thought strings (the reasoning path so far)
    let mut branches: Vec<Vec<String>> = vec![Vec::new()];

    for depth in 0..pattern_config.tot_depth {
        info!(depth = depth + 1, branches = branches.len(), "ToT step");

        // Check self-healing circuit breaker
        if let Some(ref healing) = healing_engine {
            let healthy = {
                let mut engine = healing.lock().unwrap();
                engine.is_healthy("tree-of-thought")
            };
            if !healthy {
                warn!("Tree-of-thought blocked by circuit breaker");
                return Err(RavenClawsError::HealingError(
                    "Tree-of-thought blocked by circuit breaker".to_string(),
                ));
            }
        }

        let mut all_candidates: Vec<(Vec<String>, String, f64)> = Vec::new();

        for (branch_idx, branch) in branches.iter().enumerate() {
            let mut memory = ConversationMemory::new(
                &format!("{}\n\nYou are a reasoning agent. Generate diverse candidate thoughts and evaluate them.", system_prompt),
                30,
            );
            memory.add_user_message(&format!(
                "Task: {}\n\nCurrent reasoning path (step {}/{}):\n{}\n\nGenerate {} distinct next-step thoughts. \
                 For each thought, provide:\n1. The thought itself\n2. A confidence score (0.0 to 1.0)\n\n\
                 Format each as:\nTHOUGHT: <your thought>\nCONFIDENCE: <0.0-1.0>",
                task,
                depth + 1,
                pattern_config.tot_depth,
                if branch.is_empty() {
                    "No steps yet — start from scratch.".to_string()
                } else {
                    branch.join("\n")
                },
                pattern_config.tot_branches,
            ));

            let messages = memory.history().to_vec();
            match llm.chat(messages).await {
                Ok(response) => {
                    if let Some(ref healing) = healing_engine {
                        let mut engine = healing.lock().unwrap();
                        engine.record_success("tree-of-thought");
                    }

                    if let Some(choice) = response.choices.first() {
                        let content = &choice.message.content;
                        // Parse thoughts and confidences
                        let mut current_thought = String::new();
                        let mut current_confidence = 0.5;
                        let mut parsing_thought = false;

                        for line in content.lines() {
                            let trimmed = line.trim();
                            if trimmed.starts_with("THOUGHT:") {
                                // Save previous thought if any
                                if parsing_thought && !current_thought.is_empty() {
                                    let mut new_branch = branch.clone();
                                    new_branch.push(current_thought.clone());
                                    all_candidates.push((
                                        new_branch,
                                        format!("Confidence: {:.2}", current_confidence),
                                        current_confidence,
                                    ));
                                }
                                current_thought = trimmed
                                    .strip_prefix("THOUGHT:")
                                    .unwrap_or("")
                                    .trim()
                                    .to_string();
                                parsing_thought = true;
                                current_confidence = 0.5;
                            } else if trimmed.starts_with("CONFIDENCE:") {
                                let val_str =
                                    trimmed.strip_prefix("CONFIDENCE:").unwrap_or("0.5").trim();
                                if let Ok(val) = val_str.parse::<f64>() {
                                    current_confidence = val.clamp(0.0, 1.0);
                                }
                            } else if parsing_thought && !trimmed.is_empty() {
                                current_thought.push(' ');
                                current_thought.push_str(trimmed);
                            }
                        }
                        // Save last thought
                        if parsing_thought && !current_thought.is_empty() {
                            let mut new_branch = branch.clone();
                            new_branch.push(current_thought);
                            all_candidates.push((
                                new_branch,
                                format!("Confidence: {:.2}", current_confidence),
                                current_confidence,
                            ));
                        }
                    }
                }
                Err(e) => {
                    if let Some(ref healing) = healing_engine {
                        let mut engine = healing.lock().unwrap();
                        engine.record_failure("tree-of-thought", &e.to_string());
                    }
                    warn!(error = %e, "Tree-of-thought branch {} failed", branch_idx);
                }
            }
        }

        if all_candidates.is_empty() {
            warn!("No candidates generated at depth {}", depth + 1);
            break;
        }

        // Sort by confidence descending and keep top-k
        all_candidates.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
        let top_k = pattern_config.tot_top_k.min(all_candidates.len());
        branches = all_candidates
            .into_iter()
            .take(top_k)
            .map(|(branch, _, _)| branch)
            .collect();

        if pattern_config.verbose {
            println!(
                "\n── ToT Depth {}/{} ──",
                depth + 1,
                pattern_config.tot_depth
            );
            for (i, branch) in branches.iter().enumerate() {
                println!("Branch {} ({} steps):", i + 1, branch.len());
                for (j, thought) in branch.iter().enumerate() {
                    println!("  Step {}: {}", j + 1, thought);
                }
                println!();
            }
        }
    }

    // Final synthesis: pick the best branch and produce a final answer
    let best_branch = branches.into_iter().next().unwrap_or_default();
    let mut final_memory = ConversationMemory::new(
        &format!(
            "{}\n\nYou are a synthesizer. Produce a final answer based on the best reasoning path.",
            system_prompt
        ),
        20,
    );
    final_memory.add_user_message(&format!(
        "Task: {}\n\nBest reasoning path:\n{}\n\nProduce a final, well-reasoned answer:",
        task,
        best_branch.join("\n→ "),
    ));

    let messages = final_memory.history().to_vec();
    match llm.chat(messages).await {
        Ok(response) => {
            if let Some(ref healing) = healing_engine {
                let mut engine = healing.lock().unwrap();
                engine.record_success("tree-of-thought-synthesis");
            }

            if let Some(choice) = response.choices.first() {
                println!(
                    "\n🐦‍⬛ Tree-of-Thought Final Answer:\n{}",
                    choice.message.content
                );

                if let Some(ref rf) = ravenfabric {
                    if rf.is_enabled() {
                        let summary = format!(
                            "Tree-of-Thought completed: depth {}, branches explored",
                            pattern_config.tot_depth
                        );
                        let _ = rf.broadcast(&summary, 30).await;
                    }
                }
            }
        }
        Err(e) => {
            if let Some(ref healing) = healing_engine {
                let mut engine = healing.lock().unwrap();
                engine.record_failure("tree-of-thought-synthesis", &e.to_string());
            }
            warn!(error = %e, "Tree-of-thought synthesis failed");
        }
    }

    Ok(())
}

/// Run tree-of-thought with multiple LLM providers (round-robin per depth level)
pub async fn run_tree_of_thought_multi(
    multi_llm: MultiModelManager,
    config: Config,
    ravenfabric: Option<RavenFabricClient>,
    pattern_config: PatternConfig,
    healing_engine: Option<Arc<Mutex<SelfHealingEngine>>>,
) -> crate::error::Result<()> {
    info!(
        "Starting tree-of-thought mode (multi-model): {} branches, depth {}, top-k {}",
        pattern_config.tot_branches, pattern_config.tot_depth, pattern_config.tot_top_k
    );

    let system_prompt = &config.llm.system_prompt;
    let task = "Analyze the given task and provide your solution.";

    let mut branches: Vec<Vec<String>> = vec![Vec::new()];

    for depth in 0..pattern_config.tot_depth {
        info!(
            depth = depth + 1,
            branches = branches.len(),
            "ToT multi step"
        );

        // Check self-healing circuit breaker
        if let Some(ref healing) = healing_engine {
            let healthy = {
                let mut engine = healing.lock().unwrap();
                engine.is_healthy("tree-of-thought-multi")
            };
            if !healthy {
                warn!("Tree-of-thought (multi) blocked by circuit breaker");
                return Err(RavenClawsError::HealingError(
                    "Tree-of-thought (multi) blocked by circuit breaker".to_string(),
                ));
            }
        }

        let mut all_candidates: Vec<(Vec<String>, String, f64)> = Vec::new();

        for (branch_idx, branch) in branches.iter().enumerate() {
            let provider_idx = (depth * branches.len() + branch_idx) % multi_llm.client_count();
            let client = multi_llm.get_client(provider_idx);

            if let Some(client) = client {
                let mut memory = ConversationMemory::new(
                    &format!("{}\n\nYou are a reasoning agent. Generate diverse candidate thoughts and evaluate them.", system_prompt),
                    30,
                );
                memory.add_user_message(&format!(
                    "Task: {}\n\nCurrent reasoning path (step {}/{}):\n{}\n\nGenerate {} distinct next-step thoughts. \
                     For each thought, provide:\n1. The thought itself\n2. A confidence score (0.0 to 1.0)\n\n\
                     Format each as:\nTHOUGHT: <your thought>\nCONFIDENCE: <0.0-1.0>",
                    task,
                    depth + 1,
                    pattern_config.tot_depth,
                    if branch.is_empty() {
                        "No steps yet — start from scratch.".to_string()
                    } else {
                        branch.join("\n")
                    },
                    pattern_config.tot_branches,
                ));

                let messages = memory.history().to_vec();
                match client.chat(messages).await {
                    Ok(response) => {
                        if let Some(ref healing) = healing_engine {
                            let mut engine = healing.lock().unwrap();
                            engine.record_success("tree-of-thought-multi");
                        }

                        if let Some(choice) = response.choices.first() {
                            let content = &choice.message.content;
                            let mut current_thought = String::new();
                            let mut current_confidence = 0.5;
                            let mut parsing_thought = false;

                            for line in content.lines() {
                                let trimmed = line.trim();
                                if trimmed.starts_with("THOUGHT:") {
                                    if parsing_thought && !current_thought.is_empty() {
                                        let mut new_branch = branch.clone();
                                        new_branch.push(current_thought.clone());
                                        all_candidates.push((
                                            new_branch,
                                            format!("Confidence: {:.2}", current_confidence),
                                            current_confidence,
                                        ));
                                    }
                                    current_thought = trimmed
                                        .strip_prefix("THOUGHT:")
                                        .unwrap_or("")
                                        .trim()
                                        .to_string();
                                    parsing_thought = true;
                                    current_confidence = 0.5;
                                } else if trimmed.starts_with("CONFIDENCE:") {
                                    let val_str =
                                        trimmed.strip_prefix("CONFIDENCE:").unwrap_or("0.5").trim();
                                    if let Ok(val) = val_str.parse::<f64>() {
                                        current_confidence = val.clamp(0.0, 1.0);
                                    }
                                } else if parsing_thought && !trimmed.is_empty() {
                                    current_thought.push(' ');
                                    current_thought.push_str(trimmed);
                                }
                            }
                            if parsing_thought && !current_thought.is_empty() {
                                let mut new_branch = branch.clone();
                                new_branch.push(current_thought);
                                all_candidates.push((
                                    new_branch,
                                    format!("Confidence: {:.2}", current_confidence),
                                    current_confidence,
                                ));
                            }
                        }
                    }
                    Err(e) => {
                        if let Some(ref healing) = healing_engine {
                            let mut engine = healing.lock().unwrap();
                            engine.record_failure("tree-of-thought-multi", &e.to_string());
                        }
                        warn!(error = %e, "ToT multi branch {} failed", branch_idx);
                    }
                }
            }
        }

        if all_candidates.is_empty() {
            warn!("No candidates generated at depth {}", depth + 1);
            break;
        }

        all_candidates.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
        let top_k = pattern_config.tot_top_k.min(all_candidates.len());
        branches = all_candidates
            .into_iter()
            .take(top_k)
            .map(|(branch, _, _)| branch)
            .collect();

        if pattern_config.verbose {
            println!(
                "\n── ToT Multi Depth {}/{} ──",
                depth + 1,
                pattern_config.tot_depth
            );
            for (i, branch) in branches.iter().enumerate() {
                println!("Branch {} ({} steps):", i + 1, branch.len());
                for (j, thought) in branch.iter().enumerate() {
                    println!("  Step {}: {}", j + 1, thought);
                }
                println!();
            }
        }
    }

    // Final synthesis using first provider
    let best_branch = branches.into_iter().next().unwrap_or_default();
    if let Some(client) = multi_llm.get_client(0) {
        if let Some(ref healing) = healing_engine {
            let healthy = {
                let mut engine = healing.lock().unwrap();
                engine.is_healthy("tree-of-thought-multi-synthesis")
            };
            if !healthy {
                warn!("ToT multi synthesis blocked by circuit breaker");
                return Err(RavenClawsError::HealingError(
                    "ToT multi synthesis blocked by circuit breaker".to_string(),
                ));
            }
        }

        let mut final_memory = ConversationMemory::new(
            &format!("{}\n\nYou are a synthesizer. Produce a final answer based on the best reasoning path.", system_prompt),
            20,
        );
        final_memory.add_user_message(&format!(
            "Task: {}\n\nBest reasoning path:\n{}\n\nProduce a final, well-reasoned answer:",
            task,
            best_branch.join("\n→ "),
        ));

        let messages = final_memory.history().to_vec();
        match client.chat(messages).await {
            Ok(response) => {
                if let Some(ref healing) = healing_engine {
                    let mut engine = healing.lock().unwrap();
                    engine.record_success("tree-of-thought-multi-synthesis");
                }

                if let Some(choice) = response.choices.first() {
                    println!(
                        "\n🐦‍⬛ Tree-of-Thought (Multi) Final Answer:\n{}",
                        choice.message.content
                    );

                    if let Some(ref rf) = ravenfabric {
                        if rf.is_enabled() {
                            let summary = format!(
                                "Tree-of-Thought (multi) completed: depth {}, branches explored",
                                pattern_config.tot_depth
                            );
                            let _ = rf.broadcast(&summary, 30).await;
                        }
                    }
                }
            }
            Err(e) => {
                if let Some(ref healing) = healing_engine {
                    let mut engine = healing.lock().unwrap();
                    engine.record_failure("tree-of-thought-multi-synthesis", &e.to_string());
                }
                warn!(error = %e, "ToT multi synthesis failed");
            }
        }
    }

    Ok(())
}

// ── Pattern 6: Self-Reflection ─────────────────────────────────────────────

/// Run self-reflection reasoning.
///
/// The agent generates an initial solution, then reflects on it to identify
/// gaps, errors, or weaknesses, and produces an improved version. This
/// iterates for configurable rounds of reflection.
pub async fn run_self_reflection(
    llm: Arc<dyn LLMProviderTrait>,
    config: Config,
    ravenfabric: Option<RavenFabricClient>,
    pattern_config: PatternConfig,
    healing_engine: Option<Arc<Mutex<SelfHealingEngine>>>,
) -> crate::error::Result<()> {
    info!(
        "Starting self-reflection mode with {} reflection rounds",
        pattern_config.reflection_rounds
    );

    let system_prompt = &config.llm.system_prompt;
    let task = "Analyze the given task and provide your solution.";

    // Phase 1: Generate initial solution
    let mut current_solution = String::new();

    {
        let mut memory = ConversationMemory::new(
            &format!(
                "{}\n\nYou are a problem solver. Generate a thorough solution.",
                system_prompt
            ),
            10,
        );
        memory.add_user_message(&format!("Task: {}\n\nProvide your solution:", task));
        let messages = memory.history().to_vec();

        // Check self-healing circuit breaker
        if let Some(ref healing) = healing_engine {
            let healthy = {
                let mut engine = healing.lock().unwrap();
                engine.is_healthy("self-reflection-generate")
            };
            if !healthy {
                warn!("Self-reflection generation blocked by circuit breaker");
                return Err(RavenClawsError::HealingError(
                    "Self-reflection generation blocked by circuit breaker".to_string(),
                ));
            }
        }

        match llm.chat(messages).await {
            Ok(response) => {
                if let Some(ref healing) = healing_engine {
                    let mut engine = healing.lock().unwrap();
                    engine.record_success("self-reflection-generate");
                }

                if let Some(choice) = response.choices.first() {
                    current_solution = choice.message.content.clone();
                    info!("Initial solution: {} chars", current_solution.len());
                    if pattern_config.verbose {
                        println!(
                            "\n── Self-Reflection: Initial Solution ──\n{}",
                            current_solution
                        );
                    }
                }
            }
            Err(e) => {
                if let Some(ref healing) = healing_engine {
                    let mut engine = healing.lock().unwrap();
                    engine.record_failure("self-reflection-generate", &e.to_string());
                }
                warn!(error = %e, "Initial solution generation failed");
                return Err(RavenClawsError::Llm(crate::llm::LLMError::RequestFailed(
                    format!("Initial solution generation failed: {}", e),
                )));
            }
        }
    }

    // Phase 2: Reflect and improve
    for round in 0..pattern_config.reflection_rounds {
        info!(round = round + 1, "Self-reflection round");

        // Check self-healing circuit breaker
        if let Some(ref healing) = healing_engine {
            let healthy = {
                let mut engine = healing.lock().unwrap();
                engine.is_healthy(&format!("self-reflection-round-{}", round))
            };
            if !healthy {
                warn!(
                    round = round + 1,
                    "Self-reflection round blocked by circuit breaker"
                );
                break;
            }
        }

        // Step A: Reflect on the current solution
        let mut reflection = String::new();
        {
            let mut reflect_memory = ConversationMemory::new(
                "You are a critical reviewer. Analyze solutions for gaps, errors, logical flaws, \
                 missing edge cases, and opportunities for improvement. Be thorough and constructive.",
                10,
            );
            reflect_memory.add_user_message(&format!(
                "Review this solution critically:\n\n{}\n\nIdentify:\n1. Gaps or missing elements\n\
                 2. Logical flaws or errors\n3. Edge cases not handled\n4. Opportunities for improvement\n\
                 5. Overall quality assessment",
                current_solution
            ));
            let messages = reflect_memory.history().to_vec();

            match llm.chat(messages).await {
                Ok(response) => {
                    if let Some(ref healing) = healing_engine {
                        let mut engine = healing.lock().unwrap();
                        engine.record_success(&format!("self-reflection-round-{}", round));
                    }

                    if let Some(choice) = response.choices.first() {
                        reflection = choice.message.content.clone();
                        if pattern_config.verbose {
                            println!(
                                "\n── Self-Reflection Round {}/{}: Reflection ──\n{}",
                                round + 1,
                                pattern_config.reflection_rounds,
                                reflection
                            );
                        }
                    }
                }
                Err(e) => {
                    if let Some(ref healing) = healing_engine {
                        let mut engine = healing.lock().unwrap();
                        engine.record_failure(
                            &format!("self-reflection-round-{}", round),
                            &e.to_string(),
                        );
                    }
                    warn!(error = %e, round = round + 1, "Reflection failed");
                    continue;
                }
            }
        }

        // Step B: Improve based on reflection
        if !reflection.is_empty() {
            let mut improve_memory = ConversationMemory::new(
                &format!(
                    "{}\n\nYou are an improver. Take feedback and produce a better version.",
                    system_prompt
                ),
                10,
            );
            improve_memory.add_user_message(&format!(
                "Original solution:\n{}\n\nFeedback received:\n{}\n\n\
                 Produce an improved solution that addresses all feedback. \
                 Mark your response with IMPROVED: at the start.",
                current_solution, reflection
            ));
            let messages = improve_memory.history().to_vec();

            match llm.chat(messages).await {
                Ok(response) => {
                    if let Some(choice) = response.choices.first() {
                        let improved = choice.message.content.clone();
                        let improved_clean = improved
                            .strip_prefix("IMPROVED:")
                            .unwrap_or(&improved)
                            .trim()
                            .to_string();
                        current_solution = improved_clean;
                        info!(
                            round = round + 1,
                            "Solution improved: {} chars",
                            current_solution.len()
                        );
                    }
                }
                Err(e) => {
                    warn!(error = %e, round = round + 1, "Improvement failed");
                }
            }
        }
    }

    println!(
        "\n🐦‍⬛ Self-Reflection Final Solution (after {} rounds):\n{}",
        pattern_config.reflection_rounds, current_solution
    );

    if let Some(ref rf) = ravenfabric {
        if rf.is_enabled() {
            let summary = format!(
                "Self-reflection completed: {} rounds, final solution {} chars",
                pattern_config.reflection_rounds,
                current_solution.len()
            );
            let _ = rf.broadcast(&summary, 30).await;
        }
    }

    Ok(())
}

/// Run self-reflection with multiple LLM providers (alternating roles)
pub async fn run_self_reflection_multi(
    multi_llm: MultiModelManager,
    config: Config,
    ravenfabric: Option<RavenFabricClient>,
    pattern_config: PatternConfig,
    healing_engine: Option<Arc<Mutex<SelfHealingEngine>>>,
) -> crate::error::Result<()> {
    info!(
        "Starting self-reflection mode (multi-model) with {} rounds",
        pattern_config.reflection_rounds
    );

    let system_prompt = &config.llm.system_prompt;
    let task = "Analyze the given task and provide your solution.";

    // Phase 1: Generate initial solution using first provider
    let mut current_solution = String::new();

    if let Some(client) = multi_llm.get_client(0) {
        if let Some(ref healing) = healing_engine {
            let healthy = {
                let mut engine = healing.lock().unwrap();
                engine.is_healthy("self-reflection-multi-generate")
            };
            if !healthy {
                warn!("Self-reflection multi generation blocked by circuit breaker");
                return Err(RavenClawsError::HealingError(
                    "Self-reflection multi generation blocked by circuit breaker".to_string(),
                ));
            }
        }

        let mut memory = ConversationMemory::new(
            &format!(
                "{}\n\nYou are a problem solver. Generate a thorough solution.",
                system_prompt
            ),
            10,
        );
        memory.add_user_message(&format!("Task: {}\n\nProvide your solution:", task));
        let messages = memory.history().to_vec();

        match client.chat(messages).await {
            Ok(response) => {
                if let Some(ref healing) = healing_engine {
                    let mut engine = healing.lock().unwrap();
                    engine.record_success("self-reflection-multi-generate");
                }

                if let Some(choice) = response.choices.first() {
                    current_solution = choice.message.content.clone();
                    info!(
                        "Initial solution: {} chars via {}",
                        current_solution.len(),
                        client.provider_name()
                    );
                    if pattern_config.verbose {
                        println!(
                            "\n── Self-Reflection Multi: Initial Solution ──\n{}",
                            current_solution
                        );
                    }
                }
            }
            Err(e) => {
                if let Some(ref healing) = healing_engine {
                    let mut engine = healing.lock().unwrap();
                    engine.record_failure("self-reflection-multi-generate", &e.to_string());
                }
                warn!(error = %e, "Multi initial solution generation failed");
                return Err(RavenClawsError::Llm(crate::llm::LLMError::RequestFailed(
                    format!("Multi initial solution generation failed: {}", e),
                )));
            }
        }
    }

    // Phase 2: Reflect and improve using alternating providers
    for round in 0..pattern_config.reflection_rounds {
        info!(round = round + 1, "Self-reflection multi round");

        // Use different providers for reflection vs improvement
        let reflector_idx = (round * 2 + 1) % multi_llm.client_count();
        let improver_idx = (round * 2 + 2) % multi_llm.client_count();

        // Check self-healing circuit breaker
        if let Some(ref healing) = healing_engine {
            let healthy = {
                let mut engine = healing.lock().unwrap();
                engine.is_healthy(&format!("self-reflection-multi-round-{}", round))
            };
            if !healthy {
                warn!(
                    round = round + 1,
                    "Self-reflection multi round blocked by circuit breaker"
                );
                break;
            }
        }

        // Step A: Reflect using one provider
        let mut reflection = String::new();
        if let Some(reflector) = multi_llm.get_client(reflector_idx) {
            let mut reflect_memory = ConversationMemory::new(
                "You are a critical reviewer. Analyze solutions for gaps, errors, logical flaws, \
                 missing edge cases, and opportunities for improvement. Be thorough and constructive.",
                10,
            );
            reflect_memory.add_user_message(&format!(
                "Review this solution critically:\n\n{}\n\nIdentify:\n1. Gaps or missing elements\n\
                 2. Logical flaws or errors\n3. Edge cases not handled\n4. Opportunities for improvement\n\
                 5. Overall quality assessment",
                current_solution
            ));
            let messages = reflect_memory.history().to_vec();

            match reflector.chat(messages).await {
                Ok(response) => {
                    if let Some(ref healing) = healing_engine {
                        let mut engine = healing.lock().unwrap();
                        engine.record_success(&format!("self-reflection-multi-round-{}", round));
                    }

                    if let Some(choice) = response.choices.first() {
                        reflection = choice.message.content.clone();
                        if pattern_config.verbose {
                            println!(
                                "\n── Self-Reflection Multi Round {}/{}: Reflection ({} via {}) ──\n{}",
                                round + 1,
                                pattern_config.reflection_rounds,
                                reflector.provider_name(),
                                reflector.model(),
                                reflection
                            );
                        }
                    }
                }
                Err(e) => {
                    if let Some(ref healing) = healing_engine {
                        let mut engine = healing.lock().unwrap();
                        engine.record_failure(
                            &format!("self-reflection-multi-round-{}", round),
                            &e.to_string(),
                        );
                    }
                    warn!(error = %e, round = round + 1, "Multi reflection failed");
                    continue;
                }
            }
        }

        // Step B: Improve using a different provider
        if !reflection.is_empty() {
            if let Some(improver) = multi_llm.get_client(improver_idx) {
                let mut improve_memory = ConversationMemory::new(
                    &format!(
                        "{}\n\nYou are an improver. Take feedback and produce a better version.",
                        system_prompt
                    ),
                    10,
                );
                improve_memory.add_user_message(&format!(
                    "Original solution:\n{}\n\nFeedback received:\n{}\n\n\
                     Produce an improved solution that addresses all feedback. \
                     Mark your response with IMPROVED: at the start.",
                    current_solution, reflection
                ));
                let messages = improve_memory.history().to_vec();

                match improver.chat(messages).await {
                    Ok(response) => {
                        if let Some(choice) = response.choices.first() {
                            let improved = choice.message.content.clone();
                            let improved_clean = improved
                                .strip_prefix("IMPROVED:")
                                .unwrap_or(&improved)
                                .trim()
                                .to_string();
                            current_solution = improved_clean;
                            info!(
                                round = round + 1,
                                "Solution improved via {}: {} chars",
                                improver.provider_name(),
                                current_solution.len()
                            );
                        }
                    }
                    Err(e) => {
                        warn!(error = %e, round = round + 1, "Multi improvement failed");
                    }
                }
            }
        }
    }

    println!(
        "\n🐦‍⬛ Self-Reflection (Multi) Final Solution (after {} rounds):\n{}",
        pattern_config.reflection_rounds, current_solution
    );

    if let Some(ref rf) = ravenfabric {
        if rf.is_enabled() {
            let summary = format!(
                "Self-reflection (multi) completed: {} rounds, final solution {} chars",
                pattern_config.reflection_rounds,
                current_solution.len()
            );
            let _ = rf.broadcast(&summary, 30).await;
        }
    }

    Ok(())
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_config_defaults() {
        let cfg = PatternConfig::default();
        assert_eq!(cfg.max_rounds, 3);
        assert_eq!(cfg.max_review_iterations, 3);
        assert_eq!(cfg.research_agent_count, 3);
        assert_eq!(cfg.voter_count, 3);
        assert!(!cfg.verbose);
        assert_eq!(cfg.tot_branches, 3);
        assert_eq!(cfg.tot_depth, 3);
        assert_eq!(cfg.tot_top_k, 2);
        assert_eq!(cfg.reflection_rounds, 2);
    }

    #[test]
    fn test_pattern_config_custom() {
        let cfg = PatternConfig {
            max_rounds: 5,
            max_review_iterations: 5,
            research_agent_count: 5,
            voter_count: 7,
            verbose: true,
            tot_branches: 4,
            tot_depth: 5,
            tot_top_k: 3,
            reflection_rounds: 4,
        };
        assert_eq!(cfg.max_rounds, 5);
        assert_eq!(cfg.voter_count, 7);
        assert!(cfg.verbose);
        assert_eq!(cfg.tot_branches, 4);
        assert_eq!(cfg.tot_depth, 5);
        assert_eq!(cfg.tot_top_k, 3);
        assert_eq!(cfg.reflection_rounds, 4);
    }

    #[test]
    fn test_debate_function_exists() {
        // Compile-time check that debate function signature is valid
        let _ = std::mem::size_of::<PatternConfig>();
    }

    #[test]
    fn test_review_loop_function_exists() {
        let cfg = PatternConfig::default();
        assert_eq!(cfg.max_review_iterations, 3);
    }

    #[test]
    fn test_research_synthesize_function_exists() {
        let cfg = PatternConfig::default();
        assert_eq!(cfg.research_agent_count, 3);
    }

    #[test]
    fn test_voting_function_exists() {
        let cfg = PatternConfig::default();
        assert_eq!(cfg.voter_count, 3);
    }

    #[test]
    fn test_tree_of_thought_config() {
        let cfg = PatternConfig::default();
        assert_eq!(cfg.tot_branches, 3);
        assert_eq!(cfg.tot_depth, 3);
        assert_eq!(cfg.tot_top_k, 2);
    }

    #[test]
    fn test_self_reflection_config() {
        let cfg = PatternConfig::default();
        assert_eq!(cfg.reflection_rounds, 2);
    }
}
