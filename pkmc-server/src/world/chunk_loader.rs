use std::collections::HashSet;

use itertools::Itertools;
use pkmc_util::retain_returned_hashset;

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
    radius: i32,
    to_load: HashSet<ChunkPosition>,
    loaded: HashSet<ChunkPosition>,
    to_unload: Vec<ChunkPosition>,
}

// For some reason needed?
const EXTRA_RADIUS: i32 = 4;

fn iter_radius(center: ChunkPosition, radius: i32) -> impl Iterator<Item = ChunkPosition> {
    (-radius..=radius)
        .flat_map(move |dx| (-radius..=radius).map(move |dz| (dx, dz)))
        .map(move |(dx, dz)| ChunkPosition {
            chunk_x: center.chunk_x + dx,
            chunk_z: center.chunk_z + dz,
        })
        .filter(move |chunk| center.distance(chunk) < radius as f32)
}

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

    fn force_update(&mut self) {
        let Some(center) = self.center else {
            self.to_load.clear();
            self.to_unload.append(&mut self.loaded.drain().collect());
            return;
        };

        self.to_load
            .retain(|chunk| center.distance(chunk) < (self.radius + EXTRA_RADIUS) as f32);
        self.to_unload
            .append(&mut retain_returned_hashset(&mut self.loaded, |chunk| {
                center.distance(chunk) < (self.radius + EXTRA_RADIUS) as f32
            }));
        iter_radius(center, self.radius + EXTRA_RADIUS).for_each(|chunk| {
            if self.to_load.contains(&chunk) || self.loaded.contains(&chunk) {
                return;
            }
            self.to_load.insert(chunk);
        });
    }

    /// Returns if updated center is new.
    pub fn update_center(&mut self, center: Option<ChunkPosition>) -> bool {
        if center == self.center {
            return false;
        }
        self.center = center;
        self.force_update();
        true
    }

    pub fn update_radius(&mut self, radius: i32) {
        self.radius = radius;
        self.force_update();
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

    pub fn force_reload(&mut self, position: ChunkPosition) {
        self.to_unload.retain(|p| *p != position);
        if self.loaded.remove(&position) {
            self.to_load.insert(position);
        }
    }

    pub fn has_loaded(&self, position: ChunkPosition) -> bool {
        self.loaded.contains(&position) || self.to_unload.iter().contains(&position)
    }

    pub fn unload_all(&mut self) -> HashSet<ChunkPosition> {
        let unloaded = HashSet::from_iter(self.to_unload.drain(..).chain(self.loaded.drain()));
        self.to_load.drain();
        unloaded
    }
}
