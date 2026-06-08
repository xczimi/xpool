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

        let mut entries: Vec<ScoreEntry> = board
            .entries
            .iter()
            .filter(|(pid, _)| allowed.as_ref().is_none_or(|m| m.contains(pid)))
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
                Ok(Some(Viewer::Player(Box::new(Player::from(p.as_ref())))))
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
    /// §6, UC-9). A prediction is visible to others only once it is
    /// effective-locked or the match has kicked off.
    async fn tips(&self, ctx: &Context<'_>, group_id: String) -> async_graphql::Result<Vec<Tip>> {
        let viewer = CurrentPlayer::require(ctx)?;
        let repo = repo(ctx);
        let tournament = repo
            .get_tournament()
            .await?
            .ok_or_else(|| async_graphql::Error::new("no tournament loaded"))?;
        let players = repo.list_players().await?;

        let games = tournament.games_in(&group_id);
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
        for player in &players {
            if player.is_result_user {
                continue;
            }
            for game in &games {
                let prediction = player.match_prediction(&game.id);
                // Own predictions are always visible to the viewer.
                let is_own = player.id == viewer.id;
                let visible = is_own
                    || prediction.is_some_and(|p| {
                        // effective-locked: locked, OR deadline passed, OR
                        // the match itself kicked off.
                        p.locked || now >= game.kickoff || deadline.is_some_and(|d| now > d)
                    });
                // Score a visible prediction once the game has a result. By the
                // time a result exists the match has kicked off, so a scored
                // tip is always already visible — no hidden info is revealed.
                let result = result_user.and_then(|r| r.match_prediction(&game.id));
                let breakdown = match (visible, prediction, result) {
                    (true, Some(pred), Some(res)) => Some(PointsBreakdown::build(
                        score_match_parts(pred, res, &config),
                        config.multiplier(round_of(game)),
                        &config,
                    )),
                    _ => None,
                };
                let points = breakdown.as_ref().map(|b| b.points);
                let is_perfect_tip = breakdown
                    .as_ref()
                    .is_some_and(|b| b.base >= config.perfect_threshold);
                tips.push(Tip {
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
                });
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
            for player in &players {
                if player.is_result_user {
                    continue;
                }
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

    /// Every "perfect" (maximum-scoring) prediction across all players (UC-10).
    async fn perfects(&self, ctx: &Context<'_>) -> async_graphql::Result<Vec<Perfect>> {
        let repo = repo(ctx);
        let players = repo.list_players().await?;
        let config = ScoringConfig::default();

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
}
