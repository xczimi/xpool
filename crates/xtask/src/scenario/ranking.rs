//! Team-strength input for the generator. A flat `{team_id: strength}` map kept
//! out of the domain `Team` contract — it is a generator-only signal.

use anyhow::{bail, Context};
use domain::Tournament;
use std::collections::HashMap;
use std::path::Path;

/// Per-team strength. Higher = stronger. Drives upset probability and "chalk".
#[derive(Clone, Debug)]
pub struct Ranking {
    strengths: HashMap<String, u32>,
}

impl Ranking {
    /// Load and parse the rankings JSON. Does not validate against a tournament.
    pub fn load(path: &Path) -> anyhow::Result<Ranking> {
        let raw = std::fs::read_to_string(path)
            .with_context(|| format!("reading rankings file `{}`", path.display()))?;
        let strengths: HashMap<String, u32> = serde_json::from_str(&raw)
            .with_context(|| format!("parsing rankings JSON `{}`", path.display()))?;
        Ok(Ranking { strengths })
    }

    /// Strength for a team; 0 if absent (callers should `validate` first).
    pub fn strength(&self, team: &str) -> u32 {
        self.strengths.get(team).copied().unwrap_or(0)
    }

    /// Every tournament team must have a strength, or this errors loudly.
    pub fn validate(&self, t: &Tournament) -> anyhow::Result<()> {
        let missing: Vec<&String> = t
            .teams
            .keys()
            .filter(|id| !self.strengths.contains_key(*id))
            .collect();
        if !missing.is_empty() {
            bail!("rankings missing strengths for teams: {missing:?}");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap as Map;

    fn tournament_with_teams(ids: &[&str]) -> Tournament {
        let teams = ids
            .iter()
            .map(|id| {
                (
                    id.to_string(),
                    domain::Team {
                        id: id.to_string(),
                        name: id.to_string(),
                        short_code: id.to_string(),
                        flag: None,
                        external_id: None,
                    },
                )
            })
            .collect();
        Tournament {
            root: "ROOT".into(),
            groups: Map::new(),
            games: Map::new(),
            teams,
        }
    }

    #[test]
    fn strength_reads_back() {
        let r = Ranking {
            strengths: Map::from([("ARG".to_string(), 96), ("BIH".to_string(), 39)]),
        };
        assert_eq!(r.strength("ARG"), 96);
        assert_eq!(r.strength("BIH"), 39);
        assert_eq!(r.strength("???"), 0);
    }

    #[test]
    fn validate_flags_missing_team() {
        let r = Ranking {
            strengths: Map::from([("ARG".to_string(), 96)]),
        };
        assert!(r.validate(&tournament_with_teams(&["ARG"])).is_ok());
        assert!(r.validate(&tournament_with_teams(&["ARG", "BIH"])).is_err());
    }

    #[test]
    fn real_file_covers_every_team() {
        // The shipped file must cover the real tournament.
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tournaments/fwc26.json");
        let t = crate::load_tournament(&path).expect("load tournament");
        let rpath = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tournaments/fwc26-rankings.json");
        let r = Ranking::load(&rpath).expect("load rankings");
        r.validate(&t).expect("every team ranked");
    }
}
