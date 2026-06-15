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
    async fn tips(&self, ctx: &Context<'_>, group_id: String) -> async_graphql::Result<Vec<Tip>> {
        let viewer = CurrentPlayer::require(ctx)?;
        let repo = repo(ctx);
        let tournament = repo
            .get_tournament()
            .await?
            .ok_or_else(|| async_graphql::Error::new("no tournament loaded"))?;
        let players = repo.list_players().await?;

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
        for player in domain::participation::tippers_in(&players, &game_ids) {
            for game in &games {
                let prediction = player.match_prediction(&game.id);
                // Own predictions are always visible to the viewer.
                let is_own = player.id == viewer.id;
                // Once the match kicks off (or the group deadline passes) the
                // game is open to everyone — viewer and target are both then
                // effective-locked regardless of their explicit lock flags.
                let time_open = now >= game.kickoff || deadline.is_some_and(|d| now > d);
                // Mutual commitment (legacy `AllTipsHandler`): another player's
                // tip is revealed only once the *viewer* has effective-locked
                // this match — so you can't peek at others' tips for a game you
                // can still change. We also keep the target's lock in the gate
                // so an un-locked draft is never exposed before the deadline.
                let viewer_committed =
                    time_open || viewer.match_prediction(&game.id).is_some_and(|p| p.locked);
                let visible = is_own
                    || (viewer_committed && prediction.is_some_and(|p| p.locked || time_open));
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
        let viewer = CurrentPlayer::require(ctx)?;
        if !viewer.is_result_user {
            return Err(async_graphql::Error::new("not authorized"));
        }

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

        // Fetch reported results; any error degrades to empty.
        let source = ctx.data_unchecked::<Arc<dyn crate::reported::ReportedResultSource>>();
        let events = source.finished_results().await.unwrap_or_default();

        let mut out = Vec::new();
        for e in events {
            if !e.is_finished() {
                continue;
            }
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
        async fn finished_results(&self) -> anyhow::Result<Vec<Event>> {
            Ok(self.0.clone())
        }
    }

    fn finished(id_event: &str, h: i64, a: i64) -> Event {
        Event {
            id_event: id_event.into(),
            date_event: "2026-06-11".into(),
            id_home_team: "H".into(),
            id_away_team: "A".into(),
            int_home_score: Some(h),
            int_away_score: Some(a),
            str_status: "Match Finished".into(),
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
}
