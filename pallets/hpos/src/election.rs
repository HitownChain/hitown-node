use frame_support::sp_runtime::traits::Saturating;
use frame_support::sp_runtime::FixedPointNumber;
use frame_support::sp_runtime::FixedU128;
use frame_support::sp_runtime::traits::One;
use alloc::vec::Vec;

#[derive(Clone, Debug, PartialEq)]
pub struct CandidateData<AccountId> {
    pub account: AccountId,
    pub total_stake: u128,
    pub nominator_count: u32,
    pub activity: u32, // expected 0-100 (percentage)
}

// 粗略的自然对数近似实现 ln(1 + x)，仅用于链上选举权重计算
// 采用泰勒展开或分段线性拟合。为了节省链上计算资源，这里使用分段线性近似
fn approx_ln_one_plus(x: u32) -> FixedU128 {
    if x == 0 {
        return FixedU128::from_inner(0);
    }
    // ln(1+x) 的几个关键点近似值：
    // x=1 -> ln(2) ≈ 0.693
    // x=2 -> ln(3) ≈ 1.098
    // x=5 -> ln(6) ≈ 1.791
    // x=10 -> ln(11) ≈ 2.397
    // x=20 -> ln(21) ≈ 3.044
    // x=50 -> ln(51) ≈ 3.931
    // x=100 -> ln(101) ≈ 4.615
    if x <= 1 {
        FixedU128::saturating_from_rational(693, 1000)
    } else if x <= 5 {
        // y = 0.693 + (x-1)*(1.791-0.693)/4
        let base = FixedU128::saturating_from_rational(693, 1000);
        let slope = FixedU128::saturating_from_rational(1098, 4000);
        base.saturating_add(slope.saturating_mul(FixedU128::from((x - 1) as u128)))
    } else if x <= 10 {
        let base = FixedU128::saturating_from_rational(1791, 1000);
        let slope = FixedU128::saturating_from_rational(606, 5000);
        base.saturating_add(slope.saturating_mul(FixedU128::from((x - 5) as u128)))
    } else if x <= 50 {
        let base = FixedU128::saturating_from_rational(2397, 1000);
        let slope = FixedU128::saturating_from_rational(1534, 40000);
        base.saturating_add(slope.saturating_mul(FixedU128::from((x - 10) as u128)))
    } else {
        let base = FixedU128::saturating_from_rational(3931, 1000);
        let slope = FixedU128::saturating_from_rational(684, 50000);
        base.saturating_add(slope.saturating_mul(FixedU128::from((x - 50) as u128)))
    }
}

pub fn compute_election<AccountId: Clone>(
    candidates: Vec<CandidateData<AccountId>>,
    max_validators: usize,
) -> Vec<AccountId> {
    if candidates.is_empty() {
        return Vec::new();
    }

    let mut max_stake: u128 = 1;
    let mut max_nominator_count: u32 = 1;

    for c in &candidates {
        if c.total_stake > max_stake {
            max_stake = c.total_stake;
        }
        if c.nominator_count > max_nominator_count {
            max_nominator_count = c.nominator_count;
        }
    }

    let mut scored_candidates: Vec<(AccountId, FixedU128)> = candidates
        .into_iter()
        .map(|c| {
            // S_stake = total_stake_i / max(total_stake)
            let s_stake = FixedU128::saturating_from_rational(c.total_stake, max_stake);
            
            // S_nom = nominator_count_i / max(nominator_count)
            let s_nom = FixedU128::saturating_from_rational(c.nominator_count, max_nominator_count);
            
            // S_activity = activity_i (0~100) -> 0.0~1.0
            let activity = FixedU128::saturating_from_rational(c.activity, 100);

            // Step 2: Base score = 0.55 * S_stake + 0.30 * S_activity + 0.15 * S_nom
            let w_stake = FixedU128::saturating_from_rational(55, 100);
            let w_act = FixedU128::saturating_from_rational(30, 100);
            let w_nom = FixedU128::saturating_from_rational(15, 100);

            let score_stake = s_stake.saturating_mul(w_stake);
            let score_act = activity.saturating_mul(w_act);
            let score_nom = s_nom.saturating_mul(w_nom);

            let base_score = score_stake.saturating_add(score_act).saturating_add(score_nom);

            // Step 3: Decentralization bonus = 1 + 0.1 * min(1, nominator_count_i / 10)
            let ratio = if c.nominator_count > 10 { 10 } else { c.nominator_count };
            let bonus_ratio = FixedU128::saturating_from_rational(ratio, 10);
            let bonus_multiplier = FixedU128::saturating_from_rational(1, 10).saturating_mul(bonus_ratio);
            let decentralization_bonus = FixedU128::one().saturating_add(bonus_multiplier);

            // Step 4: Final score = score_i * (1 + ln(1 + nominator_count_i)) * decentralization_bonus_i
            let ln_factor = FixedU128::one().saturating_add(approx_ln_one_plus(c.nominator_count));
            let final_score = base_score.saturating_mul(ln_factor).saturating_mul(decentralization_bonus);

            (c.account, final_score)
        })
        .collect();

    // Sort descending by score
    scored_candidates.sort_by(|a, b| b.1.cmp(&a.1));

    // Step 5: Select top V validators
    scored_candidates
        .into_iter()
        .take(max_validators)
        .map(|(account, _score)| account)
        .collect()
}