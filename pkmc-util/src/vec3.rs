use num_traits::{Float, FloatConst};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec3<T> {
    pub x: T,
    pub y: T,
    pub z: T,
}

impl<T> Vec3<T> {
    pub const fn new(x: T, y: T, z: T) -> Self {
        Self { x, y, z }
    }

    pub fn set(&mut self, x: T, y: T, z: T) {
        self.x = x;
        self.y = y;
        self.z = z;
    }
}

macro_rules! impl_vec3_vec3_basic_operation {
    ($trait_name:ident, $fn_name:ident, $assign_trait_name:ident, $assign_fn_name:ident, $oper:tt) => {
        impl<T: Float> std::ops::$trait_name for Vec3<T> {
            type Output = Self;
            fn $fn_name(self, rhs: Self) -> Self::Output {
                Self::new(self.x $oper rhs.x, self.y $oper rhs.y, self.z $oper rhs.z)
            }
        }

        impl<T: Float> std::ops::$assign_trait_name for Vec3<T> {
            fn $assign_fn_name(&mut self, rhs: Self) {
                *self = Self::new(self.x $oper rhs.x, self.y $oper rhs.y, self.z $oper rhs.z)
            }
        }
    };
}

impl_vec3_vec3_basic_operation!(Add, add, AddAssign, add_assign, +);
impl_vec3_vec3_basic_operation!(Sub, sub, SubAssign, sub_assign, -);
impl_vec3_vec3_basic_operation!(Mul, mul, MulAssign, mul_assign, *);
impl_vec3_vec3_basic_operation!(Div, div, DivAssign, div_assign, /);

macro_rules! impl_vec3_f64_basic_operation {
    ($trait_name:ident, $fn_name:ident, $oper:tt) => {
        impl<T: Float> std::ops::$trait_name<T> for Vec3<T> {
            type Output = Self;
            fn $fn_name(self, rhs: T) -> Self::Output {
                Self::new(self.x $oper rhs, self.y $oper rhs, self.z $oper rhs)
            }
        }
    };
}

impl_vec3_f64_basic_operation!(Mul, mul, *);
impl_vec3_f64_basic_operation!(Div, div, /);

impl<T: Float> std::ops::Neg for Vec3<T> {
    type Output = Self;
    fn neg(self) -> Self::Output {
        Self::new(-self.x, -self.y, -self.z)
    }
}

impl<T: Float> Vec3<T> {
    pub fn zero() -> Self {
        Self::new(T::zero(), T::zero(), T::zero())
    }

    pub fn min(&self) -> T {
        T::min(T::min(self.x, self.y), self.z)
    }

    pub fn max(&self) -> T {
        T::max(T::max(self.x, self.y), self.z)
    }

    pub fn length(&self) -> T {
        (self.x.powi(2) + self.y.powi(2) + self.z.powi(2)).sqrt()
    }

    pub fn distance(&self, other: &Self) -> T {
        (*other - *self).length()
    }

    pub fn normalized(&self) -> Self {
        match self.length() {
            length if length <= T::epsilon() => Self::zero(),
            length => *self / length,
        }
    }

    pub fn dot(&self, other: &Self) -> T {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    pub fn cross(&self, other: &Self) -> Self {
        Self::new(
            self.y * other.z - self.z * other.y,
            self.z * other.x - self.x * other.z,
            self.x * other.y - self.y * other.x,
        )
    }

    pub fn is_zero(&self) -> bool {
        self.x.is_zero() && self.y.is_zero() && self.z.is_zero()
    }
}

impl<T: Float + FloatConst> Vec3<T> {
    pub fn get_vector_for_rotation(pitch: T, yaw: T) -> Self {
        let f0 = T::cos((-yaw).to_radians() - T::PI());
        let f1 = T::sin((-yaw).to_radians() - T::PI());
        let f2 = -T::cos((-pitch).to_radians());
        let f3 = T::sin((-pitch).to_radians());
        Self::new(f1 * f2, f3, f0 * f2)
    }
}
