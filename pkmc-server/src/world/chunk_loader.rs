use std::collections::HashSet;

use pkmc_util::IterRetain as _;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct ChunkPosition {
    pub chunk_x: i32,
    pub chunk_z: i32,
}

impl ChunkPosition {
    pub fn new(chunk_x: i32, chunk_z: i32) -> Self {
        Self { chunk_x, chunk_z }
    }

    pub fn distance(&self, other: &ChunkPosition) -> f32 {
        let dx = (other.chunk_x - self.chunk_x) as f32;
        let dz = (other.chunk_z - self.chunk_z) as f32;
        (dx * dx + dz * dz).sqrt()
    }
}

#[derive(Debug)]
pub struct ChunkLoader {
    center: Option<ChunkPosition>,
    pub radius: i32,
    to_load: HashSet<ChunkPosition>,
    loaded: HashSet<ChunkPosition>,
    to_unload: Vec<ChunkPosition>,
}

// For some reason needed?
const EXTRA_RADIUS: i32 = 4;

impl ChunkLoader {
    pub fn new(radius: i32) -> Self {
        Self {
            center: None,
            radius,
            to_load: HashSet::new(),
            loaded: HashSet::new(),
            to_unload: Vec::new(),
        }
    }

    fn iter_radius(&self) -> impl Iterator<Item = ChunkPosition> {
        let center = self.center.unwrap();
        let radius = self.radius + EXTRA_RADIUS;
        (-radius..=radius)
            .flat_map(move |dx| (-radius..=radius).map(move |dz| (dx, dz)))
            .map(move |(dx, dz)| ChunkPosition {
                chunk_x: center.chunk_x + dx,
                chunk_z: center.chunk_z + dz,
            })
            .filter(move |chunk| center.distance(chunk) < radius as f32)
    }

    /// Returns if updated center is new.
    pub fn update_center(&mut self, center: Option<ChunkPosition>) -> bool {
        if center == self.center {
            return false;
        }
        self.center = center;

        let Some(center) = center else {
            self.to_load.clear();
            self.to_unload.append(&mut self.loaded.drain().collect());
            return true;
        };

        self.to_load
            .retain(|chunk| center.distance(chunk) < (self.radius + EXTRA_RADIUS) as f32);
        self.to_unload.append(
            &mut self.loaded.retain_returned(|chunk| {
                center.distance(chunk) < (self.radius + EXTRA_RADIUS) as f32
            }),
        );
        self.iter_radius().for_each(|chunk| {
            if self.to_load.contains(&chunk) || self.loaded.contains(&chunk) {
                return;
            }
            self.to_load.insert(chunk);
        });

        true
    }

    pub fn next_to_load(&mut self) -> Option<ChunkPosition> {
        if let Some(closest) =
            self.to_load
                .iter()
                .fold(None, |closest: Option<ChunkPosition>, chunk| {
                    if let Some(closest) = closest {
                        if let Some(center) = self.center {
                            if closest.distance(&center) < chunk.distance(&center) {
                                Some(closest)
                            } else {
                                Some(*chunk)
                            }
                        } else {
                            None
                        }
                    } else {
                        Some(*chunk)
                    }
                })
        {
            self.to_load.remove(&closest);
            self.loaded.insert(closest);
            Some(closest)
        } else if let Some(next) = self.to_load.iter().next().cloned() {
            self.to_load.remove(&next);
            self.loaded.insert(next);
            Some(next)
        } else {
            None
        }
    }

    pub fn next_to_unload(&mut self) -> Option<ChunkPosition> {
        self.to_unload.pop()
    }
}
