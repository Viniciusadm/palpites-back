use std::collections::BTreeMap;

use crate::domain::predictions::HitKind;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RankedStanding {
    pub pool_member_id: String,
    pub total_points: i32,
    pub exact_count: i32,
    pub outcome_count: i32,
    pub hits_count: i32,
    pub position: i32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Tally {
    total_points: i32,
    exact_count: i32,
    outcome_count: i32,
    hits_count: i32,
}

/// Pure aggregation of scored predictions into an ordered standings table.
///
/// Every member in `members` gets a row (seeded at zero) so the ranking lists
/// all participants even before they score. Each `(pool_member_id, points,
/// hit)` line adds points and increments the matching counters. Rows are sorted
/// by `total_points` (desc), then `hits_count` (desc), then `pool_member_id`
/// (asc) for determinism, and `position` uses standard competition ranking:
/// members tied on `total_points` share the same position (e.g. 1, 2, 2, 4).
pub fn compute(
    members: impl IntoIterator<Item = String>,
    lines: impl IntoIterator<Item = (String, i16, HitKind)>,
) -> Vec<RankedStanding> {
    let mut tallies: BTreeMap<String, Tally> = BTreeMap::new();
    for member_id in members {
        tallies.entry(member_id).or_default();
    }

    for (member_id, points, hit) in lines {
        let tally = tallies.entry(member_id).or_default();
        tally.total_points += i32::from(points);
        match hit {
            HitKind::Exact => {
                tally.exact_count += 1;
                tally.hits_count += 1;
            }
            HitKind::Outcome => {
                tally.outcome_count += 1;
                tally.hits_count += 1;
            }
            HitKind::None => {}
        }
    }

    let mut rows: Vec<RankedStanding> = tallies
        .into_iter()
        .map(|(pool_member_id, tally)| RankedStanding {
            pool_member_id,
            total_points: tally.total_points,
            exact_count: tally.exact_count,
            outcome_count: tally.outcome_count,
            hits_count: tally.hits_count,
            position: 0,
        })
        .collect();

    rows.sort_by(|a, b| {
        b.total_points
            .cmp(&a.total_points)
            .then(b.hits_count.cmp(&a.hits_count))
            .then(a.pool_member_id.cmp(&b.pool_member_id))
    });

    let mut last_points: Option<i32> = None;
    let mut position = 0;
    for (index, row) in rows.iter_mut().enumerate() {
        if last_points != Some(row.total_points) {
            position = (index as i32) + 1;
            last_points = Some(row.total_points);
        }
        row.position = position;
    }

    rows
}
