use chrono::{Duration, NaiveDateTime};
use uuid::Uuid;

use crate::application::matches::MatchRepository;
use crate::application::pools::{PoolMemberRepository, PoolRepository, ScoringRuleRepository};
use crate::application::results::{
    EnterResult, HistoryEntry, HistorySummary, MemberPredictionView, PoolRecompute,
    PredictionLine, Ranking, ResultApplication, ScoredPredictionRecord, StandingRecord,
    StandingRepository,
};
use crate::application::shared::{Clock, Notifier};
use crate::domain::matches::{Match, MatchStatus};
use crate::domain::pools::{MemberStatus, Pool};
use crate::domain::predictions::{penalty_bonus, score, HitKind, ScoringRules};
use crate::domain::standings::ranking;
use crate::domain::{PenaltySide, Score};
use crate::errors::AppError;

pub struct ResultUseCases<S, MatchR, ScoringR, PoolR, MemberR, C, N> {
    standings: S,
    matches: MatchR,
    scoring_rules: ScoringR,
    pools: PoolR,
    members: MemberR,
    clock: C,
    notifier: N,
}

impl<S, MatchR, ScoringR, PoolR, MemberR, C, N>
    ResultUseCases<S, MatchR, ScoringR, PoolR, MemberR, C, N>
where
    S: StandingRepository,
    MatchR: MatchRepository,
    ScoringR: ScoringRuleRepository,
    PoolR: PoolRepository,
    MemberR: PoolMemberRepository,
    C: Clock,
    N: Notifier,
{
    pub fn new(
        standings: S,
        matches: MatchR,
        scoring_rules: ScoringR,
        pools: PoolR,
        members: MemberR,
        clock: C,
        notifier: N,
    ) -> Self {
        Self {
            standings,
            matches,
            scoring_rules,
            pools,
            members,
            clock,
            notifier,
        }
    }

    pub async fn enter_result(
        &self,
        match_id: &str,
        command: EnterResult,
    ) -> Result<Match, AppError> {
        let home_score = Score::new(command.home_score)?;
        let away_score = Score::new(command.away_score)?;

        let game = self
            .matches
            .find_by_id(match_id)
            .await?
            .ok_or_else(|| AppError::NotFound("match was not found".to_owned()))?;

        if game.status == MatchStatus::Canceled {
            return Err(AppError::conflict_code(
                "match_canceled",
                "a result cannot be entered for a canceled match",
            ));
        }

        let penalties_winner = resolve_penalties_winner(
            &game,
            home_score.value(),
            away_score.value(),
            command.penalties_winner.as_deref(),
        )?;

        let now = self.now()?;
        let kickoff = parse_datetime(game.kickoff_at.as_str()).ok_or_else(|| {
            AppError::Internal("match has an invalid kickoff date-time".to_owned())
        })?;
        if now < kickoff + Duration::minutes(110) {
            return Err(AppError::conflict_code(
                "result_too_early",
                "a result can only be entered after the match has been played",
            ));
        }

        let finished_at = self.clock.now().as_str().to_owned();
        let overlay = Overlay {
            match_id: match_id.to_owned(),
            home: home_score.value(),
            away: away_score.value(),
            penalties_winner,
        };

        let affected = self.standings.affected_pools_for_match(match_id).await?;
        let pool_ids = affected.clone();
        let mut pools = Vec::with_capacity(affected.len());
        for pool_id in affected {
            pools.push(self.compute_pool(&pool_id, Some(&overlay), &finished_at).await?);
        }

        self.standings
            .apply_result(ResultApplication {
                match_id: match_id.to_owned(),
                home_score: home_score.value(),
                away_score: away_score.value(),
                penalties_winner,
                finished_at,
                pools,
            })
            .await?;

        let game = self
            .matches
            .find_by_id(match_id)
            .await?
            .ok_or_else(|| AppError::Internal("match disappeared after result entry".to_owned()))?;

        self.notifier.match_result(&game, &pool_ids).await?;
        Ok(game)
    }

    pub async fn recompute_pool(&self, pool_id: &str) -> Result<(), AppError> {
        let finished_at = self.clock.now().as_str().to_owned();
        let recompute = self.compute_pool(pool_id, None, &finished_at).await?;
        self.standings.rescore_pool(recompute).await
    }

    pub async fn ranking(
        &self,
        pool_id: &str,
        viewer_user_id: &str,
    ) -> Result<Ranking, AppError> {
        let pool = self.load_pool(pool_id).await?;
        self.ensure_can_view(&pool, viewer_user_id).await?;
        let standings = self.standings.list_for_pool_with_names(pool_id).await?;
        Ok(Ranking { standings })
    }

    pub async fn history(
        &self,
        pool_id: &str,
        user_id: &str,
    ) -> Result<HistorySummary, AppError> {
        let member_id = self.resolve_active_member(pool_id, user_id).await?;
        let rules = self.load_rules(pool_id).await?;
        let mut lines: Vec<PredictionLine> = self
            .standings
            .prediction_lines_for_pool(pool_id)
            .await?
            .into_iter()
            .filter(|line| line.pool_member_id == member_id)
            .collect();
        lines.sort_by(|a, b| a.kickoff_at.cmp(&b.kickoff_at));

        let mut summary = HistorySummary {
            pool_member_id: member_id,
            total_points: 0,
            exact_count: 0,
            outcome_count: 0,
            hits_count: 0,
            penalties_count: 0,
            errors_count: 0,
            pending_count: 0,
            entries: Vec::with_capacity(lines.len()),
        };

        for line in lines {
            let prediction_penalties_pick = line.prediction_penalties_pick.map(side_to_string);
            let result_penalties_winner = line.result_penalties_winner.map(side_to_string);
            match finished_result(&line, &rules)? {
                Some((points, hit, penalty_hit)) => {
                    summary.total_points += i32::from(points);
                    match hit {
                        HitKind::Exact => {
                            summary.exact_count += 1;
                            summary.hits_count += 1;
                        }
                        HitKind::Outcome => {
                            summary.outcome_count += 1;
                            summary.hits_count += 1;
                        }
                        HitKind::None => summary.errors_count += 1,
                    }
                    if penalty_hit {
                        summary.penalties_count += 1;
                    }
                    summary.entries.push(HistoryEntry {
                        match_id: line.match_id,
                        match_status: line.match_status,
                        kickoff_at: line.kickoff_at,
                        prediction_home: line.prediction_home,
                        prediction_away: line.prediction_away,
                        result_home: line.result_home,
                        result_away: line.result_away,
                        prediction_penalties_pick,
                        result_penalties_winner,
                        points_awarded: Some(points),
                        hit_kind: Some(hit.as_str().to_owned()),
                    });
                }
                None => {
                    summary.pending_count += 1;
                    summary.entries.push(HistoryEntry {
                        match_id: line.match_id,
                        match_status: line.match_status,
                        kickoff_at: line.kickoff_at,
                        prediction_home: line.prediction_home,
                        prediction_away: line.prediction_away,
                        result_home: line.result_home,
                        result_away: line.result_away,
                        prediction_penalties_pick,
                        result_penalties_winner,
                        points_awarded: None,
                        hit_kind: None,
                    });
                }
            }
        }

        Ok(summary)
    }

    pub async fn member_predictions(
        &self,
        pool_id: &str,
        target_member_id: &str,
        viewer_user_id: &str,
    ) -> Result<Vec<MemberPredictionView>, AppError> {
        let pool = self.load_pool(pool_id).await?;
        self.ensure_can_view(&pool, viewer_user_id).await?;

        let target = self
            .members
            .find_by_id(target_member_id)
            .await?
            .filter(|member| member.pool_id.as_str() == pool_id)
            .ok_or_else(|| AppError::NotFound("pool member was not found".to_owned()))?;

        let now = self.now()?;
        let mut views: Vec<MemberPredictionView> = self
            .standings
            .prediction_lines_for_pool(pool_id)
            .await?
            .into_iter()
            .filter(|line| line.pool_member_id == target.id.as_str())
            .filter(|line| {
                is_revealed(
                    &line.match_status,
                    &line.kickoff_at,
                    pool.prediction_lock_offset_minutes,
                    now,
                )
            })
            .map(|line| MemberPredictionView {
                match_id: line.match_id,
                match_status: line.match_status,
                kickoff_at: line.kickoff_at,
                prediction_home: line.prediction_home,
                prediction_away: line.prediction_away,
                result_home: line.result_home,
                result_away: line.result_away,
                prediction_penalties_pick: line.prediction_penalties_pick.map(side_to_string),
                result_penalties_winner: line.result_penalties_winner.map(side_to_string),
                points_awarded: None,
            })
            .collect();
        views.sort_by(|a, b| a.kickoff_at.cmp(&b.kickoff_at));
        Ok(views)
    }

    async fn compute_pool(
        &self,
        pool_id: &str,
        overlay: Option<&Overlay>,
        scored_at: &str,
    ) -> Result<PoolRecompute, AppError> {
        let rules = self.load_rules(pool_id).await?;
        let lines = self.standings.prediction_lines_for_pool(pool_id).await?;
        let member_ids: Vec<String> = self
            .members
            .list_for_pool(pool_id)
            .await?
            .into_iter()
            .filter(|member| member.status == MemberStatus::Active)
            .map(|member| member.id.as_str().to_owned())
            .collect();

        let mut scored_predictions = Vec::new();
        let mut scored_lines = Vec::new();
        for line in &lines {
            let (result, result_penalties_winner) = match overlay {
                Some(overlay) if overlay.match_id == line.match_id => {
                    (Some((overlay.home, overlay.away)), overlay.penalties_winner)
                }
                _ => (stored_result(line), line.result_penalties_winner),
            };
            let Some((result_home, result_away)) = result else {
                continue;
            };

            let (base_points, hit) = score(
                (Score::new(line.prediction_home)?, Score::new(line.prediction_away)?),
                (Score::new(result_home)?, Score::new(result_away)?),
                &rules,
            );
            let (bonus, penalty_hit) = penalty_bonus(
                line.prediction_home == line.prediction_away,
                line.prediction_penalties_pick,
                result_penalties_winner,
                &rules,
            );
            let points = base_points + bonus;
            scored_predictions.push(ScoredPredictionRecord {
                prediction_id: line.prediction_id.clone(),
                points_awarded: points,
                scored_at: scored_at.to_owned(),
            });
            scored_lines.push((line.pool_member_id.clone(), points, hit, penalty_hit));
        }

        let standings = ranking::compute(member_ids, scored_lines)
            .into_iter()
            .map(|ranked| StandingRecord {
                standing_id: Uuid::new_v4().to_string(),
                pool_id: pool_id.to_owned(),
                pool_member_id: ranked.pool_member_id,
                total_points: ranked.total_points,
                exact_count: ranked.exact_count,
                outcome_count: ranked.outcome_count,
                hits_count: ranked.hits_count,
                penalties_count: ranked.penalties_count,
                position: ranked.position,
            })
            .collect();

        Ok(PoolRecompute {
            pool_id: pool_id.to_owned(),
            scored_predictions,
            standings,
        })
    }

    async fn load_pool(&self, pool_id: &str) -> Result<Pool, AppError> {
        self.pools
            .find_by_id(pool_id)
            .await?
            .ok_or_else(|| AppError::NotFound("pool was not found".to_owned()))
    }

    async fn load_rules(&self, pool_id: &str) -> Result<ScoringRules, AppError> {
        let rules = self.scoring_rules.list_for_pool(pool_id).await?;
        Ok(ScoringRules::from_rules(
            rules.into_iter().map(|rule| (rule.rule_key, rule.points)),
        ))
    }

    async fn resolve_active_member(
        &self,
        pool_id: &str,
        user_id: &str,
    ) -> Result<String, AppError> {
        let membership = self
            .members
            .find_membership(pool_id, user_id)
            .await?
            .filter(|member| member.status == MemberStatus::Active)
            .ok_or_else(|| {
                AppError::forbidden_code("not_pool_member", "you are not a member of this pool")
            })?;
        Ok(membership.id.as_str().to_owned())
    }

    async fn ensure_can_view(&self, pool: &Pool, viewer_user_id: &str) -> Result<(), AppError> {
        let is_member = self
            .members
            .find_membership(pool.id.as_str(), viewer_user_id)
            .await?
            .filter(|member| member.status == MemberStatus::Active)
            .is_some();
        if !is_member {
            return Err(AppError::forbidden_code(
                "not_pool_member",
                "this pool's ranking is private to its members",
            ));
        }
        Ok(())
    }

    fn now(&self) -> Result<NaiveDateTime, AppError> {
        parse_datetime(self.clock.now().as_str())
            .ok_or_else(|| AppError::Internal("clock produced an invalid date-time".to_owned()))
    }
}

struct Overlay {
    match_id: String,
    home: u8,
    away: u8,
    penalties_winner: Option<PenaltySide>,
}

fn side_to_string(side: PenaltySide) -> String {
    side.as_str().to_owned()
}

/// Validates and parses the penalty-shootout winner supplied with a result.
///
/// A winner is only accepted for a match that can go to penalties and that ended
/// in a draw; conversely, a drawn match that can go to penalties must have one.
fn resolve_penalties_winner(
    game: &Match,
    home_score: u8,
    away_score: u8,
    winner: Option<&str>,
) -> Result<Option<PenaltySide>, AppError> {
    let is_draw = home_score == away_score;

    if winner.is_some() && !game.can_go_to_penalties {
        return Err(AppError::conflict_code(
            "penalties_not_allowed",
            "this match cannot go to penalties",
        ));
    }
    if winner.is_some() && !is_draw {
        return Err(AppError::conflict_code(
            "penalties_winner_requires_draw",
            "a penalty-shootout winner is only valid for a drawn match",
        ));
    }

    match winner {
        Some(value) => Ok(Some(PenaltySide::parse(value)?)),
        None => {
            if game.can_go_to_penalties && is_draw {
                return Err(AppError::validation_code(
                    "penalties_winner_required",
                    "inform who won the penalty shootout for this drawn match",
                ));
            }
            Ok(None)
        }
    }
}

fn stored_result(line: &PredictionLine) -> Option<(u8, u8)> {
    if line.match_status != MatchStatus::Finished.as_str() {
        return None;
    }
    match (line.result_home, line.result_away) {
        (Some(home), Some(away)) => Some((home, away)),
        _ => None,
    }
}

fn finished_result(
    line: &PredictionLine,
    rules: &ScoringRules,
) -> Result<Option<(i16, HitKind, bool)>, AppError> {
    Ok(match stored_result(line) {
        Some((result_home, result_away)) => {
            let (base_points, hit) = score(
                (Score::new(line.prediction_home)?, Score::new(line.prediction_away)?),
                (Score::new(result_home)?, Score::new(result_away)?),
                rules,
            );
            let (bonus, penalty_hit) = penalty_bonus(
                line.prediction_home == line.prediction_away,
                line.prediction_penalties_pick,
                line.result_penalties_winner,
                rules,
            );
            Some((base_points + bonus, hit, penalty_hit))
        }
        None => None,
    })
}

fn is_revealed(status: &str, kickoff_at: &str, lock_offset_minutes: u16, now: NaiveDateTime) -> bool {
    if status != MatchStatus::Scheduled.as_str() {
        return true;
    }
    match parse_datetime(kickoff_at) {
        Some(kickoff) => {
            let lock_at = kickoff - Duration::minutes(i64::from(lock_offset_minutes));
            now >= lock_at
        }
        None => false,
    }
}

fn parse_datetime(value: &str) -> Option<NaiveDateTime> {
    let trimmed = value.trim();
    if let Ok(parsed) = chrono::DateTime::parse_from_rfc3339(trimmed) {
        return Some(parsed.naive_utc());
    }
    let formats = [
        "%Y-%m-%dT%H:%M:%S%.f",
        "%Y-%m-%d %H:%M:%S%.f",
        "%Y-%m-%dT%H:%M:%S",
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%dT%H:%M",
        "%Y-%m-%d %H:%M",
    ];
    for format in formats {
        if let Ok(parsed) = NaiveDateTime::parse_from_str(trimmed, format) {
            return Some(parsed);
        }
    }
    None
}
