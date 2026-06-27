//! The GraphQL query root (`API.md` §4).
//!
//! Each query loads the coarse storage items it needs once, then assembles
//! the response from memory. Resolvers call the pure `domain`/`fwc26`
//! functions; they contain no domain logic.

use crate::auth::CurrentPlayer;
use crate::gql::types::*;
use async_graphql::{Context, Object};
use domain::scoring::{score_match_parts, standings_score, ScoringConfig};
use std::collections::HashMap;
use std::sync::Arc;
use storage::Repository;

pub struct QueryRoot;

/// Only consult the live source while a match could plausibly be in progress
/// (covers a knockout's extra time), so SportsDB is never queried for
/// long-finished or far-future games. Also caps live calls to genuinely-live
/// matches.
const LIVE_WINDOW: chrono::Duration = chrono::Duration::hours(3);

/// Pull the `Repository` out of the GraphQL context.
fn repo<'a>(ctx: &'a Context<'_>) -> &'a dyn Repository {
    ctx.data_unchecked::<std::sync::Arc<dyn Repository>>()
        .as_ref()
}

/// The request's `now` (the clock seam — `.specs/TESTING.md` §3.2).
fn now(ctx: &Context<'_>) -> chrono::DateTime<chrono::Utc> {
    ctx.data_unchecked::<crate::clock::RequestNow>().0
}

/// Collect the leaf groups (those that directly hold games) in the subtree
/// rooted at `node_id`, in tree order. A leaf group passed directly returns
/// itself; a round node returns its one-match leaf groups.
fn collect_leaf_groups<'a>(
    t: &'a domain::Tournament,
    node_id: &str,
    out: &mut Vec<&'a domain::GroupGame>,
) {
    let Some(g) = t.groups.get(node_id) else {
        return;
    };
    match &g.children {
        domain::GroupChildren::Games(_) => out.push(g),
        domain::GroupChildren::Groups(ids) => {
            for id in ids {
                collect_leaf_groups(t, id, out);
            }
        }
    }
}

/// Build one `(player, game)` tip row: apply the mutual-commitment visibility
/// gate (legacy `AllTipsHandler`) and, when the prediction is visible and an
/// `actual` exists, score it. `actual` is the result-user's prediction for the
/// official path, or a synthesized live score for the provisional path — the
/// scoring is identical. Shared by `tips` and `match`.
#[allow(clippy::too_many_arguments)]
fn scored_tip(
    viewer_id: &str,
    viewer_prediction: Option<&domain::MatchPrediction>,
    player: &domain::Player,
    game: &domain::SingleGame,
    deadline: Option<chrono::DateTime<chrono::Utc>>,
    now: chrono::DateTime<chrono::Utc>,
    actual: Option<&domain::MatchPrediction>,
    result_is_live: bool,
    multiplier: i64,
    config: &ScoringConfig,
) -> Tip {
    let prediction = player.match_prediction(&game.id);
    let is_own = player.id == viewer_id;
    // Match kickoff or group-deadline opens every tip for the game to everyone.
    let time_open = now >= game.kickoff || deadline.is_some_and(|d| now > d);
    // Mutual commitment: another player's tip shows only once the viewer has
    // effective-locked this match; we keep the target's lock so an un-locked
    // draft is never exposed before the deadline.
    let viewer_committed = time_open || viewer_prediction.is_some_and(|p| p.locked);
    let visible = is_own || (viewer_committed && prediction.is_some_and(|p| p.locked || time_open));
    let breakdown = match (visible, prediction, actual) {
        (true, Some(pred), Some(res)) => Some(PointsBreakdown::build(
            score_match_parts(pred, res, config),
            multiplier,
            config,
        )),
        _ => None,
    };
    let points = breakdown.as_ref().map(|b| b.points);
    let is_perfect_tip = breakdown
        .as_ref()
        .is_some_and(|b| b.base >= config.perfect_threshold);
    // Best points still reachable IN THIS match while it is live (provisional).
    // `actual` carries the live provisional score during a live match; null
    // otherwise (pre-kickoff or once an official result is entered).
    let max_reachable = match (result_is_live, visible, prediction, actual) {
        (true, true, Some(pred), Some(res)) => Some(domain::scoring::max_reachable_score(
            pred, res, config, multiplier,
        )),
        _ => None,
    };
    Tip {
        player_id: player.id.clone(),
        nick: player.nick.clone(),
        game_id: game.id.clone(),
        prediction: if visible {
            prediction.map(MatchPrediction::from)
        } else {
            None
        },
        points,
        is_perfect: is_perfect_tip,
        breakdown,
        max_reachable,
    }
}

#[Object]
impl QueryRoot {
    /// The `<t>#TOURNAMENT` structure with time-derived flags (`TESTING.md` §3.3).
    async fn tournament(&self, ctx: &Context<'_>) -> async_graphql::Result<Option<Tournament>> {
        let repo = repo(ctx);
        let Some(t) = repo.get_tournament().await? else {
            return Ok(None);
        };
        // Official results = the games the result user has entered a result for.
        // Entered ⇒ effective-locked (results are entered post-kickoff), so this
        // drives the `resultPending` flag the same way the scoreboard scores them.
        let players = repo.list_players().await?;
        let entered_results: std::collections::HashSet<String> = players
            .iter()
            .find(|p| p.is_result_user)
            .map(|r| {
                r.match_predictions
                    .iter()
                    .map(|p| p.game_id.clone())
                    .collect()
            })
            .unwrap_or_default();
        Ok(Some(Tournament::build(&t, now(ctx), &entered_results)))
    }

    /// The request's resolved `now` — the server clock the SPA renders against.
    async fn now(&self, ctx: &Context<'_>) -> chrono::DateTime<chrono::Utc> {
        now(ctx)
    }

    /// The materialised scoreboard, optionally filtered to a pool's members.
    async fn scoreboard(
        &self,
        ctx: &Context<'_>,
        pool: Option<String>,
    ) -> async_graphql::Result<Vec<ScoreEntry>> {
        let repo = repo(ctx);
        let board = repo.get_scoreboard().await?.unwrap_or_default();
        let players = repo.list_players().await?;
        let nick_by_id: HashMap<&str, &str> = players
            .iter()
            .map(|p| (p.id.as_str(), p.nick.as_str()))
            .collect();

        // Restrict to a pool's members if requested. Issue 04 — pool
        // membership is private: a pool filter requires authentication and the
        // caller must be a member (or owner) of that pool.
        let allowed: Option<Vec<String>> = match pool {
            Some(pool_id) => {
                let viewer = CurrentPlayer::require(ctx)?;
                let pools = repo.list_pools().await?;
                let p = pools
                    .into_iter()
                    .find(|p| p.id == pool_id)
                    .ok_or_else(|| async_graphql::Error::new("pool not found"))?;
                if !p.members.contains(&viewer.id) && p.owner != viewer.id {
                    return Err(async_graphql::Error::new(
                        "you are not a member of this pool",
                    ));
                }
                Some(p.members)
            }
            None => None,
        };

        // Drop non-participants' all-zero rows. The materialised board scores
        // every player (recompute.rs), but only participants belong in the
        // listing — the same category of rule as excluding the result-user,
        // computed by the pure domain selector.
        let participant_ids: std::collections::HashSet<&str> =
            domain::participation::participants(&players)
                .iter()
                .map(|p| p.id.as_str())
                .collect();

        let mut entries: Vec<ScoreEntry> = board
            .entries
            .iter()
            .filter(|(pid, _)| allowed.as_ref().is_none_or(|m| m.contains(pid)))
            .filter(|(pid, _)| participant_ids.contains(pid.as_str()))
            .map(|(pid, breakdown)| {
                let stages: Vec<StageScore> = breakdown
                    .iter()
                    .map(|(round, points)| StageScore {
                        round: (*round).into(),
                        points: *points,
                    })
                    .collect();
                let total: i64 = breakdown.values().sum();
                ScoreEntry {
                    player_id: pid.clone(),
                    nick: nick_by_id
                        .get(pid.as_str())
                        .copied()
                        .unwrap_or("")
                        .to_owned(),
                    total,
                    stages,
                }
            })
            .collect();
        entries.sort_by(|a, b| b.total.cmp(&a.total).then(a.player_id.cmp(&b.player_id)));
        Ok(entries)
    }

    /// The current viewer — either a resolved `Player` or an `UnclaimedViewer`
    /// (authenticated but not yet linked to a Person/Player). Returns `null`
    /// for unauthenticated visitors. When `UnclaimedViewer.linkCandidate` is
    /// set, the UI should prompt AUTH-13 cross-provider linking via
    /// `confirmLink` rather than a normal `claimInvite`.
    async fn me(&self, ctx: &Context<'_>) -> async_graphql::Result<Option<Viewer>> {
        match ctx.data_unchecked::<CurrentPlayer>() {
            CurrentPlayer::Visitor => Ok(None),
            CurrentPlayer::Player(p) => {
                let repo = ctx.data_unchecked::<Arc<dyn Repository>>();
                let ruid = crate::gql::result_user_id(repo.as_ref()).await?;
                let player = Player {
                    may_create_pool: domain::pool::may_create_pool(p.as_ref(), &ruid),
                    ..Player::from(p.as_ref())
                };
                Ok(Some(Viewer::Player(Box::new(player))))
            }
            CurrentPlayer::AuthenticatedUnclaimed(u) => {
                let repo = ctx.data_unchecked::<Arc<dyn Repository>>();
                let candidate = if let Some(email) = &u.verified_email {
                    repo.find_identities_by_verified_email(email)
                        .await
                        .ok()
                        .and_then(|mut hits| hits.pop())
                        .map(|i| LinkCandidate {
                            person_id: i.person_id,
                            provider: i.provider,
                        })
                } else {
                    None
                };
                Ok(Some(Viewer::Unclaimed(UnclaimedViewer {
                    email: u.verified_email.clone(),
                    phone: u.verified_phone.clone(),
                    link_candidate: candidate,
                })))
            }
        }
    }

    /// The pools the current player belongs to. Requires authentication.
    async fn pools(&self, ctx: &Context<'_>) -> async_graphql::Result<Vec<Pool>> {
        let player = CurrentPlayer::require(ctx)?;
        let pools = repo(ctx).list_pools().await?;
        Ok(pools
            .iter()
            .filter(|p| p.members.contains(&player.id) || p.owner == player.id)
            .map(Pool::from)
            .collect())
    }

    /// Every player's *visible* predictions for a group's matches (`API.md`
    /// §6, UC-9). Mutual commitment: another player's tip is revealed to the
    /// viewer only once *both* have effective-locked that match (the viewer
    /// can't peek at a game they can still change, and an un-locked draft is
    /// never exposed); the deadline/kickoff opens every tip for that match.
    async fn tips(
        &self,
        ctx: &Context<'_>,
        group_id: String,
        pool: Option<String>,
    ) -> async_graphql::Result<Vec<Tip>> {
        let viewer = CurrentPlayer::require(ctx)?;
        let repo = repo(ctx);
        let tournament = repo
            .get_tournament()
            .await?
            .ok_or_else(|| async_graphql::Error::new("no tournament loaded"))?;
        let players = repo.list_players().await?;

        // Optional pool scoping — mirrors `scoreboard`. A pool filter is private:
        // it requires the viewer to be a member (or owner) of that pool, and
        // restricts the grid to that pool's members. `None` = the global grid.
        let allowed: Option<Vec<String>> = match &pool {
            Some(pool_id) => {
                let pools = repo.list_pools().await?;
                let p = pools
                    .into_iter()
                    .find(|p| &p.id == pool_id)
                    .ok_or_else(|| async_graphql::Error::new("pool not found"))?;
                if !p.members.contains(&viewer.id) && p.owner != viewer.id {
                    return Err(async_graphql::Error::new(
                        "you are not a member of this pool",
                    ));
                }
                Some(p.members)
            }
            None => None,
        };

        let games = tournament.games_in(&group_id);
        let game_ids: Vec<domain::GameId> = games.iter().map(|g| g.id.clone()).collect();
        let deadline = tournament.deadline(&group_id);
        let now = now(ctx);

        // Official results = the result user's predictions. Used to score each
        // visible tip via the pure domain scoring functions (no domain logic
        // here — just the lookup + call). Absent until the result is entered.
        let config = ScoringConfig::default();
        let result_user = players.iter().find(|p| p.is_result_user);
        let round_of = |game: &domain::SingleGame| {
            tournament
                .groups
                .get(&game.group_id)
                .map(|g| g.round)
                .unwrap_or(domain::Round::GroupStage)
        };

        let mut tips = Vec::new();
        for player in domain::participation::tippers_in(&players, &game_ids)
            .into_iter()
            .filter(|p| allowed.as_ref().is_none_or(|m| m.contains(&p.id)))
        {
            for game in &games {
                let result = result_user.and_then(|r| r.match_prediction(&game.id));
                tips.push(scored_tip(
                    &viewer.id,
                    viewer.match_prediction(&game.id),
                    player,
                    game,
                    deadline,
                    now,
                    result,
                    false, // the all-tips grid never shows the live-match ceiling
                    config.multiplier(round_of(game)),
                    &config,
                ));
            }
        }
        Ok(tips)
    }

    /// Every player's *scoreable* standings (group-table) bonus for the leaf
    /// groups under `group_id` — the per-(player, group) sibling of the `tips`
    /// grid. Mirrors the scoreboard's standings computation (`standings_score`),
    /// so it only appears once both the official table and the player's
    /// standings prediction are effective-locked. A group-stage leaf id returns
    /// that one group; a knockout round-node id returns its one-match groups.
    async fn standings(
        &self,
        ctx: &Context<'_>,
        group_id: String,
    ) -> async_graphql::Result<Vec<StandingsScore>> {
        CurrentPlayer::require(ctx)?;
        let repo = repo(ctx);
        let tournament = repo
            .get_tournament()
            .await?
            .ok_or_else(|| async_graphql::Error::new("no tournament loaded"))?;
        let players = repo.list_players().await?;
        let now = now(ctx);
        let config = ScoringConfig::default();

        let Some(result_user) = players.iter().find(|p| p.is_result_user) else {
            return Ok(Vec::new());
        };

        let mut leaves = Vec::new();
        collect_leaf_groups(&tournament, &group_id, &mut leaves);
        let leaf_group_ids: Vec<domain::GroupId> = leaves.iter().map(|g| g.id.clone()).collect();
        let roster = domain::participation::standings_tippers(&players, &leaf_group_ids);

        let mut out = Vec::new();
        for group in leaves {
            if !group.carries_standings {
                continue;
            }
            let games = tournament.games_in(&group.id);
            let deadline = tournament
                .deadline(&group.id)
                .unwrap_or(chrono::DateTime::<chrono::Utc>::MAX_UTC);
            let multiplier = config.multiplier(group.round);
            for player in roster.iter().copied() {
                if let Some(sb) =
                    standings_score(group, &games, player, result_user, now, deadline, &config)
                {
                    out.push(StandingsScore {
                        player_id: player.id.clone(),
                        nick: player.nick.clone(),
                        group_id: group.id.clone(),
                        pairs_correct: sb.pairs_correct as i64,
                        pairs_total: sb.pairs_total as i64,
                        bonus: sb.bonus,
                        multiplier,
                        points: sb.bonus * multiplier,
                    });
                }
            }
        }
        Ok(out)
    }

    /// Every "perfect" (maximum-scoring) prediction across all players (UC-10),
    /// optionally scoped to a pool's members. A pool filter is private — it
    /// requires the viewer to be a member (or owner) of that pool, mirroring
    /// `scoreboard` / `tips` (Issue 04). `None` = the global listing (public).
    async fn perfects(
        &self,
        ctx: &Context<'_>,
        pool: Option<String>,
    ) -> async_graphql::Result<Vec<Perfect>> {
        let repo = repo(ctx);
        let players = repo.list_players().await?;
        let config = ScoringConfig::default();

        // Optional pool scoping — same private-membership rule as `scoreboard`.
        let allowed: Option<Vec<String>> = match &pool {
            Some(pool_id) => {
                let viewer = CurrentPlayer::require(ctx)?;
                let pools = repo.list_pools().await?;
                let p = pools
                    .into_iter()
                    .find(|p| &p.id == pool_id)
                    .ok_or_else(|| async_graphql::Error::new("pool not found"))?;
                if !p.members.contains(&viewer.id) && p.owner != viewer.id {
                    return Err(async_graphql::Error::new(
                        "you are not a member of this pool",
                    ));
                }
                Some(p.members)
            }
            None => None,
        };

        // The tournament gives each game's round, for the points multiplier.
        let tournament = repo.get_tournament().await?;
        let round_of = |game_id: &str| {
            tournament
                .as_ref()
                .and_then(|t| t.games.get(game_id).and_then(|g| t.groups.get(&g.group_id)))
                .map(|grp| grp.round)
                .unwrap_or(domain::Round::GroupStage)
        };

        let result_user = match players.iter().find(|p| p.is_result_user) {
            Some(r) => r,
            None => return Ok(Vec::new()),
        };

        let mut perfects = Vec::new();
        for player in &players {
            if player.is_result_user {
                continue;
            }
            if allowed.as_ref().is_some_and(|m| !m.contains(&player.id)) {
                continue;
            }
            for prediction in &player.match_predictions {
                if let Some(result) = result_user.match_prediction(&prediction.game_id) {
                    let breakdown = PointsBreakdown::build(
                        score_match_parts(prediction, result, &config),
                        config.multiplier(round_of(&prediction.game_id)),
                        &config,
                    );
                    if breakdown.base >= config.perfect_threshold {
                        perfects.push(Perfect {
                            player_id: player.id.clone(),
                            nick: player.nick.clone(),
                            game_id: prediction.game_id.clone(),
                            points: breakdown.points,
                            breakdown,
                        });
                    }
                }
            }
        }
        Ok(perfects)
    }

    /// The result user's *entered* match predictions — the official scores.
    /// Public so any client can overlay official results onto the schedule.
    async fn results(&self, ctx: &Context<'_>) -> async_graphql::Result<Vec<MatchPrediction>> {
        let players = repo(ctx).list_players().await?;
        Ok(players
            .iter()
            .find(|p| p.is_result_user)
            .map(|r| {
                r.match_predictions
                    .iter()
                    .map(MatchPrediction::from)
                    .collect()
            })
            .unwrap_or_default())
    }

    /// Every player (id, nick) — powers the dev-login picker and the admin
    /// player list (UC-16). Public: the dev auth stub needs it pre-login.
    async fn players(&self, ctx: &Context<'_>) -> async_graphql::Result<Vec<PlayerSummary>> {
        let players = repo(ctx).list_players().await?;
        let mut out: Vec<PlayerSummary> = players.iter().map(PlayerSummary::from).collect();
        out.sort_by(|a, b| {
            // Real players first, then the result user; alphabetical within.
            a.is_result_user
                .cmp(&b.is_result_user)
                .then(a.nick.cmp(&b.nick))
        });
        Ok(out)
    }

    /// External (TheSportsDB) reported results for a group's result-pending
    /// games — the admin pre-fill source. Admin-only (the result user). Returns
    /// only finished, mapped, not-yet-entered games; `[]` if the source is
    /// absent or errors (manual entry is never blocked).
    async fn reported_results(
        &self,
        ctx: &Context<'_>,
        group_id: String,
    ) -> async_graphql::Result<Vec<ReportedResult>> {
        // Gate: only the result user (the official-results admin).
        CurrentPlayer::require_admin(ctx)?;

        let repo = repo(ctx);
        let now = now(ctx);
        let Some(tournament) = repo.get_tournament().await? else {
            return Ok(Vec::new());
        };
        let players = repo.list_players().await?;
        let entered: std::collections::HashSet<String> = players
            .iter()
            .find(|p| p.is_result_user)
            .map(|r| {
                r.match_predictions
                    .iter()
                    .map(|p| p.game_id.clone())
                    .collect()
            })
            .unwrap_or_default();

        // Games in this group that are result-pending and have an idEvent.
        // event idEvent -> (game_id, round) for O(1) join below.
        let mut by_event: std::collections::HashMap<String, (String, domain::Round)> =
            std::collections::HashMap::new();
        for game in tournament.games_in(&group_id) {
            let round = tournament
                .groups
                .get(&game.group_id)
                .map(|g| g.round)
                .unwrap_or(domain::Round::GroupStage);
            let pending = crate::timeflags::result_pending(
                game.kickoff,
                round,
                entered.contains(&game.id),
                now,
            );
            if pending {
                if let Some(ext) = &game.external_id {
                    by_event.insert(ext.clone(), (game.id.clone(), round));
                }
            }
        }
        if by_event.is_empty() {
            return Ok(Vec::new());
        }

        // Look up each candidate event individually — per-event lookup has
        // accurate status/score while the bulk feed lags. Any error degrades
        // to empty so manual entry is never blocked.
        let ids: Vec<String> = by_event.keys().cloned().collect();
        let source = ctx.data_unchecked::<Arc<dyn crate::reported::ReportedResultSource>>();
        let events = source.lookup_events(&ids).await.unwrap_or_default();

        let mut out = Vec::new();
        for e in events {
            // Suggest the scoreline whenever both scores are present —
            // do NOT gate on finished-status (status lags in the bulk feed).
            // The admin confirms before submitting; a deep-stoppage score is acceptable.
            if let Some((game_id, round)) = by_event.get(&e.id_event) {
                if let (Some(h), Some(a)) = (e.int_home_score, e.int_away_score) {
                    out.push(ReportedResult {
                        game_id: game_id.clone(),
                        home_score: h as i32,
                        away_score: a as i32,
                        source: "thesportsdb".to_string(),
                        source_status: e.str_status.clone(),
                        ninety_minute_uncertain: *round != domain::Round::GroupStage,
                    });
                }
            }
        }
        out.sort_by(|a, b| a.game_id.cmp(&b.game_id));
        Ok(out)
    }

    /// One match's detail (`#2`): the all-players tip grid plus the best
    /// available actual score — official if entered, else the live score
    /// during the match (provisional), else none. Read-only and ephemeral:
    /// it never writes, and never calls recompute()/put_scoreboard().
    #[graphql(name = "match")]
    async fn match_detail(
        &self,
        ctx: &Context<'_>,
        game_id: String,
        pool: Option<String>,
    ) -> async_graphql::Result<Option<MatchDetail>> {
        let viewer = CurrentPlayer::require(ctx)?;
        let repo = repo(ctx);
        let now = now(ctx);
        let Some(tournament) = repo.get_tournament().await? else {
            return Ok(None);
        };
        let Some(game) = tournament.games.get(&game_id) else {
            return Ok(None);
        };
        let round = tournament
            .groups
            .get(&game.group_id)
            .map(|g| g.round)
            .unwrap_or(domain::Round::GroupStage);
        let players = repo.list_players().await?;
        let config = ScoringConfig::default();
        let result_user = players.iter().find(|p| p.is_result_user);

        // entered-result game ids → for Game::build's resultPending flag.
        let entered: std::collections::HashSet<String> = result_user
            .map(|r| {
                r.match_predictions
                    .iter()
                    .map(|p| p.game_id.clone())
                    .collect()
            })
            .unwrap_or_default();

        // Resolve the one actual to score against: official → live → none.
        let official = result_user.and_then(|r| r.match_prediction(&game_id));
        let (actual_pred, actual_score): (Option<domain::MatchPrediction>, Option<MatchScore>) =
            if let Some(off) = official {
                (
                    Some(off.clone()),
                    Some(MatchScore {
                        home_score: off.home_score as i32,
                        away_score: off.away_score as i32,
                        provisional: false,
                        source: None,
                        source_status: None,
                        ninety_minute_uncertain: false,
                    }),
                )
            } else if now >= game.kickoff
                && now <= game.kickoff + LIVE_WINDOW
                && game.external_id.is_some()
            {
                // Live window, no official result yet → consult the source.
                // Any error/absence degrades to "no score" (page still works).
                let ext = game.external_id.clone().unwrap();
                let source = ctx.data_unchecked::<Arc<dyn crate::reported::ReportedResultSource>>();
                let events = source.lookup_events(&[ext]).await.unwrap_or_default();
                let live =
                    events
                        .into_iter()
                        .find_map(|e| match (e.int_home_score, e.int_away_score) {
                            (Some(h), Some(a))
                                if (0..=255).contains(&h) && (0..=255).contains(&a) =>
                            {
                                Some((h as u8, a as u8, e.str_status))
                            }
                            _ => None,
                        });
                match live {
                    Some((h, a, status)) => (
                        Some(domain::MatchPrediction {
                            game_id: game_id.clone(),
                            home_score: h,
                            away_score: a,
                            locked: true,
                        }),
                        Some(MatchScore {
                            home_score: h as i32,
                            away_score: a as i32,
                            provisional: true,
                            source: Some("thesportsdb".to_string()),
                            source_status: Some(status),
                            ninety_minute_uncertain: round != domain::Round::GroupStage,
                        }),
                    ),
                    None => (None, None),
                }
            } else {
                (None, None)
            };

        // Optional pool scoping — same rule as the scoreboard (Issue 04): pool
        // membership is private, so a pool filter requires the viewer to be a
        // member (or owner) of that pool. `None` → every tipper (global).
        let allowed: Option<Vec<String>> = match &pool {
            Some(pool_id) => {
                let pools = repo.list_pools().await?;
                let p = pools
                    .into_iter()
                    .find(|p| &p.id == pool_id)
                    .ok_or_else(|| async_graphql::Error::new("pool not found"))?;
                if !p.members.contains(&viewer.id) && p.owner != viewer.id {
                    return Err(async_graphql::Error::new(
                        "you are not a member of this pool",
                    ));
                }
                Some(p.members)
            }
            None => None,
        };

        // The all-players grid — same gate as `tips`, restricted to the pool.
        let deadline = tournament.deadline(&game.group_id);
        let multiplier = config.multiplier(round);
        let viewer_pred = viewer.match_prediction(&game_id);
        // A live provisional score drives the per-player max-reachable ceiling;
        // an official (final) result or no score yet leaves it null.
        let result_is_live = actual_score
            .as_ref()
            .map(|s| s.provisional)
            .unwrap_or(false);
        let game_ids = [game_id.clone()];
        let rows: Vec<Tip> = domain::participation::tippers_in(&players, &game_ids)
            .into_iter()
            .filter(|player| allowed.as_ref().is_none_or(|m| m.contains(&player.id)))
            .map(|player| {
                scored_tip(
                    &viewer.id,
                    viewer_pred,
                    player,
                    game,
                    deadline,
                    now,
                    actual_pred.as_ref(),
                    result_is_live,
                    multiplier,
                    &config,
                )
            })
            .collect();

        Ok(Some(MatchDetail {
            game: Game::build(game, round, now, &entered),
            actual: actual_score,
            rows,
        }))
    }

    /// The best third-placed-teams ranking (`FWC26_RULES.md` §3) for visibility
    /// and transparency. `player: null` → the official result user's ranking; a
    /// player id → that player's predicted ranking. Public (the schedule shows
    /// the official ranking without login). Resolves the pure
    /// `fwc26::third_place_ranking` — no domain logic here.
    async fn third_place_ranking(
        &self,
        ctx: &Context<'_>,
        player: Option<String>,
    ) -> async_graphql::Result<ThirdPlaceRanking> {
        let repo = repo(ctx);
        let Some(t) = repo.get_tournament().await? else {
            return Ok(ThirdPlaceRanking {
                entries: Vec::new(),
                complete: false,
            });
        };
        let players = repo.list_players().await?;

        // Perspective: an explicit player id, else the official result user.
        let subject = match &player {
            Some(pid) => players.iter().find(|p| &p.id == pid),
            None => players.iter().find(|p| p.is_result_user),
        };
        let Some(subject) = subject else {
            return Ok(ThirdPlaceRanking {
                entries: Vec::new(),
                complete: false,
            });
        };

        let ranking = fwc26::third_place_ranking(&t, subject);
        // t.teams contains every team_id rank_group can return (import populates both
        // consistently), so the filter_map's None branch is unreachable on valid data.
        let entries: Vec<ThirdPlaceEntry> = ranking
            .rows
            .iter()
            .filter_map(|r| {
                t.teams.get(&r.team_id).map(|team| ThirdPlaceEntry {
                    group: r.group.to_string(),
                    team: Team::from(team),
                    points: r.points,
                    goal_diff: r.goal_diff,
                    goals_for: r.goals_for,
                    rank: r.rank as i32,
                    qualifies: r.qualifies,
                    faces_winner_group: r.faces_winner_group.map(|c| c.to_string()),
                    faces_game: r.faces_game.clone(),
                })
            })
            .collect();
        // `complete` ⇔ all 12 groups final (now always 12 entries, so the old
        // `entries.len() == 12` is meaningless). Sourced from fwc26.
        Ok(ThirdPlaceRanking {
            entries,
            complete: ranking.all_groups_final,
        })
    }
}

#[cfg(test)]
mod third_place_tests {
    use crate::auth::CurrentPlayer;
    use crate::reported::ReportedResultSource;
    use async_trait::async_trait;
    use chrono::{TimeZone, Utc};
    use domain::{
        GroupChildren, GroupGame, LockMode, MatchPrediction, Player, Round, SingleGame,
        StandingsPrediction, Team, TeamSlot, Tournament,
    };
    use sportsdb::Event;
    use std::collections::HashMap;
    use std::sync::Arc;
    use storage::{InMemoryRepository, Repository};

    struct NoSource;
    #[async_trait]
    impl ReportedResultSource for NoSource {
        async fn lookup_events(&self, _ids: &[String]) -> anyhow::Result<Vec<Event>> {
            Ok(vec![])
        }
    }

    fn team(id: &str) -> Team {
        Team {
            id: id.into(),
            name: id.into(),
            short_code: id.into(),
            flag: None,
            external_id: None,
        }
    }

    fn slot(team_id: &str) -> TeamSlot {
        TeamSlot {
            team_id: Some(team_id.into()),
            description: team_id.into(),
        }
    }

    fn g(id: &str, home: &str, away: &str) -> SingleGame {
        SingleGame {
            id: id.into(),
            kickoff: Utc.with_ymd_and_hms(2026, 6, 11, 18, 0, 0).unwrap(),
            venue: None,
            group_id: "group-A".into(),
            home: slot(home),
            away: slot(away),
            external_id: None,
        }
    }

    fn preds(p1: u8, p2: u8, p3: u8, p4: u8, p5: u8, p6: u8) -> Vec<MatchPrediction> {
        vec![
            MatchPrediction {
                game_id: "M1".into(),
                home_score: p1,
                away_score: p2,
                locked: true,
            },
            MatchPrediction {
                game_id: "M2".into(),
                home_score: p3,
                away_score: p4,
                locked: true,
            },
            MatchPrediction {
                game_id: "M3".into(),
                home_score: p5,
                away_score: p6,
                locked: true,
            },
        ]
    }

    fn player(id: &str, is_result_user: bool, mp: Vec<MatchPrediction>) -> Player {
        Player {
            id: id.into(),
            person_id: format!("p-{id}"),
            nick: id.into(),
            full_name: id.into(),
            referrer: None,
            is_result_user,
            version: 0,
            match_predictions: mp,
            standings_predictions: vec![StandingsPrediction {
                group_id: "group-A".into(),
                ordering: vec![],
                draw_order: vec![],
                locked: true,
            }],
        }
    }

    /// One group A with teams AAA/BBB/CCC, round-robin M1/M2/M3.
    async fn repo_one_group() -> Arc<dyn Repository> {
        let group = GroupGame {
            id: "group-A".into(),
            name: "Group A".into(),
            parent: None,
            round: Round::GroupStage,
            lock_mode: LockMode::LockTogether,
            carries_standings: true,
            children: GroupChildren::Games(vec!["M1".into(), "M2".into(), "M3".into()]),
        };
        let t = Tournament {
            root: "group-A".into(),
            groups: HashMap::from([("group-A".to_string(), group)]),
            games: HashMap::from([
                ("M1".to_string(), g("M1", "AAA", "BBB")),
                ("M2".to_string(), g("M2", "AAA", "CCC")),
                ("M3".to_string(), g("M3", "BBB", "CCC")),
            ]),
            teams: HashMap::from([
                ("AAA".to_string(), team("AAA")),
                ("BBB".to_string(), team("BBB")),
                ("CCC".to_string(), team("CCC")),
            ]),
        };
        let repo = InMemoryRepository::new();
        repo.put_tournament(&t).await.unwrap();
        // Official: AAA wins both, BBB beats CCC -> 3rd = CCC.
        repo.put_player(&player("result-user", true, preds(2, 0, 2, 0, 1, 0)))
            .await
            .unwrap();
        // demo-ada's results: AAA loses both, CCC wins both -> 3rd = AAA.
        repo.put_player(&player("demo-ada", false, preds(0, 1, 0, 2, 0, 1)))
            .await
            .unwrap();
        Arc::new(repo)
    }

    async fn exec(repo: Arc<dyn Repository>, query: &str) -> serde_json::Value {
        let source: Arc<dyn ReportedResultSource> = Arc::new(NoSource);
        let schema = crate::gql::build_schema(repo, source);
        let req = async_graphql::Request::new(query)
            .data(CurrentPlayer::Visitor)
            .data(crate::clock::RequestNow(
                "2026-06-20T12:00:00Z".parse().unwrap(),
            ));
        let resp = schema.execute(req).await;
        assert!(resp.errors.is_empty(), "{:?}", resp.errors);
        resp.data.into_json().unwrap()
    }

    #[tokio::test]
    async fn null_player_yields_official_ranking() {
        let repo = repo_one_group().await;
        let data = exec(
            repo,
            r#"{ thirdPlaceRanking { complete entries { group rank team { id } qualifies facesGame } } }"#,
        )
        .await;
        let r = &data["thirdPlaceRanking"];
        assert_eq!(r["complete"], false);
        let entries = r["entries"].as_array().unwrap();
        assert_eq!(entries.len(), 1, "one determinable third");
        assert_eq!(entries[0]["team"]["id"], "CCC");
        assert_eq!(entries[0]["rank"], 1);
        assert_eq!(entries[0]["qualifies"], true);
        assert_eq!(entries[0]["facesGame"], serde_json::Value::Null);
    }

    #[tokio::test]
    async fn explicit_player_yields_that_players_ranking() {
        let repo = repo_one_group().await;
        let data = exec(
            repo,
            r#"{ thirdPlaceRanking(player: "demo-ada") { entries { team { id } } } }"#,
        )
        .await;
        let entries = data["thirdPlaceRanking"]["entries"].as_array().unwrap();
        assert_eq!(entries[0]["team"]["id"], "AAA");
    }

    #[tokio::test]
    async fn unknown_player_yields_empty_ranking() {
        let repo = repo_one_group().await;
        let data = exec(
            repo,
            r#"{ thirdPlaceRanking(player: "nobody") { complete entries { group } } }"#,
        )
        .await;
        let r = &data["thirdPlaceRanking"];
        assert_eq!(r["complete"], false);
        assert_eq!(r["entries"].as_array().unwrap().len(), 0);
    }
}

#[cfg(test)]
mod reported_tests {
    use crate::auth::CurrentPlayer;
    use crate::reported::ReportedResultSource;
    use async_trait::async_trait;
    use chrono::{TimeZone, Utc};
    use domain::{
        GroupChildren, GroupGame, LockMode, Player, Round, SingleGame, Team, TeamSlot, Tournament,
    };
    use sportsdb::Event;
    use std::collections::HashMap;
    use std::sync::Arc;
    use storage::{InMemoryRepository, Repository};

    struct StubSource(Vec<Event>);
    #[async_trait]
    impl ReportedResultSource for StubSource {
        async fn lookup_events(&self, ids: &[String]) -> anyhow::Result<Vec<Event>> {
            Ok(self
                .0
                .iter()
                .filter(|e| ids.contains(&e.id_event))
                .cloned()
                .collect())
        }
    }

    fn event_with_status(id_event: &str, h: i64, a: i64, status: &str) -> Event {
        Event {
            id_event: id_event.into(),
            date_event: "2026-06-11".into(),
            id_home_team: "H".into(),
            id_away_team: "A".into(),
            int_home_score: Some(h),
            int_away_score: Some(a),
            str_status: status.into(),
            str_timestamp: None,
        }
    }

    fn finished(id_event: &str, h: i64, a: i64) -> Event {
        event_with_status(id_event, h, a, "Match Finished")
    }

    // An authenticated, ordinary player (NOT the result user).
    fn regular_player() -> Player {
        Player {
            id: "demo-ada".into(),
            person_id: "pa".into(),
            nick: "ada".into(),
            full_name: "Ada".into(),
            referrer: None,
            is_result_user: false,
            version: 0,
            match_predictions: vec![],
            standings_predictions: vec![],
        }
    }

    // Result user with NO prediction for M1 -> M1 is result-pending.
    fn result_user() -> Player {
        Player {
            id: "result-user".into(),
            person_id: "p".into(),
            nick: "official".into(),
            full_name: "Official".into(),
            referrer: None,
            is_result_user: true,
            version: 0,
            match_predictions: vec![],
            standings_predictions: vec![],
        }
    }

    async fn repo_with_pending_m1() -> Arc<dyn Repository> {
        let team = |id: &str| Team {
            id: id.into(),
            name: id.into(),
            short_code: id.into(),
            flag: None,
            external_id: None,
        };
        let g1 = SingleGame {
            id: "M1".into(),
            kickoff: Utc.with_ymd_and_hms(2026, 6, 11, 19, 0, 0).unwrap(),
            venue: None,
            group_id: "A".into(),
            home: TeamSlot {
                team_id: Some("AAA".into()),
                description: "A1".into(),
            },
            away: TeamSlot {
                team_id: Some("BBB".into()),
                description: "A2".into(),
            },
            external_id: Some("E1".into()),
        };
        let group = GroupGame {
            id: "A".into(),
            name: "A".into(),
            parent: None,
            round: Round::GroupStage,
            lock_mode: LockMode::LockTogether,
            carries_standings: true,
            children: GroupChildren::Games(vec!["M1".into()]),
        };
        let t = Tournament {
            root: "A".into(),
            groups: HashMap::from([("A".to_string(), group)]),
            games: HashMap::from([("M1".to_string(), g1)]),
            teams: HashMap::from([
                ("AAA".to_string(), team("AAA")),
                ("BBB".to_string(), team("BBB")),
            ]),
        };
        let repo = InMemoryRepository::new();
        repo.put_tournament(&t).await.unwrap();
        repo.put_player(&result_user()).await.unwrap();
        Arc::new(repo)
    }

    #[tokio::test]
    async fn maps_finished_event_to_pending_game_for_result_user() {
        let repo = repo_with_pending_m1().await;
        let source: Arc<dyn ReportedResultSource> =
            Arc::new(StubSource(vec![finished("E1", 2, 1)]));
        let schema = crate::gql::build_schema(repo, source);
        let req = async_graphql::Request::new(
            r#"{ reportedResults(groupId:"A"){ gameId homeScore awayScore source sourceStatus ninetyMinuteUncertain } }"#,
        )
        .data(CurrentPlayer::Player(Box::new(result_user())))
        // kickoff 19:00 + 105min buffer -> pending after 20:45; noon next day is pending.
        .data(crate::clock::RequestNow("2026-06-12T12:00:00Z".parse().unwrap()));
        let resp = schema.execute(req).await;
        assert!(resp.errors.is_empty(), "{:?}", resp.errors);
        let json = resp.data.into_json().unwrap();
        let row = &json["reportedResults"][0];
        assert_eq!(row["gameId"], "M1");
        assert_eq!(row["homeScore"], 2);
        assert_eq!(row["awayScore"], 1);
        assert_eq!(row["source"], "thesportsdb");
        assert_eq!(row["ninetyMinuteUncertain"], false);
    }

    #[tokio::test]
    async fn non_result_user_is_rejected() {
        let repo = repo_with_pending_m1().await;
        let source: Arc<dyn ReportedResultSource> =
            Arc::new(StubSource(vec![finished("E1", 2, 1)]));
        let schema = crate::gql::build_schema(repo, source);
        let req = async_graphql::Request::new(r#"{ reportedResults(groupId:"A"){ gameId } }"#)
            .data(CurrentPlayer::Visitor)
            .data(crate::clock::RequestNow(
                "2026-06-12T12:00:00Z".parse().unwrap(),
            ));
        let resp = schema.execute(req).await;
        assert!(!resp.errors.is_empty());
    }

    #[tokio::test]
    async fn authenticated_non_admin_is_rejected() {
        let repo = repo_with_pending_m1().await;
        let source: Arc<dyn ReportedResultSource> =
            Arc::new(StubSource(vec![finished("E1", 2, 1)]));
        let schema = crate::gql::build_schema(repo, source);
        let req = async_graphql::Request::new(r#"{ reportedResults(groupId:"A"){ gameId } }"#)
            .data(CurrentPlayer::Player(Box::new(regular_player())))
            .data(crate::clock::RequestNow(
                "2026-06-12T12:00:00Z".parse().unwrap(),
            ));
        let resp = schema.execute(req).await;
        assert!(!resp.errors.is_empty());
    }

    #[tokio::test]
    async fn suggests_in_progress_scoreline_when_status_not_final() {
        // An event with status "2H" (in-progress) but both scores present IS
        // suggested — we no longer gate on finished-status.
        let repo = repo_with_pending_m1().await;
        let source: Arc<dyn ReportedResultSource> =
            Arc::new(StubSource(vec![event_with_status("E1", 3, 0, "2H")]));
        let schema = crate::gql::build_schema(repo, source);
        let req = async_graphql::Request::new(
            r#"{ reportedResults(groupId:"A"){ gameId homeScore awayScore sourceStatus } }"#,
        )
        .data(CurrentPlayer::Player(Box::new(result_user())))
        .data(crate::clock::RequestNow(
            "2026-06-12T12:00:00Z".parse().unwrap(),
        ));
        let resp = schema.execute(req).await;
        assert!(resp.errors.is_empty(), "{:?}", resp.errors);
        let json = resp.data.into_json().unwrap();
        let row = &json["reportedResults"][0];
        assert_eq!(row["gameId"], "M1");
        assert_eq!(row["homeScore"], 3);
        assert_eq!(row["awayScore"], 0);
        assert_eq!(row["sourceStatus"], "2H");
    }

    #[tokio::test]
    async fn event_without_scores_is_not_suggested() {
        // An event with no scores (null int_home_score) must NOT appear in results.
        let repo = repo_with_pending_m1().await;
        let no_score_event = Event {
            id_event: "E1".into(),
            date_event: "2026-06-11".into(),
            id_home_team: "H".into(),
            id_away_team: "A".into(),
            int_home_score: None,
            int_away_score: None,
            str_status: "NS".into(),
            str_timestamp: None,
        };
        let source: Arc<dyn ReportedResultSource> = Arc::new(StubSource(vec![no_score_event]));
        let schema = crate::gql::build_schema(repo, source);
        let req = async_graphql::Request::new(r#"{ reportedResults(groupId:"A"){ gameId } }"#)
            .data(CurrentPlayer::Player(Box::new(result_user())))
            .data(crate::clock::RequestNow(
                "2026-06-12T12:00:00Z".parse().unwrap(),
            ));
        let resp = schema.execute(req).await;
        assert!(resp.errors.is_empty(), "{:?}", resp.errors);
        let json = resp.data.into_json().unwrap();
        assert_eq!(json["reportedResults"].as_array().unwrap().len(), 0);
    }
}

#[cfg(test)]
mod match_tests {
    use crate::auth::CurrentPlayer;
    use crate::reported::ReportedResultSource;
    use async_trait::async_trait;
    use chrono::{DateTime, TimeZone, Utc};
    use domain::{
        GroupChildren, GroupGame, LockMode, MatchPrediction, Player, Pool, Round, SingleGame, Team,
        TeamSlot, Tournament,
    };
    use sportsdb::Event;
    use std::collections::HashMap;
    use std::sync::Arc;
    use storage::{InMemoryRepository, Repository};

    struct StubSource(Vec<Event>);
    #[async_trait]
    impl ReportedResultSource for StubSource {
        async fn lookup_events(&self, ids: &[String]) -> anyhow::Result<Vec<Event>> {
            Ok(self
                .0
                .iter()
                .filter(|e| ids.contains(&e.id_event))
                .cloned()
                .collect())
        }
    }

    fn live_event(id_event: &str, h: i64, a: i64, status: &str) -> Event {
        Event {
            id_event: id_event.into(),
            date_event: "2026-06-11".into(),
            id_home_team: "AAA".into(),
            id_away_team: "BBB".into(),
            int_home_score: Some(h),
            int_away_score: Some(a),
            str_status: status.into(),
            str_timestamp: None,
        }
    }

    fn team(id: &str) -> Team {
        Team {
            id: id.into(),
            name: id.into(),
            short_code: id.into(),
            flag: None,
            external_id: None,
        }
    }

    /// A full ordinary player with one prediction for `M1` (mirrors the field
    /// set used by `reported_tests` — there is no `Player::new`).
    fn player(id: &str, h: u8, a: u8, locked: bool) -> Player {
        Player {
            id: id.into(),
            person_id: format!("p-{id}"),
            nick: id.into(),
            full_name: id.into(),
            referrer: None,
            is_result_user: false,
            version: 0,
            match_predictions: vec![MatchPrediction {
                game_id: "M1".into(),
                home_score: h,
                away_score: a,
                locked,
            }],
            standings_predictions: vec![],
        }
    }

    /// One group-stage game `M1` (idEvent `E1`) kicking off at `kickoff`.
    async fn repo_with_m1(kickoff: DateTime<Utc>) -> InMemoryRepository {
        let game = SingleGame {
            id: "M1".into(),
            kickoff,
            venue: None,
            group_id: "A".into(),
            home: TeamSlot {
                team_id: Some("AAA".into()),
                description: "A1".into(),
            },
            away: TeamSlot {
                team_id: Some("BBB".into()),
                description: "A2".into(),
            },
            external_id: Some("E1".into()),
        };
        let group = GroupGame {
            id: "A".into(),
            name: "A".into(),
            parent: None,
            round: Round::GroupStage,
            lock_mode: LockMode::LockTogether,
            carries_standings: true,
            children: GroupChildren::Games(vec!["M1".into()]),
        };
        let t = Tournament {
            root: "A".into(),
            groups: HashMap::from([("A".to_string(), group)]),
            games: HashMap::from([("M1".to_string(), game)]),
            teams: HashMap::from([
                ("AAA".to_string(), team("AAA")),
                ("BBB".to_string(), team("BBB")),
            ]),
        };
        let repo = InMemoryRepository::new();
        repo.put_tournament(&t).await.unwrap();
        repo
    }

    fn kickoff() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 6, 11, 18, 0, 0).unwrap()
    }

    /// Like `repo_with_m1` but `M1` sits in a knockout one-match group, so the
    /// resolver flags provisional points 90-minute-uncertain.
    async fn repo_with_knockout_m1(kickoff: DateTime<Utc>) -> InMemoryRepository {
        let game = SingleGame {
            id: "M1".into(),
            kickoff,
            venue: None,
            group_id: "K".into(),
            home: TeamSlot {
                team_id: Some("AAA".into()),
                description: "1A".into(),
            },
            away: TeamSlot {
                team_id: Some("BBB".into()),
                description: "2B".into(),
            },
            external_id: Some("E1".into()),
        };
        let group = GroupGame {
            id: "K".into(),
            name: "Knockout — match 1".into(),
            parent: None,
            round: Round::R32,
            lock_mode: LockMode::LockPerMatch,
            carries_standings: false,
            children: GroupChildren::Games(vec!["M1".into()]),
        };
        let t = Tournament {
            root: "K".into(),
            groups: HashMap::from([("K".to_string(), group)]),
            games: HashMap::from([("M1".to_string(), game)]),
            teams: HashMap::from([
                ("AAA".to_string(), team("AAA")),
                ("BBB".to_string(), team("BBB")),
            ]),
        };
        let repo = InMemoryRepository::new();
        repo.put_tournament(&t).await.unwrap();
        repo
    }

    /// Execute `query` as `viewer` at `now`, returning the JSON `data`. Mirrors
    /// the `reported_tests` pattern: `build_schema` + a `Request` with the
    /// `CurrentPlayer` and `RequestNow` injected as context data.
    async fn exec(
        repo: InMemoryRepository,
        source: Arc<dyn ReportedResultSource>,
        viewer: Player,
        now: DateTime<Utc>,
        query: &str,
    ) -> serde_json::Value {
        let repo: Arc<dyn Repository> = Arc::new(repo);
        let schema = crate::gql::build_schema(repo, source);
        let req = async_graphql::Request::new(query)
            .data(CurrentPlayer::Player(Box::new(viewer)))
            .data(crate::clock::RequestNow(now));
        let resp = schema.execute(req).await;
        assert!(resp.errors.is_empty(), "{:?}", resp.errors);
        resp.data.into_json().unwrap()
    }

    #[tokio::test]
    async fn live_score_yields_provisional_points() {
        let repo = repo_with_m1(kickoff()).await;
        // viewer predicted 1–0; live score is 1–0 → provisional, scored.
        let alice = player("alice", 1, 0, true);
        repo.put_player(&alice).await.unwrap();
        let source: Arc<dyn ReportedResultSource> =
            Arc::new(StubSource(vec![live_event("E1", 1, 0, "2H")]));
        let now = kickoff() + chrono::Duration::minutes(67); // in-play
        let data = exec(
            repo,
            source,
            alice,
            now,
            r#"{ match(gameId:"M1"){ actual{ homeScore awayScore provisional sourceStatus ninetyMinuteUncertain } rows{ playerId points } } }"#,
        )
        .await;
        let m = &data["match"];
        assert_eq!(m["actual"]["provisional"], true);
        assert_eq!(m["actual"]["sourceStatus"], "2H");
        assert_eq!(m["actual"]["ninetyMinuteUncertain"], false);
        // alice's 1–0 vs live 1–0 scores > 0.
        let row = m["rows"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["playerId"] == "alice")
            .unwrap();
        assert!(row["points"].as_i64().unwrap() > 0);
    }

    #[tokio::test]
    async fn live_match_row_exposes_per_player_max_reachable() {
        let repo = repo_with_m1(kickoff()).await;
        // alice predicted 1–0; live score is 1–0 → still perfect-reachable.
        // max_reachable_score(1–0, live 1–0, group ×1) = 4 (domain-verified).
        let alice = player("alice", 1, 0, true);
        repo.put_player(&alice).await.unwrap();
        let source: Arc<dyn ReportedResultSource> =
            Arc::new(StubSource(vec![live_event("E1", 1, 0, "2H")]));
        let now = kickoff() + chrono::Duration::minutes(67); // in the live window
        let data = exec(
            repo,
            source,
            alice,
            now,
            r#"{ match(gameId:"M1"){ rows{ playerId maxReachable } } }"#,
        )
        .await;
        let row = data["match"]["rows"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["playerId"] == "alice")
            .unwrap();
        assert_eq!(row["maxReachable"], 4);
    }

    #[tokio::test]
    async fn max_reachable_is_null_when_match_not_live() {
        // Before kickoff there is no provisional score, so the per-match ceiling
        // is null — it only ever appears while the match is live.
        let repo = repo_with_m1(kickoff()).await;
        let alice = player("alice", 1, 0, true);
        repo.put_player(&alice).await.unwrap();
        let source: Arc<dyn ReportedResultSource> =
            Arc::new(StubSource(vec![live_event("E1", 1, 0, "2H")]));
        let now = kickoff() - chrono::Duration::hours(1); // pre-kickoff: not live
        let data = exec(
            repo,
            source,
            alice,
            now,
            r#"{ match(gameId:"M1"){ rows{ playerId maxReachable } } }"#,
        )
        .await;
        let row = data["match"]["rows"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["playerId"] == "alice")
            .unwrap();
        assert_eq!(row["maxReachable"], serde_json::Value::Null);
    }

    #[tokio::test]
    async fn max_reachable_is_null_for_official_final_result() {
        // An official (final) entered result is not "live" — no ceiling.
        let repo = repo_with_m1(kickoff()).await;
        let mut ru = player("result-user", 1, 0, true);
        ru.is_result_user = true;
        repo.put_player(&ru).await.unwrap();
        let alice = player("alice", 1, 0, true);
        repo.put_player(&alice).await.unwrap();
        let source: Arc<dyn ReportedResultSource> = Arc::new(StubSource(vec![]));
        let now = kickoff() + chrono::Duration::minutes(67);
        let data = exec(
            repo,
            source,
            alice,
            now,
            r#"{ match(gameId:"M1"){ actual{ provisional } rows{ playerId maxReachable points } } }"#,
        )
        .await;
        assert_eq!(data["match"]["actual"]["provisional"], false);
        let row = data["match"]["rows"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["playerId"] == "alice")
            .unwrap();
        // Official result is scored, but the live ceiling is null (not live).
        assert!(row["points"].as_i64().unwrap() > 0);
        assert_eq!(row["maxReachable"], serde_json::Value::Null);
    }

    #[tokio::test]
    async fn official_result_takes_priority_over_live() {
        let repo = repo_with_m1(kickoff()).await;
        // result user entered 2–2 (official); the stub says 1–0 but must be ignored.
        let mut ru = player("result-user", 2, 2, true);
        ru.is_result_user = true;
        repo.put_player(&ru).await.unwrap();
        let alice = player("alice", 2, 2, true);
        repo.put_player(&alice).await.unwrap();
        let source: Arc<dyn ReportedResultSource> =
            Arc::new(StubSource(vec![live_event("E1", 1, 0, "2H")]));
        let now = kickoff() + chrono::Duration::minutes(67);
        let data = exec(
            repo,
            source,
            alice,
            now,
            r#"{ match(gameId:"M1"){ actual{ homeScore awayScore provisional source } } }"#,
        )
        .await;
        let a = &data["match"]["actual"];
        assert_eq!(a["homeScore"], 2);
        assert_eq!(a["provisional"], false);
        assert_eq!(a["source"], serde_json::Value::Null);
    }

    #[tokio::test]
    async fn no_score_before_kickoff_and_others_hidden() {
        let repo = repo_with_m1(kickoff()).await;
        // alice has NOT locked her tip — she hasn't committed, so mutual-commitment
        // gate keeps bob's prediction hidden even though bob has locked.
        let alice = player("alice", 1, 0, false);
        repo.put_player(&alice).await.unwrap();
        repo.put_player(&player("bob", 3, 1, true)).await.unwrap();
        let source: Arc<dyn ReportedResultSource> = Arc::new(StubSource(vec![]));
        let now = kickoff() - chrono::Duration::hours(2); // before kickoff
        let data = exec(
            repo,
            source,
            alice,
            now,
            r#"{ match(gameId:"M1"){ actual{ homeScore } rows{ playerId prediction{ homeScore } } } }"#,
        )
        .await;
        assert_eq!(data["match"]["actual"], serde_json::Value::Null);
        // bob's prediction is hidden: alice hasn't committed (unlocked draft).
        let rows = data["match"]["rows"].as_array().unwrap();
        let bob = rows.iter().find(|r| r["playerId"] == "bob").unwrap();
        assert!(bob["prediction"].is_null());
    }

    #[tokio::test]
    async fn knockout_live_score_is_ninety_minute_uncertain() {
        let repo = repo_with_knockout_m1(kickoff()).await;
        let alice = player("alice", 1, 0, true);
        repo.put_player(&alice).await.unwrap();
        let source: Arc<dyn ReportedResultSource> =
            Arc::new(StubSource(vec![live_event("E1", 1, 0, "2H")]));
        let now = kickoff() + chrono::Duration::minutes(67); // in live window
        let data = exec(
            repo,
            source,
            alice,
            now,
            r#"{ match(gameId:"M1"){ actual{ provisional ninetyMinuteUncertain } } }"#,
        )
        .await;
        let a = &data["match"]["actual"];
        assert_eq!(a["provisional"], true);
        assert_eq!(a["ninetyMinuteUncertain"], true);
    }

    #[tokio::test]
    async fn live_window_with_empty_source_yields_no_score() {
        let repo = repo_with_m1(kickoff()).await;
        let alice = player("alice", 1, 0, true);
        repo.put_player(&alice).await.unwrap();
        // Inside the live window (post-kickoff), but the source returns nothing →
        // graceful no score. Distinct from the pre-kickoff case (source not consulted).
        let source: Arc<dyn ReportedResultSource> = Arc::new(StubSource(vec![]));
        let now = kickoff() + chrono::Duration::minutes(30);
        let data = exec(
            repo,
            source,
            alice,
            now,
            r#"{ match(gameId:"M1"){ actual{ homeScore } rows{ playerId } } }"#,
        )
        .await;
        assert_eq!(data["match"]["actual"], serde_json::Value::Null);
        // The grid still renders — degradation never blocks the page.
        assert!(!data["match"]["rows"].as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn pool_filter_restricts_rows_to_members() {
        let repo = repo_with_m1(kickoff()).await;
        let alice = player("alice", 1, 0, true);
        let bob = player("bob", 2, 0, true);
        repo.put_player(&alice).await.unwrap();
        repo.put_player(&bob).await.unwrap();
        // Pool P1 contains only alice (the viewer) — bob is excluded from her view.
        repo.put_pool(&Pool {
            id: "P1".into(),
            name: "Pool 1".into(),
            owner: "alice".into(),
            members: vec!["alice".into()],
            prefix: "P1".into(),
        })
        .await
        .unwrap();
        let source: Arc<dyn ReportedResultSource> = Arc::new(StubSource(vec![]));
        let now = kickoff() + chrono::Duration::minutes(30);
        let data = exec(
            repo,
            source,
            alice,
            now,
            r#"{ match(gameId:"M1", pool:"P1"){ rows{ playerId } } }"#,
        )
        .await;
        let ids: Vec<String> = data["match"]["rows"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| r["playerId"].as_str().unwrap().to_string())
            .collect();
        assert!(ids.contains(&"alice".to_string()), "alice (member) shown");
        assert!(!ids.contains(&"bob".to_string()), "bob (non-member) hidden");
    }
}

#[cfg(test)]
mod perfects_tests {
    use crate::auth::CurrentPlayer;
    use crate::reported::ReportedResultSource;
    use async_trait::async_trait;
    use chrono::{TimeZone, Utc};
    use domain::{
        GroupChildren, GroupGame, LockMode, MatchPrediction, Player, Pool, Round, SingleGame, Team,
        TeamSlot, Tournament,
    };
    use sportsdb::Event;
    use std::collections::HashMap;
    use std::sync::Arc;
    use storage::{InMemoryRepository, Repository};

    struct NoSource;
    #[async_trait]
    impl ReportedResultSource for NoSource {
        async fn lookup_events(&self, _ids: &[String]) -> anyhow::Result<Vec<Event>> {
            Ok(vec![])
        }
    }

    fn team(id: &str) -> Team {
        Team {
            id: id.into(),
            name: id.into(),
            short_code: id.into(),
            flag: None,
            external_id: None,
        }
    }

    fn player_with(id: &str, h: u8, a: u8, is_result_user: bool) -> Player {
        Player {
            id: id.into(),
            person_id: format!("p-{id}"),
            nick: id.into(),
            full_name: id.into(),
            referrer: None,
            is_result_user,
            version: 0,
            match_predictions: vec![MatchPrediction {
                game_id: "M1".into(),
                home_score: h,
                away_score: a,
                locked: true,
            }],
            standings_predictions: vec![],
        }
    }

    async fn repo_with_two_perfects() -> InMemoryRepository {
        let game = SingleGame {
            id: "M1".into(),
            kickoff: Utc.with_ymd_and_hms(2026, 6, 11, 18, 0, 0).unwrap(),
            venue: None,
            group_id: "A".into(),
            home: TeamSlot {
                team_id: Some("AAA".into()),
                description: "A1".into(),
            },
            away: TeamSlot {
                team_id: Some("BBB".into()),
                description: "A2".into(),
            },
            external_id: None,
        };
        let group = GroupGame {
            id: "A".into(),
            name: "A".into(),
            parent: None,
            round: Round::GroupStage,
            lock_mode: LockMode::LockTogether,
            carries_standings: true,
            children: GroupChildren::Games(vec!["M1".into()]),
        };
        let t = Tournament {
            root: "A".into(),
            groups: HashMap::from([("A".to_string(), group)]),
            games: HashMap::from([("M1".to_string(), game)]),
            teams: HashMap::from([
                ("AAA".to_string(), team("AAA")),
                ("BBB".to_string(), team("BBB")),
            ]),
        };
        let repo = InMemoryRepository::new();
        repo.put_tournament(&t).await.unwrap();
        // Official result 2–1; both players predicted 2–1 → both perfect.
        repo.put_player(&player_with("result-user", 2, 1, true))
            .await
            .unwrap();
        repo.put_player(&player_with("alice", 2, 1, false))
            .await
            .unwrap();
        repo.put_player(&player_with("bob", 2, 1, false))
            .await
            .unwrap();
        repo
    }

    async fn exec(repo: InMemoryRepository, viewer: Player, query: &str) -> serde_json::Value {
        let repo: Arc<dyn Repository> = Arc::new(repo);
        let source: Arc<dyn ReportedResultSource> = Arc::new(NoSource);
        let schema = crate::gql::build_schema(repo, source);
        let req = async_graphql::Request::new(query)
            .data(CurrentPlayer::Player(Box::new(viewer)))
            .data(crate::clock::RequestNow(
                "2026-06-12T12:00:00Z".parse().unwrap(),
            ));
        let resp = schema.execute(req).await;
        assert!(resp.errors.is_empty(), "{:?}", resp.errors);
        resp.data.into_json().unwrap()
    }

    #[tokio::test]
    async fn no_pool_returns_every_perfect() {
        let repo = repo_with_two_perfects().await;
        let data = exec(
            repo,
            player_with("alice", 2, 1, false),
            r#"{ perfects { playerId gameId } }"#,
        )
        .await;
        let ids: Vec<String> = data["perfects"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| p["playerId"].as_str().unwrap().to_string())
            .collect();
        assert!(ids.contains(&"alice".to_string()));
        assert!(ids.contains(&"bob".to_string()));
    }

    #[tokio::test]
    async fn pool_filter_restricts_perfects_to_members() {
        let repo = repo_with_two_perfects().await;
        // Pool P1 contains only alice (the viewer); bob is excluded.
        repo.put_pool(&Pool {
            id: "P1".into(),
            name: "Pool 1".into(),
            owner: "alice".into(),
            members: vec!["alice".into()],
            prefix: "P1".into(),
        })
        .await
        .unwrap();
        let data = exec(
            repo,
            player_with("alice", 2, 1, false),
            r#"{ perfects(pool:"P1") { playerId gameId } }"#,
        )
        .await;
        let ids: Vec<String> = data["perfects"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| p["playerId"].as_str().unwrap().to_string())
            .collect();
        assert!(ids.contains(&"alice".to_string()), "alice (member) shown");
        assert!(!ids.contains(&"bob".to_string()), "bob (non-member) hidden");
    }

    #[tokio::test]
    async fn pool_filter_requires_membership() {
        let repo = repo_with_two_perfects().await;
        // Pool P2 does NOT contain bob — bob asking to scope to it is rejected.
        repo.put_pool(&Pool {
            id: "P2".into(),
            name: "Pool 2".into(),
            owner: "alice".into(),
            members: vec!["alice".into()],
            prefix: "P2".into(),
        })
        .await
        .unwrap();
        let repo: Arc<dyn Repository> = Arc::new(repo);
        let source: Arc<dyn ReportedResultSource> = Arc::new(NoSource);
        let schema = crate::gql::build_schema(repo, source);
        let req = async_graphql::Request::new(r#"{ perfects(pool:"P2") { playerId } }"#)
            .data(CurrentPlayer::Player(Box::new(player_with(
                "bob", 2, 1, false,
            ))))
            .data(crate::clock::RequestNow(
                "2026-06-12T12:00:00Z".parse().unwrap(),
            ));
        let resp = schema.execute(req).await;
        assert!(!resp.errors.is_empty(), "non-member must be rejected");
    }
}
