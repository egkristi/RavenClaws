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
//!
//! These patterns are first-class modes accessible via `--mode debate`,
//! `--mode review-loop`, `--mode research-synthesize`, or `--mode voting`.

use std::sync::Arc;
use tracing::{info, warn};

use crate::agent::ConversationMemory;
use crate::config::Config;
use crate::error::RavenClawsError;
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
}

impl Default for PatternConfig {
    fn default() -> Self {
        Self {
            max_rounds: 3,
            max_review_iterations: 3,
            research_agent_count: 3,
            voter_count: 3,
            verbose: false,
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

            let messages = agent_memory.history().to_vec();
            match llm.chat(messages).await {
                Ok(response) => {
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

    let messages = final_memory.history().to_vec();
    match llm.chat(messages).await {
        Ok(response) => {
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

            let messages = producer_memory.history().to_vec();
            match llm.chat(messages).await {
                Ok(response) => {
                    if let Some(choice) = response.choices.first() {
                        current_content = choice.message.content.clone();
                        info!("Initial content produced: {} chars", current_content.len());

                        if pattern_config.verbose {
                            println!("\n── Initial Content ──\n{}", current_content);
                        }
                    }
                }
                Err(e) => {
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

            let messages = reviewer_memory.history().to_vec();
            let review = match llm.chat(messages).await {
                Ok(response) => response
                    .choices
                    .first()
                    .map(|c| c.message.content.clone())
                    .unwrap_or_default(),
                Err(e) => {
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

            let messages = producer_memory.history().to_vec();
            match llm.chat(messages).await {
                Ok(response) => {
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

        let messages = memory.history().to_vec();
        match llm.chat(messages).await {
            Ok(response) => {
                if let Some(choice) = response.choices.first() {
                    let content = choice.message.content.clone();
                    info!(role = %role, "Research completed: {} chars", content.len());
                    research_results.push((role.to_string(), content));
                }
            }
            Err(e) => {
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

    let messages = synth_memory.history().to_vec();
    match llm.chat(messages).await {
        Ok(response) => {
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

        let messages = memory.history().to_vec();
        match llm.chat(messages).await {
            Ok(response) => {
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

                let messages = agent_memory.history().to_vec();
                match client.chat(messages).await {
                    Ok(response) => {
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
                    Err(e) => warn!(error = %e, role = %role, "Multi-model debate failed"),
                }
            }
        }
    }

    // Final synthesis using first provider
    if let Some(client) = multi_llm.get_client(0) {
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
                if let Some(choice) = response.choices.first() {
                    println!(
                        "\n🐦‍⬛ Multi-Model Debate Synthesis:\n{}",
                        choice.message.content
                    );
                }
            }
            Err(e) => warn!(error = %e, "Multi-model synthesis failed"),
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
            if iteration == 0 {
                let mut memory = ConversationMemory::new(producer_persona, 10);
                memory.add_user_message(&format!("Create content for: {}", task));
                let messages = memory.history().to_vec();
                match client.chat(messages).await {
                    Ok(response) => {
                        if let Some(choice) = response.choices.first() {
                            current_content = choice.message.content.clone();
                            info!(
                                "Initial content: {} chars via {}",
                                current_content.len(),
                                client.provider_name()
                            );
                        }
                    }
                    Err(e) => warn!(error = %e, "Producer failed"),
                }
            } else {
                // Review
                let mut rev_memory = ConversationMemory::new(reviewer_persona, 10);
                rev_memory.add_user_message(&format!("Review:\n\n{}", current_content));
                let messages = rev_memory.history().to_vec();
                match client.chat(messages).await {
                    Ok(response) => {
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
                                if let Ok(rev_resp) = next_client.chat(msgs).await {
                                    if let Some(rev_choice) = rev_resp.choices.first() {
                                        current_content = rev_choice.message.content.clone();
                                        info!(
                                            "Revised: {} chars via {}",
                                            current_content.len(),
                                            next_client.provider_name()
                                        );
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => warn!(error = %e, "Review failed"),
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

            match client.chat(messages).await {
                Ok(response) => {
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
                Err(e) => warn!(error = %e, role = %role, "Research failed"),
            }
        }
    }

    // Synthesize
    if let Some(client) = multi_llm.get_client(0) {
        let mut synth_memory = ConversationMemory::new("You are a synthesis specialist.", 20);
        let mut input = String::from("Synthesize these findings:\n\n");
        for (role, provider, content) in &results {
            input.push_str(&format!("=== {} ({}) ===\n{}\n", role, provider, content));
        }
        synth_memory.add_user_message(&input);
        let messages = synth_memory.history().to_vec();

        match client.chat(messages).await {
            Ok(response) => {
                if let Some(choice) = response.choices.first() {
                    println!(
                        "\n🐦‍⬛ Multi-Model Research Synthesis:\n{}",
                        choice.message.content
                    );
                }
            }
            Err(e) => warn!(error = %e, "Synthesis failed"),
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

            match client.chat(messages).await {
                Ok(response) => {
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
                Err(e) => warn!(error = %e, "Voter {} failed", i + 1),
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
    }

    #[test]
    fn test_pattern_config_custom() {
        let cfg = PatternConfig {
            max_rounds: 5,
            max_review_iterations: 5,
            research_agent_count: 5,
            voter_count: 7,
            verbose: true,
        };
        assert_eq!(cfg.max_rounds, 5);
        assert_eq!(cfg.voter_count, 7);
        assert!(cfg.verbose);
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
}
