use crate::{
    packet::{PacketDecodable, PacketEncodable},
    ReadExt as _, Transmutable, Vec3,
};

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
/// An in-world block position
pub struct Position {
    pub x: i32,
    pub y: i16,
    pub z: i32,
}

impl Position {
    pub const fn new(x: i32, y: i16, z: i32) -> Self {
        Self { x, y, z }
    }

    pub fn from_f64(x: f64, y: f64, z: f64) -> Option<Self> {
        let x = x.floor();
        let y = y.floor();
        let z = z.floor();
        if x < i32::MIN as f64 || x > i32::MAX as f64 {
            return None;
        }
        if y < i16::MIN as f64 || y > i16::MAX as f64 {
            return None;
        }
        if z < i32::MIN as f64 || z > i32::MAX as f64 {
            return None;
        }
        Some(Self::new(x as i32, y as i16, z as i32))
    }

    pub fn from_vec3(vec3: Vec3<f64>) -> Option<Self> {
        Self::from_f64(vec3.x, vec3.y, vec3.z)
    }

    pub fn checked_add(self, rhs: Position) -> Option<Position> {
        Some(Position::new(
            self.x.checked_add(rhs.x)?,
            self.y.checked_add(rhs.y)?,
            self.z.checked_add(rhs.z)?,
        ))
    }

    pub fn checked_sub(self, rhs: Position) -> Option<Position> {
        Some(Position::new(
            self.x.checked_sub(rhs.x)?,
            self.y.checked_sub(rhs.y)?,
            self.z.checked_sub(rhs.z)?,
        ))
    }

    pub fn length(&self) -> f32 {
        ((self.x as f32).powi(2) + (self.y as f32).powi(2) + (self.z as f32).powi(2)).sqrt()
    }
}

impl std::ops::Add for Position {
    type Output = Position;

    fn add(self, rhs: Self) -> Self::Output {
        Position::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl std::ops::AddAssign for Position {
    fn add_assign(&mut self, rhs: Self) {
        *self = Position::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z);
    }
}

impl std::ops::Sub for Position {
    type Output = Position;

    fn sub(self, rhs: Self) -> Self::Output {
        Position::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl std::ops::SubAssign for Position {
    fn sub_assign(&mut self, rhs: Self) {
        *self = Position::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z);
    }
}

impl Position {
    pub fn iter_cube(dx: i32, dy: i16, dz: i32) -> impl Iterator<Item = Position> {
        (0..dx).flat_map(move |x| {
            (0..dz).flat_map(move |z| (0..dy).map(move |y| Position::new(x, y, z)))
        })
    }

    pub fn iter_sphere(radius: f32) -> impl Iterator<Item = Position> {
        Position::iter_offset(
            Position::iter_cube(
                (radius.ceil() as i32) * 2,
                (radius.ceil() as i16) * 2,
                (radius.ceil() as i32) * 2,
            ),
            Position::new(
                -radius.round() as i32,
                -radius.round() as i16,
                -radius.round() as i32,
            ),
        )
        .filter(move |p| p.length() <= radius)
    }

    pub fn iter_ray(
        origin: Vec3<f64>,
        direction: Vec3<f64>,
        max_distance: f64,
    ) -> impl Iterator<Item = Position> {
        // This is pretty ugly, maybe like https://www.shadertoy.com/view/4dX3zl would be better?
        let normal = direction.normalized();
        let nx = normal.x;
        let ny = normal.y;
        let nz = normal.z;

        // Tracking 2 different positions, to fix floating point imprecision
        let mut pos = Position::from_vec3(origin).unwrap();
        let mut x = origin.x;
        let mut y = origin.y;
        let mut z = origin.z;

        let mut distance = 0.0;

        std::iter::from_fn(move || {
            if distance > max_distance {
                return None;
            }

            let current_pos = pos;

            let next_x = match nx {
                nx if nx > 0.0 => (pos.x as f64 + 1.0 - x) / nx,
                nx if nx < 0.0 => (pos.x as f64 - x) / nx,
                _ => f64::INFINITY,
            };
            let next_y = match ny {
                ny if ny > 0.0 => (pos.y as f64 + 1.0 - y) / ny,
                ny if ny < 0.0 => (pos.y as f64 - y) / ny,
                _ => f64::INFINITY,
            };
            let next_z = match nz {
                nz if nz > 0.0 => (pos.z as f64 + 1.0 - z) / nz,
                nz if nz < 0.0 => (pos.z as f64 - z) / nz,
                _ => f64::INFINITY,
            };

            let min_dist = f64::min(f64::min(next_x, next_y), next_z);

            x += nx * min_dist;
            y += ny * min_dist;
            z += nz * min_dist;
            distance += min_dist;

            if next_x <= next_y && next_x <= next_z {
                pos.x = pos.x.checked_add(if nx > 0.0 { 1 } else { -1 })?;
            } else if next_y <= next_x && next_y <= next_z {
                pos.y = pos.y.checked_add(if ny > 0.0 { 1 } else { -1 })?;
            } else {
                pos.z = pos.z.checked_add(if nz > 0.0 { 1 } else { -1 })?;
            }

            Some(current_pos)
        })
    }

    pub fn iter_offset(
        iter: impl Iterator<Item = Position>,
        offset: Position,
    ) -> impl Iterator<Item = Position> {
        iter.flat_map(move |p| p.checked_add(offset))
    }
}

impl PacketEncodable for &Position {
    fn packet_encode(self, mut writer: impl std::io::Write) -> std::io::Result<()> {
        let v: u64 = Transmutable::<u64>::transmute((self.x as i64) << 38)
            | (Transmutable::<u64>::transmute((self.y as i64) << 52) >> 52)
            | (Transmutable::<u64>::transmute((self.z as i64) << 38) >> 26);
        writer.write_all(&v.to_be_bytes())?;
        Ok(())
    }
}

impl PacketDecodable for Position {
    fn packet_decode(mut reader: impl std::io::Read) -> std::io::Result<Self> {
        let v = i64::from_be_bytes(reader.read_const()?);
        Ok(Position {
            x: (v >> 38) as i32,
            y: (v << 52 >> 52) as i16,
            z: (v << 26 >> 38) as i32,
        })
    }
}
