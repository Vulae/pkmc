/// Usually a wrapper around std::mem::transmute for cleaner code.
pub trait Transmutable<T> {
    fn transmute(self) -> T;
}

#[macro_export]
macro_rules! primitive_int_int_transmutable {
    ($a:ty, $b:ty) => {
        impl Transmutable<$b> for $a {
            fn transmute(self) -> $b {
                unsafe { std::mem::transmute::<$a, $b>(self) }
            }
        }

        impl Transmutable<$a> for $b {
            fn transmute(self) -> $a {
                unsafe { std::mem::transmute::<$b, $a>(self) }
            }
        }
    };
}

#[macro_export]
macro_rules! primitive_int_float_transmutable {
    ($int:ty, $int_uint:ty, $float:ty) => {
        impl Transmutable<$float> for $int {
            fn transmute(self) -> $float {
                #[allow(clippy::useless_transmute)]
                <$float>::from_bits(unsafe { std::mem::transmute::<$int, $int_uint>(self) })
            }
        }

        impl Transmutable<$int> for $float {
            fn transmute(self) -> $int {
                #[allow(clippy::useless_transmute)]
                unsafe {
                    std::mem::transmute::<$int_uint, $int>(self.to_bits())
                }
            }
        }
    };
}

primitive_int_int_transmutable!(u8, i8);
primitive_int_int_transmutable!(u16, i16);
primitive_int_int_transmutable!(u32, i32);
primitive_int_float_transmutable!(u32, u32, f32);
primitive_int_float_transmutable!(i32, u32, f32);
primitive_int_int_transmutable!(u64, i64);
primitive_int_float_transmutable!(u64, u64, f64);
primitive_int_float_transmutable!(i64, u64, f64);

impl<I: Transmutable<O>, O> Transmutable<Box<[O]>> for Box<[I]> {
    fn transmute(self) -> Box<[O]> {
        unsafe { std::mem::transmute(self) }
    }
}

impl<'a, I: Transmutable<O>, O> Transmutable<&'a [O]> for &'a [I] {
    fn transmute(self) -> &'a [O] {
        unsafe { std::mem::transmute(self) }
    }
}

impl<'a, I: Transmutable<O>, O> Transmutable<&'a mut [O]> for &'a mut [I] {
    fn transmute(self) -> &'a mut [O] {
        unsafe { std::mem::transmute(self) }
    }
}
