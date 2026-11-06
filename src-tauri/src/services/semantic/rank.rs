use std::{cmp::Ordering, collections::{BTreeMap, HashMap}};

use super::types::SegmentHit;

const RRF_K: f32 = 60.0;
const MAX_SEGMENTS_PER_ASSET: usize = 8;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Channel {
  Visual,
  Text,
}

impl Channel {
  fn of(kind: &str) -> Self {
    if kind == "visual" { Self::Visual } else { Self::Text }
  }

  fn tie_rank(self) -> u8 {
    match self {
      Self::Visual => 0,
      Self::Text => 1,
    }
  }
}

struct RankedSegment {
  hit: SegmentHit,
  rank_score: f32,
}

fn compare_similarity(left: &SegmentHit, right: &SegmentHit) -> Ordering {
  right.score.partial_cmp(&left.score).unwrap_or(Ordering::Equal).then(left.segment_id.cmp(&right.segment_id))
}

fn reciprocal_rank_scores(hits: &[SegmentHit]) -> HashMap<i64, f32> {
  let mut scores = HashMap::new();
  for channel in [Channel::Visual, Channel::Text] {
    let mut ranked = hits.iter().filter(|hit| Channel::of(&hit.kind) == channel).collect::<Vec<_>>();
    ranked.sort_by(|left, right| compare_similarity(left, right));
    for (index, hit) in ranked.into_iter().enumerate() {
      scores.insert(hit.segment_id, 1.0 / (RRF_K + index as f32 + 1.0));
    }
  }
  scores
}

fn compare_fused(left: &RankedSegment, right: &RankedSegment) -> Ordering {
  right.rank_score.partial_cmp(&left.rank_score).unwrap_or(Ordering::Equal)
    .then_with(|| Channel::of(&left.hit.kind).tie_rank().cmp(&Channel::of(&right.hit.kind).tie_rank()))
}

pub fn rank_segments(hits: Vec<SegmentHit>, limit: usize) -> Vec<SegmentHit> {
  if hits.is_empty() || limit == 0 {
    return Vec::new();
  }
  let scores = reciprocal_rank_scores(&hits);
  let mut grouped = BTreeMap::new();
  for hit in hits {
    let rank_score = scores.get(&hit.segment_id).copied().expect("each segment has a channel rank");
    let segments: &mut Vec<RankedSegment> = grouped.entry(hit.asset_id.clone()).or_default();
    segments.push(RankedSegment { hit, rank_score });
  }
  let mut assets = grouped.into_values().collect::<Vec<Vec<_>>>();
  for segments in &mut assets {
    segments.sort_by(|left, right| compare_fused(left, right).then(left.hit.segment_id.cmp(&right.hit.segment_id)));
    segments.truncate(MAX_SEGMENTS_PER_ASSET);
  }
  assets.sort_by(|left, right| compare_fused(&left[0], &right[0]).then(left[0].hit.asset_id.cmp(&right[0].hit.asset_id)));
  assets.truncate(limit);
  assets.into_iter().flatten().map(|ranked| ranked.hit).collect()
}

#[cfg(test)]
mod tests {
  use super::*;

  fn hit(asset_id: &str, segment_id: i64, kind: &str, score: f32) -> SegmentHit {
    SegmentHit {
      segment_id,
      asset_id: asset_id.into(),
      start_ms: segment_id * 1000,
      end_ms: segment_id * 1000 + 500,
      start_frame: None,
      end_frame: None,
      score,
      kind: kind.into(),
      certainty: None,
      text: format!("{kind}-{segment_id}"),
    }
  }

  fn asset_order(hits: &[SegmentHit]) -> Vec<&str> {
    hits.iter().fold(Vec::new(), |mut order, hit| {
      if !order.contains(&hit.asset_id.as_str()) {
        order.push(&hit.asset_id);
      }
      order
    })
  }

  #[test]
  fn single_channel_preserves_similarity_order() {
    let ranked = rank_segments(vec![
      hit("a1", 1, "visual", 0.2),
      hit("a2", 2, "visual", 0.4),
      hit("a1", 3, "visual", 0.3),
    ], 10);
    assert_eq!(asset_order(&ranked), ["a2", "a1"]);
    assert_eq!(ranked.iter().map(|hit| hit.segment_id).collect::<Vec<_>>(), [2, 3, 1]);
  }

  #[test]
  fn raw_scores_do_not_leak_across_channels() {
    let ranked = rank_segments(vec![
      hit("visual-target", 1, "visual", 0.2),
      hit("text-a", 2, "event", 0.99),
      hit("text-b", 3, "description", 0.98),
    ], 10);
    assert_eq!(asset_order(&ranked), ["visual-target", "text-a", "text-b"]);
  }

  #[test]
  fn higher_text_rank_can_beat_lower_visual_rank() {
    let ranked = rank_segments(vec![
      hit("visual-a", 1, "visual", 0.4),
      hit("visual-b", 2, "visual", 0.3),
      hit("text-target", 3, "event", 0.9),
    ], 10);
    assert_eq!(asset_order(&ranked), ["visual-a", "text-target", "visual-b"]);
  }

  #[test]
  fn best_segment_uses_fused_rank_without_rewriting_raw_score() {
    let ranked = rank_segments(vec![
      hit("a1", 1, "event", 0.9),
      hit("a1", 2, "visual", 0.2),
    ], 10);
    assert_eq!(ranked[0].kind, "visual");
    assert_eq!(ranked[0].score, 0.2);
    assert_eq!(ranked[1].kind, "event");
    assert_eq!(ranked[1].score, 0.9);
  }

  #[test]
  fn limits_apply_after_fused_ranking() {
    let hits = (0..10).map(|index| hit("a1", index, "visual", 1.0 - index as f32 * 0.01)).collect();
    let ranked = rank_segments(hits, 1);
    assert_eq!(ranked.len(), MAX_SEGMENTS_PER_ASSET);
    assert_eq!(ranked[0].segment_id, 0);
    assert!(rank_segments(vec![hit("a1", 1, "visual", 1.0)], 0).is_empty());
    assert!(rank_segments(Vec::new(), 10).is_empty());
  }
}
