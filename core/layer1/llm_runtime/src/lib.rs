// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer1/llm_runtime (authoritative)

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelRuntimeKind {
    CloudApi,
    LocalApi,
    LocalPath,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelSpecialty {
    General,
    Coding,
    Reasoning,
    LongContext,
    FastResponse,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModelMetadata {
    pub id: String,
    pub provider: String,
    pub name: String,
    pub runtime_kind: ModelRuntimeKind,
    pub context_tokens: Option<u32>,
    pub parameter_billions: Option<f32>,
    pub pricing_input_per_1m_usd: Option<f32>,
    pub pricing_output_per_1m_usd: Option<f32>,
    pub hardware_vram_gb: Option<f32>,
    pub specialties: Vec<ModelSpecialty>,
    pub power_score_1_to_5: u8,
    pub cost_score_1_to_5: u8,
}

impl ModelMetadata {
    pub fn new(id: &str, provider: &str, name: &str, runtime_kind: ModelRuntimeKind) -> Self {
        Self {
            id: id.to_string(),
            provider: provider.to_string(),
            name: name.to_string(),
            runtime_kind,
            context_tokens: None,
            parameter_billions: None,
            pricing_input_per_1m_usd: None,
            pricing_output_per_1m_usd: None,
            hardware_vram_gb: None,
            specialties: vec![ModelSpecialty::General],
            power_score_1_to_5: 3,
            cost_score_1_to_5: 3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkloadClass {
    General,
    Coding,
    LongContext,
    FastTriage,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoutingRequest {
    pub workload: WorkloadClass,
    pub min_context_tokens: u32,
    pub max_cost_score_1_to_5: u8,
    pub local_only: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContextBudget {
    pub max_context_tokens: u32,
    pub reserve_tokens: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CompactionPlan {
    pub input_tokens: u32,
    pub available_tokens: u32,
    pub tokens_to_compact: u32,
    pub target_tokens_after_compaction: u32,
    pub compaction_ratio: f32,
}

fn score_position_1_to_5(position: usize, total: usize) -> u8 {
    if total <= 1 {
        return 3;
    }
    // Force bounded spread so a non-empty model set has an explicit floor/ceiling.
    let ratio = position as f32 / (total - 1) as f32;
    (1.0 + ratio * 4.0).round().clamp(1.0, 5.0) as u8
}

fn power_signal(model: &ModelMetadata) -> f32 {
    let params = model.parameter_billions.unwrap_or(1.0).max(0.1);
    let ctx = model.context_tokens.unwrap_or(4096) as f32;
    let specialty_bonus = if model
        .specialties
        .iter()
        .any(|s| matches!(s, ModelSpecialty::Coding | ModelSpecialty::Reasoning))
    {
        1.2
    } else {
        1.0
    };
    (params * 0.7 + (ctx / 8192.0) * 0.3) * specialty_bonus
}

fn cost_signal(model: &ModelMetadata) -> f32 {
    match model.runtime_kind {
        ModelRuntimeKind::CloudApi => {
            let input = model.pricing_input_per_1m_usd.unwrap_or(0.0).max(0.0);
            let output = model.pricing_output_per_1m_usd.unwrap_or(0.0).max(0.0);
            (input + output) / 2.0
        }
        // Local cost approximated by required VRAM footprint.
        ModelRuntimeKind::LocalApi | ModelRuntimeKind::LocalPath => {
            model.hardware_vram_gb.unwrap_or(2.0).max(0.5)
        }
    }
}

pub fn normalize_model_scores(models: &mut [ModelMetadata]) {
    if models.is_empty() {
        return;
    }

    let mut by_power: Vec<(usize, f32)> = models
        .iter()
        .enumerate()
        .map(|(idx, model)| (idx, power_signal(model)))
        .collect();
    by_power.sort_by(|a, b| a.1.total_cmp(&b.1));
    for (position, (idx, _)) in by_power.into_iter().enumerate() {
        models[idx].power_score_1_to_5 = score_position_1_to_5(position, models.len());
    }

    let mut by_cost: Vec<(usize, f32)> = models
        .iter()
        .enumerate()
        .map(|(idx, model)| (idx, cost_signal(model)))
        .collect();
    by_cost.sort_by(|a, b| a.1.total_cmp(&b.1));
    for (position, (idx, _)) in by_cost.into_iter().enumerate() {
        models[idx].cost_score_1_to_5 = score_position_1_to_5(position, models.len());
    }
}

fn workload_fit_bonus(model: &ModelMetadata, workload: WorkloadClass) -> f32 {
    match workload {
        WorkloadClass::General => 0.0,
        WorkloadClass::Coding => {
            if model.specialties.contains(&ModelSpecialty::Coding) {
                1.0
            } else {
                0.0
            }
        }
        WorkloadClass::LongContext => {
            if model.specialties.contains(&ModelSpecialty::LongContext) {
                1.0
            } else {
                0.0
            }
        }
        WorkloadClass::FastTriage => {
            if model.specialties.contains(&ModelSpecialty::FastResponse) {
                1.0
            } else {
                0.0
            }
        }
    }
}

fn prefer_candidate_on_tie(candidate: &ModelMetadata, incumbent: &ModelMetadata) -> bool {
    let candidate_local = !matches!(candidate.runtime_kind, ModelRuntimeKind::CloudApi);
    let incumbent_local = !matches!(incumbent.runtime_kind, ModelRuntimeKind::CloudApi);
    if candidate_local != incumbent_local {
        return candidate_local;
    }

    let candidate_ctx = candidate.context_tokens.unwrap_or(0);
    let incumbent_ctx = incumbent.context_tokens.unwrap_or(0);
    if candidate_ctx != incumbent_ctx {
        return candidate_ctx > incumbent_ctx;
    }

    if candidate.cost_score_1_to_5 != incumbent.cost_score_1_to_5 {
        return candidate.cost_score_1_to_5 < incumbent.cost_score_1_to_5;
    }

    if candidate.provider != incumbent.provider {
        return candidate.provider < incumbent.provider;
    }
    candidate.id < incumbent.id
}

pub fn choose_best_model(
    models: &[ModelMetadata],
    request: &RoutingRequest,
) -> Option<ModelMetadata> {
    let mut best: Option<(f32, usize)> = None;
    for (idx, model) in models.iter().enumerate() {
        if request.local_only && matches!(model.runtime_kind, ModelRuntimeKind::CloudApi) {
            continue;
        }
        if model.context_tokens.unwrap_or(0) < request.min_context_tokens {
            continue;
        }
        if model.cost_score_1_to_5 > request.max_cost_score_1_to_5 {
            continue;
        }
        // Favor stronger models but penalize higher cost.
        let base = model.power_score_1_to_5 as f32 - (model.cost_score_1_to_5 as f32 * 0.35);
        let score = base + workload_fit_bonus(model, request.workload);
        match best {
            Some((best_score, best_idx)) => {
                if score > best_score {
                    best = Some((score, idx));
                } else if (score - best_score).abs() <= f32::EPSILON
                    && prefer_candidate_on_tie(model, &models[best_idx])
                {
                    best = Some((score, idx));
                }
            }
            None => best = Some((score, idx)),
        }
    }
    best.map(|(_, idx)| models[idx].clone())
}

pub fn plan_context_compaction(input_tokens: u32, budget: ContextBudget) -> CompactionPlan {
    let available_tokens = budget
        .max_context_tokens
        .saturating_sub(budget.reserve_tokens);
    let tokens_to_compact = input_tokens.saturating_sub(available_tokens);
    let target_tokens_after_compaction = if tokens_to_compact > 0 {
        available_tokens
    } else {
        input_tokens
    };
    let compaction_ratio = if input_tokens == 0 {
        0.0
    } else {
        tokens_to_compact as f32 / input_tokens as f32
    };
    CompactionPlan {
        input_tokens,
        available_tokens,
        tokens_to_compact,
        target_tokens_after_compaction,
        compaction_ratio,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scores_span_floor_and_ceiling_when_multiple_models_exist() {
        let mut models = vec![
            ModelMetadata::new("a", "p1", "tiny", ModelRuntimeKind::LocalApi),
            ModelMetadata::new("b", "p1", "mid", ModelRuntimeKind::CloudApi),
            ModelMetadata::new("c", "p1", "large", ModelRuntimeKind::CloudApi),
        ];
        models[0].parameter_billions = Some(1.0);
        models[1].parameter_billions = Some(8.0);
        models[2].parameter_billions = Some(70.0);
        models[0].context_tokens = Some(4096);
        models[1].context_tokens = Some(8192);
        models[2].context_tokens = Some(128000);
        models[0].hardware_vram_gb = Some(2.0);
        models[1].pricing_input_per_1m_usd = Some(1.0);
        models[1].pricing_output_per_1m_usd = Some(3.0);
        models[2].pricing_input_per_1m_usd = Some(15.0);
        models[2].pricing_output_per_1m_usd = Some(60.0);

        normalize_model_scores(&mut models);

        let min_power = models.iter().map(|m| m.power_score_1_to_5).min();
        let max_power = models.iter().map(|m| m.power_score_1_to_5).max();
        let min_cost = models.iter().map(|m| m.cost_score_1_to_5).min();
        let max_cost = models.iter().map(|m| m.cost_score_1_to_5).max();
        assert_eq!(min_power, Some(1));
        assert_eq!(max_power, Some(5));
        assert_eq!(min_cost, Some(1));
        assert_eq!(max_cost, Some(5));
    }

    #[test]
    fn routing_prefers_local_coding_model_when_local_only() {
        let mut cloud = ModelMetadata::new("c", "cloud", "code-pro", ModelRuntimeKind::CloudApi);
        cloud.context_tokens = Some(32000);
        cloud.specialties = vec![ModelSpecialty::Coding];
        cloud.power_score_1_to_5 = 5;
        cloud.cost_score_1_to_5 = 5;

        let mut local = ModelMetadata::new("l", "ollama", "qwen-coder", ModelRuntimeKind::LocalApi);
        local.context_tokens = Some(32000);
        local.specialties = vec![ModelSpecialty::Coding];
        local.power_score_1_to_5 = 4;
        local.cost_score_1_to_5 = 2;

        let request = RoutingRequest {
            workload: WorkloadClass::Coding,
            min_context_tokens: 8000,
            max_cost_score_1_to_5: 5,
            local_only: true,
        };
        let selected = choose_best_model(&[cloud, local.clone()], &request).expect("model");
        assert_eq!(selected.id, local.id);
    }

    #[test]
    fn context_compaction_plan_flags_overflow() {
        let plan = plan_context_compaction(
            12000,
            ContextBudget {
                max_context_tokens: 8192,
                reserve_tokens: 1024,
            },
        );
        assert_eq!(plan.available_tokens, 7168);
        assert_eq!(plan.tokens_to_compact, 4832);
        assert!(plan.compaction_ratio > 0.3);
    }

    #[test]
    fn routing_tie_break_prefers_local_and_stable_id_ordering() {
        let mut cloud = ModelMetadata::new("z-cloud", "cloud", "cloud-fast", ModelRuntimeKind::CloudApi);
        cloud.context_tokens = Some(32000);
        cloud.specialties = vec![ModelSpecialty::Coding];
        cloud.power_score_1_to_5 = 4;
        cloud.cost_score_1_to_5 = 2;

        let mut local_b =
            ModelMetadata::new("b-local", "local", "local-b", ModelRuntimeKind::LocalApi);
        local_b.context_tokens = Some(32000);
        local_b.specialties = vec![ModelSpecialty::Coding];
        local_b.power_score_1_to_5 = 4;
        local_b.cost_score_1_to_5 = 2;

        let mut local_a =
            ModelMetadata::new("a-local", "local", "local-a", ModelRuntimeKind::LocalApi);
        local_a.context_tokens = Some(32000);
        local_a.specialties = vec![ModelSpecialty::Coding];
        local_a.power_score_1_to_5 = 4;
        local_a.cost_score_1_to_5 = 2;

        let request = RoutingRequest {
            workload: WorkloadClass::Coding,
            min_context_tokens: 4096,
            max_cost_score_1_to_5: 5,
            local_only: false,
        };
        let selected =
            choose_best_model(&[cloud, local_b, local_a.clone()], &request).expect("model");
        assert_eq!(selected.id, local_a.id);
    }
}
