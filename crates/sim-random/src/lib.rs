#![forbid(unsafe_code)]

use sim_types::{SubsystemId, Tick};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RootSeed(pub [u8; 32]);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RandomKey {
    pub subsystem: SubsystemId,
    pub tick: Tick,
    pub entity: u64,
    pub event_kind: u32,
    pub draw_index: u32,
}

impl RandomKey {
    pub fn canonical_fingerprint(self, seed: RootSeed) -> [u8; 32] {
        let mut hasher = blake3::Hasher::new_keyed(&seed.0);
        hasher.update(b"new-engine.random-key.v0");
        hasher.update(&(self.subsystem as u16).to_le_bytes());
        hasher.update(&self.tick.0.to_le_bytes());
        hasher.update(&self.entity.to_le_bytes());
        hasher.update(&self.event_kind.to_le_bytes());
        hasher.update(&self.draw_index.to_le_bytes());
        *hasher.finalize().as_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_key_has_same_fingerprint() {
        let seed = RootSeed([7; 32]);
        let key = RandomKey {
            subsystem: SubsystemId::Energy,
            tick: Tick(12),
            entity: 4,
            event_kind: 2,
            draw_index: 0,
        };

        assert_eq!(
            key.canonical_fingerprint(seed),
            key.canonical_fingerprint(seed)
        );
    }
}
