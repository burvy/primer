//! Chapter content for the server.
//!
//! STUB. Slice 12 replaces this with a loader that reads the human-written
//! content files. Until then every chapter gets one lesson and one set, so the
//! whole ladder can be played. The rest of the server only calls [`chapters`].

use primer_core::policy::{Chapter, Step};
use primer_core::{Look, SKILLS, generate_set};
use rand::SeedableRng;
use rand::rngs::StdRng;

/// One chapter per skill: a lesson called `intro`, then a stand-in "written"
/// set. The set is built from a fixed seed, so it is the same every run.
pub fn chapters() -> Vec<Chapter> {
    SKILLS
        .iter()
        .map(|skill| {
            let mut set = generate_set(skill, Look::Gentle, &mut StdRng::seed_from_u64(1));
            set.id = format!("{}:stub", skill.id);
            Chapter {
                skill: skill.id,
                steps: vec![Step::Lesson("intro".into()), Step::Set(set)],
            }
        })
        .collect()
}
