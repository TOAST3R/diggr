//! The director: ordered rules (`director.ron`) that turn section boundaries, energy changes,
//! track changes and idle time into scene transitions, with per-track memory of which look
//! each section label got.

use std::collections::{HashMap, HashSet};

use analysis::{SectionKind, SongScore};
use serde::{Deserialize, Serialize};

use crate::compositor::TransitionKind;
use crate::modulation::{Macro, Source};
use crate::signals::Signals;
use crate::variants::Rng;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Event {
    /// The new section is at least this many dB louder than the last.
    Rise(f32),
    /// The new section is at least this many dB quieter than the last.
    Fall(f32),
    /// Any section boundary.
    Change,
    /// The new section's label was already seen in this track.
    Repeat,
    /// The new section has this (heuristic) kind.
    Kind(SectionKind),
    TrackChange,
    /// This many bars without a transition (fires on a downbeat).
    Idle(f32),
    /// Every frame (for continuous `set_macro`).
    Always,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Pick {
    Tag(String),
    SameAsLabel,
    HighestRatedUnused,
    RandomWeighted,
    /// Another variant of the current scene.
    Sibling,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Value {
    Const(f32),
    Signal(Source),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Action {
    Cut(Pick),
    /// Pick and length in bars.
    Crossfade(Pick, f32),
    Morph(Pick, f32),
    /// Macro, value, glide in beats.
    SetMacro(Macro, Value, #[serde(default)] f32),
    /// A white flash decaying over this many beats.
    Flash(f32),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Rule {
    pub on: Event,
    #[serde(rename = "do")]
    pub actions: Vec<Action>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Rules {
    pub rules: Vec<Rule>,
}

impl Rules {
    pub fn parse(text: &str) -> Result<Self, String> {
        ron::from_str(text).map_err(|e| format!("line {}: {}", e.span.start.line, e.code))
    }

    /// `set_macro` actions of `always` rules, applied every frame.
    pub fn continuous(&self) -> impl Iterator<Item = (Macro, Value, f32)> + '_ {
        self.rules
            .iter()
            .filter(|r| r.on == Event::Always)
            .flat_map(|r| &r.actions)
            .filter_map(|a| match a {
                Action::SetMacro(m, v, g) => Some((*m, *v, *g)),
                _ => None,
            })
    }
}

/// The bundled default show.
pub const DEFAULT_RULES: &str = include_str!("../assets/director.ron");

/// A look the director can pick: a variant of a scene.
#[derive(Debug, Clone, PartialEq)]
pub struct LookInfo {
    pub scene: String,
    pub variant: String,
    /// Scene tags plus variant tags.
    pub tags: Vec<String>,
    pub rating: u8,
}

/// What the engine should do now.
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    Transition {
        kind: TransitionKind,
        look: usize,
        beats: f64,
    },
    SetMacro {
        which: Macro,
        value: Value,
        glide_beats: f32,
    },
    Flash {
        beats: f32,
    },
}

/// Beats a provisional boundary must stay put before it is trusted.
pub const STABLE_BEATS: f64 = 8.0;

/// Per-track show state.
pub struct Director {
    pub rules: Rules,
    track: Option<u64>,
    seed: u64,
    rng: Rng,
    section: Option<usize>,
    /// Section start beat → musical time when first seen at that position.
    first_seen: HashMap<usize, f64>,
    /// Section label → look index.
    labels: HashMap<u8, usize>,
    used: HashSet<usize>,
    last_transition: f64,
}

fn section_energy(score: &SongScore, s: &analysis::score::Section) -> f32 {
    let e = &score.curves.energy;
    let (a, b) = (s.start_beat.min(e.len()), s.end_beat.min(e.len()));
    if b <= a {
        return f32::NEG_INFINITY;
    }
    e[a..b].iter().sum::<f32>() / (b - a) as f32
}

impl Director {
    pub fn new(rules: Rules) -> Self {
        Self {
            rules,
            track: None,
            seed: 0,
            rng: Rng(0),
            section: None,
            first_seen: HashMap::new(),
            labels: HashMap::new(),
            used: HashSet::new(),
            last_transition: 0.0,
        }
    }

    /// Advances one frame. `track` identifies the audible track; `seed` makes its show
    /// deterministic (it may change once analysis provides the content hash). `current` is the
    /// look on screen; `looks` everything available.
    pub fn update(
        &mut self,
        s: &Signals,
        score: Option<&SongScore>,
        track: u64,
        seed: u64,
        current: usize,
        looks: &[LookInfo],
    ) -> Vec<Command> {
        let mut out = Vec::new();
        if self.seed != seed {
            self.seed = seed;
            self.rng = Rng(seed);
        }
        if self.track != Some(track) {
            let first = self.track.is_none();
            self.track = Some(track);
            self.section = s.section_index;
            self.first_seen.clear();
            self.labels.clear();
            self.used.clear();
            self.used.insert(current);
            self.last_transition = s.beats;
            if !first {
                self.fire(
                    |e| *e == Event::TrackChange,
                    s,
                    current,
                    looks,
                    None,
                    &mut out,
                );
            }
        }
        let Some(score) = score else { return out };
        // Remember when each upcoming boundary was first seen where it is now.
        let now = s.beats;
        let start = s.section_index.map_or(0, |i| i + 1);
        for sec in score.sections.iter().skip(start) {
            self.first_seen.entry(sec.start_beat).or_insert(now);
        }
        if s.section_index != self.section {
            let prev = self.section;
            self.section = s.section_index;
            if let (Some(i), Some(j)) = (prev, s.section_index)
                && let (Some(a), Some(b)) = (score.sections.get(i), score.sections.get(j))
            {
                let trusted = b.is_final
                    || self
                        .first_seen
                        .get(&b.start_beat)
                        .is_some_and(|&t| now - t >= STABLE_BEATS);
                if trusted {
                    let rise = section_energy(score, b) - section_energy(score, a);
                    let repeat = self.labels.contains_key(&b.label);
                    let matches = |e: &Event| match e {
                        Event::Rise(db) => rise >= *db,
                        Event::Fall(db) => -rise >= *db,
                        Event::Change => true,
                        Event::Repeat => repeat,
                        Event::Kind(k) => b.kind == *k,
                        _ => false,
                    };
                    self.fire(matches, s, current, looks, Some(b.label), &mut out);
                }
            }
        } else if s.triggers.downbeat.fired() {
            let since = now - self.last_transition;
            let idle = move |e: &Event| matches!(e, Event::Idle(bars) if since >= *bars as f64 * 4.0 - 1e-6);
            if self.rules.rules.iter().any(|r| idle(&r.on)) {
                self.fire(
                    idle,
                    s,
                    current,
                    looks,
                    s.section_index
                        .and_then(|i| score.sections.get(i))
                        .map(|x| x.label),
                    &mut out,
                );
            }
        }
        out
    }

    fn fire(
        &mut self,
        matches: impl Fn(&Event) -> bool,
        s: &Signals,
        current: usize,
        looks: &[LookInfo],
        label: Option<u8>,
        out: &mut Vec<Command>,
    ) {
        let Some(rule) = self.rules.rules.iter().find(|r| matches(&r.on)).cloned() else {
            return;
        };
        self.last_transition = s.beats;
        for a in rule.actions {
            let (kind, pick, bars) = match a {
                Action::Cut(p) => (TransitionKind::Cut, p, 0.0),
                Action::Crossfade(p, bars) => (TransitionKind::Crossfade, p, bars),
                Action::Morph(p, bars) => (TransitionKind::Morph, p, bars),
                Action::SetMacro(which, value, glide_beats) => {
                    out.push(Command::SetMacro {
                        which,
                        value,
                        glide_beats,
                    });
                    continue;
                }
                Action::Flash(beats) => {
                    out.push(Command::Flash { beats });
                    continue;
                }
            };
            let Some(look) = self.pick(&pick, current, looks, label) else {
                continue;
            };
            if let Some(l) = label {
                self.labels.insert(l, look);
            }
            self.used.insert(look);
            // A morph needs the same scene; across scenes it becomes a crossfade.
            let kind = if kind == TransitionKind::Morph && looks[look].scene != looks[current].scene
            {
                TransitionKind::Crossfade
            } else {
                kind
            };
            if look != current || kind != TransitionKind::Cut {
                out.push(Command::Transition {
                    kind,
                    look,
                    beats: bars as f64 * 4.0,
                });
            }
        }
    }

    fn weighted(&mut self, candidates: &[usize], looks: &[LookInfo]) -> Option<usize> {
        let total: u32 = candidates.iter().map(|&i| 1 + looks[i].rating as u32).sum();
        if total == 0 {
            return None;
        }
        let mut r = (self.rng.next_u64() % total as u64) as u32;
        for &i in candidates {
            let w = 1 + looks[i].rating as u32;
            if r < w {
                return Some(i);
            }
            r -= w;
        }
        None
    }

    fn pick(
        &mut self,
        pick: &Pick,
        current: usize,
        looks: &[LookInfo],
        label: Option<u8>,
    ) -> Option<usize> {
        let others: Vec<usize> = (0..looks.len()).filter(|&i| i != current).collect();
        let unused: Vec<usize> = others
            .iter()
            .copied()
            .filter(|i| !self.used.contains(i))
            .collect();
        match pick {
            Pick::SameAsLabel => match label.and_then(|l| self.labels.get(&l).copied()) {
                Some(i) if i < looks.len() => Some(i),
                _ => self.pick(&Pick::HighestRatedUnused, current, looks, label),
            },
            Pick::HighestRatedUnused => {
                let pool = if unused.is_empty() { &others } else { &unused };
                let best = pool.iter().map(|&i| looks[i].rating).max()?;
                let top: Vec<usize> = pool
                    .iter()
                    .copied()
                    .filter(|&i| looks[i].rating == best)
                    .collect();
                Some(top[self.rng.below(top.len())])
            }
            Pick::RandomWeighted => self.weighted(&others, looks),
            Pick::Tag(t) => {
                let tagged: Vec<usize> = others
                    .iter()
                    .copied()
                    .filter(|&i| looks[i].tags.iter().any(|x| x == t))
                    .collect();
                let fresh: Vec<usize> = tagged
                    .iter()
                    .copied()
                    .filter(|i| !self.used.contains(i))
                    .collect();
                if tagged.is_empty() {
                    self.weighted(&others, looks)
                } else {
                    self.weighted(if fresh.is_empty() { &tagged } else { &fresh }, looks)
                }
            }
            Pick::Sibling => {
                let sib: Vec<usize> = others
                    .iter()
                    .copied()
                    .filter(|&i| looks[i].scene == looks[current].scene)
                    .collect();
                self.weighted(&sib, looks)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signals::Trigger;
    use analysis::score::Section;

    fn looks() -> Vec<LookInfo> {
        let l = |scene: &str, variant: &str, tags: &[&str], rating| LookInfo {
            scene: scene.into(),
            variant: variant.into(),
            tags: tags.iter().map(|t| t.to_string()).collect(),
            rating,
        };
        vec![
            l("julia", "a", &[], 3),
            l("julia", "b", &[], 1),
            l("flame", "a", &["calm"], 2),
            l("kifs", "a", &["3d"], 5),
            l("liquid", "a", &[], 4),
        ]
    }

    /// Sections of 32 beats with the given (label, energy dB) each; all final unless noted.
    fn score(sections: &[(u8, f32)]) -> SongScore {
        let n = sections.len() * 32;
        let mut s = SongScore {
            beats: (0..n).map(|i| i as f64 * 0.5).collect(),
            ..Default::default()
        };
        s.curves.energy = sections
            .iter()
            .flat_map(|&(_, e)| std::iter::repeat_n(e, 32))
            .collect();
        s.sections = sections
            .iter()
            .enumerate()
            .map(|(i, &(label, _))| Section {
                start_beat: i * 32,
                end_beat: i * 32 + 32,
                label,
                kind: SectionKind::Groove,
                is_final: true,
            })
            .collect();
        s
    }

    fn at(beats: f64, section: usize) -> Signals {
        Signals {
            beats,
            section_index: Some(section),
            playing: true,
            ..Default::default()
        }
    }

    fn default_director() -> Director {
        Director::new(Rules::parse(DEFAULT_RULES).unwrap())
    }

    #[test]
    fn default_rules_parse_and_follow_tension() {
        let d = default_director();
        let c: Vec<_> = d.rules.continuous().collect();
        assert_eq!(c, [(Macro::Stretch, Value::Signal(Source::Tension), 0.0)]);
    }

    #[test]
    fn rise_cuts_on_the_boundary_frame_with_a_flash() {
        let sc = score(&[(0, -20.0), (1, -12.0)]);
        let mut d = default_director();
        let lk = looks();
        assert!(d.update(&at(0.0, 0), Some(&sc), 1, 1, 0, &lk).is_empty());
        assert!(
            d.update(&at(31.9, 0), Some(&sc), 1, 1, 0, &lk).is_empty(),
            "nothing before the boundary"
        );
        let cmds = d.update(&at(32.0, 1), Some(&sc), 1, 1, 0, &lk);
        assert_eq!(
            cmds[0],
            Command::Transition {
                kind: TransitionKind::Cut,
                look: 3,
                beats: 0.0
            },
            "highest rated unused"
        );
        assert_eq!(cmds[1], Command::Flash { beats: 1.0 });
    }

    #[test]
    fn fall_crossfades_to_calm_over_a_bar_and_edits_apply() {
        let sc = score(&[(0, -10.0), (1, -18.0)]);
        let mut d = default_director();
        let lk = looks();
        d.update(&at(0.0, 0), Some(&sc), 1, 1, 0, &lk);
        let cmds = d.update(&at(32.0, 1), Some(&sc), 1, 1, 0, &lk);
        assert_eq!(
            cmds,
            [Command::Transition {
                kind: TransitionKind::Crossfade,
                look: 2,
                beats: 4.0
            }]
        );
        // Spec: change the fall rule to 4 bars → the next quieter section uses 4 bars.
        let edited = DEFAULT_RULES.replace(
            "Crossfade(Tag(\"calm\"), 1.0)",
            "Crossfade(Tag(\"calm\"), 4.0)",
        );
        assert_ne!(edited, DEFAULT_RULES);
        let mut d = Director::new(Rules::parse(&edited).unwrap());
        d.update(&at(0.0, 0), Some(&sc), 1, 1, 0, &lk);
        let cmds = d.update(&at(32.0, 1), Some(&sc), 1, 1, 0, &lk);
        assert_eq!(
            cmds,
            [Command::Transition {
                kind: TransitionKind::Crossfade,
                look: 2,
                beats: 16.0
            }]
        );
    }

    #[test]
    fn repeated_label_returns_to_its_look() {
        // A → B → A → B at equal energy: plain changes morph to siblings, repeats return.
        let sc = score(&[(0, -12.0), (1, -12.0), (0, -12.0), (1, -12.0)]);
        let mut d = default_director();
        let lk = looks();
        d.update(&at(0.0, 0), Some(&sc), 1, 1, 0, &lk);
        let first_b = match d.update(&at(32.0, 1), Some(&sc), 1, 1, 0, &lk)[0] {
            Command::Transition { look, kind, .. } => {
                assert_eq!(kind, TransitionKind::Morph, "a change morphs to a sibling");
                look
            }
            ref c => panic!("{c:?}"),
        };
        assert_eq!(first_b, 1);
        // Show something else during the second A, then B returns.
        d.update(&at(64.0, 2), Some(&sc), 1, 1, 3, &lk);
        let cmds = d.update(&at(96.0, 3), Some(&sc), 1, 1, 3, &lk);
        assert!(
            matches!(cmds[0], Command::Transition { look, .. } if look == first_b),
            "{cmds:?}"
        );
    }

    #[test]
    fn provisional_boundaries_must_be_stable() {
        let mut sc = score(&[(0, -20.0), (1, -10.0)]);
        sc.sections[1].is_final = false;
        let mut d = default_director();
        let lk = looks();
        d.update(&at(0.0, 0), Some(&sc), 1, 1, 0, &lk);
        // Seen only 4 beats before it is reached: ignored.
        let mut late = default_director();
        late.update(&at(0.0, 0), Some(&score(&[(0, -20.0)])), 1, 1, 0, &lk);
        late.update(&at(28.0, 0), Some(&sc), 1, 1, 0, &lk);
        assert!(
            late.update(&at(32.0, 1), Some(&sc), 1, 1, 0, &lk)
                .is_empty(),
            "moved/new boundary: no premature cut"
        );
        // Seen 32 beats ahead: trusted.
        assert!(!d.update(&at(32.0, 1), Some(&sc), 1, 1, 0, &lk).is_empty());
    }

    #[test]
    fn idle_morphs_on_a_downbeat_and_track_change_crossfades() {
        let sc = score(&[(0, -12.0); 4].map(|x| x));
        let mut long = sc.clone();
        long.sections = vec![Section {
            start_beat: 0,
            end_beat: 128,
            label: 0,
            kind: SectionKind::Groove,
            is_final: true,
        }];
        let mut d = default_director();
        let lk = looks();
        d.update(&at(0.0, 0), Some(&long), 1, 1, 0, &lk);
        let mut s = at(63.0, 0);
        s.triggers.downbeat = Trigger { count: 1, ago: 0.0 };
        assert!(
            d.update(&s, Some(&long), 1, 1, 0, &lk).is_empty(),
            "not yet 16 bars"
        );
        s.beats = 64.0;
        let cmds = d.update(&s, Some(&long), 1, 1, 0, &lk);
        assert_eq!(
            cmds,
            [Command::Transition {
                kind: TransitionKind::Morph,
                look: 1,
                beats: 16.0
            }]
        );
        let cmds = d.update(&at(0.0, 0), None, 2, 2, 1, &lk);
        assert!(
            matches!(
                cmds[0],
                Command::Transition {
                    kind: TransitionKind::Crossfade,
                    beats: 8.0,
                    ..
                }
            ),
            "{cmds:?}"
        );
    }

    #[test]
    fn same_seed_same_show() {
        let sc = score(&[(0, -20.0), (1, -12.0), (2, -20.0), (3, -12.0)]);
        let run = |seed| {
            let mut d = default_director();
            let lk = looks();
            let mut cur = 0;
            let mut shown = Vec::new();
            d.update(&at(0.0, 0), Some(&sc), seed, seed, cur, &lk);
            for i in 1..4 {
                for c in d.update(&at(i as f64 * 32.0, i), Some(&sc), seed, seed, cur, &lk) {
                    if let Command::Transition { look, .. } = c {
                        cur = look;
                        shown.push(look);
                    }
                }
            }
            shown
        };
        assert_eq!(run(5), run(5));
    }
}
