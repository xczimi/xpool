//! Serde DTOs mirroring the `tournaments/fwc26.json` schema, plus the converter
//! into `domain::Tournament`.
//!
//! The on-disk file stores `groups` and `games` as **arrays** (authoring is
//! easier as a list); `domain::Tournament` keys them by id in `HashMap`s. The
//! converter performs that reshape and validates the result loudly.

use anyhow::{bail, Context};
use domain::{GroupChildren, GroupGame, LockMode, Round, SingleGame, Team, TeamSlot, Tournament};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct TournamentDto {
    pub tournament_id: String,
    pub teams: Vec<TeamDto>,
    pub groups: Vec<GroupDto>,
    pub games: Vec<GameDto>,
}

#[derive(Debug, Deserialize)]
pub struct TeamDto {
    pub id: String,
    pub name: String,
    pub short_code: String,
    #[serde(default)]
    pub flag: Option<String>,
    #[serde(default)]
    pub external_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GroupDto {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub parent: Option<String>,
    pub round: String,
    pub lock_mode: String,
    pub carries_standings: bool,
    pub children: GroupChildrenDto,
}

/// `{"groups":[...]}` or `{"games":[...]}` — exactly one variant.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GroupChildrenDto {
    Groups(Vec<String>),
    Games(Vec<String>),
}

#[derive(Debug, Deserialize)]
pub struct GameDto {
    pub id: String,
    pub kickoff: chrono::DateTime<chrono::Utc>,
    #[serde(default)]
    pub venue: Option<String>,
    pub group_id: String,
    pub home: TeamSlotDto,
    pub away: TeamSlotDto,
}

#[derive(Debug, Deserialize)]
pub struct TeamSlotDto {
    #[serde(default)]
    pub team_id: Option<String>,
    pub description: String,
}

fn parse_round(s: &str) -> anyhow::Result<Round> {
    Ok(match s {
        "GroupStage" => Round::GroupStage,
        "R32" => Round::R32,
        "R16" => Round::R16,
        "QF" => Round::QF,
        "SF" => Round::SF,
        "ThirdPlace" => Round::ThirdPlace,
        "Final" => Round::Final,
        other => bail!("unknown round variant `{other}`"),
    })
}

fn parse_lock_mode(s: &str) -> anyhow::Result<LockMode> {
    Ok(match s {
        "LockTogether" => LockMode::LockTogether,
        "LockPerMatch" => LockMode::LockPerMatch,
        other => bail!("unknown lock_mode variant `{other}`"),
    })
}

impl TeamSlotDto {
    fn into_domain(self) -> TeamSlot {
        TeamSlot {
            team_id: self.team_id,
            description: self.description,
        }
    }
}

impl GroupChildrenDto {
    fn into_domain(self) -> GroupChildren {
        match self {
            GroupChildrenDto::Groups(v) => GroupChildren::Groups(v),
            GroupChildrenDto::Games(v) => GroupChildren::Games(v),
        }
    }
}

impl TournamentDto {
    /// Convert into a `domain::Tournament`, reshaping arrays into id-keyed maps.
    /// The single root group is the one with no `parent`.
    pub fn into_domain(self) -> anyhow::Result<Tournament> {
        let teams: HashMap<String, Team> = self
            .teams
            .into_iter()
            .map(|t| {
                (
                    t.id.clone(),
                    Team {
                        id: t.id,
                        name: t.name,
                        short_code: t.short_code,
                        flag: t.flag,
                        external_id: t.external_id,
                    },
                )
            })
            .collect();

        let games: HashMap<String, SingleGame> = self
            .games
            .into_iter()
            .map(|g| {
                (
                    g.id.clone(),
                    SingleGame {
                        id: g.id,
                        kickoff: g.kickoff,
                        venue: g.venue,
                        group_id: g.group_id,
                        home: g.home.into_domain(),
                        away: g.away.into_domain(),
                    },
                )
            })
            .collect();

        let mut roots: Vec<String> = Vec::new();
        let mut groups: HashMap<String, GroupGame> = HashMap::new();
        for g in self.groups {
            if g.parent.is_none() {
                roots.push(g.id.clone());
            }
            let round = parse_round(&g.round).with_context(|| format!("group `{}`", g.id))?;
            let lock_mode =
                parse_lock_mode(&g.lock_mode).with_context(|| format!("group `{}`", g.id))?;
            groups.insert(
                g.id.clone(),
                GroupGame {
                    id: g.id,
                    name: g.name,
                    parent: g.parent,
                    round,
                    lock_mode,
                    carries_standings: g.carries_standings,
                    children: g.children.into_domain(),
                },
            );
        }

        let root = match roots.as_slice() {
            [r] => r.clone(),
            [] => bail!("no root group found (every group has a parent)"),
            many => bail!("multiple root groups found: {many:?}"),
        };

        Ok(Tournament {
            root,
            groups,
            games,
            teams,
        })
    }
}
